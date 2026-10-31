use crate::parser::TreeNode;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Structured diagnostic output for machine-readable error reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub success: bool,
    pub tool: String,
    pub file: String,
    pub language: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<DiagnosticError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<VerboseTrace>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ast_diff: Option<AstDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticError {
    pub message: String,
    pub error_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_line: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerboseTrace {
    pub steps: Vec<VerboseStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerboseStep {
    pub phase: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub duration_ms: Option<u64>,
}

/// AST diff result — compares tree structure before and after an edit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstDiff {
    pub before_root_type: String,
    pub after_root_type: String,
    pub before_node_count: usize,
    pub after_node_count: usize,
    pub before_depth: usize,
    pub after_depth: usize,
    pub structural_changes: Vec<StructuralChange>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralChange {
    pub change_type: String, // "added", "removed", "type_changed"
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Doctor check result for system validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub overall_healthy: bool,
    pub checks: Vec<DoctorCheck>,
    pub total_checks: usize,
    pub passed: usize,
    pub failed: usize,
    pub warnings: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorCheck {
    pub category: String,
    pub name: String,
    pub status: String, // "pass", "fail", "warn"
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Pure parser smoke — builds the check without touching the report, so
/// run_full_doctor can run all smokes in parallel (they were costing ~60 s
/// in series: every grammar cold-loads per process).
pub fn run_parser_smoke(extension: &str, code: &str) -> DoctorCheck {
    use crate::parser::get_parser;
    let file_name = format!("test.{}", extension);
    let path = std::path::PathBuf::from(&file_name);
    let mut check = DoctorCheck {
        category: "parser".to_string(),
        name: format!(".{}", extension),
        status: "fail".to_string(),
        message: String::new(),
        detail: None,
    };
    check.message = match get_parser(&path) {
        Ok(parser) => match parser.parse(code) {
            Ok(tree) => {
                check.status = "pass".to_string();
                format!(
                    "Parsed OK — root: '{}', {} nodes",
                    tree.node_type,
                    count_nodes(&tree)
                )
            }
            Err(e) => format!("Parse failed: {}", e.message),
        },
        Err(e) => format!("No parser available: {}", e),
    };
    check
}

// ---- Implementation ----

impl DiagnosticReport {
    pub fn ok(tool: &str, file: &str, language: &str) -> Self {
        Self {
            success: true,
            tool: tool.to_string(),
            file: file.to_string(),
            language: language.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            error: None,
            verbose: None,
            ast_diff: None,
        }
    }

    pub fn err(tool: &str, file: &str, language: &str, error: DiagnosticError) -> Self {
        Self {
            success: false,
            tool: tool.to_string(),
            file: file.to_string(),
            language: language.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            error: Some(error),
            verbose: None,
            ast_diff: None,
        }
    }

    pub fn with_verbose(&mut self, trace: VerboseTrace) -> &mut Self {
        self.verbose = Some(trace);
        self
    }

    pub fn with_ast_diff(&mut self, diff: AstDiff) -> &mut Self {
        self.ast_diff = Some(diff);
        self
    }
}

impl DiagnosticError {
    pub fn from_anyhow(err: &anyhow::Error, language: &str) -> Self {
        let msg = err.to_string();

        // Try to extract line number from common patterns
        let line = extract_line_number(&msg);
        let column = None;

        // Determine error type
        let error_type = if msg.contains("Validation failed") || msg.contains("invalid syntax") {
            "syntax_error".to_string()
        } else if msg.contains("Could not resolve") || msg.contains("not found") {
            "path_resolution".to_string()
        } else if msg.contains("GUARDIAN") {
            "guardian_block".to_string()
        } else if msg.contains("Failed to parse") {
            "parse_error".to_string()
        } else if msg.contains("Failed to read") || msg.contains("Failed to write") {
            "io_error".to_string()
        } else {
            "unknown".to_string()
        };

        // Generate suggestion based on error type
        let suggestion = generate_suggestion(&error_type, language, &msg);

        DiagnosticError {
            message: msg,
            error_type,
            line,
            column,
            context_line: None,
            suggestion,
        }
    }

    pub fn with_context_line(mut self, source: &str) -> Self {
        if let Some(line) = self.line {
            let lines: Vec<&str> = source.lines().collect();
            if line > 0 && line <= lines.len() {
                self.context_line = Some(lines[line - 1].to_string());
            }
        }
        self
    }
}

impl Default for VerboseTrace {
    fn default() -> Self {
        Self::new()
    }
}

impl VerboseTrace {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn step(&mut self, phase: &str, message: &str) -> &mut Self {
        self.steps.push(VerboseStep {
            phase: phase.to_string(),
            message: message.to_string(),
            detail: None,
            duration_ms: None,
        });
        self
    }

    pub fn step_with_detail(&mut self, phase: &str, message: &str, detail: &str) -> &mut Self {
        self.steps.push(VerboseStep {
            phase: phase.to_string(),
            message: message.to_string(),
            detail: Some(detail.to_string()),
            duration_ms: None,
        });
        self
    }
}

impl AstDiff {
    /// Compare two AST trees and report structural differences
    pub fn compare(before: &TreeNode, after: &TreeNode) -> Self {
        let before_types = collect_node_types(before);
        let after_types = collect_node_types(after);

        let before_node_count = count_nodes(before);
        let after_node_count = count_nodes(after);

        let before_depth = max_depth(before, 0);
        let after_depth = max_depth(after, 0);

        let mut changes = Vec::new();
        let mut warnings = Vec::new();

        // Check if root type changed
        if before.node_type != after.node_type {
            changes.push(StructuralChange {
                change_type: "type_changed".to_string(),
                path: "".to_string(),
                before_type: Some(before.node_type.clone()),
                after_type: Some(after.node_type.clone()),
                description: Some(format!(
                    "Root node type changed from '{}' to '{}'",
                    before.node_type, after.node_type
                )),
            });
        }

        // Compare top-level children
        Self::compare_children(before, after, &mut changes);

        // Warnings for significant structural changes
        let count_diff = (after_node_count as i64 - before_node_count as i64).abs();
        if count_diff as f64 / (before_node_count as f64).max(1.0) > 0.5 {
            warnings.push(format!(
                "Node count changed significantly: {} -> {} ({}% change)",
                before_node_count,
                after_node_count,
                if before_node_count > 0 {
                    (count_diff as f64 / before_node_count as f64 * 100.0) as usize
                } else {
                    100
                }
            ));
        }

        // Check for lost node types (e.g. function_declaration removed)
        let before_type_set: HashSet<_> = before_types.keys().collect();
        let after_type_set: HashSet<_> = after_types.keys().collect();
        for lost in before_type_set.difference(&after_type_set) {
            if is_important_node_type(lost.to_string()) {
                warnings.push(format!(
                    "Lost {} '{}' node(s) — this may indicate unintended deletion",
                    before_types[*lost], lost
                ));
            }
        }

        AstDiff {
            before_root_type: before.node_type.clone(),
            after_root_type: after.node_type.clone(),
            before_node_count,
            after_node_count,
            before_depth,
            after_depth,
            structural_changes: changes,
            warnings,
        }
    }

    fn compare_children(before: &TreeNode, after: &TreeNode, changes: &mut Vec<StructuralChange>) {
        let before_names: Vec<_> = before
            .children
            .iter()
            .filter_map(|c| c.get_name().map(|n| (c.node_type.clone(), n)))
            .collect();
        let after_names: Vec<_> = after
            .children
            .iter()
            .filter_map(|c| c.get_name().map(|n| (c.node_type.clone(), n)))
            .collect();

        let before_set: HashSet<_> = before_names.iter().collect();
        let after_set: HashSet<_> = after_names.iter().collect();

        // Removed named nodes
        for (ntype, name) in before_set.difference(&after_set) {
            changes.push(StructuralChange {
                change_type: "removed".to_string(),
                path: String::new(),
                before_type: Some(format!("{} {}", ntype, name)),
                after_type: None,
                description: Some(format!("Removed {} '{}'", ntype, name)),
            });
        }

        // Added named nodes
        for (ntype, name) in after_set.difference(&before_set) {
            changes.push(StructuralChange {
                change_type: "added".to_string(),
                path: String::new(),
                before_type: None,
                after_type: Some(format!("{} {}", ntype, name)),
                description: Some(format!("Added {} '{}'", ntype, name)),
            });
        }
    }
}

impl Default for DoctorReport {
    fn default() -> Self {
        Self::new()
    }
}

impl DoctorReport {
    pub fn new() -> Self {
        Self {
            overall_healthy: true,
            checks: Vec::new(),
            total_checks: 0,
            passed: 0,
            failed: 0,
            warnings: 0,
        }
    }

    pub fn pass(&mut self, category: &str, name: &str, message: &str) {
        self.checks.push(DoctorCheck {
            category: category.to_string(),
            name: name.to_string(),
            status: "pass".to_string(),
            message: message.to_string(),
            detail: None,
        });
        self.passed += 1;
        self.total_checks += 1;
    }

    pub fn fail(&mut self, category: &str, name: &str, message: &str) {
        self.checks.push(DoctorCheck {
            category: category.to_string(),
            name: name.to_string(),
            status: "fail".to_string(),
            message: message.to_string(),
            detail: None,
        });
        self.failed += 1;
        self.total_checks += 1;
        self.overall_healthy = false;
    }

    pub fn warn(&mut self, category: &str, name: &str, message: &str) {
        self.checks.push(DoctorCheck {
            category: category.to_string(),
            name: name.to_string(),
            status: "warn".to_string(),
            message: message.to_string(),
            detail: None,
        });
        self.warnings += 1;
        self.total_checks += 1;
    }

    /// Record a prebuilt check with the SAME counter semantics as
    /// pass()/fail()/warn() — one accounting path for sequential and
    /// parallel checks alike.
    pub fn record_check(&mut self, check: DoctorCheck) {
        match check.status.as_str() {
            "pass" => self.passed += 1,
            "fail" => {
                self.failed += 1;
                self.overall_healthy = false;
            }
            "warn" => self.warnings += 1,
            _ => {}
        }
        self.total_checks += 1;
        self.checks.push(check);
    }

    /// Run a parser health check for a given extension
    pub fn check_parser(&mut self, extension: &str, code: &str) {
        self.record_check(run_parser_smoke(extension, code));
    }

    /// Check backup integrity
    pub fn check_backups(&mut self, project_root: &std::path::Path) {
        let backup_dir = project_root.join(".gnawtreewriter_backups");

        if !backup_dir.exists() {
            self.warn("backup", "backup_dir", "No backup directory found");
            return;
        }

        // LIGHT SCAN: list_backup_files() fully parses every backup (2 GB of
        // JSON = ~80 s) just to sort by timestamp. For health we only need
        // the count plus a validity sample — count by filename, validate the
        // 3 newest by mtime, never load the rest.
        let entries = match std::fs::read_dir(&backup_dir) {
            Ok(e) => e,
            Err(e) => {
                self.fail(
                    "backup",
                    "backup_scan",
                    &format!("Failed to scan backups: {}", e),
                );
                return;
            }
        };
        let mut files: Vec<(std::path::PathBuf, std::time::SystemTime)> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                let mtime = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                files.push((path, mtime));
            }
        }

        if files.is_empty() {
            self.warn(
                "backup",
                "backup_count",
                "Backup directory exists but is empty",
            );
            return;
        }

        self.pass(
            "backup",
            "backup_count",
            &format!("Found {} backup(s)", files.len()),
        );

        files.sort_by_key(|(_, m)| std::cmp::Reverse(*m));
        for (path, _) in files.iter().take(3) {
            let name = format!(
                "backup_{}",
                path.file_name().unwrap_or_default().to_string_lossy()
            );
            match crate::core::backup::parse_backup_file(path) {
                Ok(_) => self.pass("backup", &name, "Backup file is valid"),
                Err(e) => self.fail("backup", &name, &format!("Corrupt backup: {}", e)),
            }
        }
    }

    /// Check transaction log integrity
    pub fn check_transaction_log(&mut self, project_root: &std::path::Path) {
        match crate::core::transaction_log::TransactionLog::load(project_root) {
            Ok(tlog) => match tlog.get_full_history() {
                Ok(history) => {
                    self.pass(
                        "transaction_log",
                        "log_readable",
                        &format!("Transaction log OK — {} entries", history.len()),
                    );
                }
                Err(e) => {
                    self.fail(
                        "transaction_log",
                        "log_history",
                        &format!("Cannot read history: {}", e),
                    );
                }
            },
            Err(e) => {
                self.fail(
                    "transaction_log",
                    "log_load",
                    &format!("Cannot load transaction log: {}", e),
                );
            }
        }
    }

    /// T5 (issue log 2026-10-23): the semantic channels degrade silently
    /// away without the bge-base model files; one doctor run should
    /// reveal that state instead of a sense call discovering it. Mirrors
    /// AiManager's cache resolution: project-local first, then $HOME.
    #[cfg(feature = "modernbert")]
    pub fn check_semantic_model(&mut self, project_root: &std::path::Path) {
        let local = project_root.join(".gnawtreewriter_ai").join("models");
        let cache_dir = if local.exists() && local.join("modernbert").exists() {
            local
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            std::path::PathBuf::from(home)
                .join(".gnawtreewriter_ai")
                .join("models")
        };
        let cfg = cache_dir.join("bge-base").join("config.json");
        self.record_check(DoctorCheck {
            category: "ai_runtime".to_string(),
            name: "semantic_model".to_string(),
            status: if cfg.is_file() { "pass" } else { "warn" }.to_string(),
            message: if cfg.is_file() {
                "bge-base model files present — sense/search_semantic semantic ranking available."
                    .to_string()
            } else {
                "bge-base model files missing — sense/search_semantic DEGRADE to lexical text matches (named in every reply). Run `gnawtreewriter ai setup` to download; `ai status` shows what is installed.".to_string()
            },
            detail: None,
        });
    }

    /// Fas 4.2 step 1: knowledge-graph resolution quality — how much of
    /// the call graph is unambiguously resolvable. This is the trust
    /// level of impact reports; surfaced so agents and humans can see
    /// when to treat caller counts as conservative.
    pub fn check_knowledge_graph(&mut self, project_root: &std::path::Path) {
        let indexer = crate::llm::RelationalIndexer::new(project_root);
        match indexer.resolution_stats() {
            Ok(stats) if stats.calls_total == 0 => {
                self.record_check(DoctorCheck {
                    category: "knowledge_graph".to_string(),
                    name: "resolution".to_string(),
                    status: "pass".to_string(),
                    message: "No call relations indexed yet — build the graph with index_relations or ordinary edits. Impact reports will appear as signatures change.".to_string(),
                    detail: None,
                });
            }
            Ok(stats) => {
                let pct = if stats.calls_total > 0 {
                    (stats.calls_resolved as f64 / stats.calls_total as f64 * 100.0) as u64
                } else {
                    0
                };
                let status = if pct >= 60 { "pass" } else { "warn" };
                self.record_check(DoctorCheck {
                    category: "knowledge_graph".to_string(),
                    name: "resolution".to_string(),
                    status: status.to_string(),
                    message: format!(
                        "{}/{} call relations unambiguously resolved ({}%). {} ambiguous symbol name(s){}.",
                        stats.calls_resolved,
                        stats.calls_total,
                        pct,
                        stats.ambiguous_symbols,
                        if stats.top_ambiguous.is_empty() {
                            String::new()
                        } else {
                            format!(
                                "; top: {}",
                                stats
                                    .top_ambiguous
                                    .iter()
                                    .map(|a| a.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        }
                    ),
                    detail: Some(serde_json::to_string(&stats).unwrap_or_default()),
                });
            }
            Err(e) => {
                self.record_check(DoctorCheck {
                    category: "knowledge_graph".to_string(),
                    name: "resolution".to_string(),
                    status: "warn".to_string(),
                    message: format!("Could not read knowledge graph: {}", e),
                    detail: None,
                });
            }
        }
    }

    /// Stale-binary check (the dogfooding case): when doctor runs inside
    /// the gnawtreewriter repo itself, compare the RUNNING binary against
    /// the repo's Cargo.toml. A binary older than the repo means released
    /// fixes exist that are NOT executing — the 2026-10-28 incident had a
    /// 2026-10-09 edit_delta.rs fix hidden behind a stale 0.18.0 install
    /// for three weeks, and the panic's file:line pointed straight into
    /// the comment documenting its own fix.
    ///
    /// Outside the gnawtreewriter repo this is a no-op: freshness vs this
    /// repo says nothing about other projects.
    pub fn check_binary_freshness(&mut self, project_root: &std::path::Path) {
        let Ok(text) = std::fs::read_to_string(project_root.join("Cargo.toml")) else {
            return;
        };
        if !is_gnawtreewriter_manifest(&text) {
            return;
        }
        let Some(repo_version) = cargo_toml_version(&text) else {
            return;
        };
        let binary_version = env!("CARGO_PKG_VERSION");
        let installed = installed_binary_date();
        match compare_version_triples(binary_version, &repo_version) {
            std::cmp::Ordering::Less => self.warn(
                "binary",
                "freshness",
                &format!(
                    "STALE BINARY: running {binary_version} (installed {installed}) but this repo \
                     builds {repo_version}. Fixes in source are NOT what executes. Redeploy: \
                     scripts/build-gpu.sh (CUDA) or cargo install --path . (CPU), then reconnect MCP."
                ),
            ),
            std::cmp::Ordering::Equal => self.pass(
                "binary",
                "freshness",
                &format!("binary matches repo ({binary_version}, installed {installed})"),
            ),
            // Newer-than-repo is the normal dev-build state (built from a
            // dirty tree or an uncommitted bump) — informational, not a nag.
            std::cmp::Ordering::Greater => self.pass(
                "binary",
                "freshness",
                &format!(
                    "dev build ahead of repo: running {binary_version} (installed {installed}), \
                     repo at {repo_version} — commit + release to sync"
                ),
            ),
        }
    }
}

// ---- Helpers ----

fn extract_line_number(msg: &str) -> Option<usize> {
    // Try common patterns: "line X", "near line X", ":X:"
    for part in msg.split_whitespace() {
        if let Some(num_str) = part.strip_suffix(',') {
            if let Ok(n) = num_str.parse::<usize>() {
                return Some(n);
            }
        }
    }

    // Pattern: "Check near line X"
    if let Some(pos) = msg.find("line ") {
        let rest = &msg[pos + 5..];
        let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = num_str.parse::<usize>() {
            return Some(n);
        }
    }

    None
}

fn generate_suggestion(error_type: &str, language: &str, _msg: &str) -> Option<String> {
    match error_type {
        "syntax_error" => Some(match language {
            "rs" => {
                "Check for missing semicolons, unbalanced braces, or incorrect type annotations"
                    .to_string()
            }
            "py" => {
                "Verify indentation levels and ensure colons after def/if/for/while".to_string()
            }
            "js" | "ts" => "Check for missing brackets, braces, or semicolons".to_string(),
            "go" => "Verify that all imports are used and braces are balanced".to_string(),
            "java" | "kt" => {
                "Ensure all methods have return types and braces are balanced".to_string()
            }
            _ => "Check syntax: balanced braces, semicolons, and proper punctuation".to_string(),
        }),
        "path_resolution" => {
            Some("Run `gnawtreewriter skeleton <file>` to see valid node paths".to_string())
        }
        "guardian_block" => Some(
            "Review the edit — it may be removing critical logic. Use --force to override"
                .to_string(),
        ),
        "parse_error" => Some(format!(
            "The {} parser could not process this file. Check for encoding issues or mixed content",
            language
        )),
        "io_error" => Some("Verify file permissions and that the path exists".to_string()),
        _ => None,
    }
}

fn collect_node_types(node: &TreeNode) -> std::collections::HashMap<String, usize> {
    let mut types = std::collections::HashMap::new();
    fn walk(n: &TreeNode, acc: &mut std::collections::HashMap<String, usize>) {
        *acc.entry(n.node_type.clone()).or_insert(0) += 1;
        for c in &n.children {
            walk(c, acc);
        }
    }
    walk(node, &mut types);
    types
}

fn count_nodes(node: &TreeNode) -> usize {
    1 + node.children.iter().map(count_nodes).sum::<usize>()
}

fn max_depth(node: &TreeNode, current: usize) -> usize {
    if node.children.is_empty() {
        current
    } else {
        node.children
            .iter()
            .map(|c| max_depth(c, current + 1))
            .max()
            .unwrap_or(current)
    }
}

fn is_important_node_type(ntype: String) -> bool {
    let important = [
        "function_declaration",
        "function_definition",
        "function_item",
        "class_declaration",
        "class_definition",
        "struct_item",
        "enum_item",
        "impl_item",
        "method_declaration",
        "method_definition",
    ];
    important.iter().any(|&i| ntype == i)
}

/// True if the manifest is the gnawtreewriter package itself (the check
/// is scoped to the [package] section so a stray dependency named alike
/// cannot false-positive).
fn is_gnawtreewriter_manifest(text: &str) -> bool {
    text.lines()
        .skip_while(|l| l.trim() != "[package]")
        .take_while(|l| !l.trim_start().starts_with('[') || l.trim() == "[package]")
        .any(|l| {
            let t = l.trim_start();
            t.starts_with("name") && t.contains("\"gnawtreewriter\"")
        })
}

/// The `version = "X.Y.Z"` value from the [package] section.
fn cargo_toml_version(text: &str) -> Option<String> {
    let mut in_package = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "[package]" {
            in_package = true;
            continue;
        }
        if in_package && trimmed.starts_with('[') {
            break;
        }
        if in_package {
            if let Some(rest) = trimmed.strip_prefix("version") {
                let rest = rest.trim_start().strip_prefix('=')?.trim();
                let val = rest.trim_matches('"');
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

/// Numeric triple comparison of "major.minor.patch". Pre-release/build
/// suffixes after '-'/'+' are ignored (0.21.0-beta == 0.21.0): this check
/// only needs release-level staleness, not semver ordering subtleties.
fn compare_version_triples(a: &str, b: &str) -> std::cmp::Ordering {
    fn triple(v: &str) -> Vec<u64> {
        v.split(['-', '+'])
            .next()
            .unwrap_or("")
            .split('.')
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect()
    }
    let (a, b) = (triple(a), triple(b));
    for i in 0..3 {
        let av = a.get(i).copied().unwrap_or(0);
        let bv = b.get(i).copied().unwrap_or(0);
        match av.cmp(&bv) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

/// Install/build date of the RUNNING binary: cargo writes a fresh file on
/// install/rebuild, so the executable's mtime is the deploy date.
fn installed_binary_date() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.metadata().ok())
        .and_then(|m| m.modified().ok())
        .map(|t| {
            chrono::DateTime::<chrono::Utc>::from(t)
                .format("%Y-%m-%d")
                .to_string()
        })
        .unwrap_or_else(|| "unknown date".to_string())
}

pub fn run_full_doctor(project_root: &std::path::Path) -> DoctorReport {
    let mut report = DoctorReport::new();

    let parser_tests = [
        ("py", "def hello(): pass"),
        ("rs", "fn main() {}"),
        ("js", "function hello() {}"),
        ("ts", "const x: number = 1;"),
        ("go", "package main\nfunc main() {}"),
        ("java", "class Main {}"),
        ("c", "int main() { return 0; }"),
        ("cpp", "int main() { return 0; }"),
        ("html", "<html></html>"),
        ("css", "body { margin: 0; }"),
        ("json", "{\"key\": \"value\"}"),
        ("yaml", "key: value"),
        ("toml", "[section]\nkey = \"value\""),
        ("sql", "SELECT 1;"),
        ("sh", "echo hello"),
        ("zig", "pub fn main() void {}"),
        ("php", "<?php echo 1;"),
        ("svelte", "<script>let x = 0;</script>"),
        ("dart", "void main() {}"),
        ("cs", "using System;"),
    ];
    // Grammar cold-loads cost ~60 s in series — run the smokes in parallel
    // (each thread builds its own parser instance; grammars are static).
    let smoke_results: Vec<DoctorCheck> = std::thread::scope(|s| {
        let handles: Vec<_> = parser_tests
            .iter()
            .map(|(ext, code)| s.spawn(move || run_parser_smoke(ext, code)))
            .collect();
        handles
            .into_iter()
            .map(|h| {
                h.join().unwrap_or_else(|_| DoctorCheck {
                    category: "parser".to_string(),
                    name: "unknown".to_string(),
                    status: "fail".to_string(),
                    message: "parser smoke thread panicked".to_string(),
                    detail: None,
                })
            })
            .collect()
    });
    for check in smoke_results {
        report.record_check(check);
    }

    report.check_backups(project_root);
    report.check_transaction_log(project_root);
    report.check_knowledge_graph(project_root);

    // T5: semantic-channel availability (bge-base present / degraded
    // mode). Feature-gated: no sense exists in feature-light builds, so
    // there is nothing to degrade or report there.
    #[cfg(feature = "modernbert")]
    report.check_semantic_model(project_root);
    report.check_binary_freshness(project_root);
    report
}

#[cfg(test)]
mod binary_freshness_tests {
    use super::*;

    #[test]
    fn version_comparison_is_numeric_not_lexicographic() {
        assert_eq!(
            compare_version_triples("0.18.0", "0.20.0"),
            std::cmp::Ordering::Less
        );
        // The trap that motivates a numeric parse: "0.9" > "0.10" as text.
        assert_eq!(
            compare_version_triples("0.9.8", "0.10.0"),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_version_triples("0.20.0", "0.20.0"),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            compare_version_triples("1.0.0", "0.99.0"),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn version_suffixes_are_ignored() {
        assert_eq!(
            compare_version_triples("0.21.0-beta.1", "0.21.0"),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            compare_version_triples("0.21.0+build.7", "0.21.0"),
            std::cmp::Ordering::Equal
        );
    }

    #[test]
    fn cargo_toml_version_reads_package_section_only() {
        let manifest = "[package]\nname = \"gnawtreewriter\"\nversion = \"0.20.0\"\n\
                        [dependencies]\nchrono = { version = \"0.4\" }\n";
        assert_eq!(
            cargo_toml_version(manifest).as_deref(),
            Some("0.20.0"),
            "must not pick up versions from later sections"
        );
        assert_eq!(cargo_toml_version("[package]\nname = \"x\"\n"), None);
    }

    #[test]
    fn staleness_inside_gtw_repo_warns_with_redeploy_hint() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"gnawtreewriter\"\nversion = \"99.0.0\"\n",
        )
        .unwrap();
        let mut report = DoctorReport::new();
        report.check_binary_freshness(dir.path());
        assert_eq!(report.total_checks, 1);
        assert_eq!(report.warnings, 1);
        let check = &report.checks[0];
        assert_eq!(check.category, "binary");
        assert!(check.message.contains("STALE BINARY"), "{}", check.message);
        assert!(
            check.message.contains("build-gpu.sh"),
            "must name the redeploy path: {}",
            check.message
        );
    }

    #[test]
    fn older_repo_version_passes_as_dev_build() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"gnawtreewriter\"\nversion = \"0.0.1\"\n",
        )
        .unwrap();
        let mut report = DoctorReport::new();
        report.check_binary_freshness(dir.path());
        assert_eq!(report.total_checks, 1);
        assert_eq!(report.warnings, 0);
        assert!(report.checks[0].message.contains("dev build"));
    }

    #[test]
    fn other_projects_and_non_cargo_dirs_are_skipped() {
        // A foreign package (even one named similarly in a dependency) is
        // not the gnawtreewriter repo — no check, no noise in its doctor.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"some-other-tool\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        let mut report = DoctorReport::new();
        report.check_binary_freshness(dir.path());
        assert_eq!(report.total_checks, 0);

        let empty = tempfile::tempdir().unwrap();
        let mut report2 = DoctorReport::new();
        report2.check_binary_freshness(empty.path());
        assert_eq!(report2.total_checks, 0);
    }
}
