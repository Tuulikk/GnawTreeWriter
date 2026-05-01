//! gnaw-graph: Visualize code relations and dependencies

use crate::{GnawTreeWriter, TreeNode};
use anyhow::Result;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Clone)]
pub struct GraphResult {
    pub file: String,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub summary: GraphSummary,
}

#[derive(Serialize, Clone)]
pub struct GraphNode {
    pub id: String,
    pub name: String,
    pub node_type: String,
    pub path: String,
    pub line: usize,
}

#[derive(Serialize, Clone)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
}

#[derive(Serialize, Clone)]
pub struct GraphSummary {
    pub nodes: usize,
    pub edges: usize,
    pub max_depth: usize,
    pub orphans: usize,
}

/// Generate a code relation graph
pub fn graph(
    file_path: &str,
    max_depth: usize,
    include_external: bool,
    _recursive: bool,
) -> Result<GraphResult> {
    let writer = GnawTreeWriter::new(file_path)?;
    let tree = writer.analyze();

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut functions: HashMap<String, Vec<String>> = HashMap::new();

    // Collect all function definitions and calls
    collect_functions_and_calls(&tree, &mut nodes, &mut edges, &mut functions, max_depth);

    // Find orphan nodes (functions not called by anything)
    let called_functions: HashSet<String> = edges.iter()
        .filter(|e| e.relation == "calls")
        .map(|e| e.to.clone())
        .collect();

    let orphans = nodes.iter()
        .filter(|n| n.node_type.contains("function") && !called_functions.contains(&n.id))
        .count();

    let node_count = nodes.len();
    let edge_count = edges.len();

    Ok(GraphResult {
        file: file_path.to_string(),
        nodes,
        edges,
        summary: GraphSummary {
            nodes: node_count,
            edges: edge_count,
            max_depth,
            orphans,
        },
    })
}

fn collect_functions_and_calls(
    tree: &TreeNode,
    nodes: &mut Vec<GraphNode>,
    edges: &mut Vec<GraphEdge>,
    _functions: &mut HashMap<String, Vec<String>>,
    _max_depth: usize,
) {
    let node_type = tree.node_type.to_lowercase();

    // Function/method definitions
    if node_type.contains("function") || node_type.contains("method") {
        if let Some(name) = tree.get_name() {
            let id = format!("{}:{}", tree.path, name);
            nodes.push(GraphNode {
                id: id.clone(),
                name: name.clone(),
                node_type: tree.node_type.clone(),
                path: tree.path.clone(),
                line: tree.start_line,
            });
            _functions.insert(id, Vec::new());
        }
    }

    // Function calls
    if node_type == "call_expression" {
        if let Some(caller_id) = find_parent_function_id(tree, nodes) {
            if let Some(callee_name) = get_call_target(tree) {
                let callee_id = find_function_id(callee_name.as_str(), nodes);
                if let Some(cid) = callee_id {
                    edges.push(GraphEdge {
                        from: caller_id,
                        to: cid,
                        relation: "calls".to_string(),
                    });
                }
            }
        }
    }

    // Struct/impl relationships
    if node_type == "struct_item" || node_type == "impl_item" {
        if let Some(name) = tree.get_name() {
            let id = format!("{}:{}", tree.path, name);
            nodes.push(GraphNode {
                id: id.clone(),
                name: name.clone(),
                node_type: tree.node_type.clone(),
                path: tree.path.clone(),
                line: tree.start_line,
            });
        }
    }

    // Use declarations (imports)
    if node_type == "use_declaration" {
        let content = tree.content.trim();
        if !content.is_empty() && content != "::" {
            let id = format!("use:{}", content.replace("::", "_").replace(" ", "_"));
            nodes.push(GraphNode {
                id,
                name: content.to_string(),
                node_type: "use_declaration".to_string(),
                path: tree.path.clone(),
                line: tree.start_line,
            });
        }
    }

    // Recurse into children
    for child in &tree.children {
        collect_functions_and_calls(child, nodes, edges, _functions, _max_depth);
    }
}

fn find_parent_function_id(node: &TreeNode, nodes: &Vec<GraphNode>) -> Option<String> {
    let path_parts: Vec<&str> = node.path.split('.').collect();
    for i in (0..path_parts.len()).rev() {
        let partial_path = path_parts[..i].join(".");
        for n in nodes {
            if n.path == partial_path && n.node_type.contains("function") {
                return Some(n.id.clone());
            }
        }
    }
    None
}

fn get_call_target(node: &TreeNode) -> Option<String> {
    // Get the function name from a call expression
    for child in &node.children {
        if child.node_type == "identifier" || child.node_type == "field_expression" {
            return Some(child.content.trim().to_string());
        }
    }
    None
}

fn find_function_id(name: &str, nodes: &Vec<GraphNode>) -> Option<String> {
    for n in nodes {
        if n.name == name && n.node_type.contains("function") {
            return Some(n.id.clone());
        }
    }
    // Fuzzy match
    for n in nodes {
        if n.name.to_lowercase().contains(&name.to_lowercase()) || 
           name.to_lowercase().contains(&n.name.to_lowercase()) {
            return Some(n.id.clone());
        }
    }
    None
}

/// Format graph as ASCII art
pub fn format_graph_text(result: &GraphResult) -> String {
    let mut output = String::new();

    output.push_str("\n🔗 CODE RELATION GRAPH\n");
    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    output.push_str(&format!("File: {}\n", result.file));
    output.push_str(&format!("📊 {} nodes, {} edges, {} orphans\n",
        result.summary.nodes, result.summary.edges, result.summary.orphans));

    if !result.nodes.is_empty() {
        output.push_str("\n📦 NODES (functions, structs, etc.):\n");
        let mut by_type: HashMap<String, Vec<&GraphNode>> = HashMap::new();
        for node in &result.nodes {
            by_type.entry(node.node_type.clone()).or_default().push(node);
        }
        for (node_type, group) in by_type {
            output.push_str(&format!("  [{}] {}\n", group.len(), node_type));
            for n in group {
                output.push_str(&format!("    • {} @ {}\n", n.name, n.line));
            }
        }
    }

    if !result.edges.is_empty() {
        output.push_str("\n🔗 CALL RELATIONS:\n");
        for edge in &result.edges {
            let from_name = result.nodes.iter()
                .find(|n| n.id == edge.from)
                .map(|n| n.name.clone())
                .unwrap_or_else(|| edge.from.clone());
            let to_name = result.nodes.iter()
                .find(|n| n.id == edge.to)
                .map(|n| n.name.clone())
                .unwrap_or_else(|| edge.to.clone());
            output.push_str(&format!("  {} → {} [{}]\n", from_name, to_name, edge.relation));
        }
    }

    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    output
}

/// Format as Mermaid diagram
pub fn format_graph_mermaid(result: &GraphResult) -> String {
    let mut output = String::new();

    output.push_str("```mermaid\n");
    output.push_str("flowchart LR\n");

    // Add nodes
    for node in &result.nodes {
        let label = node.name.replace('"', "").replace('<', "&lt;").replace('>', "&gt;");
        let node_id = node.id.replace(':', "_").replace('.', "_");
        output.push_str(&format!("    {}(\"{}\")\n", node_id, label));
    }

    // Add edges
    for edge in &result.edges {
        let from_id = edge.from.replace(':', "_").replace('.', "_");
        let to_id = edge.to.replace(':', "_").replace('.', "_");
        output.push_str(&format!("    {} --> {}\n", from_id, to_id));
    }

    output.push_str("```\n");
    output
}

/// Format as DOT (Graphviz)
pub fn format_graph_dot(result: &GraphResult) -> String {
    let mut output = String::new();

    output.push_str("digraph G {\n");
    output.push_str("    rankdir=LR;\n");
    output.push_str("    node [shape=box];\n");

    // Add nodes
    for node in &result.nodes {
        let label = node.name.replace('"', "").replace('\\', "\\\\");
        let node_id = node.id.replace(':', "_").replace('.', "_");
        output.push_str(&format!("    \"{}\" [label=\"{}\"];\n", node_id, label));
    }

    // Add edges
    for edge in &result.edges {
        let from_id = edge.from.replace(':', "_").replace('.', "_");
        let to_id = edge.to.replace(':', "_").replace('.', "_");
        output.push_str(&format!("    \"{}\" -> \"{}\";\n", from_id, to_id));
    }

    output.push_str("}\n");
    output
}

/// Format as simple tree view
pub fn format_graph_tree(result: &GraphResult) -> String {
    let mut output = String::new();

    output.push_str("\n🌳 CODE TREE\n");
    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Group by file
    let mut funcs: Vec<&GraphNode> = Vec::new();
    let mut structs: Vec<&GraphNode> = Vec::new();
    let mut others: Vec<&GraphNode> = Vec::new();

    for node in &result.nodes {
        if node.node_type.contains("function") {
            funcs.push(node);
        } else if node.node_type.contains("struct") || node.node_type.contains("impl") {
            structs.push(node);
        } else {
            others.push(node);
        }
    }

    if !structs.is_empty() {
        output.push_str("\n📦 Structs/Impls:\n");
        for s in structs {
            output.push_str(&format!("  📦 {}\n", s.name));
            // Find methods
            let methods: Vec<&GraphNode> = result.nodes.iter()
                .filter(|n| n.path.starts_with(&s.path) && n.node_type.contains("function"))
                .collect();
            for m in methods {
                output.push_str(&format!("    └── {}\n", m.name));
            }
        }
    }

    if !funcs.is_empty() {
        output.push_str("\n🔧 Functions:\n");
        for f in funcs {
            let calls: Vec<&GraphNode> = result.edges.iter()
                .filter(|e| e.from == f.id)
                .filter_map(|e| result.nodes.iter().find(|n| n.id == e.to))
                .collect();
            output.push_str(&format!("  🔧 {}()\n", f.name));
            for c in calls {
                output.push_str(&format!("      → {}\n", c.name));
            }
        }
    }

    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    output
}
