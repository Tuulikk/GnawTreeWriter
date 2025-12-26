use anyhow::Result;
use clap::{Parser, Subcommand};
use crate::core::{GnawTreeWriter, EditOperation};
use crate::parser::TreeNode;
use similar::{ChangeTag, TextDiff};

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
        #[arg(short, long)]
        preview: bool,
    },
    /// Delete a node
    Delete {
        file_path: String,
        node_path: String,
        #[arg(short, long)]
        preview: bool,
    },
    /// QML-specific: Add a property to a component
    AddProperty {
        file_path: String,
        target_path: String,
        name: String,
        r#type: String,
        value: String,
        #[arg(short, long)]
        preview: bool,
    },
    /// QML-specific: Add a child component
    AddComponent {
        file_path: String,
        target_path: String,
        name: String,
        #[arg(short, long)]
        content: Option<String>,
        #[arg(short, long)]
        preview: bool,
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
                let op = EditOperation::Edit { node_path, content };
                if preview {
                    let modified = writer.preview_edit(op)?;
                    print_diff(writer.get_source(), &modified);
                } else {
                    writer.edit(op)?;
                }
            }
            Commands::Insert { file_path, parent_path, position, content, preview } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                let op = EditOperation::Insert { parent_path, position, content };
                if preview {
                    let modified = writer.preview_edit(op)?;
                    print_diff(writer.get_source(), &modified);
                } else {
                    writer.edit(op)?;
                }
            }
            Commands::Delete { file_path, node_path, preview } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                let op = EditOperation::Delete { node_path };
                if preview {
                    let modified = writer.preview_edit(op)?;
                    print_diff(writer.get_source(), &modified);
                } else {
                    writer.edit(op)?;
                }
            }
            Commands::AddProperty { file_path, target_path, name, r#type, value, preview } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                let property_code = format!("property {} {}: {}", r#type, name, value);
                let op = EditOperation::Insert { 
                    parent_path: target_path.clone(), 
                    position: 2, 
                    content: property_code 
                };
                if preview {
                    let modified = writer.preview_edit(op)?;
                    print_diff(writer.get_source(), &modified);
                } else {
                    writer.edit(op)?;
                    println!("Successfully added property '{}' to {}", name, target_path);
                }
            }
            Commands::AddComponent { file_path, target_path, name, content, preview } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                let component_code = match content {
                    Some(c) => format!("{} {{\n    {}\n}}", name, c),
                    None => format!("{} {{}}\n", name),
                };
                let op = EditOperation::Insert { 
                    parent_path: target_path.clone(), 
                    position: 1, 
                    content: component_code 
                };
                if preview {
                    let modified = writer.preview_edit(op)?;
                    print_diff(writer.get_source(), &modified);
                } else {
                    writer.edit(op)?;
                    println!("Successfully added component '{}' to {}", name, target_path);
                }
            }
        }
        Ok(())
    }
}

fn print_diff(old: &str, new: &str) {
    let diff = TextDiff::from_lines(old, new);
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        print!("{}{}", sign, change);
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
    println!("{}{} [{}] (line {}-{})", indent, node.path, node.node_type, node.start_line, node.end_line);
}