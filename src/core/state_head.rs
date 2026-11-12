//! HEAD: a PROJECTION of the journal — never a second authority.
//!
//! Owner's rule (2026-11-12): "Head ska vara en återskapningsbar projektion av
//! journalen, inte en andra oberoende auktoritet." This module therefore only
//! *computes* a head from the journal and writes it as a CACHE with a
//! watermark. Deleting `.gnawtreewriter_head.json` loses nothing: the next read
//! projects the same answer from the append-only journal.
//!
//! What it answers: *which content is applied to this file right now, and where
//! are those bytes verifiably stored?* The second half matters because a
//! content HASH is not storage: `content_source` names a v2 backup blob that
//! can be read back and re-hashed. When it cannot, the entry says so
//! (`journal:no-blob`) instead of implying recoverability.
//!
//! Deliberate non-couplings:
//! * `content_hash` is the JOURNAL's hash (`DefaultHasher`). It is NOT the
//!   index's md5 and must never be compared with it (owner's rule #5).
//! * nothing here reads or writes the Code Index; the journal works alone.

use crate::core::transaction_log::{OperationType, Transaction};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const HEAD_FILE: &str = ".gnawtreewriter_head.json";
const SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadEntry {
    /// Project-relative or absolute path exactly as the journal stores it.
    pub path: String,
    /// Journal hash of the content currently applied. `None` when the applied
    /// state is "file absent" (a reverted creation).
    pub content_hash: Option<String>,
    /// Where those bytes can be read back and verified, e.g.
    /// `blob:cli.rs_after_20261112_132555_542.json`, `journal:no-blob`.
    pub content_source: String,
    /// Applied transactions for this file, in journal order (undo walks back).
    pub applied: Vec<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Head {
    pub schema: u32,
    /// Journal rows the projection was computed from.
    pub watermark_entries: usize,
    /// Id of the newest row at projection time — catches a log that was
    /// truncated and re-appended to the same length.
    pub watermark_last_id: Option<String>,
    pub files: BTreeMap<String, HeadEntry>,
    /// Reactions the fold could not interpret. Declared, never repaired.
    pub unreadable_events: Vec<String>,
}

/// A reason the head and reality disagree. Reported, never auto-fixed.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Discrepancy {
    /// The head names content but no blob backs it.
    NoContentSource { path: String, source: String },
    /// Named blob exists but cannot be read/parsed.
    SourceUnreadable {
        path: String,
        source: String,
        error: String,
    },
    /// Named blob does not hash to the head's content_hash.
    SourceHashMismatch {
        path: String,
        source: String,
        expected: String,
        actual: String,
    },
    /// The working file no longer matches the head: something else wrote it.
    WorkingTreeMismatch {
        path: String,
        head: String,
        on_disk: String,
    },
    /// The working file is gone although the head says content is applied.
    FileMissing { path: String },
    /// The cached head on disk predates the journal. Reported as a fact: the
    /// projection used for this run ignored the cache entirely.
    CacheStale {
        cached_entries: usize,
        projected_entries: usize,
    },
}

/// Project a head from the journal. Pure; touches no files.
pub fn project(history: &[Transaction]) -> Head {
    let mut files: BTreeMap<String, HeadEntry> = BTreeMap::new();
    let mut unreadable: Vec<String> = Vec::new();

    for t in history {
        let key = t.file_path.to_string_lossy().to_string();
        let ts = t.timestamp.timestamp();

        match &t.operation {
            OperationType::Undo => match t.metadata.get("reverted_txn") {
                Some(reverted) => {
                    // Direction: `record_reaction` stores the undo event as
                    // before=original.after, after=original.before — so
                    // `after_hash` is the state the file moved TO.
                    if let Some(e) = files.get_mut(&key) {
                        e.applied.retain(|a| a != reverted);
                        e.content_hash = t.after_hash.clone();
                        e.updated_at = ts;
                    }
                }
                None => unreadable.push(t.id.clone()),
            },
            OperationType::Redo => match t.metadata.get("restored_txn") {
                Some(restored) => {
                    let e = files.entry(key.clone()).or_insert_with(|| HeadEntry {
                        path: key.clone(),
                        content_hash: None,
                        content_source: "journal:no-blob".to_string(),
                        applied: Vec::new(),
                        updated_at: ts,
                    });
                    if !e.applied.iter().any(|a| a == restored) {
                        e.applied.push(restored.clone());
                    }
                    e.content_hash = t.after_hash.clone();
                    e.updated_at = ts;
                }
                None => unreadable.push(t.id.clone()),
            },
            op if op.is_content_operation() => {
                let e = files.entry(key.clone()).or_insert_with(|| HeadEntry {
                    path: key.clone(),
                    content_hash: None,
                    content_source: "journal:no-blob".to_string(),
                    applied: Vec::new(),
                    updated_at: ts,
                });
                e.applied.push(t.id.clone());
                e.content_hash = t.after_hash.clone();
                e.content_source = match t.metadata.get("after_blob") {
                    Some(name) => format!("blob:{name}"),
                    None => "journal:no-blob".to_string(),
                };
                e.updated_at = ts;
            }
            _ => {}
        }
    }

    Head {
        schema: SCHEMA,
        watermark_entries: history.len(),
        watermark_last_id: history.last().map(|t| t.id.clone()),
        files,
        unreadable_events: unreadable,
    }
}

/// Project from the journal and compare with the cached file on disk.
///
/// Read-only: the cache is never healed here. `cache_stale` is a fact to report,
/// not a repair to perform — a silently rewritten cache would be a second
/// authority again.
pub fn load(project_root: &Path) -> Result<(Head, bool)> {
    let log = crate::core::transaction_log::TransactionLog::load(project_root)?;
    let history = log.get_full_history()?;
    let projected = project(&history);

    let path = project_root.join(HEAD_FILE);
    if !path.exists() {
        return Ok((projected, false));
    }
    let cached: Option<Head> = fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok());
    match cached {
        Some(c) => {
            let stale = c.watermark_entries != projected.watermark_entries
                || c.watermark_last_id != projected.watermark_last_id
                || c.files != projected.files;
            Ok((projected, stale))
        }
        // An unreadable cache is absent with extra steps: report it as stale so
        // the caller says something instead of passing silently.
        None => Ok((projected, true)),
    }
}

/// Write the projection as a cache: tmp file + rename (atomic on one FS).
pub fn save(head: &Head, project_root: &Path) -> Result<()> {
    let path = project_root.join(HEAD_FILE);
    let tmp = project_root.join(format!("{HEAD_FILE}.tmp"));
    fs::write(&tmp, serde_json::to_string_pretty(head)?)
        .with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, &path).with_context(|| format!("renaming into {}", path.display()))?;
    Ok(())
}

/// Verify every entry against the blobs and the working tree.
///
/// Answers the owner's #3 ("a content_hash does not guarantee the content is
/// stored") and #4 (the whole write chain): a head whose blob is missing, whose
/// blob does not hash back, or whose working file was changed by someone else
/// is reported — never silently corrected.
pub fn verify(head: &Head, project_root: &Path) -> Vec<Discrepancy> {
    let backup_dir: PathBuf = project_root.join(".gnawtreewriter_backups");
    let mut out = Vec::new();

    for (key, entry) in &head.files {
        let Some(expected) = entry.content_hash.as_ref() else {
            // "file absent" is a legitimate state (a reverted creation).
            continue;
        };
        if entry.content_source == "journal:no-blob" {
            out.push(Discrepancy::NoContentSource {
                path: key.clone(),
                source: entry.content_source.clone(),
            });
        } else if let Some(name) = entry.content_source.strip_prefix("blob:") {
            let blob = backup_dir.join(name);
            match crate::core::backup::parse_backup_file(&blob) {
                Ok(b) => {
                    let actual = b.content_hash.clone().unwrap_or_default();
                    if &actual != expected {
                        out.push(Discrepancy::SourceHashMismatch {
                            path: key.clone(),
                            source: entry.content_source.clone(),
                            expected: expected.clone(),
                            actual,
                        });
                    }
                }
                Err(e) => out.push(Discrepancy::SourceUnreadable {
                    path: key.clone(),
                    source: entry.content_source.clone(),
                    error: format!("{e:#}"),
                }),
            }
        }

        // The journal stores the path AS GIVEN (often relative to the project),
        // so resolve it against the project root before touching the disk —
        // otherwise a relative entry looks "missing" from any other cwd.
        let resolved = if Path::new(key).is_absolute() {
            PathBuf::from(key)
        } else {
            project_root.join(key)
        };

        // The working file is a SECOND witness, not the authority.
        match fs::read_to_string(&resolved) {
            Ok(text) => {
                let on_disk = crate::core::transaction_log::calculate_content_hash(&text);
                if &on_disk != expected {
                    out.push(Discrepancy::WorkingTreeMismatch {
                        path: key.clone(),
                        head: expected.clone(),
                        on_disk,
                    });
                }
            }
            Err(_) => out.push(Discrepancy::FileMissing { path: key.clone() }),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::transaction_log::{OperationType, Transaction};
    use chrono::Utc;
    use std::collections::HashMap;

    fn txn(id: &str, op: OperationType, file: &str, before: &str, after: &str) -> Transaction {
        Transaction {
            id: id.to_string(),
            timestamp: Utc::now(),
            operation: op,
            file_path: PathBuf::from(file),
            node_path: None,
            before_hash: Some(before.to_string()),
            after_hash: Some(after.to_string()),
            description: id.to_string(),
            session_id: "t".to_string(),
            metadata: HashMap::new(),
        }
    }

    fn meta(mut t: Transaction, k: &str, v: &str) -> Transaction {
        t.metadata.insert(k.to_string(), v.to_string());
        t
    }

    #[test]
    fn projection_follows_undo_and_redo_events() {
        let undo_evt = meta(
            txn("u1", OperationType::Undo, "a.rs", "h2", "h1"),
            "reverted_txn",
            "e2",
        );
        let redo_evt = meta(
            txn("r1", OperationType::Redo, "a.rs", "h1", "h2"),
            "restored_txn",
            "e2",
        );

        let history = vec![
            txn("e1", OperationType::Edit, "a.rs", "h0", "h1"),
            txn("e2", OperationType::Edit, "a.rs", "h1", "h2"),
            undo_evt,
            redo_evt,
        ];
        let head = project(&history);
        let e = head.files.get("a.rs").unwrap();
        assert_eq!(e.content_hash.as_deref(), Some("h2"), "redo re-applies h2");
        assert_eq!(e.applied, vec!["e1".to_string(), "e2".to_string()]);

        // Without the redo the newest applied state is h1 and e2 is gone.
        let head = project(&history[..3]);
        let e = head.files.get("a.rs").unwrap();
        assert_eq!(e.content_hash.as_deref(), Some("h1"));
        assert_eq!(e.applied, vec!["e1".to_string()]);
    }

    #[test]
    fn unreadable_reaction_is_reported_not_guessed() {
        let mut broken = txn("u1", OperationType::Undo, "a.rs", "h1", "h0");
        broken.metadata.clear(); // no reverted_txn
        let head = project(&[txn("e1", OperationType::Edit, "a.rs", "h0", "h1"), broken]);
        assert_eq!(head.unreadable_events, vec!["u1".to_string()]);
        // An unreadable event never REMOVES anything: the edit stays applied.
        assert_eq!(head.files["a.rs"].applied, vec!["e1".to_string()]);
    }

    #[test]
    fn content_source_is_named_per_entry() {
        let e = meta(
            txn("e1", OperationType::Edit, "a.rs", "h0", "h1"),
            "after_blob",
            "a.rs_after_1.json",
        );
        let head = project(&[e]);
        assert_eq!(head.files["a.rs"].content_source, "blob:a.rs_after_1.json");

        let plain = project(&[txn("e1", OperationType::Edit, "a.rs", "h0", "h1")]);
        assert_eq!(plain.files["a.rs"].content_source, "journal:no-blob");
    }

    #[test]
    fn verify_reports_missing_blob_and_external_change() {
        // Hermetic temp project: one entry names a blob that does not exist,
        // another file is later changed behind GTW's back. Both are FACTS.
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.rs");
        let changed = dir.path().join("changed.rs");
        std::fs::write(&missing, "fn a() {}\n").unwrap();
        std::fs::write(&changed, "fn b() {}\n").unwrap();

        let mut head = project(&[
            meta(
                txn(
                    "e1",
                    OperationType::Edit,
                    missing.to_str().unwrap(),
                    "h0",
                    "h1",
                ),
                "after_blob",
                "nope.json",
            ),
            txn(
                "e2",
                OperationType::Edit,
                changed.to_str().unwrap(),
                "h0",
                "h2",
            ),
        ]);
        // Make the `changed` claim match its bytes, so it is NOT reported yet.
        let text = std::fs::read_to_string(&changed).unwrap();
        head.files
            .get_mut(changed.to_str().unwrap())
            .unwrap()
            .content_hash = Some(crate::core::transaction_log::calculate_content_hash(&text));

        let d = verify(&head, dir.path());
        assert!(
            d.iter()
                .any(|x| matches!(x, Discrepancy::SourceUnreadable { .. })),
            "a named-but-absent blob must be reported: {d:?}"
        );
        assert!(
            d.iter()
                .any(|x| matches!(x, Discrepancy::NoContentSource { .. })),
            "journal:no-blob must be reported, never implied recoverable: {d:?}"
        );
        assert!(
            !d.iter().any(|x| matches!(
                x,
                Discrepancy::WorkingTreeMismatch { path, .. } if path.ends_with("changed.rs")
            )),
            "a file whose bytes match the head must not be flagged as changed: {d:?}"
        );

        // Now change the file behind GTW's back.
        std::fs::write(&changed, "fn b() { /* other tool */ }\n").unwrap();
        let d = verify(&head, dir.path());
        assert!(
            d.iter()
                .any(|x| matches!(x, Discrepancy::WorkingTreeMismatch { .. })),
            "an external write must be reported: {d:?}"
        );
    }

    #[test]
    fn cache_is_a_projection_and_deleting_it_loses_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join(".gnawtreewriter_session.json");
        let t = txn("e1", OperationType::Edit, "a.rs", "h0", "h1");
        std::fs::write(
            &log_path,
            format!("{}\n", serde_json::to_string(&t).unwrap()),
        )
        .unwrap();

        let (first, stale_without_cache) = load(dir.path()).unwrap();
        assert!(
            !stale_without_cache,
            "no cache yet means nothing to be stale against"
        );
        save(&first, dir.path()).unwrap();

        let (second, stale) = load(dir.path()).unwrap();
        assert!(!stale, "a fresh cache must not be stale");
        assert_eq!(first, second);

        // Delete the cache: the projection is unchanged — proof that the head
        // is derived, not an authority.
        std::fs::remove_file(dir.path().join(HEAD_FILE)).unwrap();
        let (third, _) = load(dir.path()).unwrap();
        assert_eq!(first.files, third.files);
        assert_eq!(first.watermark_entries, third.watermark_entries);
    }
}
