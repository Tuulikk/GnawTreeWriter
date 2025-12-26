use anyhow::Result;
use clap::{Parser, Subcommand};
use crate::core::{GnawTreeWriter, EditOperation};

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
        /// Path to file
        file_path: String,
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
            Commands::Analyze { file_path } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                let tree = writer.analyze();
                println!("{}", serde_json::to_string_pretty(&tree)?);
            }
            Commands::Show { file_path, node_path } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                let node = writer.show_node(&node_path)?;
                println!("{}", node);
            }
            Commands::Edit { file_path, node_path, content } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                writer.edit(EditOperation::Edit { node_path, content })?;
                println!("Edited successfully");
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
