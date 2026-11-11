use crate::parser::{get_parser, TreeNode};
use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub mod age;
pub mod alf;
pub mod anchor;

pub mod backup;
pub mod batch;
pub mod blast;
pub mod blueprint;
pub mod code_graph;
pub mod code_index;
pub mod code_index_coverage;
pub mod code_index_db;
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
pub mod project_root;
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
    /// Fas 4: impact report when an edit changed a symbol signature and
    /// the knowledge graph has callers. None otherwise (omitted, not an
    /// error — missing index is normal).
    last_impact: Option<serde_json::Value>,
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
    /// B1(a): the node path AFTER the edit, re-resolved from the fresh
    /// tree. Content-replacement keeps the ordinal position, so this is
    /// normally the input path — structural multi-item content is where
    /// it diverges. Sibling paths AFTER the node may shift (B1).
    pub new_node_path: String,
    /// B1(a): content hash of the edited node post-write — the freshness
    /// key for consumers replaying stored paths (compare before trusting).
    pub node_content_hash: String,
}

/// B3 (GTW_RAPPORT_2026-11-01): outline skeleton — named structure only.
/// Comments, use/import statements, attributes and unnamed siblings
/// collapse into a count, so the big-file answer is orientation instead
/// of a 289-row flat dump. `outline=false` keeps the raw everything.
/// Returns (rendered text, visible nodes, collapsed nodes).
pub(crate) fn skeleton_outline(
    tree: &TreeNode,
    max_depth: usize,
    outline: bool,
) -> (String, usize, usize) {
    const NODE_LIMIT: usize = 500;
    let mut out = String::new();
    let mut visible = 0usize;
    let mut collapsed = 0usize;

    fn is_noise(n: &TreeNode) -> bool {
        let t = n.node_type.as_str();
        t.contains("comment")
            || t == "use_declaration"
            || t == "import_statement"
            || t == "import_from_statement"
            || t == "attribute_item"
            || t.contains("empty")
    }

    fn build(
        n: &TreeNode,
        out: &mut String,
        d: usize,
        md: usize,
        visible: &mut usize,
        collapsed: &mut usize,
        outline: bool,
    ) {
        if *visible >= NODE_LIMIT {
            return;
        }
        if outline && is_noise(n) {
            *collapsed += 1;
            return;
        }
        if outline && d > 0 && n.get_name().is_none() {
            *collapsed += 1;
            return;
        }
        if d > md {
            return;
        }
        *visible += 1;
        out.push_str(&format!(
            "{}{} [{}] {}\n",
            "  ".repeat(d),
            n.path,
            n.node_type,
            n.get_name().unwrap_or_default()
        ));
        if *visible == NODE_LIMIT {
            out.push_str("... (limit of 500 nodes reached)\n");
            return;
        }
        for c in &n.children {
            build(c, out, d + 1, md, visible, collapsed, outline);
        }
    }

    build(
        tree,
        &mut out,
        0,
        max_depth,
        &mut visible,
        &mut collapsed,
        outline,
    );
    (out, visible, collapsed)
}

/// B2 (GTW_RAPPORT_2026-11-01): partial-replacement detector — if the OLD
/// node's distinctive content ends up AFTER the new content's last line,
/// the edit degraded to insertion (duplicate definitions downstream while
/// the tool reported success). Soundness rules: distinctive = trimmed,
/// ≥ 12 chars (braces/one-liners too generic), absent from the new
/// content AND absent from the original file OUTSIDE the node's span —
/// lines that already lived before/after the node are other code, not
/// remnants. Returns the remnant lines when found.
pub(crate) fn detect_replacement_remnant(
    modified: &str,
    original_source: &str,
    old_content: &str,
    new_content: &str,
    region_start_line: usize,
    node_end_line: usize,
) -> Option<Vec<String>> {
    let new_set: std::collections::HashSet<&str> = new_content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    // Original lines OUTSIDE the node's own span — other code, never remnants.
    let outside_original: std::collections::HashSet<&str> = original_source
        .lines()
        .enumerate()
        .filter(|(i, _)| *i + 1 < region_start_line || *i + 1 > node_end_line)
        .map(|(_, l)| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    let distinctive: Vec<&str> = old_content
        .lines()
        .map(|l| l.trim())
        .filter(|l| l.len() >= 12 && !new_set.contains(l) && !outside_original.contains(l))
        .collect();
    if distinctive.is_empty() {
        return None;
    }
    let last_new_trim = new_content
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())?;
    let lines: Vec<&str> = modified.lines().collect();
    let start_idx = lines
        .iter()
        .enumerate()
        .skip(region_start_line.saturating_sub(1))
        .find_map(|(i, l)| {
            if l.trim() == last_new_trim {
                Some(i)
            } else {
                None
            }
        })?;
    let remnants: Vec<String> = distinctive
        .iter()
        .filter(|d| lines.iter().skip(start_idx + 1).any(|l| l.trim() == **d))
        .map(|d| (*d).to_string())
        .collect();
    if remnants.is_empty() {
        None
    } else {
        Some(remnants)
    }
}

/// E_DUPLICATE_ATTRIBUTE (dogfood finding 2026-11-10): when the lines
/// directly above the target node are attributes (`#[...]`) and the
/// replacement content re-includes the SAME attribute line, the write
/// stacks a second copy. Valid Rust — the syntax gate cannot see it — but
/// doubled behaviour (a doubled `#[test]` registers the test twice: the
/// diagnostic-round trap this guard exists to kill). Loud rejection,
/// nothing written. The fix is mechanical: node content starts at the
/// item — attributes live OUTSIDE the node, so drop them from the
/// replacement. Attributes are scanned upward while they are directly
/// adjacent; a doc comment or blank line stops the scan (conservative —
/// attribute-above-doc layouts are missed rather than false-positived).
pub(crate) fn check_duplicate_attribute_above(
    source_lines: &[&str],
    node_start_line: usize,
    new_content: &str,
) -> Result<()> {
    let mut attr_above: Vec<String> = Vec::new();
    let mut idx = node_start_line.saturating_sub(2);
    loop {
        if idx >= source_lines.len() {
            break;
        }
        let trimmed = source_lines[idx].trim();
        if trimmed.starts_with("#[") {
            attr_above.push(trimmed.to_string());
            if idx == 0 {
                break;
            }
            idx -= 1;
        } else {
            break;
        }
    }
    if attr_above.is_empty() {
        return Ok(());
    }
    let first_new = new_content.lines().map(str::trim).find(|l| !l.is_empty());
    if let Some(first_new) = first_new {
        if attr_above.iter().any(|a| a == first_new) {
            anyhow::bail!(
                "DUPLICATE ATTRIBUTE: the replacement content re-includes '{}' which is                  already attached directly above the target node — writing it would stack                  a second copy (valid syntax, doubled behaviour, e.g. a doubled #[test]                  registers the test twice). Nothing written — E_DUPLICATE_ATTRIBUTE                  (dogfood finding 2026-11-10). Fix: node content starts at the item;                  attributes live OUTSIDE the node — drop the attribute line from the                  replacement.",
                first_new
            );
        }
    }
    Ok(())
}

impl GnawTreeWriter {
    pub fn new(file_path: &str) -> Result<Self> {
        Self::new_with_boundary(file_path, None)
    }

    /// Boundary-aware constructor (2026-11-04, "en bas").
    ///
    /// The transaction log — and therefore the backups and the session marker —
    /// is anchored at the caller's workspace root, and discovery NEVER walks
    /// above it. Without a boundary the legacy discovery applies, so CLI/editor
    /// callers keep today's behaviour.
    pub fn new_with_boundary(file_path: &str, boundary: Option<&Path>) -> Result<Self> {
        let path = Path::new(file_path);
        let source_code =
            fs::read_to_string(path).context(format!("Failed to read file: {}", file_path))?;

        let parser = get_parser(path)?;
        let tree = parser.parse(&source_code)?;

        // Initialize transaction log for the project root
        // Use find_project_root to ensure we log to the correct centralized location
        let project_root = find_project_root_in(path, boundary);
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
            last_impact: None,
        })
    }

    /// Read-path constructor (partial-grace): like `new`, but uses the
    /// parser's `parse_lenient` — a localized syntax error yields a
    /// PARTIAL tree plus the warning instead of refusing the file. Only
    /// read-only MCP tools (analyze/skeleton/list/read) opt in; editors
    /// (`new` + edit/insert/quick-replace) stay strict. Parsers without
    /// a lenient override behave exactly like `new`.
    pub fn new_lenient(file_path: &str) -> Result<(Self, Option<crate::parser::SyntaxError>)> {
        Self::new_lenient_with_boundary(file_path, None)
    }

    /// Boundary-aware read-path constructor — same contract as
    /// `new_with_boundary`: the transaction log/marker never escapes the
    /// caller's workspace root.
    pub fn new_lenient_with_boundary(
        file_path: &str,
        boundary: Option<&Path>,
    ) -> Result<(Self, Option<crate::parser::SyntaxError>)> {
        let path = Path::new(file_path);
        let source_code =
            fs::read_to_string(path).context(format!("Failed to read file: {}", file_path))?;

        let parser = get_parser(path)?;
        let (tree_result, warning) = parser.parse_lenient(&source_code);
        let tree = tree_result?;

        let project_root = find_project_root_in(path, boundary);
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
                last_impact: None,
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
        self.last_verdict = Some(crate::core::verdict_json(
            level,
            score,
            findings,
            suggestions,
        ));
    }

    /// Fas 4: impact report for the last edit (changed signature with
    /// known callers), or None when the edit did not change a signature,
    /// was rejected, or the index is missing.
    pub fn last_edit_impact(&self) -> Option<&serde_json::Value> {
        self.last_impact.as_ref()
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

    /// B1(b): declare the expected identity of the node at `node_path`
    /// BEFORE editing — a mismatch rejects with "PATH STALE" instead of
    /// writing the wrong node. Paths are positional (B1,
    /// GTW_RAPPORT_2026-11-01): they invalidate after every structural
    /// edit, so callers replaying stored paths MUST verify first.
    /// `expect_name` compares the node's identifier, `expect_kind` the
    /// raw tree-sitter kind (e.g. "function_item", "struct_item").
    pub fn expect_node(
        &self,
        node_path: &str,
        expect_name: Option<&str>,
        expect_kind: Option<&str>,
    ) -> Result<()> {
        // Canonical resolver: numeric dot-paths AND @kind:name anchors.
        // Anchors are self-validating (resolved BY name) — the stale-path
        // guard matters for positional paths, which is exactly what the
        // callers replaying stored paths use.
        let node = self.resolve_path(node_path).ok_or_else(|| {
            anyhow::anyhow!(
                "Node not found at path: {} — node paths are positional and invalidate after structural edits; re-run search_nodes.",
                node_path
            )
        })?;
        let name = node.get_name().unwrap_or_default();
        if let Some(want) = expect_name {
            if name != want {
                anyhow::bail!(
                    "PATH STALE: node at path {} is '{}' (kind '{}'), expected name '{}'. Node paths are positional and invalidate after structural edits — re-run search_nodes and retry with the fresh path.",
                    node_path,
                    name,
                    node.node_type,
                    want
                );
            }
        }
        if let Some(want) = expect_kind {
            if node.node_type != want {
                anyhow::bail!(
                    "PATH STALE: node at path {} has kind '{}', expected kind '{}'. Re-run search_nodes and retry with the fresh path.",
                    node_path,
                    node.node_type,
                    want
                );
            }
        }
        Ok(())
    }

    // Test indent insert
    pub fn edit(&mut self, operation: EditOperation, force: bool) -> Result<()> {
        // Calculate before hash
        let before_hash = calculate_content_hash(&self.source_code);
        let pre_edit_source = self.source_code.clone();
        self.last_verdict = None;
        self.last_impact = None;

        // J1 (2026-11-08, rapport 2026-11-05): JSON redigeras SEMANTISKT —
        // den legacy-JSON-parsern är positionsblind (alla noder rad 1..1,
        // kolumn 0), så edit_node_at_path:s rad-splicing kan aldrig fungera
        // för JSON (båda citat-formerna nekades: "expected ident" med rå
        // text, "trailing characters" med citerad). Värde-nivån är rätt
        // abstraktion för JSON: navigera nod-pathen, ersätt värdet,
        // re-serialisera hela dokumentet (pretty 2-space; preserve_order
        // håller nyckelordningen).
        let is_json = self.is_json_document();

        // B1(a): the receipt carries the addressed path — consumers diff
        // intent (expect) vs outcome (new path + hash).
        let target_for_receipt = match &operation {
            EditOperation::Edit { node_path, .. } => Some(node_path.clone()),
            EditOperation::Delete { node_path } => Some(node_path.clone()),
            _ => None,
        };

        let modified_code = match &operation {
            EditOperation::Edit { node_path, content } => {
                let resolved = self
                    .resolve_path(node_path)
                    .context(format!("Could not resolve node path: {}", node_path))?;
                let resolved_path = resolved.path.clone();
                let resolved_content = resolved.content.clone();
                if resolved_content == *content {
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
                    // T4 (issue log 2026-10-19): a NO-OP is a successful
                    // edit that changed nothing — Ok + changed=false, not
                    // an Err. Callers branch on the receipt; the verdict
                    // and the history row still tell the story. (The MCP
                    // edit_node tool already shielded its users via the
                    // idempotent-retry path — this harmonizes CLI,
                    // library and batch flows with that behavior.)
                    let transaction_id = self.transaction_log.log_transaction(
                        OperationType::Edit,
                        PathBuf::from(&self.file_path),
                        Some(resolved_path.clone()),
                        Some(resolved_content.clone()),
                        Some(resolved_content.clone()),
                        "NO-OP (identical content) — nothing written".to_string(),
                        HashMap::new(),
                    )?;
                    self.last_transaction_id = Some(transaction_id.clone());
                    self.last_receipt = Some(EditReceipt {
                        transaction_id,
                        verified: true,
                        changed: false,
                        bytes_changed: 0,
                        new_node_path: resolved_path.clone(),
                        node_content_hash: crate::core::transaction_log::calculate_content_hash(
                            &resolved_content,
                        ),
                    });
                    return Ok(());
                }
                if is_json {
                    self.json_semantic_edit(&resolved.path, content)?
                } else {
                    self.edit_node_at_path(&resolved.path, content)?
                }
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

                // Fas 4: signature-aware impact via the knowledge graph.
                // Missing index ⇒ None ⇒ field omitted, never an error.
                // (Runs before set_verdict: resolved borrows self, so it
                // must not be used after the mutable verdict write.)
                self.last_impact = signature_impact_for(&self.file_path, resolved, &report.deltas);

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
                        return Err(anyhow::anyhow!("🛑 GUARDIAN BLOCK: This edit removes critical logic or structure.\nMessages: {}\nOverride deliberately after confirming the flagged reduction is intentional: CLI `--force`, MCP edit_node `force: true`.", report.messages.join(", ")));
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

        // B1(a): re-resolve the addressed node in the FRESH tree — content
        // replacement keeps the ordinal position, so this path is
        // authoritative for the next edit; the hash is the freshness key
        // for stored-path consumers (compare before trusting).
        let (new_node_path, node_content_hash) = match &target_for_receipt {
            Some(p) => match self.find_node_by_path(&self.tree, p) {
                Some(found) => (
                    found.path.clone(),
                    crate::core::transaction_log::calculate_content_hash(&found.content),
                ),
                None => (p.clone(), String::new()),
            },
            None => (String::new(), String::new()),
        };
        let receipt = EditReceipt {
            transaction_id,
            verified,
            changed: before_hash != after_hash,
            bytes_changed: count_changed_bytes(&pre_edit_source, &self.source_code),
            new_node_path,
            node_content_hash,
        };
        let verified = receipt.verified;
        self.last_receipt = Some(receipt);

        if !verified {
            return Err(anyhow::anyhow!(
                "Post-write verification FAILED: file on disk does not match the intended content. The write may not have landed — inspect with history/doctor before retrying."
            ));
        }

        // Phase 4: staleness bookkeeping — the store (when it exists)
        // learns which file was just edited (decision #8: open-existing,
        // never create; best-effort, never fails the edit).
        let proj_root = find_project_root(Path::new(&self.file_path));
        crate::core::code_index_db::mark_stale(&proj_root, &self.file_path);

        Ok(())
    }

    /// Is this document a JSON file? JSON edits navigate **key names**
    /// (`json_semantic_edit`), everything else uses the numeric/text split.
    ///
    /// ONE predicate on purpose: apply and preview each had their own copy of
    /// this test, and preview's copy was missing — so `--preview` rendered the
    /// raw split while the apply rewrote the value semantically. A preview of a
    /// document that will never exist is worse than no preview (field finding
    /// 2026-11-10).
    fn is_json_document(&self) -> bool {
        Path::new(&self.file_path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("json"))
            .unwrap_or(false)
    }

    pub fn preview_edit(&self, operation: EditOperation) -> Result<String> {
        match operation {
            EditOperation::Edit { node_path, content } => {
                let resolved = self
                    .resolve_path(&node_path)
                    .context(format!("Could not resolve node path: {}", node_path))?;
                // Same branch as the real edit — see is_json_document().
                if self.is_json_document() {
                    self.json_semantic_edit(&resolved.path, &content)
                } else {
                    self.edit_node_at_path(&resolved.path, &content)
                }
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

    /// F2 (batt 2026-10-13): doc-kommentarer (///) är EGNA syskon-noder i
    /// tree-sitter-trädet — de är inte barn till funktionen de dokumenterar.
    /// Vid replace/delete av en definition ska därför det omedelbart
    /// föregående ///-blocket följa med i regionen, annars blir gamla doc
    /// kvar (dubletter) när nytt innehåll bär egen doc. Returnerar 1-baserad
    /// första raden i doc-blocket (eller nodens egen start vid inget block).
    fn doc_block_start_line(&self, node: &TreeNode) -> usize {
        let lines: Vec<&str> = self.source_code.lines().collect();
        let mut first = node.start_line;
        let mut idx = node.start_line as isize - 2;
        while idx >= 0 {
            let trimmed = lines.get(idx as usize).map(|l| l.trim()).unwrap_or("");
            if trimmed.starts_with("///") {
                first = idx as usize + 1;
                idx -= 1;
            } else {
                break;
            }
        }
        first
    }

    /// Identitets-eko (F3, batt 2026-10-13): numeriska paths är POSITIONER
    /// som skiftar vid varje edit — en modell som återanvänt en gammal path
    /// kan träffa fel nod (bevis: "11" resolve:ade till doc_comment efter en
    /// tidigare edit, och ersatte den tyst i stället för staticen). Verktygs-
    /// svaret ska kunna redovisa VAD som faktiskt resolve:ades: path, typ,
    /// namn. None = path resolve:ar inte (fel-redovisningen tar vid).
    pub fn describe_node(&self, query: &str) -> Option<(String, String, Option<String>)> {
        let node = self.resolve_path(query)?;
        Some((node.path.clone(), node.node_type.clone(), node.get_name()))
    }

    /// J1 (2026-11-08): semantisk JSON-redigering — rad-splicing är
    /// meningslöst för en positionsblind parser, så JSON-edit går på
    /// VÄRDE-nivå: nod-pathen navigeras (nycklar + array-index, samma
    /// form som JsonParser bygger), värdet ersätts, hela dokumentet
    /// re-serialiseras deterministiskt (pretty 2-space, preserve_order
    /// bevarar nyckelordningen). Sträng-noder tar BÅDA formerna:
    /// citerad JSON (`"text"`) tolkas som JSON-värde, ociterad text
    /// blir literal-strängen (ergonomin ur rapporten — ingen agent ska
    /// behöva gissa citat-form).
    ///
    /// **INTERIM (användaren 2026-11-08):** detta är en tillfällig lösning.
    /// JSON ska ha en riktig AST-parser med span som de andra språken
    /// (tree-sitter-json eller egen span-spårare) — då återfår JSON
    /// kirurgisk byte-precision och re-renderingen försvinner. Kostnaden
    /// idag: formateringsnormalisering (pretty 2-space) vid avvikande
    /// underlag. Parkerad uppföljning, se CHANGELOG 0.23.1.
    fn json_semantic_edit(&self, node_path: &str, new_content: &str) -> Result<String> {
        let mut root: serde_json::Value = serde_json::from_str(&self.source_code)
            .map_err(|e| anyhow::anyhow!("JSON-edit: källfilen är inte giltig JSON: {e}"))?;

        // Tom path = roten (JsonParser ger roten id "").
        if node_path.is_empty() {
            let v = Self::json_new_value(new_content, true)
                .ok_or_else(|| Self::json_content_error(new_content, "dokumentet (roten)"))?;
            return Ok(Self::json_render(&v));
        }

        // Navigera till föräldern, segment för segment.
        let segments: Vec<&str> = node_path.split('.').collect();
        let (last, parents) = segments
            .split_last()
            .expect("icke-tom path har minst ett segment");
        let mut cursor = &mut root;
        for (i, seg) in parents.iter().enumerate() {
            cursor = match cursor {
                serde_json::Value::Object(map) => map.get_mut(*seg).ok_or_else(|| {
                    anyhow::anyhow!(
                        "JSON-edit: segment {} ('{}') finns inte — pathen pekar ingenstans",
                        i + 1,
                        seg
                    )
                })?,
                serde_json::Value::Array(arr) => {
                    let idx: usize = seg.parse().map_err(|_| {
                        anyhow::anyhow!(
                            "JSON-edit: segment {} ('{}') är inte ett array-index",
                            i + 1,
                            seg
                        )
                    })?;
                    arr.get_mut(idx).ok_or_else(|| {
                        anyhow::anyhow!(
                            "JSON-edit: array-index {} utanför gränsen (segment {})",
                            idx,
                            i + 1
                        )
                    })?
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "JSON-edit: segment {} ('{}') går genom ett skalärt värde",
                        i + 1,
                        seg
                    ))
                }
            };
        }

        // Målets nuvarande typ styr vad ociterat innehåll får vara.
        let target_is_string = match cursor {
            serde_json::Value::Object(map) => {
                matches!(map.get(*last), Some(serde_json::Value::String(_)))
            }
            serde_json::Value::Array(arr) => last
                .parse::<usize>()
                .ok()
                .and_then(|i| arr.get(i))
                .is_some_and(|v| v.is_string()),
            _ => false,
        };
        let new_value = Self::json_new_value(new_content, target_is_string)
            .ok_or_else(|| Self::json_content_error(new_content, "noden"))?;

        match cursor {
            serde_json::Value::Object(map) => {
                if !map.contains_key(*last) {
                    return Err(anyhow::anyhow!(
                        "JSON-edit: nyckeln '{}' finns inte i objektet — edit ändrar, den skapar inte (insert_node för tillägg)",
                        last
                    ));
                }
                map.insert(last.to_string(), new_value);
            }
            serde_json::Value::Array(arr) => {
                let idx: usize = last.parse().map_err(|_| {
                    anyhow::anyhow!("JSON-edit: '{}' är inte ett array-index", last)
                })?;
                if idx >= arr.len() {
                    return Err(anyhow::anyhow!(
                        "JSON-edit: array-index {} utanför gränsen (längd {})",
                        idx,
                        arr.len()
                    ));
                }
                arr[idx] = new_value;
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "JSON-edit: föräldern till '{}' är ett skalärt värde",
                    last
                ))
            }
        }
        Ok(Self::json_render(&root))
    }

    /// Nytt värde ur edit-innehållet: giltig JSON först (citerade
    /// strängar, tal, objekt, arrayer); ociterad text = literal sträng
    /// NÄR målet är en sträng-nod (allow_string).
    fn json_new_value(content: &str, allow_string: bool) -> Option<serde_json::Value> {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(content) {
            return Some(v);
        }
        if allow_string {
            return Some(serde_json::Value::String(content.to_string()));
        }
        None
    }

    fn json_content_error(content: &str, target: &str) -> anyhow::Error {
        anyhow::anyhow!(
            "JSON-edit: innehållet är inte giltig JSON och målet ({}) är inte en sträng-nod \
             (sträng-noder tar ociterad text som literal; objekt/array/tal kräver giltig JSON: \
             {{…}}, […], 42, true). Innehållet börjar med: '{}'",
            target,
            content.chars().take(40).collect::<String>()
        )
    }

    /// Deterministisk rendering: pretty 2-space + avslutande radbrytning
    /// (samma form som flödesfilerna skrivs i).
    fn json_render(v: &serde_json::Value) -> String {
        let mut s = serde_json::to_string_pretty(v).unwrap_or_default();
        if !s.ends_with('\n') {
            s.push('\n');
        }
        s
    }

    /// Re-join split lines WITHOUT losing the file's final newline.
    ///
    /// `str::lines()` drops the trailing terminator and `join("\n")` never puts
    /// it back, so every edit silently removed the last newline: git then
    /// reported "no newline at end of file" on content nobody had touched, and
    /// the byte-level contract behind `bytes_changed` was off by one. Restore
    /// exactly the terminator the source used (LF or CRLF).
    fn join_preserving_newline<S: AsRef<str>>(lines: &[S], source: &str) -> String {
        let mut out = lines
            .iter()
            .map(|l| l.as_ref())
            .collect::<Vec<&str>>()
            .join("\n");
        let term = if source.ends_with("\r\n") {
            "\r\n"
        } else if source.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        if !term.is_empty() && !out.ends_with(term) {
            out.push_str(term);
        }
        out
    }

    fn edit_node_at_path(&self, node_path: &str, new_content: &str) -> Result<String> {
        let node = self
            .find_node_by_path(&self.tree, node_path)
            .context(format!("Node not found at path: {}", node_path))?;

        let lines: Vec<&str> = self.source_code.lines().collect();

        // F2: när nytt innehåll bär egen doc ska gamla ///-blocket swallas.
        let region_start = if new_content.trim_start().starts_with("///") {
            self.doc_block_start_line(node)
        } else {
            node.start_line
        };

        // E_DUPLICATE_ATTRIBUTE funnel hook — see check_duplicate_attribute_above.
        check_duplicate_attribute_above(&lines, node.start_line, new_content)?;

        // If we have column information, use it for surgical precision
        let modified = if node.start_col > 0 && node.end_col > 0 {
            let mut new_lines: Vec<String> = Vec::new();

            // Lines before the node's start line
            for i in 0..region_start - 1 {
                if i < lines.len() {
                    new_lines.push(lines[i].to_string());
                }
            }

            // Handle the start line (with prefix) — endast när regionen börjar
            // på nodens egen rad; en utvidgad region (doc-rader) är hel-rader.
            let start_line_idx = region_start - 1;
            let prefix: String = if region_start == node.start_line {
                let start_line_text = lines[start_line_idx];
                start_line_text.chars().take(node.start_col - 1).collect()
            } else {
                String::new()
            };

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

            Self::join_preserving_newline(&new_lines, &self.source_code)
        } else {
            let mut new_lines: Vec<String> = Vec::new();

            // Lines before the node (F2: inklusive swalltat doc-block)
            for i in 0..region_start - 1 {
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

            Self::join_preserving_newline(&new_lines, &self.source_code)
        };

        // B2 (GTW_RAPPORT 2026-11-01): partial-replacement guard — the
        // old node's distinctive content must not survive after the new
        // content. Loud rejection, nothing written (the funnel converts
        // this to a rejection with the B2 class named).
        if let Some(remnants) = detect_replacement_remnant(
            &modified,
            &self.source_code,
            &node.content,
            new_content,
            region_start,
            node.end_line,
        ) {
            anyhow::bail!(
                "PARTIAL REPLACEMENT detected: the old node's content survives after the replacement ({} distinctive line(s), first: '{}'). Nothing written — B2 class (GTW_RAPPORT_2026-11-01); inspect with preview_edit and re-check the node path/content.",
                remnants.len(),
                remnants[0]
            );
        }
        Ok(modified)
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
                // Documented modes are 0 (first / after opening brace),
                // 1 (append to parent / before closing brace) and 2 (after
                // UI properties). Larger values address a sibling slot:
                // `idx - 3` is the 0-based child to insert BEFORE, at that
                // child's start boundary — never inside a sibling. A
                // position BEYOND the child count is the common "append at
                // the end" encoding (Motor2's GTW bridge sends
                // children.len() + 1) and must land AFTER the LAST child.
                // History: this branch previously used
                // `children[idx - 3].end_line` as a raw insert index, which
                // landed in the MIDDLE of a sibling for single-line or token
                // children (anonymous `fn`/`identifier` nodes): Motor2 batt
                // 2026-10-13, and triage 2026-10-31 (T2), where position 3
                // of 5 root fns nested the new fn inside the first one.
                if parent.children.is_empty() {
                    parent.start_line.saturating_sub(1)
                } else if idx > parent.children.len() {
                    parent.children[parent.children.len() - 1].end_line
                } else {
                    // start_line is 1-based, so start_line - 1 is the
                    // 0-based insert index of the line BEFORE the sibling.
                    parent.children[idx - 3].start_line.saturating_sub(1)
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

        Ok(Self::join_preserving_newline(&new_lines, &self.source_code))
    }

    fn delete_node_at_path(&self, node_path: &str) -> Result<String> {
        let node = self
            .find_node_by_path(&self.tree, node_path)
            .context(format!("Node not found at path: {}", node_path))?;

        // OBS: doc-kommentarer ovanför målet raderas INTE här — det sköts av
        // Motor2-sidans F2d (attached_comment_paths, motor2-gtw/src/lib.rs)
        // som äger hela kedjan målet → bifogade kommentarer. En sanning per
        // regel (§45): core håller ren region-logik, F2d håller kedjan.
        let lines: Vec<&str> = self.source_code.lines().collect();
        let start_idx = node.start_line - 1;
        let end_idx = node.end_line;
        // Vakt (Motor2-batt 2026-10-14, panic mod.rs:998): en nod-path från
        // en STALE tree (filen har krympt sedan skeleton/analyze) panikade
        // med byte/rad-slice out of range — ärligt fel i stället: tvinga
        // fram re-analyze, korrumpiera aldrig källan.
        if start_idx >= lines.len() || end_idx > lines.len() {
            anyhow::bail!(
                "Node range ({}-{}) is out of bounds for the current source ({} lines) — the tree is stale; re-run analyze/list_nodes to get fresh node paths",
                node.start_line, node.end_line, lines.len()
            );
        }

        let new_lines: Vec<_> = lines[..start_idx]
            .iter()
            .chain(lines[end_idx..].iter())
            .copied()
            .collect();

        Ok(Self::join_preserving_newline(&new_lines, &self.source_code))
    }
    pub fn get_source(&self) -> &str {
        &self.source_code
    }
}

/// Helper function to find the project root
/// Searches upwards for .gnawtreewriter_session.json or .git
pub fn find_project_root(start_path: &Path) -> PathBuf {
    find_project_root_in(start_path, None)
}

/// Boundary-bounded project-root discovery (2026-11-04, "en bas").
///
/// Same marker rules (session file / `.git`) as `find_project_root`, but the
/// search NEVER walks above `boundary`. A caller that owns a workspace root
/// (Motor2: the session's workspace root) states it here, so a stray marker
/// file in a parent directory cannot capture the project — the class that put
/// 172 backups in `Gnaw-Software/` instead of in the project the agent worked
/// in (the lying rollback track, 2026-10-28). Discovery without a boundary
/// keeps the legacy behaviour for CLI/editor callers.
pub fn find_project_root_in(start_path: &Path, boundary: Option<&Path>) -> PathBuf {
    let mut current = if start_path.is_file() {
        start_path.parent().unwrap_or(Path::new(".")).to_path_buf()
    } else {
        start_path.to_path_buf()
    };

    // Try to make it absolute if possible, but don't fail if we can't
    if let Ok(abs) = fs::canonicalize(&current) {
        current = abs;
    }

    // The ceiling: canonicalised once so symlinked/relative boundaries match.
    let stop = boundary.map(|b| fs::canonicalize(b).unwrap_or_else(|_| b.to_path_buf()));
    let start = current.clone();

    loop {
        // Check for session file or git
        if current.join(".gnawtreewriter_session.json").exists() || current.join(".git").exists() {
            return current;
        }

        if let Some(stop) = &stop {
            if &current == stop {
                // Boundary reached without a marker: the boundary IS the root.
                return current;
            }
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

/// Canonical shape of a rejection verdict (Fas 5.1) — shared by
/// GnawTreeWriter::set_verdict and batch validation so agents get the
/// identical edit_verdict contract from both paths.
pub(crate) fn verdict_json(
    level: &str,
    score: f32,
    findings: Vec<serde_json::Value>,
    suggestions: Vec<String>,
) -> serde_json::Value {
    serde_json::json!({
        "level": level,
        "score": score,
        "findings": findings,
        "suggestions": suggestions,
    })
}

/// Fas 4 shared helper: impact report when `deltas` contain a
/// SignatureChange on a named node with indexed callers, based on the
/// knowledge graph under the project root of `file_path`. None when the
/// edit does not change a signature, the node is unnamed, or the index
/// has no callers — omitted downstream, never an error.
pub(crate) fn signature_impact_for(
    file_path: &str,
    resolved: &TreeNode,
    deltas: &[crate::core::edit_delta::EditDelta],
) -> Option<serde_json::Value> {
    let signature_changed = deltas
        .iter()
        .any(|d| matches!(d.kind, crate::core::edit_delta::DeltaKind::SignatureChange));
    if !signature_changed {
        return None;
    }
    let name = resolved.get_name()?;
    let root = find_project_root(Path::new(file_path));
    let analyzer = crate::llm::ImpactAnalyzer::new_with_root(&root);
    let ir = analyzer.analyze_impact(&name, file_path).ok()?;
    let mut sites = Vec::new();
    for af in &ir.affected_files {
        for cp in &af.call_paths {
            sites.push(format!("{}:{}", af.file_path, cp));
        }
    }
    let callers = sites.len();
    if callers == 0 {
        return None;
    }
    let mut payload = serde_json::json!({
        "symbol": name,
        "callers": callers,
        "sites": sites,
    });
    // Fas 4.2 step 1: surface how much of the count is conservative
    // (unknown definition sites) so agents can scale their confidence.
    if ir.unresolved_calls > 0 {
        payload["unresolved"] = serde_json::json!(ir.unresolved_calls);
    }
    Some(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_tree(code: &str) -> TreeNode {
        let parser = crate::parser::get_parser(Path::new("x.rs")).unwrap();
        parser.parse_lenient(code).0.unwrap()
    }

    /// Field finding 2026-11-10: `--preview` for JSON rendered the raw text
    /// split while the apply went through `json_semantic_edit` — a preview of a
    /// document that would never exist. The invariant that pins the fix:
    /// **after applying an edit, previewing the same edit must show no change.**
    #[test]
    fn json_preview_and_apply_agree() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cfg.json");
        std::fs::write(&path, "{\n  \"name\": \"old\",\n  \"keep\": 1\n}\n").unwrap();

        let op = || EditOperation::Edit {
            node_path: "name".to_string(), // JSON navigates key names, not positions
            content: "\"new\"".to_string(),
        };

        let writer = GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
        let before = writer.get_source().to_string();
        let preview_before = writer.preview_edit(op()).unwrap();
        assert_ne!(
            preview_before, before,
            "preview of a pending change must show a change"
        );

        let mut writer = GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
        writer.edit(op(), false).unwrap();
        let applied = std::fs::read_to_string(&path).unwrap();
        assert!(
            applied.contains("\"new\""),
            "the apply wrote the value: {applied}"
        );

        // Fresh handle: the claim is about what a caller sees on disk.
        let reread = GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
        let preview_after = reread.preview_edit(op()).unwrap();
        assert_eq!(
            preview_after, applied,
            "preview and apply disagree for JSON — the preview showed a document that does not exist"
        );
    }

    fn temp_rust_file(code: &str) -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.rs");
        std::fs::write(&path, code).unwrap();
        let path_str = path.to_str().unwrap().to_string();
        (dir, path_str)
    }

    /// J1 (2026-11-08, rapport 2026-11-05): JSON-strängnoder ska kunna
    /// redigeras i BÅDA citat-formerna (ociterad = literal; citerad =
    /// JSON-värde) — förr nekades båda ("expected ident"/"trailing
    /// characters") eftersom rad-splicing aldrig kan fungera mot den
    /// positionsblinda JSON-parsern. Nu: värde-nivå + re-serialisering.
    #[test]
    fn json_edit_both_quote_forms_and_nesting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("flow.json");
        let original = concat!(
            "{\n",
            "  \"name\": \"gammal\",\n",
            "  \"steps\": [\n",
            "    {\"path\": \"/a\", \"expect_status\": 200}\n",
            "  ]\n",
            "}\n"
        );
        std::fs::write(&path, original).unwrap();
        let path_str = path.to_str().unwrap().to_string();

        // OCITERAD text → literal sträng (ergonomin ur rapporten).
        let mut w = GnawTreeWriter::new(&path_str).unwrap();
        w.edit(
            EditOperation::Edit {
                node_path: "name".into(),
                content: "ny rubrik".into(),
            },
            false,
        )
        .unwrap();
        let doc: serde_json::Value = serde_json::from_str(w.get_source()).unwrap();
        assert_eq!(doc["name"], serde_json::json!("ny rubrik"));

        // CITERAD JSON → tolkas som JSON-värde (samma resultat).
        let mut w = GnawTreeWriter::new(&path_str).unwrap();
        w.edit(
            EditOperation::Edit {
                node_path: "name".into(),
                content: "\"citerad form\"".into(),
            },
            false,
        )
        .unwrap();
        let doc: serde_json::Value = serde_json::from_str(w.get_source()).unwrap();
        assert_eq!(doc["name"], serde_json::json!("citerad form"));

        // Nästlad path med array-index.
        let mut w = GnawTreeWriter::new(&path_str).unwrap();
        w.edit(
            EditOperation::Edit {
                node_path: "steps.0.path".into(),
                content: "/b".into(),
            },
            false,
        )
        .unwrap();
        let doc: serde_json::Value = serde_json::from_str(w.get_source()).unwrap();
        assert_eq!(doc["steps"][0]["path"], serde_json::json!("/b"));

        // Tal-nod: giltig JSON krävs (ociterad text nekas med vägledning).
        let mut w = GnawTreeWriter::new(&path_str).unwrap();
        let err = w
            .edit(
                EditOperation::Edit {
                    node_path: "steps.0.expect_status".into(),
                    content: "tvåhundra".into(),
                },
                false,
            )
            .unwrap_err();
        assert!(err.to_string().contains("inte giltig JSON"), "{err}");
        assert!(err.to_string().contains("sträng-noder"), "{err}");
    }

    /// J1: nyckelordningen bevaras vid re-serialisering (preserve_order)
    /// — utan den vore varje JSON-edit diff-brus över hela filen.
    #[test]
    fn json_edit_preserves_key_order() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ordning.json");
        std::fs::write(
            &path,
            "{\n  \"zeta\": 1,\n  \"alpha\": 2,\n  \"middle\": {\n    \"z_last\": \"x\",\n    \"a_first\": \"y\"\n  }\n}\n",
        )
        .unwrap();
        let path_str = path.to_str().unwrap().to_string();
        let mut w = GnawTreeWriter::new(&path_str).unwrap();
        w.edit(
            EditOperation::Edit {
                node_path: "middle.z_last".into(),
                content: "ändrad".into(),
            },
            false,
        )
        .unwrap();
        let src = w.get_source();
        let zeta = src.find("\"zeta\"").unwrap();
        let alpha = src.find("\"alpha\"").unwrap();
        assert!(
            zeta < alpha,
            "nyckelordningen bevaras (zeta före alpha): {src}"
        );
        let z_last = src.find("\"z_last\"").unwrap();
        let a_first = src.find("\"a_first\"").unwrap();
        assert!(z_last < a_first, "nästlade nycklar orörda i ordning: {src}");
    }

    /// J1: edit ändrar ALDRIG struktur i smyg — okänd nyckel och utanför-
    /// gräns-index nekas högt (samma kontrakt som kod-språken: edit ≠ insert).
    #[test]
    fn json_edit_refuses_structural_sneak() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vakt.json");
        std::fs::write(&path, "{\"a\": [1, 2]}\n").unwrap();
        let path_str = path.to_str().unwrap().to_string();

        let mut w = GnawTreeWriter::new(&path_str).unwrap();
        let err = w
            .edit(
                EditOperation::Edit {
                    node_path: "saknad_nyckel".into(),
                    content: "\"x\"".into(),
                },
                false,
            )
            .unwrap_err();
        // Okänd nyckel nekas HÖGT — antingen i path-upplösningen (trädet
        // känner bara existerande noder) eller i den semantiska editen.
        let m = err.to_string();
        assert!(
            m.contains("finns inte") || m.contains("Could not resolve"),
            "okänd nyckel ska nekas högt: {m}"
        );

        let mut w = GnawTreeWriter::new(&path_str).unwrap();
        let err = w
            .edit(
                EditOperation::Edit {
                    node_path: "a.5".into(),
                    content: "9".into(),
                },
                false,
            )
            .unwrap_err();
        let m = err.to_string();
        assert!(
            m.contains("utanför gränsen") || m.contains("Could not resolve"),
            "utanför-gräns-index ska nekas högt: {m}"
        );
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

    /// T4 (issue log 2026-10-19): an edit whose content equals the node
    /// content is a SUCCESSFUL no-change — Ok + changed=false, verdict
    /// notice set, nothing written, history row recorded.
    #[test]
    fn edit_noop_reports_changed_false() {
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

        assert!(
            result.is_ok(),
            "NO-OP must not be an error: {:?}",
            result.err()
        );
        let receipt = gtw.last_receipt.clone().expect("receipt present");
        assert!(!receipt.changed, "changed=false on a NO-OP");
        assert!(receipt.verified);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            before,
            "file must be untouched after a NO-OP"
        );
        // The verdict notice still tells the story.
        let verdict = gtw.last_edit_verdict().expect("verdict present");
        assert_eq!(verdict["level"], "notice");
        assert_eq!(verdict["findings"][0]["rule"], "no_op");
    }

    /// B3: outline skeleton — noise kinds collapse into a count, named
    /// structure stays; raw mode keeps everything.
    #[test]
    fn skeleton_outline_collapses_noise_but_keeps_named() {
        let code = "//! Module doc.\nuse std::fmt;\n\n/// Doc.\n#[derive(Clone)]\npub struct Widget { pub a: u16 }\n\npub fn make() -> Widget { Widget { a: 1 } }\n";
        let tree = parse_tree(code);
        let (text, visible, collapsed) = skeleton_outline(&tree, 5, true);
        assert!(text.contains("Widget"), "named stays: {text}");
        assert!(text.contains("make"), "named fn stays: {text}");
        assert!(!text.contains("use_declaration"), "use collapsed: {text}");
        assert!(!text.contains("line_comment"), "comment collapsed: {text}");
        assert!(collapsed >= 2, "noise counted: collapsed={collapsed}");
        assert!(visible >= 2);
        // Raw keeps everything — the old behavior, one flag away.
        let (raw_text, raw_visible, raw_collapsed) = skeleton_outline(&tree, 5, false);
        assert!(raw_text.contains("use_declaration"));
        assert!(raw_text.contains("line_comment"));
        assert!(raw_visible > visible || raw_collapsed == 0);
    }

    /// B2 detector: lines that ALREADY lived outside the node's span are
    #[test]
    fn duplicate_attribute_guard_rejects_reinclusion() {
        // Dogfood finding 2026-11-10: replacing a node whose leading
        // sibling is #[test] with content that STARTS with #[test] must
        // be rejected loudly (E_DUPLICATE_ATTRIBUTE) — valid Rust, so the
        // syntax gate cannot catch it, and a doubled #[test] registers the
        // test twice.
        let lines: Vec<&str> = vec!["#[test]", "fn t() { let a = 1; }"];
        let err = check_duplicate_attribute_above(&lines, 2, "#[test]\nfn t() { let a = 2; }")
            .expect_err("re-included attribute must be rejected");
        assert!(format!("{err:#}").contains("E_DUPLICATE_ATTRIBUTE"));

        // Replacement WITHOUT the attribute passes (the correct fix shape).
        check_duplicate_attribute_above(&lines, 2, "fn t() { let a = 2; }").unwrap();

        // Different attribute above: still rejected (same-line collision).
        let lines2: Vec<&str> = vec!["#[cfg(test)]", "fn t() {}"];
        let err2 = check_duplicate_attribute_above(&lines2, 2, "#[cfg(test)]\nfn t() {}")
            .expect_err("must reject");
        assert!(format!("{err2:#}").contains("E_DUPLICATE_ATTRIBUTE"));

        // No attribute above: anything goes.
        let lines3: Vec<&str> = vec!["pub struct S;", "fn t() {}"];
        check_duplicate_attribute_above(&lines3, 2, "#[test]\nfn t() {}").unwrap();

        // Doc comment between attribute and node stops the scan conservatively
        // (attribute-above-doc layout is missed, never false-positived).
        let lines4: Vec<&str> = vec!["#[test]", "/// docs", "fn t() {}"];
        check_duplicate_attribute_above(&lines4, 3, "#[test]\nfn t2() {}").unwrap();
    }
    /// other code, not remnants; true insertion remnants fire.
    #[test]
    fn replacement_remnant_detector_is_sound() {
        let old = "fn big() {\n    if true { } else { }\n    for i in 0..10 { }\n}\n";
        let new = "fn big() {}\n";
        // The reported B2 signature: new content + the OLD body surviving
        // below — no other code existed outside the node span.
        let original_solo = old;
        let modified_solo = "fn big() {}\n    if true { } else { }\n    for i in 0..10 { }\n}\n";
        let detected = detect_replacement_remnant(modified_solo, original_solo, old, new, 1, 4);
        assert!(
            detected.is_some(),
            "insertion remnant must fire: {detected:?}"
        );

        // The same lines living in a HELPER below the node: other code —
        // must NOT fire (this was the guardian-test false positive).
        let original =
            "fn big() {\n    if true { } else { }\n    for i in 0..10 { }\n}\n\nfn helper() {\n    if true { } else { }\n    for i in 0..10 { }\n}\n";
        let modified_helper = "fn big() {}\n\nfn helper() {\n    if true { } else { }\n    for i in 0..10 { }\n    match 1 { _ => {} }\n}\n";
        let detected = detect_replacement_remnant(modified_helper, original, old, new, 1, 4);
        assert!(
            detected.is_none(),
            "other code is not a remnant: {detected:?}"
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

    /// Fas 4: a signature change reports impacted call sites from the
    /// knowledge graph. Hermetic: tempdir project with .git marker.
    #[test]
    fn signature_change_reports_impact_via_knowledge_graph() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();

        let def_path = dir.path().join("lib.rs");
        let call_path = dir.path().join("caller.rs");
        std::fs::write(&def_path, "fn target(alpha: u32) -> u32 {\n    alpha\n}\n").unwrap();
        std::fs::write(&call_path, "fn caller() -> u32 {\n    target(1)\n}\n").unwrap();

        // Build the knowledge graph (saved under .gnawtreewriter_ai/graph/).
        let mut indexer = crate::llm::RelationalIndexer::new(dir.path());
        indexer.index_directory(dir.path()).unwrap();

        let mut gtw = GnawTreeWriter::new(def_path.to_str().unwrap()).unwrap();
        let result = gtw.edit(
            EditOperation::Edit {
                node_path: "@fn:target".to_string(),
                content: "fn target(alpha: u32, beta: u32) -> u32 {\n    alpha + beta\n}\n"
                    .to_string(),
            },
            false,
        );
        assert!(
            result.is_ok(),
            "signature widening must pass: {:?}",
            result.err()
        );

        let impact = gtw
            .last_edit_impact()
            .expect("signature change with indexed caller must carry impact");
        assert_eq!(impact["callers"], 1);
        let site = impact["sites"][0].as_str().unwrap();
        assert!(
            site.contains("caller.rs"),
            "site must name the caller file: {}",
            site
        );
    }

    /// Fas 4: edits that do not change a signature must not carry impact.
    #[test]
    fn no_signature_change_no_impact_field() {
        let code = "fn f(a: usize) -> usize {\n    a\n}\n";
        let (_dir, path) = temp_rust_file(code);
        let mut gtw = GnawTreeWriter::new(&path).unwrap();
        gtw.edit(
            EditOperation::Edit {
                node_path: "@fn:f".to_string(),
                content: "fn f(a: usize) -> usize {\n    a + 0\n}\n".to_string(),
            },
            false,
        )
        .unwrap();
        assert!(gtw.last_edit_impact().is_none());
    }

    /// Regression (Motor2-batt 2026-10-14, panic core/mod.rs:998): delete
    /// med en nod vars radnummer ligger utanför filens NUVARANDE längd
    /// (stale tree: källan krympt utan re-parse) panikade med slice out of
    /// range — ska vara ett ärligt fel som tvingar re-analyze, aldrig en
    /// panic som dödar turen.
    #[test]
    fn delete_with_stale_node_range_is_an_error_not_a_panic() {
        let (_dir, path_str) = temp_rust_file(concat!(
            "fn first() {}\n",
            "fn second() {}\n",
            "fn third() {}\n",
            "fn fourth() {}\n",
            "fn fifth() {}\n",
            "fn sixth() {}\n",
            "fn seventh() {}\n",
        ));
        let mut gtw = GnawTreeWriter::new(&path_str).unwrap();
        // STALE-simulering: källan krymper till 1 rad UTAN re-parse —
        // trädet har fortfarande noder med start_line 2..7 (bortom filen).
        gtw.source_code = "fn only() {}\n".to_string();
        // Någon nod med range utanför den nya källan (t.ex. "1.0" i trädet
        // pekar på rad 2+ som inte längre finns).
        let result = gtw.delete_node_at_path("1.0");
        assert!(
            result.is_err(),
            "range utanför filen ska vara ett FEL, inte panic"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("stale"),
            "felet ska namge stale-tree + re-analyze-vägen: {}",
            msg
        );
    }
}
