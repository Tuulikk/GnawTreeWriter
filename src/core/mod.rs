use crate::parser::{get_parser, TreeNode};
use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub mod alf;
pub mod anchor;
pub mod backup;
pub mod batch;
pub mod blast;
pub mod blueprint;
pub mod compress;
pub mod curator;
pub mod diagnostics;
pub mod diff_parser;
pub mod edit_delta;
pub mod explore;
pub mod file_walker;
pub mod gnaw_diff;
pub mod gnaw_find;
pub mod gnaw_graph;
pub mod gnaw_refactor;
pub mod guardian;
pub mod healer;
pub mod index_entities;
pub mod index_relations;
pub mod inspect;
pub mod label_manager;
pub mod macro_dispatcher;
pub mod pack;
pub mod parse_cache;
pub mod report;
pub mod restoration_engine;
pub mod rules;
pub mod scaffold;
pub mod secrets;
pub mod state;
pub mod stats;
pub mod tag_manager;
pub mod token_count;
pub mod transaction_log;
pub mod undo_redo;
pub mod visualizer;

pub use batch::{Batch, BatchEdit, BatchOp};
pub use gnaw_refactor::{
    format_refactor_text, refactor, Change, RefactorKind, RefactorResult, RefactorSummary,
};
pub use label_manager::LabelManager;
pub use restoration_engine::{RestorationEngine, RestorationResult, RestorationStats};
pub use scaffold::ScaffoldEngine;
pub use tag_manager::TagManager;
pub use transaction_log::{
    calculate_content_hash, FileRestorationPlan, OperationType, ProjectRestorationPlan,
    Transaction, TransactionLog,
};
pub use undo_redo::{UndoRedoManager, UndoRedoResult, UndoRedoState};

pub struct GnawTreeWriter {
    file_path: String,
    source_code: String,
    tree: TreeNode,
    transaction_log: TransactionLog,
    /// Senast skapade backup-sökväg (spec-gtw-backup-id.md, Väg A).
    /// Sätts i create_backup efter lyckad write. None om ingen edit skett.
    last_backup: Option<PathBuf>,
    /// Receipt: id of the last transaction THIS writer logged (Phase 10 —
    /// MCP callers correlate writes with history/undo). None until edit().
    last_transaction_id: Option<String>,
    /// Receipt of the last completed edit (Fas 3). None until edit() ran.
    last_receipt: Option<EditReceipt>,
    /// Structured rejection verdict (Fas 5.1). Set when edit() rejects;
    /// None while the last edit succeeded.
    last_verdict: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub enum EditOperation {
    Edit {
        node_path: String,
        content: String,
    },
    Clone {
        source_path: String,
        target_path: String,
        target_node: Option<String>,
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

/// Receipt for a completed edit (Fas 3, docs/GUARDIAN_V2_PLAN.md): proves
/// bytes actually landed on disk and reports how much changed. Finding #14
/// class: a "success" without byte change must be loud, never silent.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EditReceipt {
    pub transaction_id: String,
    /// Disk content re-read and matched the intended content.
    pub verified: bool,
    /// Whole-file content hash differed before vs after.
    pub changed: bool,
    /// Position-aligned byte difference (see count_changed_bytes).
    pub bytes_changed: usize,
}

impl GnawTreeWriter {
    pub fn new(file_path: &str) -> Result<Self> {
        let path = Path::new(file_path);
        let source_code =
            fs::read_to_string(path).context(format!("Failed to read file: {}", file_path))?;

        let parser = get_parser(path)?;
        let tree = parser.parse(&source_code)?;

        // Initialize transaction log for the project root
        // Use find_project_root to ensure we log to the correct centralized location
        let project_root = find_project_root(path);
        let transaction_log = TransactionLog::load(project_root)?;

        Ok(Self {
            file_path: file_path.to_string(),
            source_code,
            tree,
            transaction_log,
            last_backup: None,
            last_transaction_id: None,
            last_receipt: None,
            last_verdict: None,
        })
    }

    /// Read-path constructor (partial-grace): like `new`, but uses the
    /// parser's `parse_lenient` — a localized syntax error yields a
    /// PARTIAL tree plus the warning instead of refusing the file. Only
    /// read-only MCP tools (analyze/skeleton/list/read) opt in; editors
    /// (`new` + edit/insert/quick-replace) stay strict. Parsers without
    /// a lenient override behave exactly like `new`.
    pub fn new_lenient(file_path: &str) -> Result<(Self, Option<crate::parser::SyntaxError>)> {
        let path = Path::new(file_path);
        let source_code =
            fs::read_to_string(path).context(format!("Failed to read file: {}", file_path))?;

        let parser = get_parser(path)?;
        let (tree_result, warning) = parser.parse_lenient(&source_code);
        let tree = tree_result?;

        let project_root = find_project_root(path);
        let transaction_log = TransactionLog::load(project_root)?;

        Ok((
            Self {
                file_path: file_path.to_string(),
                source_code,
                tree,
                transaction_log,
                last_backup: None,
                last_transaction_id: None,
                last_receipt: None,
                last_verdict: None,
            },
            warning,
        ))
    }

    /// Senast skapade backup-sökväg (None om ingen edit skett).
    /// Exponerar backup-id:t som motor2-gtw behöver (spec-gtw-backup-id.md).
    pub fn last_backup_path(&self) -> Option<PathBuf> {
        self.last_backup.clone()
    }

    /// Receipt (Phase 10): id of the last transaction this writer logged —
    /// None if nothing went through edit().
    pub fn last_transaction_id(&self) -> Option<String> {
        self.last_transaction_id.clone()
    }

    /// Receipt of the last completed edit (Fas 3, docs/GUARDIAN_V2_PLAN.md).
    /// None until a successful edit() ran. MCP callers attach this to edit
    /// responses so agents can verify bytes landed (breaks no-op loops).
    pub fn last_edit_receipt(&self) -> Option<&EditReceipt> {
        self.last_receipt.as_ref()
    }

    /// Structured rejection verdict (Fas 5.1, docs/GUARDIAN_V2_PLAN.md):
    /// findings + suggestions for the last rejected edit. None while the
    /// last edit() succeeded. MCP edit responses attach this as
    /// `edit_verdict` so agents get machine-readable next steps.
    pub fn last_edit_verdict(&self) -> Option<&serde_json::Value> {
        self.last_verdict.as_ref()
    }

    fn set_verdict(
        &mut self,
        level: &str,
        score: f32,
        findings: Vec<serde_json::Value>,
        suggestions: Vec<String>,
    ) {
        self.last_verdict = Some(serde_json::json!({
            "level": level,
            "score": score,
            "findings": findings,
            "suggestions": suggestions,
        }));
    }

    pub(crate) fn create_backup(&mut self) -> Result<PathBuf> {
        let file_name = Path::new(&self.file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%3f");
        let backup_name = format!("{}_backup_{}.json", file_name, timestamp);

        // Backup should also be in project root to avoid scattering
        let project_root = find_project_root(Path::new(&self.file_path));
        let backup_dir = project_root.join(".gnawtreewriter_backups");

        fs::create_dir_all(&backup_dir)?;

        let backup_path = backup_dir.join(&backup_name);

        let backup_data = serde_json::json!({
            "file_path": self.file_path,
            "timestamp": Utc::now().to_rfc3339(),
            "tree": &self.tree,
            "source_code": self.source_code
        });

        fs::write(&backup_path, serde_json::to_string_pretty(&backup_data)?)
            .context(format!("Failed to write backup: {}", backup_path.display()))?;

        // Spec-gtw-backup-id.md (Väg A): spara sökvägen så last_backup_path()
        // kan exponera backup-id:t för motor2-gtw (work_unit → gtw_backup).
        self.last_backup = Some(backup_path.clone());

        Ok(backup_path)
    }

    pub fn analyze(&self) -> &TreeNode {
        &self.tree
    }

    pub fn show_node(&self, node_path: &str) -> Result<String> {
        let node = self
            .resolve_path(node_path)
            .context(format!("Node not found: {}", node_path))?;
        Ok(node.content.clone())
    }

    // Test indent insert
    pub fn edit(&mut self, operation: EditOperation, force: bool) -> Result<()> {
        // Calculate before hash
        let before_hash = calculate_content_hash(&self.source_code);
        let pre_edit_source = self.source_code.clone();
        self.last_verdict = None;

        let modified_code = match &operation {
            EditOperation::Edit { node_path, content } => {
                let resolved = self
                    .resolve_path(node_path)
                    .context(format!("Could not resolve node path: {}", node_path))?;
                if resolved.content == *content {
                    self.set_verdict(
                        "notice",
                        1.0,
                        vec![serde_json::json!({
                            "rule": "no_op",
                            "severity": "warning",
                            "message": "edit was a NO-OP (0 bytes would change)"
                        })],
                        vec![
                            "Verify the node path and the intended change".to_string(),
                            "Use preview_edit to inspect the target node".to_string(),
                        ],
                    );
                    return Err(anyhow::anyhow!(
                        "NO-OP: new content is identical to the current content of node {} (0 bytes would change). Nothing was written — verify the node path and intended change, or use preview_edit.",
                        node_path
                    ));
                }
                self.edit_node_at_path(&resolved.path, content)?
            }
            EditOperation::Insert {
                parent_path,
                position,
                content,
            } => {
                let resolved = self
                    .resolve_path(parent_path)
                    .context(format!("Could not resolve parent path: {}", parent_path))?;
                self.insert_node_at_path(&resolved.path, *position, content)?
            }
            EditOperation::Delete { node_path } => {
                let resolved = self
                    .resolve_path(node_path)
                    .context(format!("Could not resolve node path: {}", node_path))?;
                self.delete_node_at_path(&resolved.path)?
            }
            EditOperation::Clone {
                source_path,
                target_path,
                target_node,
            } => {
                // Clone is handled in CLI layer, not in core edit
                let _ = (source_path, target_path, target_node);
                return Err(anyhow::anyhow!(
                    "Clone operation should be handled in CLI layer"
                ));
            }
        };

        // GUARDIAN INTEGRITY CHECK: Analyze the impact of the change
        if let EditOperation::Edit { node_path, content } = &operation {
            if !force {
                let resolved = self
                    .resolve_path(node_path)
                    .context("Guardian could not resolve node")?;
                let guardian = crate::core::guardian::GuardianEngine::new();
                let ext = Path::new(&self.file_path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                let report = guardian.audit_edit_with_language(resolved, content, ext);

                // Fas 5.1: structured verdict for the rejection path.
                let level_str = match report.level {
                    crate::core::guardian::IntegrityLevel::Critical => "critical",
                    crate::core::guardian::IntegrityLevel::Warning => "warning",
                    crate::core::guardian::IntegrityLevel::Notice => "notice",
                    crate::core::guardian::IntegrityLevel::Safe => "safe",
                };
                let sev_str = if matches!(
                    report.level,
                    crate::core::guardian::IntegrityLevel::Critical
                ) {
                    "error"
                } else {
                    "warning"
                };
                self.set_verdict(
                    level_str,
                    report.score,
                    report
                        .messages
                        .iter()
                        .map(|m| {
                            serde_json::json!({
                                "rule": "structural_delta",
                                "severity": sev_str,
                                "message": m
                            })
                        })
                        .collect(),
                    vec![
                        "Inspect the target with read_node before retrying".to_string(),
                        "If the reduction is intentional, retry with force".to_string(),
                    ],
                );

                match report.level {
                    crate::core::guardian::IntegrityLevel::Critical => {
                        return Err(anyhow::anyhow!("🛑 GUARDIAN BLOCK: This edit removes critical logic or structure.\nMessages: {}\nUse --force to override.", report.messages.join(", ")));
                    }
                    crate::core::guardian::IntegrityLevel::Warning => {
                        eprintln!(
                            "⚠️  GUARDIAN WARNING: Significant structural loss detected: {}",
                            report.messages.join(", ")
                        );
                    }
                    crate::core::guardian::IntegrityLevel::Notice => {
                        eprintln!("ℹ️  Guardian Note: Minor structural reduction observed.");
                    }
                    _ => {}
                }
            } else {
                eprintln!("🛡️  Guardian bypassed via --force.");
            }
        }

        // VALIDATION: Try to parse the modified code in memory before saving
        let path = Path::new(&self.file_path);
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let parser = get_parser(path)?;

        let modified_code = match parser.parse(&modified_code) {
            Ok(_) => modified_code,
            Err(e) => {
                // TRY TO HEAL (Duplex Loop)
                let healer = crate::core::healer::Healer::new();
                if let Some(action) = healer.suggest_fix(&modified_code, &e, extension) {
                    let mut healed_code = modified_code.clone();
                    // Basic healing: append the fix
                    healed_code.push_str(&action.fix);

                    // Validate healed code
                    if parser.parse(&healed_code).is_ok() {
                        eprintln!(
                            "✨ Duplex Loop: Automatically healed syntax error: {}",
                            action.description
                        );
                        healed_code
                    } else {
                        self.set_verdict(
                            "critical",
                            0.0,
                            vec![serde_json::json!({
                                "rule": "syntax",
                                "severity": "error",
                                "message": e.to_string()
                            })],
                            vec![format!(
                                "Automatic healing attempt ({}) did not resolve the syntax error",
                                action.description
                            )],
                        );
                        return Err(anyhow::anyhow!(
                            "Validation failed: The proposed edit would result in invalid syntax.\nError: {}\nAutomatic healing attempt ({}) did not resolve it.\n\nChange was NOT applied.",
                            e, action.description
                        ));
                    }
                } else {
                    let tip = match extension {
                        "rs" => "\n\n💡 Tip: In Rust, check for missing semicolons ';' at the end of statements, or unbalanced braces '{}'.",
                        "qml" => "\n\n💡 Tip: In QML, ensure properties have a colon ':' and that braces '{}' and brackets '[]' are balanced.",
                        "py" => "\n\n💡 Tip: In Python, check your indentation levels and ensure colons ':' are present after def/if/for/while.",
                        _ => "\n\n💡 Tip: Ensure you included all necessary punctuation and punctuation is balanced for this file type.",
                    };

                    let mut msg = format!("Validation failed: The proposed edit would result in invalid syntax.\nError: {}", e);
                    if e.line > 0 {
                        msg.push_str(&format!("\nCheck near line {}.", e.line));
                    }
                    msg.push_str(tip);
                    msg.push_str("\nChange was NOT applied.");
                    self.set_verdict(
                        "critical",
                        0.0,
                        vec![serde_json::json!({
                            "rule": "syntax",
                            "severity": "error",
                            "message": e.to_string()
                        })],
                        vec![tip.trim().to_string()],
                    );
                    return Err(anyhow::anyhow!(msg));
                }
            }
        };

        // Calculate after hash
        let after_hash = calculate_content_hash(&modified_code);

        // RULES GUARDIAN: run builtin rules on the new code. Error-severity
        // findings block the edit (unless --force); warnings are printed.
        if !force {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let (findings, _skipped, has_error) =
                crate::core::rules::check_code_with_builtin(&modified_code, ext);
            let describe = |f: &crate::core::rules::Finding| {
                let fix = crate::core::rules::fix_for_finding(f)
                    .map(|fx| fx.split_whitespace().collect::<Vec<_>>().join(" "))
                    .map(|fx| format!(" — suggested fix: `{}`", fx))
                    .unwrap_or_default();
                format!("{} [{}:{}]{}", f.message, f.rule_id, f.line, fix)
            };
            if has_error {
                let findings_json: Vec<serde_json::Value> = findings
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
                self.set_verdict(
                    "critical",
                    0.0,
                    findings_json,
                    vec!["Rule fixes can be applied via `lint --fix` (preview first)".to_string()],
                );
                let msgs: Vec<String> = findings
                    .iter()
                    .filter(|f| f.severity == crate::core::rules::Severity::Error)
                    .map(describe)
                    .collect();
                return Err(anyhow::anyhow!(
                    "🛑 RULES GUARDIAN BLOCK: edit would introduce rule violations.\n- {}\nRule fixes can be applied via `lint --fix` (preview first).\nUse --force to override.",
                    msgs.join("\n- ")
                ));
            }
            for f in &findings {
                let sev = match f.severity {
                    crate::core::rules::Severity::Error => "error",
                    crate::core::rules::Severity::Warning => "warning",
                    crate::core::rules::Severity::Info => "info",
                };
                eprintln!("⚠️  RULES GUARDIAN {}: {}", sev, describe(f));
            }
        }

        // Only create backup and write if validation passed
        self.create_backup()?;

        // Log the transaction
        let (operation_type, node_path, description) = match &operation {
            EditOperation::Edit {
                node_path,
                content: _,
            } => (
                OperationType::Edit,
                Some(node_path.clone()),
                format!("Edited node: {}", node_path),
            ),
            EditOperation::Insert {
                parent_path,
                position,
                content: _,
            } => (
                OperationType::Insert,
                Some(parent_path.clone()),
                format!("Inserted content at {}, position {}", parent_path, position),
            ),
            EditOperation::Delete { node_path } => (
                OperationType::Delete,
                Some(node_path.clone()),
                format!("Deleted node: {}", node_path),
            ),
            EditOperation::Clone {
                source_path,
                target_path,
                target_node,
            } => {
                let _ = (source_path, target_path, target_node);
                return Err(anyhow::anyhow!(
                    "Clone operation should be handled in CLI layer"
                ));
            }
        };

        let transaction_id = self.transaction_log.log_transaction(
            operation_type,
            PathBuf::from(&self.file_path),
            node_path,
            Some(before_hash.clone()),
            Some(after_hash.clone()),
            description.clone(),
            HashMap::new(),
        )?;
        self.last_transaction_id = Some(transaction_id.clone());

        // ALF INTEGRATION: Automatically log the tool use
        let project_root = find_project_root(Path::new(&self.file_path));
        if let Ok(mut alf) = crate::core::alf::AlfManager::load(&project_root) {
            let _ = alf.log(
                crate::core::alf::AlfType::Auto,
                &format!("Tool Use: {} - {}", description, self.file_path),
                Some(transaction_id.clone()),
            );
        }

        fs::write(&self.file_path, &modified_code)
            .context(format!("Failed to write file: {}", self.file_path))?;

        // Post-write verification (Fas 3): re-read and prove the bytes
        // landed. verified=false is an error, never a silent OK.
        let written = fs::read_to_string(&self.file_path)
            .context(format!("Failed to re-read after write: {}", self.file_path))?;
        let verified = written == modified_code;

        // Refresh internal state to reflect the changes on disk
        self.source_code = modified_code;
        let parser = get_parser(Path::new(&self.file_path))?;
        self.tree = parser.parse(&self.source_code)?;

        let receipt = EditReceipt {
            transaction_id,
            verified,
            changed: before_hash != after_hash,
            bytes_changed: count_changed_bytes(&pre_edit_source, &self.source_code),
        };
        let verified = receipt.verified;
        self.last_receipt = Some(receipt);

        if !verified {
            return Err(anyhow::anyhow!(
                "Post-write verification FAILED: file on disk does not match the intended content. The write may not have landed — inspect with history/doctor before retrying."
            ));
        }

        Ok(())
    }

    pub fn preview_edit(&self, operation: EditOperation) -> Result<String> {
        match operation {
            EditOperation::Edit { node_path, content } => {
                let resolved = self
                    .resolve_path(&node_path)
                    .context(format!("Could not resolve node path: {}", node_path))?;
                self.edit_node_at_path(&resolved.path, &content)
            }
            EditOperation::Insert {
                parent_path,
                position,
                content,
            } => {
                let resolved = self
                    .resolve_path(&parent_path)
                    .context(format!("Could not resolve parent path: {}", parent_path))?;
                self.insert_node_at_path(&resolved.path, position, &content)
            }
            EditOperation::Delete { node_path } => {
                let resolved = self
                    .resolve_path(&node_path)
                    .context(format!("Could not resolve node path: {}", node_path))?;
                self.delete_node_at_path(&resolved.path)
            }
            EditOperation::Clone {
                source_path,
                target_path,
                target_node,
            } => {
                // Clone is handled in CLI layer, not core layer
                // This is just a placeholder for preview
                let _ = (source_path, target_path, target_node);
                Ok(self.source_code.clone())
            }
        }
    }

    /// Resolves a path string which can be either a numeric path (1.2.3)
    /// or a semantic query (@fn:name, @struct:name, @name).
    fn resolve_path<'a>(&'a self, query: &str) -> Option<&'a TreeNode> {
        if let Some(name_query) = query.strip_prefix('@') {
            // Semantic search
            if let Some((kind, name)) = name_query.split_once(':') {
                self.find_node_by_name(&self.tree, name, Some(kind))
            } else {
                // Generic name search
                self.find_node_by_name(&self.tree, name_query, None)
            }
        } else {
            // Standard numeric path
            self.find_node_by_path(&self.tree, query)
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn find_node_by_path<'a>(&self, tree: &'a TreeNode, path: &str) -> Option<&'a TreeNode> {
        if tree.path == path {
            return Some(tree);
        }

        for child in &tree.children {
            if let Some(node) = self.find_node_by_path(child, path) {
                return Some(node);
            }
        }

        None
    }

    #[allow(clippy::only_used_in_recursion)]
    fn find_node_by_name<'a>(
        &self,
        tree: &'a TreeNode,
        name: &str,
        kind: Option<&str>,
    ) -> Option<&'a TreeNode> {
        // Does this node match?
        if let Some(node_name) = tree.get_name() {
            if node_name == name {
                // If kind is specified, check node type
                if let Some(k) = kind {
                    let nt = tree.node_type.to_lowercase();
                    match k {
                        "fn" | "func" | "function" | "method" => {
                            if nt.contains("function") || nt.contains("method") {
                                return Some(tree);
                            }
                        }
                        "struct" | "class" | "type" => {
                            if nt.contains("struct") || nt.contains("class") || nt.contains("type")
                            {
                                return Some(tree);
                            }
                        }
                        _ => {
                            if nt.contains(k) {
                                return Some(tree);
                            }
                        }
                    }
                } else {
                    return Some(tree);
                }
            }
        }

        // Recursively check children
        for child in &tree.children {
            if let Some(node) = self.find_node_by_name(child, name, kind) {
                return Some(node);
            }
        }

        None
    }

    fn edit_node_at_path(&self, node_path: &str, new_content: &str) -> Result<String> {
        let node = self
            .find_node_by_path(&self.tree, node_path)
            .context(format!("Node not found at path: {}", node_path))?;

        let lines: Vec<&str> = self.source_code.lines().collect();

        // If we have column information, use it for surgical precision
        if node.start_col > 0 && node.end_col > 0 {
            let mut new_lines: Vec<String> = Vec::new();

            // Lines before the node's start line
            for i in 0..node.start_line - 1 {
                if i < lines.len() {
                    new_lines.push(lines[i].to_string());
                }
            }

            // Handle the start line (with prefix)
            let start_line_idx = node.start_line - 1;
            let start_line_text = lines[start_line_idx];
            let prefix: String = start_line_text.chars().take(node.start_col - 1).collect();

            // Handle the end line (with suffix)
            let end_line_idx = node.end_line - 1;
            let end_line_text = lines[end_line_idx];
            let suffix: String = end_line_text.chars().skip(node.end_col - 1).collect();

            // Combine prefix, new_content, and suffix
            let mut combined = prefix;
            combined.push_str(new_content);
            combined.push_str(&suffix);

            // Since combined might be multi-line if new_content is, we push its lines
            // We use a custom splitting to preserve empty lines at the end if needed
            let mut first = true;
            for line in combined.split('\n') {
                if first {
                    new_lines.push(line.to_string());
                    first = false;
                } else {
                    new_lines.push(line.to_string());
                }
            }

            // Lines after the node's end line
            for line in lines.iter().skip(node.end_line) {
                new_lines.push(line.to_string());
            }

            Ok(new_lines.join("\n"))
        } else {
            let mut new_lines: Vec<String> = Vec::new();

            // Lines before the node
            for i in 0..node.start_line - 1 {
                if i < lines.len() {
                    new_lines.push(lines[i].to_string());
                }
            }

            // Add the new content
            // Note: new_content might be multi-line
            for line in new_content.lines() {
                new_lines.push(line.to_string());
            }

            // Lines after the node
            for line in lines.iter().skip(node.end_line) {
                new_lines.push(line.to_string());
            }

            Ok(new_lines.join("\n"))
        }
    }

    fn insert_node_at_path(
        &self,
        node_path: &str,
        position: usize,
        content: &str,
    ) -> Result<String> {
        let parent = self
            .find_node_by_path(&self.tree, node_path)
            .context(format!("Parent node not found at path: {}", node_path))?;

        let lines: Vec<&str> = self.source_code.lines().collect();
        let mut new_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();

        let insert_pos = match position {
            0 => {
                // If it starts with a brace, insert after it
                if parent.content.trim_start().starts_with('{') {
                    parent.start_line
                } else {
                    parent.start_line - 1
                }
            }
            1 => {
                // Insert at the end of the parent node.
                // For source_file: TreeSitter end_line can exceed lines.len()
                // (trailing newline counted as extra line), so we clamp.
                // For other nodes (blocks, etc.): end_line points to the closing
                // delimiter line, so we subtract 1 to insert BEFORE it.
                if parent.node_type == "source_file" {
                    parent.end_line.min(lines.len())
                } else {
                    parent.end_line.saturating_sub(1)
                }
            }
            2 => {
                let mut last_prop_line = parent.start_line;
                let mut found = false;
                for child in &parent.children {
                    if (child.node_type == "ui_property" || child.node_type == "ui_binding")
                        && child.end_line < parent.end_line
                    {
                        last_prop_line = child.end_line;
                        found = true;
                    }
                }
                if found {
                    last_prop_line
                } else {
                    // Fallback to top (after brace if exists)
                    parent.start_line
                }
            }
            // SUPPORT FOR ARBITRARY INDICES
            idx => {
                // If we want to insert at a specific index relative to children
                if idx - 3 < parent.children.len() {
                    parent.children[idx - 3].end_line
                } else if !parent.children.is_empty() {
                    // If index is out of bounds but we have children, append after last child
                    parent.children.last().unwrap().end_line
                } else {
                    // Fallback to inside parent (start)
                    parent.start_line
                }
            }
        };

        // Detect indentation from parent or siblings
        let indentation = if !lines.is_empty() {
            let ref_line = if insert_pos < lines.len() {
                lines[insert_pos]
            } else {
                lines[lines.len() - 1]
            };
            let ws: String = ref_line.chars().take_while(|c| c.is_whitespace()).collect();
            if ws.is_empty() {
                let prev_idx = if insert_pos > 0 && insert_pos <= lines.len() {
                    insert_pos - 1
                } else {
                    lines.len() - 1
                };
                lines[prev_idx]
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .collect()
            } else {
                ws
            }
        } else {
            String::new()
        };

        let indented_content: Vec<String> = content
            .lines()
            .map(|line| format!("{}{}", indentation, line))
            .collect();

        if insert_pos >= new_lines.len() {
            new_lines.extend(indented_content);
        } else {
            for (i, line) in indented_content.into_iter().enumerate() {
                new_lines.insert(insert_pos + i, line);
            }
        }

        Ok(new_lines.join("\n"))
    }

    fn delete_node_at_path(&self, node_path: &str) -> Result<String> {
        let node = self
            .find_node_by_path(&self.tree, node_path)
            .context(format!("Node not found at path: {}", node_path))?;

        let lines: Vec<&str> = self.source_code.lines().collect();
        let start_idx = node.start_line - 1;
        let end_idx = node.end_line;

        let new_lines: Vec<_> = lines[..start_idx]
            .iter()
            .chain(lines[end_idx..].iter())
            .copied()
            .collect();

        Ok(new_lines.join("\n"))
    }
    pub fn get_source(&self) -> &str {
        &self.source_code
    }
}

/// Helper function to find the project root
/// Searches upwards for .gnawtreewriter_session.json or .git
pub fn find_project_root(start_path: &Path) -> PathBuf {
    let mut current = if start_path.is_file() {
        start_path.parent().unwrap_or(Path::new(".")).to_path_buf()
    } else {
        start_path.to_path_buf()
    };

    // Try to make it absolute if possible, but don't fail if we can't
    if let Ok(abs) = fs::canonicalize(&current) {
        current = abs;
    }

    let start = current.clone();

    loop {
        // Check for session file or git
        if current.join(".gnawtreewriter_session.json").exists() || current.join(".git").exists() {
            return current;
        }

        if !current.pop() {
            // Reached root without finding anything, return start path (fallback)
            return start;
        }
    }
}

/// Position-aligned byte difference between two source texts (Fas 3):
/// differing bytes at shared positions, plus the length delta.
fn count_changed_bytes(before: &str, after: &str) -> usize {
    before
        .bytes()
        .zip(after.bytes())
        .filter(|(a, b)| a != b)
        .count()
        + before.len().abs_diff(after.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_rust_file(code: &str) -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.rs");
        std::fs::write(&path, code).unwrap();
        let path_str = path.to_str().unwrap().to_string();
        (dir, path_str)
    }

    /// Regression: the Guardian must compare the old NODE content with the
    /// new NODE content. It was wired against the whole modified file,
    /// which made the volume/complexity checks dead code.
    #[test]
    fn guardian_blocks_drastic_node_reduction() {
        // `helper` keeps whole-file volume/complexity high after the edit:
        // the pre-fix wiring (Guardian comparing node vs WHOLE FILE) passed
        // this edit; node-vs-node correctly blocks it.
        let code = concat!(
            "fn big() {\n",
            "    // alpha comment\n",
            "    // beta comment\n",
            "    // gamma comment\n",
            "    if true { } else { }\n",
            "    for i in 0..10 { }\n",
            "    match 1 { _ => {} }\n",
            "}\n",
            "fn helper() {\n",
            "    if true { } else { }\n",
            "    for i in 0..10 { }\n",
            "    match 1 { _ => {} }\n",
            "}\n",
        );
        let (_dir, path) = temp_rust_file(code);
        let mut gtw = GnawTreeWriter::new(&path).unwrap();

        let result = gtw.edit(
            EditOperation::Edit {
                node_path: "@fn:big".to_string(),
                content: "fn big() {}\n".to_string(),
            },
            false,
        );

        let msg = format!(
            "{}",
            result.expect_err("Guardian should block a drastic reduction")
        );
        assert!(
            msg.contains("GUARDIAN BLOCK"),
            "expected GUARDIAN BLOCK, got: {}",
            msg
        );
    }

    #[test]
    fn guardian_allows_equal_size_edit() {
        let code = concat!(
            "fn small() {\n",
            "    // alpha comment\n",
            "    let x = 1;\n",
            "}\n",
        );
        let (_dir, path) = temp_rust_file(code);
        let mut gtw = GnawTreeWriter::new(&path).unwrap();

        let result = gtw.edit(
            EditOperation::Edit {
                node_path: "@fn:small".to_string(),
                content: "fn small() {\n    // beta comment\n    let x = 1;\n}\n".to_string(),
            },
            false,
        );

        assert!(
            result.is_ok(),
            "benign edit should pass: {:?}",
            result.err()
        );
    }

    /// Fas 3: an edit whose content equals the node content must be
    /// rejected loudly (finding #14 class) and write nothing.
    #[test]
    fn edit_noop_is_rejected() {
        let code = "fn small() {\n    let x = 1;\n}\n";
        let (_dir, path) = temp_rust_file(code);
        let mut gtw = GnawTreeWriter::new(&path).unwrap();
        let current = gtw.show_node("@fn:small").unwrap();
        let before = std::fs::read_to_string(&path).unwrap();

        let result = gtw.edit(
            EditOperation::Edit {
                node_path: "@fn:small".to_string(),
                content: current,
            },
            false,
        );

        let msg = format!(
            "{}",
            result.expect_err("identical content must be rejected as NO-OP")
        );
        assert!(msg.contains("NO-OP"), "expected NO-OP, got: {}", msg);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            before,
            "file must be untouched after a NO-OP rejection"
        );
    }

    /// Fas 3: a successful edit produces a receipt proving bytes landed
    /// on disk (verified) and how much changed (bytes_changed).
    #[test]
    fn edit_receipt_reports_verified_bytes() {
        let code = "fn small() {\n    let x = 1;\n}\n";
        let (_dir, path) = temp_rust_file(code);
        let mut gtw = GnawTreeWriter::new(&path).unwrap();

        gtw.edit(
            EditOperation::Edit {
                node_path: "@fn:small".to_string(),
                content: "fn small() {\n    let y = 2;\n}\n".to_string(),
            },
            false,
        )
        .expect("edit should succeed");

        let receipt = gtw
            .last_edit_receipt()
            .expect("successful edit must leave a receipt");
        assert!(receipt.verified, "disk content must match intent");
        assert!(
            receipt.changed,
            "content hash must differ after a real edit"
        );
        assert!(receipt.bytes_changed > 0, "bytes must have changed");
        assert!(
            !receipt.transaction_id.is_empty(),
            "receipt must carry the transaction id"
        );
    }

    /// Fas 5.2a: the RULES GUARDIAN BLOCK message must name the rule and
    /// line so an agent can locate the violation without re-linting.
    #[test]
    fn rules_block_names_rule_and_line() {
        let code = "fn small() {\n    let x = 1;\n}\n";
        let (_dir, path) = temp_rust_file(code);
        let mut gtw = GnawTreeWriter::new(&path).unwrap();

        let result = gtw.edit(
            EditOperation::Edit {
                node_path: "@fn:small".to_string(),
                content: "fn small() {\n    let a = 0;\n    a = a;\n}\n".to_string(),
            },
            false,
        );

        let msg = format!("{}", result.expect_err("self-assignment must be blocked"));
        assert!(msg.contains("RULES GUARDIAN BLOCK"), "got: {}", msg);
        assert!(msg.contains("rust_self_assignment"), "got: {}", msg);
        assert!(msg.contains("lint --fix"), "got: {}", msg);
    }

    /// Fas 1: an edit that removes a guard AND changes the comparison is
    /// blocked via context-aware severity upgrade, even though the file
    /// parses and the heuristics alone would not reach Critical.
    #[test]
    fn guardian_blocks_guard_removal_with_operator_change() {
        let code =
            "fn f(a: usize, b: usize) -> usize {\n    if a < b && a > 0 { a } else { b }\n}\n";
        let (_dir, path) = temp_rust_file(code);
        let mut gtw = GnawTreeWriter::new(&path).unwrap();

        let result = gtw.edit(
            EditOperation::Edit {
                node_path: "@fn:f".to_string(),
                content: "fn f(a: usize, b: usize) -> usize {\n    if a != b { a } else { b }\n}\n"
                    .to_string(),
            },
            false,
        );

        let msg = format!(
            "{}",
            result.expect_err("guard removal + operator change must block")
        );
        assert!(msg.contains("GUARDIAN BLOCK"), "got: {}", msg);
        assert!(msg.contains("condition"), "delta message missing: {}", msg);
    }

    /// Fas 1: a lone operator change (possible legit bugfix) must NOT be
    /// blocked — it surfaces as a warning/notice only (Gemini feedback:
    /// avoid --force fatigue for valid changes).
    /// Fas 2: a full regeneration that drops the bounds guard is blocked
    /// by the invariant contract even though the tree shape changed.
    #[test]
    fn guardian_contract_blocks_regeneration_losing_guard() {
        let code = "fn get(items: &[u8], i: usize) -> u8 {\n    if i < items.len() {\n        items[i]\n    } else {\n        0\n    }\n}\n";
        let (_dir, path) = temp_rust_file(code);
        let mut gtw = GnawTreeWriter::new(&path).unwrap();

        let result = gtw.edit(
            EditOperation::Edit {
                node_path: "@fn:get".to_string(),
                content: "fn get(items: &[u8], i: usize) -> u8 {\n    items[i]\n}\n".to_string(),
            },
            false,
        );

        let msg = format!(
            "{}",
            result.expect_err("regeneration losing the guard must block")
        );
        assert!(msg.contains("GUARDIAN BLOCK"), "got: {}", msg);
        assert!(
            msg.contains("Invariant contract"),
            "contract message missing: {}",
            msg
        );
    }

    #[test]
    fn lone_operator_change_is_allowed() {
        let code =
            "fn f(a: usize, b: usize) -> usize {\n    if a < b && a > 0 { a } else { b }\n}\n";
        let (_dir, path) = temp_rust_file(code);
        let mut gtw = GnawTreeWriter::new(&path).unwrap();

        let result = gtw.edit(
            EditOperation::Edit {
                node_path: "@fn:f".to_string(),
                content: "fn f(a: usize, b: usize) -> usize {\n    if a != b && a > 0 { a } else { b }\n}\n"
                    .to_string(),
            },
            false,
        );

        assert!(
            result.is_ok(),
            "lone operator change must not block: {:?}",
            result.err()
        );
    }
}
