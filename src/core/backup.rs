/*
 * GnawTreeWriter - backup helpers
 *
 * Provides utilities to list, parse and restore JSON backup files created by
 * `GnawTreeWriter::create_backup()`. These helpers centralize backup-related
 * functionality so other modules (RestorationEngine, UndoRedoManager, tests)
 * can reuse a consistent implementation.
 */

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// Parsed metadata for a backup JSON file
#[derive(Debug, Clone)]
pub struct BackupFile {
    /// Full path to the backup JSON file
    pub path: PathBuf,
    /// Timestamp recorded in the backup file
    pub timestamp: DateTime<Utc>,
    /// Original source file path stored in the backup JSON
    pub original_file_path: PathBuf,
    /// md5 of `source_code` — the unified scheme, matching `files.hash`
    /// and every transaction written since (§8.9 step 1).
    pub content_hash: Option<String>,
    /// Pre-unification DefaultHasher of `source_code`, computed so hash
    /// lookups keep finding backups written before the md5 unification.
    /// `None` when only the name is known (v3 fast path, no content read).
    pub legacy_content_hash: Option<String>,
}

/// List all backup files found in `backup_dir`.
/// Only files with `.json` extension are considered. Parses each file and
/// returns a vector of `BackupFile` sorted by timestamp (newest first).
pub fn list_backup_files<P: AsRef<Path>>(backup_dir: P) -> Result<Vec<BackupFile>> {
    let backup_dir = backup_dir.as_ref();

    let mut backups = Vec::new();

    if !backup_dir.exists() {
        return Ok(backups);
    }

    let entries = fs::read_dir(backup_dir).context("Failed to read backup directory")?;

    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
            match parse_backup_file(&path) {
                Ok(b) => backups.push(b),
                Err(_) => {
                    // Skip files that fail to parse (non-critical for listing)
                    continue;
                }
            }
        }
    }

    // Sort by timestamp (newest first)
    backups.sort_by_key(|b| std::cmp::Reverse(b.timestamp));

    Ok(backups)
}

/// Parse a single backup JSON file and return a `BackupFile` with metadata
/// (timestamp, original_file_path, content_hash).
pub fn parse_backup_file<P: AsRef<Path>>(backup_path: P) -> Result<BackupFile> {
    let backup_path = backup_path.as_ref();

    let content = fs::read_to_string(backup_path).context(format!(
        "Failed to read backup file: {}",
        backup_path.display()
    ))?;

    let json: Value = serde_json::from_str(&content).context(format!(
        "Failed to parse backup JSON: {}",
        backup_path.display()
    ))?;

    // Extract file_path
    let original_file_path_str = json["file_path"]
        .as_str()
        .ok_or_else(|| anyhow!("Backup file missing 'file_path'"))?;

    // Extract timestamp
    let timestamp_str = json["timestamp"]
        .as_str()
        .ok_or_else(|| anyhow!("Backup file missing 'timestamp'"))?;
    let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
        .context("Failed to parse backup timestamp")?
        .with_timezone(&Utc);

    // Extract source_code and hash it under BOTH schemes over the same
    // bytes: `content_hash` is the unified md5, `legacy_content_hash`
    // keeps pre-unification lookups working (docs/HISTORY_STORAGE_DESIGN.md
    // §8.9 step 1 — "parse_backup_file räknar båda hasharna").
    let source_code = json["source_code"]
        .as_str()
        .ok_or_else(|| anyhow!("Backup file missing 'source_code'"))?;
    let content_hash = Some(crate::core::calculate_content_hash(source_code));
    let legacy_content_hash = Some(crate::core::legacy_content_hash(source_code));

    Ok(BackupFile {
        path: backup_path.to_path_buf(),
        timestamp,
        original_file_path: PathBuf::from(original_file_path_str),
        content_hash,
        legacy_content_hash,
    })
}

/// Find the first backup whose content hash equals `content_hash`.
/// Returns the parsed `BackupFile` if found.
pub fn find_backup_by_content_hash<P: AsRef<Path>>(
    backup_dir: P,
    content_hash: &str,
) -> Result<Option<BackupFile>> {
    // v3 names first: the hash is IN the name, zero content reads. Old
    // backups carry 16-hex (siphash) names, new ones 32-hex (md5); the
    // caller's key is compared against the name verbatim, so each
    // generation finds its own names fast.
    if let Ok(entries) = fs::read_dir(backup_dir.as_ref()) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some((_, hash)) = parse_backup_name(&name) {
                if hash == content_hash {
                    return Ok(Some(parse_backup_file(entry.path())?));
                }
            }
        }
    }
    // Content scan: a backup matches when EITHER scheme hashes to the key —
    // that is what lets an md5 caller find a pre-unification blob with the
    // same content, and a legacy key find a blob parsed today.
    let backups = list_backup_files(backup_dir)?;
    for b in backups {
        if b.content_hash.as_deref() == Some(content_hash)
            || b.legacy_content_hash.as_deref() == Some(content_hash)
        {
            return Ok(Some(b));
        }
    }
    Ok(None)
}

/// v3 backup names carry the content hash AND the mangled original path:
/// `<mangled>_backup_<hash>_<timestamp>.json`. Returns (mangled, hash) when
/// the name is v3, None for legacy names (`<basename>_backup_<ts>.json`),
/// which are still readable through the content scan.
///
/// The legacy name was a correctness AND performance defect (measured
/// 2026-11-19): two files named mod.rs edited within the same millisecond
/// wrote the SAME filename and the second silently overwrote the first, and
/// every lookup had to open and hash every backup in the directory (3m21s
/// for two undos at 1147 files).
pub(crate) fn parse_backup_name(name: &str) -> Option<(String, String)> {
    let stem = name.strip_suffix(".json")?;
    let idx = stem.find("_backup_")?;
    let mangled = stem[..idx].to_string();
    let rest = &stem[idx + "_backup_".len()..];
    let (hash, timestamp) = rest.split_once('_')?;
    // 16-hex = pre-unification DefaultHasher names; 32-hex = md5 (the
    // unified scheme, §8.9 step 1). Both accepted so lookups keep finding
    // backups across the scheme boundary.
    let is_valid_hash =
        (hash.len() == 16 || hash.len() == 32) && hash.chars().all(|c| c.is_ascii_hexdigit());
    if is_valid_hash && !timestamp.is_empty() {
        Some((mangled, hash.to_string()))
    } else {
        None
    }
}

/// Flattens a path into a filesystem-safe name segment. `src/core/mod.rs` and
/// `src/mcp/mod.rs` must NOT map to the same segment - that collision is the
/// defect this replaces (same basename = same legacy filename).
pub(crate) fn mangle_path_for_name(path: &str) -> String {
    let s = path.trim_start_matches("./");
    s.replace(['/', '\\', ':'], "__")
}

/// v3 timestamps are `%Y%m%dT%H%M%S%3f` (underscore-free on purpose: the name
/// parser splits on underscores).
fn name_timestamp(name: &str) -> Option<DateTime<Utc>> {
    let stem = name.strip_suffix(".json")?;
    let ts = stem.rsplit('_').next()?;
    DateTime::parse_from_str(ts, "%Y%m%dT%H%M%S%3f")
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

/// Fast v3 lookup: match hash + mangled path from the FILE NAME. The content
/// is not read here; the caller (restore) reads the one file it selected.
/// Legacy (pre-v3) names return None here and fall through to the scan.
fn find_v3_by_hash_and_path<P: AsRef<Path>>(
    backup_dir: P,
    content_hash: &str,
    original_file: &Path,
) -> Result<Option<BackupFile>> {
    let entries = match fs::read_dir(backup_dir.as_ref()) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };
    let mangled_target = mangle_path_for_name(&original_file.to_string_lossy());
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some((mangled, hash)) = parse_backup_name(&name) {
            if hash == content_hash && mangled == mangled_target {
                // The recorded file_path is the one the writer saw; the name is
                // derived from it, so reconstructing it here is lossless. The
                // legacy hash is unknown without a content read — the content
                // scan fills it when a caller needs it.
                return Ok(Some(BackupFile {
                    path: entry.path(),
                    timestamp: name_timestamp(&name).unwrap_or(Utc::now()),
                    original_file_path: original_file.to_path_buf(),
                    content_hash: Some(content_hash.to_string()),
                    legacy_content_hash: None,
                }));
            }
        }
    }
    Ok(None)
}

pub fn find_backup_by_content_hash_for_file<P: AsRef<Path>>(
    backup_dir: P,
    content_hash: &str,
    original_file: &Path,
) -> Result<Option<BackupFile>> {
    // Fast pass: name carries hash + mangled path, no content read.
    if let Some(b) = find_v3_by_hash_and_path(&backup_dir, content_hash, original_file)? {
        return Ok(Some(b));
    }
    // Legacy scan (pre-v3 names): a backup matches when EITHER scheme
    // hashes to the key (md5 or pre-unification siphash).
    let backups = list_backup_files(backup_dir)?;
    // Prefer exact file match
    for b in &backups {
        let scheme_matches = b.content_hash.as_deref() == Some(content_hash)
            || b.legacy_content_hash.as_deref() == Some(content_hash);
        if scheme_matches && b.original_file_path == original_file {
            return Ok(Some(b.clone()));
        }
    }
    // Fallback: any matching hash
    for b in backups {
        if b.content_hash.as_deref() == Some(content_hash)
            || b.legacy_content_hash.as_deref() == Some(content_hash)
        {
            return Ok(Some(b));
        }
    }
    Ok(None)
}

/// Write a v2 blob for ARBITRARY content and return its basename.
///
/// Used by every write path that needs a verifiable content source for the state
/// it CREATES (the pre-edit blob cannot cover the newest state — owner's rule
/// #3: a content hash is not storage). Lives here rather than on a writer so
/// `batch` can store blobs for files it writes directly, without re-parsing them
/// through `GnawTreeWriter::new`.
pub fn write_after_blob(project_root: &Path, file_path: &str, content: &str) -> Result<String> {
    // v3 (2026-11-19): same treatment as create_backup - the name carries the
    // mangled path and the content hash, so after-blobs of two files named
    // mod.rs cannot overwrite each other, and a future reader can find the
    // newest state by hash without scanning (design doc gap #1: "the newest
    // state must have a verifiable content source").
    let timestamp = Utc::now().format("%Y%m%dT%H%M%S%3f");
    let content_hash = crate::core::transaction_log::calculate_content_hash(content);
    let mangled = mangle_path_for_name(file_path);
    let blob_name = format!("{mangled}_after_{content_hash}_{timestamp}.json");

    let backup_dir = project_root.join(".gnawtreewriter_backups");
    fs::create_dir_all(&backup_dir)?;

    let body = serde_json::json!({
        "format": "gtw-backup-v3",
        "state": "after",
        "file_path": file_path,
        "content_hash": content_hash,
        "timestamp": Utc::now().to_rfc3339(),
        "source_code": content
    });
    fs::write(
        backup_dir.join(&blob_name),
        serde_json::to_string_pretty(&body)?,
    )
    .with_context(|| format!("writing after-blob {blob_name}"))?;
    Ok(blob_name)
}

/// Read the `source_code` field from a backup JSON and write it to `target_path`.
/// Returns the written `PathBuf` on success.
pub fn restore_from_backup<P: AsRef<Path>, Q: AsRef<Path>>(
    backup_path: P,
    target_path: Q,
) -> Result<PathBuf> {
    let backup_path = backup_path.as_ref();
    let target_path = target_path.as_ref();

    let backup_content = fs::read_to_string(backup_path).context(format!(
        "Failed to read backup file: {}",
        backup_path.display()
    ))?;

    let json: Value = serde_json::from_str(&backup_content).context(format!(
        "Failed to parse backup JSON: {}",
        backup_path.display()
    ))?;

    let source_code = json["source_code"]
        .as_str()
        .ok_or_else(|| anyhow!("Backup file missing 'source_code'"))?;

    fs::write(target_path, source_code).context(format!(
        "Failed to write restored file: {}",
        target_path.display()
    ))?;

    Ok(target_path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::GnawTreeWriter;
    use chrono::Utc;
    use tempfile::tempdir;

    #[test]
    fn test_parse_backup_file() -> Result<()> {
        let tmp = tempdir()?;
        let backup_dir = tmp.path().join(".gnawtreewriter_backups");
        fs::create_dir_all(&backup_dir)?;

        let file_path = tmp.path().join("a.txt");
        let source_code = "hello world";

        let backup = serde_json::json!({
            "file_path": file_path.to_string_lossy(),
            "timestamp": Utc::now().to_rfc3339(),
            "tree": {},
            "source_code": source_code
        });

        let backup_path = backup_dir.join("b1.json");
        fs::write(&backup_path, serde_json::to_string_pretty(&backup)?)?;

        let parsed = parse_backup_file(&backup_path)?;
        assert_eq!(parsed.original_file_path, file_path);
        assert!(parsed.content_hash.is_some());
        assert_eq!(
            parsed.content_hash.unwrap(),
            crate::core::calculate_content_hash(source_code)
        );

        Ok(())
    }

    #[test]
    fn test_list_and_find_by_hash_and_restore() -> Result<()> {
        let tmp = tempdir()?;
        let backup_dir = tmp.path().join(".gnawtreewriter_backups");
        fs::create_dir_all(&backup_dir)?;

        let file_path = tmp.path().join("f.txt");

        let backup1 = serde_json::json!({
            "file_path": file_path.to_string_lossy(),
            "timestamp": Utc::now().to_rfc3339(),
            "tree": {},
            "source_code": "v1"
        });
        let backup2 = serde_json::json!({
            "file_path": file_path.to_string_lossy(),
            "timestamp": Utc::now().to_rfc3339(),
            "tree": {},
            "source_code": "v2"
        });

        let p1 = backup_dir.join("one.json");
        let p2 = backup_dir.join("two.json");
        fs::write(&p1, serde_json::to_string_pretty(&backup1)?)?;
        fs::write(&p2, serde_json::to_string_pretty(&backup2)?)?;

        let all = list_backup_files(&backup_dir)?;
        assert!(all.len() >= 2);

        let h = crate::core::calculate_content_hash("v2");
        let found = find_backup_by_content_hash(&backup_dir, &h)?;
        assert!(found.is_some());
        let b = found.unwrap();
        assert_eq!(b.content_hash.unwrap(), h);

        // Test restoring into a target file
        let target = tmp.path().join("out.txt");
        restore_from_backup(&p2, &target)?;
        let content = fs::read_to_string(&target)?;
        assert_eq!(content, "v2");

        Ok(())
    }

    /// v3 name: the hash and the mangled ORIGINAL PATH are in the filename, so
    /// two files that share a basename (src/core/mod.rs vs src/mcp/mod.rs) can
    /// never overwrite each other, and the lookup does not open a single
    /// backup. Both defects measured 2026-11-19 (same-millisecond overwrite;
    /// 3m21s for two undos at 1147 backups).
    #[test]
    fn v3_lookup_distinguishes_same_basename_files_and_overwrites_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let backups = dir.path().join(".gnawtreewriter_backups");
        std::fs::create_dir_all(&backups).unwrap();
        // Hermeticity (AGENTS #6): without a root marker find_project_root
        // walks up and can land in the session dir or the repo — measured:
        // the backups then never appear under THIS tempdir.
        std::fs::write(dir.path().join(".git"), "").unwrap();

        // Two DIFFERENT files with the SAME basename, like core/mod.rs and
        // mcp/mod.rs. Legacy naming gave both the same filename.
        let a = dir.path().join("src/core/mod.rs");
        let b = dir.path().join("src/mcp/mod.rs");
        std::fs::create_dir_all(a.parent().unwrap()).unwrap();
        std::fs::create_dir_all(b.parent().unwrap()).unwrap();
        std::fs::write(&a, "mod a { fn one() {} }").unwrap();
        std::fs::write(&b, "mod b { fn two() {} }").unwrap();

        let hash_of = |p: &Path| {
            crate::core::transaction_log::calculate_content_hash(
                &std::fs::read_to_string(p).unwrap(),
            )
        };
        let hash_a = hash_of(&a);
        let hash_b = hash_of(&b);
        assert_ne!(hash_a, hash_b, "fixture must have distinct content");

        // Write both backups the way create_backup does (same millisecond is
        // the interesting case, so no sleep).
        let mut wa = GnawTreeWriter::new(a.to_str().unwrap()).unwrap();
        let pa = wa.create_backup().unwrap();
        let mut wb = GnawTreeWriter::new(b.to_str().unwrap()).unwrap();
        let pb = wb.create_backup().unwrap();

        // 1. No overwrite: two files on disk, two distinct names.
        assert_ne!(pa, pb, "same-basename backups must not share a filename");
        assert_eq!(std::fs::read_dir(&backups).unwrap().count(), 2);

        // 2. Name parses as v3 and carries the right hash for the right file.
        let (mangled_a, hash_in_name_a) =
            parse_backup_name(pa.file_name().unwrap().to_str().unwrap()).expect("v3 name");
        let (mangled_b, hash_in_name_b) =
            parse_backup_name(pb.file_name().unwrap().to_str().unwrap()).expect("v3 name");
        assert_eq!(hash_in_name_a, hash_a);
        assert_eq!(hash_in_name_b, hash_b);
        assert_ne!(mangled_a, mangled_b, "paths must stay distinguishable");
        // file_path is stored AS GIVEN (new_with_boundary stores the
        // caller's string), so the mangle is over that same string -
        // absolute in this test, relative for a relative caller. The
        // invariant is consistency: the name derives from the same string
        // the transaction log records as file_path/before_hash.
        assert_eq!(mangled_a, mangle_path_for_name(a.to_str().unwrap()));
        assert_eq!(mangled_b, mangle_path_for_name(b.to_str().unwrap()));

        // 3. The lookup returns each file OWN backup by hash + path.
        let for_a = find_backup_by_content_hash_for_file(&backups, &hash_a, &a)
            .expect("lookup ok")
            .expect("backup for a found");
        let for_b = find_backup_by_content_hash_for_file(&backups, &hash_b, &b)
            .expect("lookup ok")
            .expect("backup for b found");
        assert_eq!(for_a.path, pa);
        assert_eq!(for_b.path, pb);
        assert_eq!(for_a.original_file_path, a);
        assert_eq!(for_b.original_file_path, b);
    }

    /// Pre-v3 backups (no hash in the name) must stay findable through the
    /// content scan: the fast pass returns None and the legacy path runs.
    #[test]
    fn legacy_names_still_resolve_through_the_content_scan() {
        let dir = tempfile::tempdir().unwrap();
        let backups = dir.path().join(".gnawtreewriter_backups");
        std::fs::create_dir_all(&backups).unwrap();

        let source = "pub fn legacy_target() {}";
        let hash = crate::core::transaction_log::calculate_content_hash(source);
        let legacy = backups.join("oldname_backup_20261110T000000000.json");
        std::fs::write(
            &legacy,
            serde_json::json!({
                "format": "gtw-backup-v2",
                "file_path": "src/oldname.rs",
                "timestamp": "2026-11-10T00:00:00Z",
                "source_code": source
            })
            .to_string(),
        )
        .unwrap();

        // No v3 name matches, so the scan must still find it.
        let found = find_backup_by_content_hash(&backups, &hash)
            .expect("lookup ok")
            .expect("legacy backup must still resolve");
        assert_eq!(found.path, legacy);

        let for_file =
            find_backup_by_content_hash_for_file(&backups, &hash, Path::new("src/oldname.rs"))
                .expect("lookup ok")
                .expect("legacy backup must resolve for its file");
        assert_eq!(for_file.original_file_path, Path::new("src/oldname.rs"));
    }

    /// The fast pass is also a CORRECTNESS guard: a v3 backup of another file
    /// with the same hash must not be claimed by the wrong path.
    #[test]
    fn v3_fast_pass_never_claims_another_files_backup() {
        let dir = tempfile::tempdir().unwrap();
        let backups = dir.path().join(".gnawtreewriter_backups");
        std::fs::create_dir_all(&backups).unwrap();

        let source = "pub fn same_bytes_everywhere() {}";
        let hash = crate::core::transaction_log::calculate_content_hash(source);

        // Same content, two paths (e.g. a copy): the hashes match but the
        // mangled paths differ, so only path a may resolve.
        let a = Path::new("src/a/dup.rs");
        let b = Path::new("src/b/dup.rs");
        let name_a = format!(
            "{}_backup_{}_20261119T000000000.json",
            mangle_path_for_name(a.to_str().unwrap()),
            hash
        );
        let name_b = format!(
            "{}_backup_{}_20261119T000000001.json",
            mangle_path_for_name(b.to_str().unwrap()),
            hash
        );
        for n in [name_a, name_b] {
            std::fs::write(
                backups.join(n),
                serde_json::json!({
                    "format": "gtw-backup-v3",
                    "file_path": source,
                    "timestamp": "2026-11-19T00:00:00Z",
                    "source_code": source
                })
                .to_string(),
            )
            .unwrap();
        }

        let hit = find_v3_by_hash_and_path(&backups, &hash, a)
            .expect("lookup ok")
            .expect("path a must resolve");
        assert!(
            hit.path.to_string_lossy().contains("src__a__dup.rs"),
            "must resolve to a backup, got {}",
            hit.path.display()
        );

        // Path c (never backed up) must NOT be satisfied by a or b.
        let c = find_v3_by_hash_and_path(&backups, &hash, Path::new("src/c/dup.rs"))
            .expect("lookup ok");
        assert!(c.is_none(), "an unrelated path must not claim a backup");
    }

    /// §8.9 step 1: backup names may carry 16-hex (pre-unification siphash)
    /// OR 32-hex (md5) — both parse; anything else is not a hash name.
    #[test]
    fn backup_names_accept_legacy_and_md5_hashes() {
        let mangled = mangle_path_for_name("src/x.rs");
        let legacy = format!("{mangled}_backup_0123456789abcdef_20261119T000000000.json");
        let md5 =
            format!("{mangled}_backup_0123456789abcdef0123456789abcdef_20261119T000000001.json");
        let junk = format!("{mangled}_backup_zz_20261119T000000002.json");
        assert!(
            parse_backup_name(&legacy).is_some(),
            "16-hex (siphash) names must parse"
        );
        assert!(
            parse_backup_name(&md5).is_some(),
            "32-hex (md5) names must parse"
        );
        assert!(
            parse_backup_name(&junk).is_none(),
            "non-hex names must not parse"
        );
    }

    /// parse_backup_file computes BOTH schemes over the same bytes, so a
    /// lookup keyed by either scheme can find the blob (§8.9 step 1:
    /// "parse_backup_file räknar båda hasharna under övergången").
    #[test]
    fn parse_computes_both_hash_schemes() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let backups = dir.path().join(".gnawtreewriter_backups");
        std::fs::create_dir_all(&backups)?;
        let source = "fn historic_content() {}";
        let path = backups.join("legacy_blob.json");
        std::fs::write(
            &path,
            serde_json::json!({
                "file_path": "src/legacy.rs",
                "timestamp": Utc::now().to_rfc3339(),
                "source_code": source
            })
            .to_string(),
        )?;

        let parsed = parse_backup_file(&path)?;
        assert_eq!(
            parsed.content_hash.as_deref(),
            Some(crate::core::calculate_content_hash(source).as_str()),
            "md5 must be the unified primary hash"
        );
        assert_eq!(
            parsed.legacy_content_hash.as_deref(),
            Some(crate::core::legacy_content_hash(source).as_str()),
            "the legacy hash keeps old keys working"
        );
        assert_ne!(parsed.content_hash, parsed.legacy_content_hash);
        Ok(())
    }

    /// A backup is findable by EITHER its md5 or its legacy hash — the
    /// parity proof for hash-keyed lookups across the scheme boundary.
    #[test]
    fn content_hash_lookup_matches_either_scheme() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let backups = dir.path().join(".gnawtreewriter_backups");
        std::fs::create_dir_all(&backups)?;
        let source = "fn either_scheme_finds_me() {}";
        let path = backups.join("mixed_blob.json");
        std::fs::write(
            &path,
            serde_json::json!({
                "file_path": "src/mixed.rs",
                "timestamp": Utc::now().to_rfc3339(),
                "source_code": source
            })
            .to_string(),
        )?;

        let by_md5 =
            find_backup_by_content_hash(&backups, &crate::core::calculate_content_hash(source))?;
        assert!(by_md5.is_some(), "md5 key must find the backup");
        let by_legacy =
            find_backup_by_content_hash(&backups, &crate::core::legacy_content_hash(source))?;
        assert!(by_legacy.is_some(), "legacy key must find the same backup");
        assert_eq!(by_md5.unwrap().path, by_legacy.unwrap().path);
        Ok(())
    }
}
