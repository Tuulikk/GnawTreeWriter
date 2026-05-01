//! gnaw-inspect: Advanced code intelligence for GnawTreeWriter

use crate::GnawTreeWriter;
use anyhow::Result;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use walkdir::WalkDir;

/// Inspect mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InspectMode {
    /// Find all callers of a symbol
    Callers,
    /// Calculate code metrics
    Metrics,
    /// Find orphan/unused code
    Orphans,
    /// Show code relations
    Relations,
    /// Full analysis
    Full,
}

/// Inspect result
#[derive(Serialize)]
pub struct InspectResult {
    pub mode: String,
    pub file: String,
    pub symbol: Option<String>,
    pub findings: Vec<Finding>,
    pub summary: HashMap<String, usize>,
}

#[derive(Serialize)]
pub struct Finding {
    pub file: String,
    pub line: usize,
    pub path: String,
    pub node_type: String,
    pub name: String,
    pub context: Option<String>,
}

/// Inspect a file or symbol for advanced analysis
pub fn inspect(
    file_path: &str,
    mode: InspectMode,
    symbol: Option<&str>,
    recursive: bool,
    directory: Option<&str>,
) -> Result<Vec<InspectResult>> {
    let _mode_name = match mode {
        InspectMode::Callers => "callers",
        InspectMode::Metrics => "metrics",
        InspectMode::Orphans => "orphans",
        InspectMode::Relations => "relations",
        InspectMode::Full => "full",
    };

    if recursive || directory.is_some() {
        inspect_project(file_path, mode, symbol, directory)
    } else {
        inspect_file(file_path, mode, symbol).map(|r| vec![r])
    }
}

fn inspect_file(file_path: &str, mode: InspectMode, symbol: Option<&str>) -> Result<InspectResult> {
    let writer = GnawTreeWriter::new(file_path)?;
    let tree = writer.analyze();
    let path_str = file_path.to_string();

    let (findings, summary) = match mode {
        InspectMode::Callers => find_callers(&tree, symbol.unwrap_or(""), &path_str),
        InspectMode::Metrics => calculate_metrics(&tree, &path_str),
        InspectMode::Orphans => find_orphans(&tree, &path_str),
        InspectMode::Relations => analyze_relations(&tree, &path_str),
        InspectMode::Full => {
            let c = calculate_metrics(&tree, &path_str);
            let o = find_orphans(&tree, &path_str);
            let mut all_findings = c.0;
            all_findings.extend(o.0);
            (all_findings, c.1)
        }
    };

    Ok(InspectResult {
        mode: format!("{:?}", mode).to_lowercase(),
        file: file_path.to_string(),
        symbol: symbol.map(|s| s.to_string()),
        findings,
        summary,
    })
}

fn inspect_project(
    _root_file: &str,
    mode: InspectMode,
    symbol: Option<&str>,
    directory: Option<&str>,
) -> Result<Vec<InspectResult>> {
    let current_dir = std::env::current_dir()?;
    let search_dir = directory.map(Path::new).unwrap_or_else(|| current_dir.as_path());

    let mut results = Vec::new();
    let _mode_name = match mode {
        InspectMode::Callers => "callers",
        InspectMode::Metrics => "metrics",
        InspectMode::Orphans => "orphans",
        InspectMode::Relations => "relations",
        InspectMode::Full => "full",
    };

    for entry in WalkDir::new(search_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(ext.to_lowercase().as_str(), 
                "rs" | "py" | "js" | "ts" | "go" | "java" | "c" | "cpp" | "h" | "hpp") 
            {
                // Skip common non-source dirs
                let skip = path.components().any(|c| {
                    let s = c.as_os_str().to_string_lossy();
                    s == "target" || s == "node_modules" || s == ".git" || s.starts_with('.')
                });
                if skip { continue; }

                if let Ok(result) = inspect_file(&path.to_string_lossy(), mode, symbol) {
                    if !result.findings.is_empty() {
                        results.push(result);
                    }
                }
            }
        }
    }

    Ok(results)
}

fn find_callers(tree: &crate::parser::TreeNode, symbol: &str, file: &str) -> (Vec<Finding>, HashMap<String, usize>) {
    let mut findings = Vec::new();
    let mut summary = HashMap::new();

    if symbol.is_empty() {
        return (findings, summary);
    }

    // Find all functions and their calls
    let mut functions: HashSet<String> = HashSet::new();
    let mut calls: HashMap<String, Vec<(String, usize, String)>> = HashMap::new();

    collect_functions_and_calls(tree, symbol, &mut functions, &mut calls, file);

    // Find callers of the target symbol
    for (caller, locations) in calls.iter() {
        for (file, line, path) in locations {
            if caller.to_lowercase().contains(&symbol.to_lowercase()) || 
               functions.contains(caller) && symbol.is_empty() || 
               symbol.is_empty() && !caller.contains("anonymous")
            {
                findings.push(Finding {
                    file: file.clone(),
                    line: *line,
                    path: path.clone(),
                    node_type: "call".to_string(),
                    name: caller.clone(),
                    context: None,
                });
            }
        }
    }

    summary.insert("callers".to_string(), findings.len());
    (findings, summary)
}

fn collect_functions_and_calls(
    tree: &crate::parser::TreeNode,
    symbol: &str,
    functions: &mut HashSet<String>,
    calls: &mut HashMap<String, Vec<(String, usize, String)>>,
    file: &str,
) {
    let node_type = tree.node_type.to_lowercase();
    let _content = tree.content.trim();

    // Collect function definitions
    if node_type == "function_declaration" || node_type == "function_item" || node_type == "method_declaration" {
        if let Some(name) = tree.get_name() {
            functions.insert(name.clone());
            let is_target = !symbol.is_empty() && name.to_lowercase().contains(&symbol.to_lowercase());
            if is_target || symbol.is_empty() {
                if let Some(finding) = tree.children.iter().find(|c| c.node_type == "identifier" || c.node_type == "attribute_identifier") {
                    let name = finding.content.trim();
                    calls.insert(name.to_string(), vec![(file.to_string(), tree.start_line, tree.path.clone())]);
                }
            }
        }
    }

    // Collect function calls
    if node_type == "call_expression" {
        if let Some(first_child) = tree.children.first() {
            if first_child.node_type == "identifier" || first_child.node_type == "field_expression" {
                let call_name = first_child.content.trim();
                if !call_name.is_empty() {
                    calls
                        .entry(call_name.to_string())
                        .or_insert_with(Vec::new)
                        .push((file.to_string(), tree.start_line, tree.path.clone()));
                }
            }
        }
    }

    for child in &tree.children {
        collect_functions_and_calls(child, symbol, functions, calls, file);
    }
}

fn calculate_metrics(tree: &crate::parser::TreeNode, file: &str) -> (Vec<Finding>, HashMap<String, usize>) {
    let mut metrics: HashMap<String, usize> = HashMap::new();
    let mut findings = Vec::new();
    let mut functions: Vec<(String, usize, usize)> = Vec::new();

    // Count nodes by type
    count_nodes(tree, &mut metrics);

    // Find function sizes
    collect_function_sizes(tree, &mut functions);

    // Report large functions (>50 lines)
    for (name, start, lines) in functions.iter().filter(|(_, _, l)| *l > 50) {
        findings.push(Finding {
            file: file.to_string(),
            line: *start,
            path: format!("large_function:{}", name),
            node_type: "warning".to_string(),
            name: name.clone(),
            context: Some(format!("{} lines", lines)),
        });
    }

    (findings, metrics)
}

fn count_nodes(tree: &crate::parser::TreeNode, metrics: &mut HashMap<String, usize>) {
    let type_name = tree.node_type.clone();
    *metrics.entry(type_name).or_insert(0) += 1;

    for child in &tree.children {
        count_nodes(child, metrics);
    }
}

fn collect_function_sizes(tree: &crate::parser::TreeNode, functions: &mut Vec<(String, usize, usize)>) {
    let node_type = tree.node_type.to_lowercase();

    if node_type == "function_declaration" || node_type == "function_item" || node_type == "method_declaration" {
        if let Some(name) = tree.get_name() {
            let lines = tree.end_line.saturating_sub(tree.start_line) + 1;
            functions.push((name, tree.start_line, lines));
        }
    }

    for child in &tree.children {
        collect_function_sizes(child, functions);
    }
}

fn find_orphans(tree: &crate::parser::TreeNode, file: &str) -> (Vec<Finding>, HashMap<String, usize>) {
    let mut findings = Vec::new();
    let mut summary = HashMap::new();

    // Find private functions that don't appear to be called
    let mut all_functions: HashSet<String> = HashSet::new();
    let mut all_calls: HashSet<String> = HashSet::new();
    let mut private_functions: Vec<(String, usize, String)> = Vec::new();

    collect_symbols(tree, &mut all_functions, &mut all_calls, &mut private_functions);

    // Find private functions not in call set
    for (name, line, path) in &private_functions {
        if !all_calls.contains(name) && !name.starts_with('_') {
            findings.push(Finding {
                file: file.to_string(),
                line: *line,
                path: path.clone(),
                node_type: "possible_orphan".to_string(),
                name: name.clone(),
                context: Some("private function with no detected calls".to_string()),
            });
        }
    }

    summary.insert("orphans".to_string(), findings.len());
    summary.insert("functions".to_string(), all_functions.len());
    summary.insert("calls".to_string(), all_calls.len());

    (findings, summary)
}

fn collect_symbols(
    tree: &crate::parser::TreeNode,
    functions: &mut HashSet<String>,
    calls: &mut HashSet<String>,
    private: &mut Vec<(String, usize, String)>,
) {
    let node_type = tree.node_type.to_lowercase();

    // Function definitions
    if node_type == "function_declaration" || node_type == "function_item" || node_type == "method_declaration" {
        if let Some(name) = tree.get_name() {
            functions.insert(name.clone());
            
            // Check if private (starts with _ or has #[allow(dead_code)])
            let is_private = name.starts_with('_');
            if is_private {
                private.push((name, tree.start_line, tree.path.clone()));
            }
        }
    }

    // Function calls
    if node_type == "call_expression" {
        if let Some(first_child) = tree.children.first() {
            if first_child.node_type == "identifier" {
                let call_name = first_child.content.trim();
                if !call_name.is_empty() {
                    calls.insert(call_name.to_string());
                }
            }
        }
    }

    for child in &tree.children {
        collect_symbols(child, functions, calls, private);
    }
}

fn analyze_relations(tree: &crate::parser::TreeNode, file: &str) -> (Vec<Finding>, HashMap<String, usize>) {
    let mut findings = Vec::new();
    let mut summary = HashMap::new();

    let mut structs: HashMap<String, Vec<String>> = HashMap::new();
    let mut implementations: Vec<(String, String, usize)> = Vec::new();
    let mut uses: Vec<String> = Vec::new();

    collect_relations(tree, &mut structs, &mut implementations, &mut uses);

    // Report struct-method relations
    for (struct_name, methods) in structs.iter() {
        for method in methods {
            findings.push(Finding {
                file: file.to_string(),
                line: 0,
                path: format!("impl:{}", struct_name),
                node_type: "relation".to_string(),
                name: method.clone(),
                context: Some(format!("implements {}", struct_name)),
            });
        }
    }

    summary.insert("structs".to_string(), structs.len());
    summary.insert("implementations".to_string(), implementations.len());
    summary.insert("uses".to_string(), uses.len());

    (findings, summary)
}

fn collect_relations(
    tree: &crate::parser::TreeNode,
    structs: &mut HashMap<String, Vec<String>>,
    impls: &mut Vec<(String, String, usize)>,
    uses: &mut Vec<String>,
) {
    let node_type = tree.node_type.to_lowercase();

    // Struct definitions
    if node_type == "struct_item" || node_type == "class_declaration" || node_type == "type_declaration" {
        if let Some(name) = tree.get_name() {
            structs.entry(name).or_insert_with(Vec::new);
        }
    }

    // Method definitions (inside impl blocks)
    if node_type == "function_item" || node_type == "method_declaration" {
        if let Some(name) = tree.get_name() {
            // Check if parent is impl block
            // Simplified: just collect methods
            impls.push((name.clone(), "impl".to_string(), tree.start_line));
        }
    }

    // Use statements
    if node_type == "use_declaration" {
        let content = tree.content.trim();
        if !content.is_empty() {
            uses.push(content.to_string());
        }
    }

    for child in &tree.children {
        collect_relations(child, structs, impls, uses);
    }
}

/// Format results as text
pub fn format_inspect_text(results: &[InspectResult]) -> String {
    let mut output = String::new();

    for result in results {
        if !result.findings.is_empty() {
            output.push_str(&format!("\n📊 {} [{}]\n", result.file, result.mode));
            
            if let Some(ref sym) = result.symbol {
                output.push_str(&format!("  Symbol: {}\n", sym));
            }

            for finding in &result.findings {
                let context = finding.context.as_ref().map(|c| format!(" - {}", c)).unwrap_or_default();
                output.push_str(&format!(
                    "  {}:{} [{}] {}{}\n",
                    finding.line, finding.path, finding.node_type, finding.name, context
                ));
            }

            if !result.summary.is_empty() {
                output.push_str("  Summary: ");
                let summary_str: Vec<String> = result.summary.iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect();
                output.push_str(&summary_str.join(", "));
                output.push('\n');
            }
        }
    }

    output
}
