use anyhow::Result;
use clap::{Parser, Subcommand};
use crate::core::{GnawTreeWriter, EditOperation};
use crate::parser::TreeNode;

#[derive(Parser)]
#[command(name = "gnawtreewriter")]
#[command(about = "A tool for tree-based code editing", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze files and output the tree structure
    Analyze {
        paths: Vec<String>,
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// List all nodes in a file
    List {
        file_path: String,
        #[arg(short, long)]
        filter_type: Option<String>,
    },
    /// Show the content of a specific node
    Show {
        file_path: String,
        node_path: String,
    },
    /// Replace the content of a node
    Edit {
        file_path: String,
        node_path: String,
        content: String,
        #[arg(short, long)]
        preview: bool,
    },
    /// Insert code into a parent node at a specific position (0: top, 1: bottom, 2: after properties)
    Insert {
        file_path: String,
        parent_path: String,
        position: usize,
        content: String,
    },
    /// Delete a node
    Delete {
        file_path: String,
        node_path: String,
    },
    /// QML-specific: Add a property to a component
    AddProperty {
        file_path: String,
        target_path: String,
        name: String,
        r#type: String,
        value: String,
    },
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::Analyze { paths, format: _fmt } => {
                let mut results = Vec::new();
                for path in &paths {
                    let writer = GnawTreeWriter::new(path)?;
                    let tree = writer.analyze();
                    results.push(serde_json::to_value(tree)?);
                }
                println!("{}", serde_json::to_string_pretty(&results)?);
            }
            Commands::List { file_path, filter_type } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                list_nodes(writer.analyze(), filter_type.as_deref());
            }
            Commands::Show { file_path, node_path } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                println!("{}", writer.show_node(&node_path)?);
            }
            Commands::Edit { file_path, node_path, content, preview } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                if preview {
                    println!("{}", writer.preview_edit(EditOperation::Edit { node_path, content })?);
                } else {
                    writer.edit(EditOperation::Edit { node_path, content })?;
                }
            }
            Commands::Insert { file_path, parent_path, position, content } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                writer.edit(EditOperation::Insert { parent_path, position, content })?;
            }
            Commands::Delete { file_path, node_path } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                writer.edit(EditOperation::Delete { node_path })?;
            }
            Commands::AddProperty { file_path, target_path, name, r#type, value } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                let property_code = format!("property {} {}: {}", r#type, name, value);
                writer.edit(EditOperation::Insert { 
                    parent_path: target_path.clone(), 
                    position: 2, // After existing properties
                    content: property_code 
                })?;
                println!("Successfully added property '{}' to {}", name, target_path);
            }
        }
        Ok(())
    }
}

fn list_nodes(tree: &TreeNode, filter_type: Option<&str>) {
    print_node(tree, 0, filter_type);
    for child in &tree.children {
        list_nodes_recursive(child, 1, filter_type);
    }
}

fn list_nodes_recursive(node: &TreeNode, depth: usize, filter_type: Option<&str>) {
    print_node(node, depth, filter_type);
    for child in &node.children {
        list_nodes_recursive(child, depth + 1, filter_type);
    }
}

fn print_node(node: &TreeNode, depth: usize, filter_type: Option<&str>) {
    if let Some(f) = filter_type {
        if node.node_type != f { return; }
    }
    let indent = "  ".repeat(depth);
    println!("{}{} [{}] (line {}- {})", indent, node.path, node.node_type, node.start_line, node.end_line);
}
