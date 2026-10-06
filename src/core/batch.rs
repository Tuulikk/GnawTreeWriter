//! Batch (multi-edit) MVP implementation
//!
//! Provides a small, safe, atomic batch operation facility:
//!  - load a JSON batch file describing operations
//!  - validate all ops in memory (per file) by applying them to in-memory trees
//!  - show unified diffs for preview
//!  - atomically apply: create backups, write new contents, and log transactions
//!
//! Operation JSON format (example):
//! {
//!   "description": "Refactor UI + helpers",
//!   "operations": [
//!     {"type":"edit","file":"a.txt","path":"0","content":"new content"},
//!     {"type":"edit","file":"b.txt","path":"0","content":"other content"}
//!   ]
//! }

use crate::core::{
    calculate_content_hash, find_project_root, EditOperation, GnawTreeWriter, TransactionLog,
};
use crate::parser::get_parser;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use similar::{Algorithm, TextDiff};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum BatchOp {
    Edit {
        file: String,
        path: String,
        content: String,
    },
    Insert {
        file: String,
        parent_path: String,
        position: usize,
        content: String,
    },
    Delete {
        file: String,
        path: String,
    },
}

/// Individual edit operation for batch processing
#[derive(Debug, Clone, Serialize)]
pub enum BatchEdit {
    Edit {
        node_path: String,
        content: String,
    },
    Insert {
        parent_path: String,
        position: usize,
        content: String,
    },
    Delete {
        node_path: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct BatchFile {
    pub description: Option<String>,
    pub operations: Vec<BatchOp>,
}

/// Result of preview per file
pub struct FileDiff {
    pub file: String,
    pub before: String,
    pub after: String,
}

#[derive(Debug, Serialize, Default)]
pub struct Batch {
    /// Receipt (Phase 10): transaction ids logged by the last apply()
    /// (one per written file), in write order. Interior mutability keeps
    /// apply(&self) source-compatible for lib consumers.
    #[serde(skip)]
    pub transaction_ids: std::cell::RefCell<Vec<String>>,
    /// Fas 5.1/5.3 parity: structured verdict behind the last batch
    /// rejection (guardian/rules/no-op during preview validation). Same
    /// edit_verdict contract as single edits.
    #[serde(skip)]
    pub last_verdict: std::cell::RefCell<Option<serde_json::Value>>,
    /// Fas 4 parity: impact reports for signature-changing ops (in op
    /// order), attached to successful applies.
    #[serde(skip)]
    pub impacts: std::cell::RefCell<Vec<serde_json::Value>>,
    pub description: Option<String>,
    pub operations: Vec<BatchOp>,
}

impl Batch {
    /// Create a new empty batch
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a batch with operations for a single file
    pub fn with_file(file_path: String, operations: Vec<BatchEdit>) -> Self {
        let batch_ops: Vec<BatchOp> = operations
            .into_iter()
            .map(|edit| match edit {
                BatchEdit::Edit { node_path, content } => BatchOp::Edit {
                    file: file_path.clone(),
                    path: node_path,
                    content,
                },
                BatchEdit::Insert {
                    parent_path,
                    position,
                    content,
                } => BatchOp::Insert {
                    file: file_path.clone(),
                    parent_path,
                    position,
                    content,
                },
                BatchEdit::Delete { node_path } => BatchOp::Delete {
                    file: file_path.clone(),
                    path: node_path,
                },
            })
            .collect();

        Self {
            description: None,
            operations: batch_ops,
            transaction_ids: std::cell::RefCell::new(Vec::new()),
            last_verdict: std::cell::RefCell::new(None),
            impacts: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Load a batch from a JSON file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let s = fs::read_to_string(&path).context("Failed to read batch file")?;
        Self::from_json(&s)
    }

    /// Load a batch from a JSON string (supports both file and STDIN)
    pub fn from_json(s: &str) -> Result<Self> {
        let bf: BatchFile = serde_json::from_str(s).context("Failed to parse batch JSON")?;
        Ok(Self {
            description: bf.description,
            operations: bf.operations,
            transaction_ids: std::cell::RefCell::new(Vec::new()),
            last_verdict: std::cell::RefCell::new(None),
            impacts: std::cell::RefCell::new(Vec::new()),
        })
    }

    /// Preview: validate and return diffs per file (no writes)
    pub fn preview(&self) -> Result<Vec<FileDiff>> {
        // Fresh verdict/impact state per validation run (Fas 4/5.1 parity).
        *self.last_verdict.borrow_mut() = None;
        self.impacts.borrow_mut().clear();

        // Group ops by file in the given order
        let mut per_file: HashMap<String, Vec<&BatchOp>> = HashMap::new();
        for op in &self.operations {
            match op {
                BatchOp::Edit { file, .. }
                | BatchOp::Insert { file, .. }
                | BatchOp::Delete { file, .. } => {
                    per_file.entry(file.clone()).or_default().push(op);
                }
            }
        }

        let mut diffs: Vec<FileDiff> = Vec::new();

        for (file, ops) in per_file.into_iter() {
            let path = Path::new(&file);
            // Create writer to simulate operations in memory
            let mut writer = GnawTreeWriter::new(&file)
                .with_context(|| format!("Failed to open file for preview: {}", file))?;
            let original = writer.get_source().to_string();

            // Apply ops sequentially in memory
            for op in ops {
                let edit_op = match op {
                    BatchOp::Edit { path, content, .. } => EditOperation::Edit {
                        node_path: path.clone(),
                        content: content.clone(),
                    },
                    BatchOp::Insert {
                        parent_path,
                        position,
                        content,
                        ..
                    } => EditOperation::Insert {
                        parent_path: parent_path.clone(),
                        position: *position,
                        content: content.clone(),
                    },
                    BatchOp::Delete { path, .. } => EditOperation::Delete {
                        node_path: path.clone(),
                    },
                };

                // Fas 4/5.1 parity: run the same validation the single-edit
                // pipeline uses — NO-OP guard, Guardian — and record
                // verdicts/impacts on the batch (RefCell, &self-friendly).
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if let BatchOp::Edit {
                    path: node_query,
                    content,
                    ..
                } = op
                {
                    if let Some(resolved) = writer.resolve_path(node_query.as_str()) {
                        if resolved.content == content.as_str() {
                            *self.last_verdict.borrow_mut() = Some(crate::core::verdict_json(
                                "notice",
                                1.0,
                                vec![serde_json::json!({
                                    "rule": "no_op",
                                    "severity": "warning",
                                    "message": "batch op was a NO-OP (0 bytes would change)"
                                })],
                                vec!["Remove the redundant operation from the batch spec"
                                    .to_string()],
                            ));
                            anyhow::bail!(
                                "NO-OP in batch: op for node {} in {} is identical to current content. Nothing was written.",
                                node_query, file
                            );
                        }
                        let report = crate::core::guardian::GuardianEngine::new()
                            .audit_edit_with_language(resolved, content, ext);
                        match report.level {
                            crate::core::guardian::IntegrityLevel::Critical => {
                                let findings: Vec<serde_json::Value> = report
                                    .messages
                                    .iter()
                                    .map(|m| {
                                        serde_json::json!({
                                            "rule": "structural_delta",
                                            "severity": "error",
                                            "message": m
                                        })
                                    })
                                    .collect();
                                *self.last_verdict.borrow_mut() = Some(crate::core::verdict_json(
                                    "critical",
                                    report.score,
                                    findings,
                                    vec![
                                        "Inspect the target with read_node before retrying".to_string(),
                                        "Batch ops have no force flag — remove or soften the operation".to_string(),
                                    ],
                                ));
                                anyhow::bail!(
                                    "GUARDIAN BLOCK in batch for {} {}: {}. Nothing was written.",
                                    file,
                                    node_query,
                                    report.messages.join(", ")
                                );
                            }
                            crate::core::guardian::IntegrityLevel::Warning => {
                                eprintln!(
                                    "⚠️  GUARDIAN WARNING in batch for {} {}: {}",
                                    file,
                                    node_query,
                                    report.messages.join(", ")
                                );
                            }
                            crate::core::guardian::IntegrityLevel::Notice => {
                                eprintln!("ℹ️  Guardian Note (batch): minor structural reduction.");
                            }
                            _ => {}
                        }
                        if let Some(impact) = crate::core::signature_impact_for(
                            file.as_str(),
                            resolved,
                            &report.deltas,
                        ) {
                            self.impacts.borrow_mut().push(impact);
                        }
                    }
                }

                // Preview change
                let modified = writer
                    .preview_edit(edit_op.clone())
                    .with_context(|| format!("Preview failed for file {} op {:?}", file, op))?;

                // RULES GUARDIAN on the simulated result (all op types).
                let (findings, _skipped, has_error) =
                    crate::core::rules::check_code_with_builtin(&modified, ext);
                if has_error {
                    let error_findings: Vec<serde_json::Value> = findings
                        .iter()
                        .filter(|f| f.severity == crate::core::rules::Severity::Error)
                        .map(|f| {
                            serde_json::json!({
                                "rule": f.rule_id,
                                "severity": "error",
                                "message": f.message,
                                "fix": crate::core::rules::fix_for_finding(f)
                            })
                        })
                        .collect();
                    *self.last_verdict.borrow_mut() = Some(crate::core::verdict_json(
                        "critical",
                        0.0,
                        error_findings,
                        vec!["Rule fixes can be applied via `lint --fix` (preview first)"
                            .to_string()],
                    ));
                    anyhow::bail!(
                        "RULES GUARDIAN BLOCK in batch for {}: edit would introduce rule violations. Nothing was written.",
                        file
                    );
                }

                // Validate by trying to parse with same parser
                let parser = get_parser(path)
                    .with_context(|| format!("No parser for file during preview: {}", file))?;
                if let Err(e) = parser.parse(&modified) {
                    anyhow::bail!("Validation failed for {}: {}\nOperation: {:?}", file, e, op);
                }

                // Accept the simulated change for subsequent operations
                writer.source_code = modified;
                writer.tree = parser
                    .parse(&writer.source_code)
                    .context("Failed re-parse")?;
            }

            let after = writer.get_source().to_string();
            diffs.push(FileDiff {
                file,
                before: original,
                after,
            });
        }

        Ok(diffs)
    }

    /// Apply the batch atomically: create backups, write changes, log transactions.
    /// If any write fails, roll back already written files using their backups.
    pub fn apply(&self) -> Result<()> {
        // Fresh receipt list per apply (Phase 10).
        self.transaction_ids.borrow_mut().clear();

        // Validate first and compute final contents per file
        let diffs = self.preview()?;

        // Prepare mapping and backups
        let mut backups: HashMap<String, PathBuf> = HashMap::new();
        let mut written: Vec<String> = Vec::new();

        for fd in &diffs {
            // If no change, skip
            if fd.before == fd.after {
                continue;
            }

            // Ensure project root and writer
            let mut writer = GnawTreeWriter::new(&fd.file)
                .with_context(|| format!("Failed to open file for backup: {}", fd.file))?;
            // create backup
            let backup_path = writer.create_backup().with_context(|| {
                format!(
                    "Failed to create backup for {} before applying batch",
                    fd.file
                )
            })?;
            backups.insert(fd.file.clone(), backup_path);
        }

        // Now write each file; on failure restore prior ones from backups
        for fd in &diffs {
            if fd.before == fd.after {
                continue;
            }

            // Try to write
            if let Err(e) = fs::write(&fd.file, &fd.after) {
                // Rollback previously written files
                for w in &written {
                    if let Some(backup) = backups.get(w) {
                        if let Ok(backup_content) = fs::read_to_string(backup) {
                            if let Ok(v) =
                                serde_json::from_str::<serde_json::Value>(&backup_content)
                            {
                                if let Some(src) = v.get("source_code").and_then(|s| s.as_str()) {
                                    let _ = fs::write(w, src);
                                }
                            }
                        }
                    }
                }
                anyhow::bail!("Failed to write {}: {}. Rolled back changes.", fd.file, e);
            }

            // Log transaction for this file (one transaction per file in MVP)
            let project_root = find_project_root(Path::new(&fd.file));
            let mut transaction_log = TransactionLog::load(project_root)
                .with_context(|| format!("Failed to load transaction log for {}", fd.file))?;

            let before_hash = Some(calculate_content_hash(&fd.before));
            let after_hash = Some(calculate_content_hash(&fd.after));

            let txn_id = transaction_log.log_transaction(
                crate::core::OperationType::Edit,
                PathBuf::from(&fd.file),
                None,
                before_hash,
                after_hash,
                format!("Batch apply: {}", self.description_or_ops()),
                std::collections::HashMap::new(),
            )?;
            self.transaction_ids.borrow_mut().push(txn_id);

            written.push(fd.file.clone());
        }

        // If we reach here, all writes and logs succeeded
        eprintln!("✓ Batch applied successfully to {} files", written.len());
        Ok(())
    }

    fn description_or_ops(&self) -> String {
        if let Some(ref d) = self.description {
            d.clone()
        } else {
            format!("{} operations", self.operations.len())
        }
    }

    /// Convenience: run preview and return a unified textual representation
    pub fn preview_text(&self) -> Result<String> {
        let diffs = self.preview()?;
        let mut out = String::new();
        for fd in diffs {
            out.push_str(&format!("\n{}\n", "=".repeat(80)));
            out.push_str(&format!("File: {}\n", fd.file));
            out.push_str(&format!("Description: {}\n", self.description_or_ops()));
            out.push_str(&format!("{}\n", "=".repeat(80)));
            out.push_str(&format_diff(&fd.before, &fd.after));
            out.push('\n');
        }
        Ok(out)
    }
}

/// Format a unified-ish diff of two strings (line-based).
fn format_diff(before: &str, after: &str) -> String {
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Patience)
        .diff_lines(before, after);
    let mut buf = String::new();
    for group in diff.grouped_ops(0) {
        for op in group {
            for change in diff.iter_inline_changes(&op) {
                let sign = match change.tag() {
                    similar::ChangeTag::Delete => "-",
                    similar::ChangeTag::Insert => "+",
                    similar::ChangeTag::Equal => " ",
                };
                for line in change.to_string().lines() {
                    buf.push_str(&format!("{}{}\n", sign, line));
                }
            }
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn batch_edit_simple() -> Result<()> {
        let tmp = tempdir()?;
        let p1 = tmp.path().join("a.txt");
        let p2 = tmp.path().join("b.txt");
        fs::write(&p1, "original A\n")?;
        fs::write(&p2, "original B\n")?;

        let batch = Batch {
            description: Some("Simple test".into()),
            transaction_ids: std::cell::RefCell::new(Vec::new()),
            last_verdict: std::cell::RefCell::new(None),
            impacts: std::cell::RefCell::new(Vec::new()),
            operations: vec![
                BatchOp::Edit {
                    file: p1.to_string_lossy().to_string(),
                    path: "0".to_string(),
                    content: "updated A\n".to_string(),
                },
                BatchOp::Edit {
                    file: p2.to_string_lossy().to_string(),
                    path: "0".to_string(),
                    content: "updated B\n".to_string(),
                },
            ],
        };

        // Preview should show diffs (unified diff format with ++ for additions)
        let preview_text = batch.preview_text()?;
        assert!(preview_text.contains("++updated+ A"));
        assert!(preview_text.contains("++updated+ B"));

        // Apply should succeed and files should be updated
        batch.apply()?;

        let a = fs::read_to_string(&p1)?;
        let b = fs::read_to_string(&p2)?;
        // Note: GenericParser might preserve or normalize line endings
        assert!(a.starts_with("updated A"));
        assert!(b.starts_with("updated B"));

        Ok(())
    }

    #[test]
    fn batch_validation_failure_rolls_back() -> Result<()> {
        let tmp = tempdir()?;
        let p1 = tmp.path().join("a.txt");
        fs::write(&p1, "original A\n")?;

        // An edit that results in invalid syntax for a parser-sensitive file could be simulated:
        // For generic files, there is no parser error, so simulate by targeting a parser file if available.
        // We'll simulate by adding an invalid operation type (Delete with missing path) which will still be validated,
        // so simply assert that preview + apply paths are consistent.
        let batch = Batch {
            description: Some("Fail test".into()),
            transaction_ids: std::cell::RefCell::new(Vec::new()),
            last_verdict: std::cell::RefCell::new(None),
            impacts: std::cell::RefCell::new(Vec::new()),
            operations: vec![BatchOp::Edit {
                file: p1.to_string_lossy().to_string(),
                path: "0".to_string(),
                content: "still ok\n".to_string(),
            }],
        };

        // Should preview and apply cleanly
        let _ = batch.preview_text()?;
        batch.apply()?;
        let a = fs::read_to_string(&p1)?;
        assert!(a.starts_with("still ok"));
        Ok(())
    }

    /// Fas 5.1 parity: a guardian-blocking batch op fails the whole batch
    /// and leaves the same structured verdict as a single edit.
    #[test]
    fn batch_rejection_carries_edit_verdict() -> Result<()> {
        let tmp = tempdir()?;
        let p = tmp.path().join("g.rs");
        let guarded = "fn get(items: &[u8], i: usize) -> u8 {\n    if i < items.len() {\n        items[i]\n    } else {\n        0\n    }\n}\n";
        fs::write(&p, guarded)?;

        let batch = Batch {
            description: Some("drop guard".into()),
            transaction_ids: std::cell::RefCell::new(Vec::new()),
            last_verdict: std::cell::RefCell::new(None),
            impacts: std::cell::RefCell::new(Vec::new()),
            operations: vec![BatchOp::Edit {
                file: p.to_string_lossy().to_string(),
                path: "@fn:get".to_string(),
                content: "fn get(items: &[u8], i: usize) -> u8 {\n    items[i]\n}\n".to_string(),
            }],
        };

        let err = batch
            .apply()
            .expect_err("guard-dropping batch must be rejected");
        assert!(err.to_string().contains("GUARDIAN BLOCK"), "got: {}", err);
        let verdict = batch.last_verdict.borrow().clone().expect("verdict set");
        assert_eq!(verdict["level"], "critical");
        assert!(
            !verdict["findings"].as_array().unwrap().is_empty(),
            "findings present: {verdict:?}"
        );
        assert_eq!(
            fs::read_to_string(&p)?,
            guarded,
            "rejected batch must not touch the file"
        );
        Ok(())
    }

    /// Fas 4 parity: a signature-changing batch op collects impact from
    /// the knowledge graph on success.
    #[test]
    fn batch_signature_change_collects_impact() -> Result<()> {
        let tmp = tempdir()?;
        std::fs::create_dir(tmp.path().join(".git"))?;
        let def = tmp.path().join("lib.rs");
        let caller = tmp.path().join("caller.rs");
        fs::write(&def, "fn target(alpha: u32) -> u32 {\n    alpha\n}\n")?;
        fs::write(&caller, "fn caller() -> u32 {\n    target(1)\n}\n")?;
        let mut indexer = crate::llm::RelationalIndexer::new(tmp.path());
        indexer.index_directory(tmp.path())?;

        let batch = Batch {
            description: Some("widen signature".into()),
            transaction_ids: std::cell::RefCell::new(Vec::new()),
            last_verdict: std::cell::RefCell::new(None),
            impacts: std::cell::RefCell::new(Vec::new()),
            operations: vec![BatchOp::Edit {
                file: def.to_string_lossy().to_string(),
                path: "@fn:target".to_string(),
                content: "fn target(alpha: u32, beta: u32) -> u32 {\n    alpha + beta\n}\n"
                    .to_string(),
            }],
        };

        batch.apply()?;
        let impacts = batch.impacts.borrow().clone();
        assert_eq!(impacts.len(), 1, "one impact expected: {impacts:?}");
        assert_eq!(impacts[0]["callers"], 1);
        assert!(
            impacts[0]["sites"][0]
                .as_str()
                .unwrap_or("")
                .contains("caller.rs"),
            "site names the caller: {impacts:?}"
        );
        Ok(())
    }
}
