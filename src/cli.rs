use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use crate::core::{GnawTreeWriter, EditOperation};
use std::fs;
use std::path::Path;
use crate::parser::TreeNode;

#[derive(Parser)]
#[command(name = "gnawtreewriter")]
#[command(about = "Tree-based code editor for LLM-assisted editing", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze file and show tree structure
    Analyze {
        /// Path to file or directory (supports multiple paths)
        paths: Vec<String>,
        /// Output format (json, compact, summary)
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// List all nodes with paths in a file
    List {
        /// Path to file
        file_path: String,
        /// Filter by node type
        #[arg(short, long)]
        filter_type: Option<String>,
    },
    /// Find nodes matching criteria
    Find {
        /// Path to file or directory
        paths: Vec<String>,
        /// Filter by node type
        #[arg(short, long)]
        node_type: Option<String>,
        /// Filter by content (partial match)
        #[arg(short, long)]
        content: Option<String>,
    },
    /// Lint files and show issues with severity levels
    Lint {
        /// Path to file or directory
        paths: Vec<String>,
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Show tree node at specific path
    Show {
        /// Path to file
        file_path: String,
        /// Tree path to node (e.g., "0.2.1")
        node_path: String,
    },
    /// Edit node at tree path
    Edit {
        /// Path to file
        file_path: String,
        /// Tree path to node
        node_path: String,
        /// New content/code snippet
        content: String,
        /// Preview changes without applying them
        #[arg(short, long)]
        preview: bool,
    },
    /// Insert new node
    Insert {
        /// Path to file
        file_path: String,
        /// Tree path to parent node
        parent_path: String,
        /// Insert position (0 = before, 1 = after, 2 = as child)
        position: usize,
        /// Content to insert
        content: String,
    },
    /// Delete node at tree path
    Delete {
        /// Path to file
        file_path: String,
        /// Tree path to node
        node_path: String,
    },
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::Analyze { paths, format: fmt } => {
                if paths.is_empty() {
                    eprintln!("Error: No paths provided");
                    std::process::exit(1);
                }

                let mut results = Vec::new();
                for path in &paths {
                    if let Err(e) = analyze_path(path, &fmt, &mut results) {
                        eprintln!("Error analyzing {}: {}", path, e);
                    }
                }

                if results.len() == 1 && fmt == "json" {
                    println!("{}", serde_json::to_string_pretty(&results[0])?);
                } else {
                    println!("{}", serde_json::to_string_pretty(&results)?);
                }
            }
            Commands::List { file_path, filter_type } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                let tree = writer.analyze();
                list_nodes(tree, &file_path, filter_type.as_deref());
            }
            Commands::Find { paths, node_type, content } => {
                if paths.is_empty() {
                    eprintln!("Error: No paths provided");
                    std::process::exit(1);
                }

                for path in &paths {
                    let path_obj = Path::new(path);
                    if path_obj.is_dir() {
                        find_in_directory(path_obj, node_type.as_deref(), content.as_deref())?;
                    } else {
                        find_in_file(path, node_type.as_deref(), content.as_deref())?;
                    }
                }
            }
            Commands::Lint { paths, format: fmt } => {
                if paths.is_empty() {
                    eprintln!("Error: No paths provided");
                    std::process::exit(1);
                }

                let mut issues = Vec::new();
                for path in &paths {
                    if let Err(e) = lint_path(path, &mut issues) {
                        eprintln!("Error linting {}: {}", path, e);
                    }
                }

                if fmt == "json" {
                    println!("{}", serde_json::to_string_pretty(&issues)?);
                } else {
                    for issue in &issues {
                        println!("{}:{}:{} {}: {} [{}]",
                            issue.file,
                            issue.line,
                            issue.column,
                            issue.severity,
                            issue.message,
                            issue.suggestion
                        );
                    }
                }

                let error_count = issues.iter().filter(|i| i.severity == "error").count();
                if error_count > 0 {
                    std::process::exit(1);
                }
            }
            Commands::Show { file_path, node_path } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                let node = writer.show_node(&node_path)?;
                println!("{}", node);
            }
            Commands::Edit { file_path, node_path, content, preview } => {
                if preview {
                    let writer = GnawTreeWriter::new(&file_path)?;
                    let modified = writer.preview_edit(EditOperation::Edit { node_path, content })?;
                    println!("Preview of changes:");
                    println!("{}", modified);
                } else {
                    let writer = GnawTreeWriter::new(&file_path)?;
                    writer.edit(EditOperation::Edit { node_path, content })?;
                    println!("Edited successfully");
                }
            }
            Commands::Insert { file_path, parent_path, position, content } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                writer.edit(EditOperation::Insert { parent_path, position, content })?;
                println!("Inserted successfully");
            }
            Commands::Delete { file_path, node_path } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                writer.edit(EditOperation::Delete { node_path })?;
                println!("Deleted successfully");
            }
        }
        Ok(())
    }

}

#[derive(Debug, Clone, serde::Serialize)]
struct LintIssue {
    file: String,
    line: usize,
    column: usize,
    severity: String,
    message: String,
    suggestion: String,
}

fn lint_path(path: &str, issues: &mut Vec<LintIssue>) -> Result<()> {
    let path_obj = Path::new(path);

    if path_obj.is_dir() {
        lint_directory(path_obj, issues)?;
    } else {
        let writer = GnawTreeWriter::new(path)?;
        let tree = writer.analyze();
        check_tree_issues(&tree, path, issues);
    }

    Ok(())
}

fn lint_directory(dir: &Path, issues: &mut Vec<LintIssue>) -> Result<()> {
    let entries = fs::read_dir(dir)
        .context(format!("Failed to read directory: {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if is_supported_extension(ext.to_str().unwrap_or("")) {
                    if let Some(path_str) = path.to_str() {
                        if let Err(e) = lint_path(path_str, issues) {
                            eprintln!("Error linting {}: {}", path.display(), e);
                        }
                    }
                }
            }
        } else if path.is_dir() {
            lint_directory(&path, issues)?;
        }
    }

    Ok(())
}

fn check_tree_issues(tree: &TreeNode, file_path: &str, issues: &mut Vec<LintIssue>) {
    check_node(tree, file_path, issues);

    for child in &tree.children {
        check_tree_issues(child, file_path, issues);
    }
}

fn check_node(node: &TreeNode, file_path: &str, issues: &mut Vec<LintIssue>) {
    if node.node_type == "Property" {
        if node.content.trim().is_empty() {
            issues.push(LintIssue {
                file: file_path.to_string(),
                line: node.start_line,
                column: 1,
                severity: "warning".to_string(),
                message: "Empty property found".to_string(),
                suggestion: "Remove or add content to property".to_string(),
            });
        }

        if node.content.len() > 200 {
            issues.push(LintIssue {
                file: file_path.to_string(),
                line: node.start_line,
                column: 1,
                severity: "info".to_string(),
                message: "Property is very long".to_string(),
                suggestion: "Consider splitting into multiple properties".to_string(),
            });
        }
    }

    if node.node_type == "Text" && node.content.len() > 100 {
        issues.push(LintIssue {
            file: file_path.to_string(),
            line: node.start_line,
            column: 1,
            severity: "info".to_string(),
            message: "Long text content".to_string(),
            suggestion: "Consider using translation keys".to_string(),
        });
    }
}

fn list_nodes(tree: &TreeNode, file_path: &str, filter_type: Option<&str>) {
    print_node(tree, file_path, filter_type, 0);

    for child in &tree.children {
        list_nodes_helper(child, file_path, filter_type, 1);
    }
}

fn list_nodes_helper(node: &TreeNode, file_path: &str, filter_type: Option<&str>, depth: usize) {
    print_node(node, file_path, filter_type, depth);

    for child in &node.children {
        list_nodes_helper(child, file_path, filter_type, depth + 1);
    }
}

fn print_node(node: &TreeNode, file_path: &str, filter_type: Option<&str>, depth: usize) {
    if let Some(filter) = filter_type {
        if node.node_type != filter {
            return;
        }
    }

    let indent = "  ".repeat(depth);
    let content_preview = if node.content.len() > 50 {
        format!("{}...", &node.content[..50])
    } else {
        node.content.clone()
    };

    println!("{}{} [{}:{}{}] {}",
        indent,
        node.path,
        node.node_type,
        node.start_line,
        if node.end_line != node.start_line { format!("-{}", node.end_line) } else { String::new() },
        if !content_preview.is_empty() { format!(": {}", content_preview) } else { String::new() }
    );
}

fn find_in_file(path: &str, node_type: Option<&str>, content: Option<&str>) -> Result<()> {
    let writer = GnawTreeWriter::new(path)?;
    let tree = writer.analyze();
    find_in_tree(tree, path, node_type, content);
    Ok(())
}

fn find_in_directory(dir: &Path, node_type: Option<&str>, content: Option<&str>) -> Result<()> {
    let entries = fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if is_supported_extension(ext.to_str().unwrap_or("")) {
                    if let Some(path_str) = path.to_str() {
                        find_in_file(path_str, node_type, content)?;
                    }
                }
            }
        } else if path.is_dir() {
            find_in_directory(&path, node_type, content)?;
        }
    }

    Ok(())
}

fn find_in_tree(tree: &TreeNode, file_path: &str, node_type: Option<&str>, content: Option<&str>) {
    let matches_type = node_type.map_or(true, |t| tree.node_type == t);
    let matches_content = content.map_or(true, |c| tree.content.contains(c));

    if matches_type && matches_content && tree.path != "root" {
        let content_preview = if tree.content.len() > 60 {
            format!("{}...", &tree.content[..60])
        } else {
            tree.content.clone()
        };

        println!("{}: {} [{}:{}]: {}",
            file_path,
            tree.path,
            tree.node_type,
            tree.start_line,
            if !content_preview.is_empty() { content_preview } else { "<empty>".to_string() }
        );
    }

    for child in &tree.children {
        find_in_tree(child, file_path, node_type, content);
    }
}

fn analyze_path(path: &str, format: &str, results: &mut Vec<serde_json::Value>) -> Result<()> {
    let path_obj = Path::new(path);

    if path_obj.is_dir() {
        analyze_directory(path_obj, format, results)?;
    } else {
        let writer = GnawTreeWriter::new(path)?;
        let tree = writer.analyze();

        let result = match format {
            "json" => serde_json::to_value(tree)?,
            "compact" => serde_json::json!({
                "file": path,
                "node_type": tree.node_type,
                "nodes_count": count_nodes(tree),
            }),
            "summary" => serde_json::json!({
                "file": path,
                "type": tree.node_type,
                "lines": format!("{}-{}", tree.start_line, tree.end_line),
            }),
            _ => return Err(anyhow::anyhow!("Unknown format: {}", format)),
        };

        results.push(result);
    }

    Ok(())
}

fn analyze_directory(dir: &Path, format: &str, results: &mut Vec<serde_json::Value>) -> Result<()> {
    let entries = fs::read_dir(dir)
        .context(format!("Failed to read directory: {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if is_supported_extension(ext.to_str().unwrap_or("")) {
                    if let Some(path_str) = path.to_str() {
                        if let Err(e) = analyze_path(path_str, format, results) {
                            eprintln!("Error analyzing {}: {}", path.display(), e);
                        }
                    }
                }
            }
        } else if path.is_dir() {
            analyze_directory(&path, format, results)?;
        }
    }

    Ok(())
}

fn count_nodes(tree: &TreeNode) -> usize {
    1 + tree.children.iter().map(|child| count_nodes(child)).sum::<usize>()
}

fn is_supported_extension(ext: &str) -> bool {
    matches!(ext.to_lowercase().as_str(), "qml" | "py" | "rs" | "ts" | "tsx" | "js" | "php" | "html")
}
