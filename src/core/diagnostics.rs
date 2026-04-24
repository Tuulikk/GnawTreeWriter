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

    fn compare_children(
        before: &TreeNode,
        after: &TreeNode,
        changes: &mut Vec<StructuralChange>,
    ) {
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

    /// Run a parser health check for a given extension
    pub fn check_parser(&mut self, extension: &str, code: &str) {
        use crate::parser::get_parser;
        use std::path::PathBuf;

        let file_name = format!("test.{}", extension);
        let path = PathBuf::from(&file_name);

        match get_parser(&path) {
            Ok(parser) => match parser.parse(code) {
                Ok(tree) => {
                    self.pass(
                        "parser",
                        &format!(".{}", extension),
                        &format!("Parsed OK — root: '{}', {} nodes", tree.node_type, count_nodes(&tree)),
                    );
                }
                Err(e) => {
                    self.fail(
                        "parser",
                        &format!(".{}", extension),
                        &format!("Parse failed: {}", e.message),
                    );
                }
            },
            Err(e) => {
                self.fail(
                    "parser",
                    &format!(".{}", extension),
                    &format!("No parser available: {}", e),
                );
            }
        }
    }

    /// Check backup integrity
    pub fn check_backups(&mut self, project_root: &std::path::Path) {
        let backup_dir = project_root.join(".gnawtreewriter_backups");

        if !backup_dir.exists() {
            self.warn("backup", "backup_dir", "No backup directory found");
            return;
        }

        match crate::core::backup::list_backup_files(&backup_dir) {
            Ok(backups) => {
                if backups.is_empty() {
                    self.warn("backup", "backup_count", "Backup directory exists but is empty");
                } else {
                    self.pass(
                        "backup",
                        "backup_count",
                        &format!("Found {} backup(s)", backups.len()),
                    );

                    // Validate a few backups can be parsed
                    let to_check = backups.iter().take(3);
                    for b in to_check {
                        match crate::core::backup::parse_backup_file(&b.path) {
                            Ok(_) => {
                                self.pass(
                                    "backup",
                                    &format!("backup_{}", b.path.file_name().unwrap_or_default().to_string_lossy()),
                                    "Backup file is valid",
                                );
                            }
                            Err(e) => {
                                self.fail(
                                    "backup",
                                    &format!("backup_{}", b.path.file_name().unwrap_or_default().to_string_lossy()),
                                    &format!("Corrupt backup: {}", e),
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                self.fail("backup", "backup_scan", &format!("Failed to scan backups: {}", e));
            }
        }
    }

    /// Check transaction log integrity
    pub fn check_transaction_log(&mut self, project_root: &std::path::Path) {
        match crate::core::transaction_log::TransactionLog::load(project_root.to_path_buf()) {
            Ok(tlog) => {
                match tlog.get_full_history() {
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
                }
            }
            Err(e) => {
                self.fail(
                    "transaction_log",
                    "log_load",
                    &format!("Cannot load transaction log: {}", e),
                );
            }
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

fn generate_suggestion(error_type: &str, language: &str, msg: &str) -> Option<String> {
    match error_type {
        "syntax_error" => Some(match language {
            "rs" => "Check for missing semicolons, unbalanced braces, or incorrect type annotations".to_string(),
            "py" => "Verify indentation levels and ensure colons after def/if/for/while".to_string(),
            "js" | "ts" => "Check for missing brackets, braces, or semicolons".to_string(),
            "go" => "Verify that all imports are used and braces are balanced".to_string(),
            "java" | "kt" => "Ensure all methods have return types and braces are balanced".to_string(),
            _ => "Check syntax: balanced braces, semicolons, and proper punctuation".to_string(),
        }),
        "path_resolution" => Some("Run `gnawtreewriter skeleton <file>` to see valid node paths".to_string()),
        "guardian_block" => Some("Review the edit — it may be removing critical logic. Use --force to override".to_string()),
        "parse_error" => Some(format!("The {} parser could not process this file. Check for encoding issues or mixed content", language)),
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
