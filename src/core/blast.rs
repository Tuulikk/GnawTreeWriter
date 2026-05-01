//! gnaw-blast: Change Impact Analysis

use crate::{GnawTreeWriter, TreeNode};
use anyhow::Result;
use serde::Serialize;
use std::collections::{HashSet};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for ImpactLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImpactLevel::Low => write!(f, "🟢 Low"),
            ImpactLevel::Medium => write!(f, "🟡 Medium"),
            ImpactLevel::High => write!(f, "🟠 High"),
            ImpactLevel::Critical => write!(f, "🔴 Critical"),
        }
    }
}

#[derive(Serialize)]
pub struct BlastResult {
    pub target_file: String,
    pub target_path: String,
    pub target_name: String,
    pub target_type: String,
    pub impact_level: String,
    pub callers: Vec<CodeRelation>,
    pub callees: Vec<CodeRelation>,
    pub related_files: Vec<String>,
    pub summary: ImpactSummary,
}

#[derive(Serialize, Clone)]
pub struct CodeRelation {
    pub file: String,
    pub line: usize,
    pub path: String,
    pub name: String,
    pub relation_type: String,
}

#[derive(Serialize)]
pub struct ImpactSummary {
    pub callers_count: usize,
    pub callees_count: usize,
    pub files_affected: usize,
    pub risk_score: f32,
}

pub fn blast(
    file_path: &str,
    node_path: &str,
    recursive: bool,
    directory: Option<&str>,
) -> Result<BlastResult> {
    let writer = GnawTreeWriter::new(file_path)?;
    let tree = writer.analyze();

    let target = tree.find_path(node_path)
        .ok_or_else(|| anyhow::anyhow!("Node not found at path: {}", node_path))?;

    let target_name = target.get_name().unwrap_or_else(|| "unnamed".to_string());

    let callers = if recursive {
        find_callers_project(&target_name, directory)?
    } else {
        find_callers_in_file(&tree, &target_name, file_path)
    };

    let callees = find_callees(target);

    let mut all_files: HashSet<String> = HashSet::new();
    for c in &callers { all_files.insert(c.file.clone()); }
    for c in &callees { if c.file != "local" { all_files.insert(c.file.clone()); } }
    all_files.remove(file_path);
    let related_files: Vec<String> = all_files.into_iter().collect();

    let risk_score = calculate_risk(&callers, &callees, related_files.len());
    let impact_level = match risk_score {
        s if s < 0.2 => ImpactLevel::Low,
        s if s < 0.5 => ImpactLevel::Medium,
        s if s < 0.8 => ImpactLevel::High,
        _ => ImpactLevel::Critical,
    };

    Ok(BlastResult {
        target_file: file_path.to_string(),
        target_path: node_path.to_string(),
        target_name,
        target_type: target.node_type.clone(),
        impact_level: impact_level.to_string(),
        callers: callers.clone(),
        callees: callees.clone(),
        related_files: related_files.clone(),
        summary: ImpactSummary {
            callers_count: callers.len(),
            callees_count: callees.len(),
            files_affected: related_files.len(),
            risk_score,
        },
    })
}

fn collect_all_nodes<'a>(node: &'a TreeNode) -> Vec<&'a TreeNode> {
    let mut nodes = vec![node];
    for child in &node.children {
        nodes.extend(collect_all_nodes(child));
    }
    nodes
}

pub fn find_callers_in_file(tree: &TreeNode, target: &str, file: &str) -> Vec<CodeRelation> {
    let mut callers = Vec::new();
    let target_lower = target.to_lowercase();

    // Find all function definitions to know context
    let _functions = find_all_functions(tree);

    // Find calls matching the target
    for node in collect_all_nodes(tree) {
        if node.node_type == "call_expression" {
            let call_name = node.get_name().unwrap_or_default();
            if call_name.to_lowercase() == target_lower {
                callers.push(CodeRelation {
                    file: file.to_string(),
                    line: node.start_line,
                    path: node.path.clone(),
                    name: call_name,
                    relation_type: "call".to_string(),
                });
            }
        } else if node.node_type == "identifier" {
            if node.content.to_lowercase() == target_lower {
                // Check if this identifier is in a function (not the definition)
                let parent_func = find_parent_function(node, tree);
                if let Some(pf) = parent_func {
                    if pf.get_name().map(|n| n.to_lowercase()).unwrap_or_default() != target_lower {
                        callers.push(CodeRelation {
                            file: file.to_string(),
                            line: node.start_line,
                            path: node.path.clone(),
                            name: node.content.clone(),
                            relation_type: "reference".to_string(),
                        });
                    }
                }
            }
        }
    }

    callers
}

fn find_all_functions(tree: &TreeNode) -> Vec<&TreeNode> {
    let mut funcs = Vec::new();
    for node in collect_all_nodes(tree) {
        if node.node_type == "function_item" || node.node_type == "function_declaration" 
            || node.node_type == "method_declaration" {
            funcs.push(node);
        }
    }
    funcs
}

fn find_parent_function<'a>(node: &'a TreeNode, tree: &'a TreeNode) -> Option<&'a TreeNode> {
    // Simple heuristic: find the containing function
    let node_path = &node.path;
    let path_parts: Vec<&str> = node_path.split('.').collect();
    
    for i in (0..path_parts.len()).rev() {
        let partial_path = path_parts[..i].join(".");
        if let Some(n) = tree.find_path(&partial_path) {
            if n.node_type == "function_item" || n.node_type == "function_declaration" 
                || n.node_type == "method_declaration" {
                return Some(n);
            }
        }
    }
    None
}

pub fn find_callers_project(target: &str, directory: Option<&str>) -> Result<Vec<CodeRelation>> {
    let current_dir = std::env::current_dir()?;
    let search_dir = directory.map(Path::new).unwrap_or_else(|| current_dir.as_path());
    let mut all_callers = Vec::new();

    for entry in WalkDir::new(search_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(ext.to_lowercase().as_str(), 
                "rs" | "py" | "js" | "ts" | "go" | "java") 
            {
                let skip = path.components().any(|c| {
                    let s = c.as_os_str().to_string_lossy();
                    s == "target" || s == "node_modules" || s == ".git"
                });
                if skip { continue; }

                let file_str = path.to_string_lossy().to_string();
                if let Ok(writer) = GnawTreeWriter::new(&file_str) {
                    let tree = writer.analyze();
                    let callers = find_callers_in_file(&tree, target, &file_str);
                    all_callers.extend(callers);
                }
            }
        }
    }

    Ok(all_callers)
}

pub fn find_callees(target: &TreeNode) -> Vec<CodeRelation> {
    let mut callees = Vec::new();
    collect_callees_recursive(target, &mut callees);
    callees
}

fn collect_callees_recursive(node: &TreeNode, results: &mut Vec<CodeRelation>) {
    if node.node_type == "call_expression" {
        let name = node.get_name().unwrap_or_else(|| node.content.clone());
        results.push(CodeRelation {
            file: "local".to_string(),
            line: node.start_line,
            path: node.path.clone(),
            name: name.trim().to_string(),
            relation_type: "calls".to_string(),
        });
    }
    for child in &node.children {
        collect_callees_recursive(child, results);
    }
}

fn calculate_risk(callers: &[CodeRelation], callees: &[CodeRelation], files_affected: usize) -> f32 {
    let callers_score = callers.len() as f32 * 2.0;
    let callees_score = callees.len() as f32 * 0.5;
    let files_score = files_affected as f32 * 1.5;
    
    (callers_score + callees_score + files_score) / 10.0
}

pub fn format_blast_text(result: &BlastResult) -> String {
    let mut output = String::new();

    output.push_str(&format!("\n💥 BLAST RADIUS ANALYSIS\n"));
    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    output.push_str(&format!("📍 {} @ {}\n", result.target_name, result.target_path));
    output.push_str(&format!("   Type: {}\n", result.target_type));
    output.push_str(&format!("\n⚠️  Impact: {} (risk: {:.2})\n", result.impact_level, result.summary.risk_score));
    output.push_str(&format!("   Callers: {} | Callees: {} | Files: {}\n", 
        result.summary.callers_count, result.summary.callees_count, result.summary.files_affected));

    if !result.callers.is_empty() {
        output.push_str("\n👆 CALLERS (who uses this):\n");
        let mut last_file = String::new();
        for caller in &result.callers {
            if caller.file != last_file {
                output.push_str(&format!("\n   📄 {}\n", caller.file));
                last_file = caller.file.clone();
            }
            output.push_str(&format!("   {}:{} [{}] {}\n", caller.line, caller.path, caller.relation_type, caller.name));
        }
    } else {
        output.push_str("\n👆 CALLERS: None found (may be entry point)\n");
    }

    if !result.callees.is_empty() {
        output.push_str("\n👇 CALLEES (what this calls):\n");
        for callee in &result.callees {
            output.push_str(&format!("   {}:{} {}\n", callee.line, callee.relation_type, callee.name));
        }
    }

    if !result.related_files.is_empty() {
        output.push_str("\n📁 RELATED FILES:\n");
        for f in &result.related_files {
            output.push_str(&format!("   {}\n", f));
        }
    }

    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    output
}
