use crate::core::{EditOperation, GnawTreeWriter, TransactionLog, UndoRedoManager};
use crate::parser::TreeNode;
use anyhow::Result;

use clap::{Parser, Subcommand};
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
    /// Undo the last N operations
    Undo {
        #[arg(short, long, default_value = "1")]
        steps: usize,
    },
    /// Redo the last N operations
    Redo {
        #[arg(short, long, default_value = "1")]
        steps: usize,
    },
    /// Show transaction history
    History {
        #[arg(short, long, default_value = "10")]
        limit: usize,
        #[arg(short, long, default_value = "table")]
        format: String,
    },
    /// Restore file to a specific transaction state
    Restore {
        file_path: String,
        transaction_id: String,
        #[arg(short, long)]
        preview: bool,
    },
    /// Start a new session (clears current session history)
    SessionStart,
    /// Show current undo/redo state
    Status,
    /// Restore entire project to a specific timestamp
    RestoreProject {
        timestamp: String,
        #[arg(short, long)]
        preview: bool,
    },
    /// Restore multiple files to state before a timestamp
    RestoreFiles {
        #[arg(short, long)]
        since: String,
        #[arg(short, long)]
        files: Vec<String>,
        #[arg(short, long)]
        preview: bool,
    },
    /// Restore all files modified in a specific session
    RestoreSession {
        session_id: String,
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
            Commands::Analyze {
                paths,
                format: _fmt,
            } => {
                let mut results = Vec::new();
                for path in &paths {
                    let writer = GnawTreeWriter::new(path)?;
                    let tree = writer.analyze();
                    results.push(serde_json::to_value(tree)?);
                }
                println!("{}", serde_json::to_string_pretty(&results)?);
            }
            Commands::List {
                file_path,
                filter_type,
            } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                list_nodes(writer.analyze(), filter_type.as_deref());
            }
            Commands::Show {
                file_path,
                node_path,
            } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                println!("{}", writer.show_node(&node_path)?);
            }
            Commands::Edit {
                file_path,
                node_path,
                content,
                preview,
            } => {
                let mut writer = GnawTreeWriter::new(&file_path)?;
                let op = EditOperation::Edit { node_path, content };
                if preview {
                    let modified = writer.preview_edit(op)?;
                    print_diff(writer.get_source(), &modified);
                } else {
                    writer.edit(op)?;
                }
            }
            Commands::Insert {
                file_path,
                parent_path,
                position,
                content,
                preview,
            } => {
                let mut writer = GnawTreeWriter::new(&file_path)?;
                let op = EditOperation::Insert {
                    parent_path,
                    position,
                    content,
                };
                if preview {
                    let modified = writer.preview_edit(op)?;
                    print_diff(writer.get_source(), &modified);
                } else {
                    writer.edit(op)?;
                }
            }
            Commands::Delete {
                file_path,
                node_path,
                preview,
            } => {
                let mut writer = GnawTreeWriter::new(&file_path)?;
                let op = EditOperation::Delete { node_path };
                if preview {
                    let modified = writer.preview_edit(op)?;
                    print_diff(writer.get_source(), &modified);
                } else {
                    writer.edit(op)?;
                }
            }
            Commands::AddProperty {
                file_path,
                target_path,
                name,
                r#type,
                value,
                preview,
            } => {
                let mut writer = GnawTreeWriter::new(&file_path)?;
                let property_code = format!("property {} {}: {}", r#type, name, value);
                let op = EditOperation::Insert {
                    parent_path: target_path.clone(),
                    position: 2,
                    content: property_code,
                };
                if preview {
                    let modified = writer.preview_edit(op)?;
                    print_diff(writer.get_source(), &modified);
                } else {
                    writer.edit(op)?;
                    println!("Successfully added property '{}' to {}", name, target_path);
                }
            }
            Commands::AddComponent {
                file_path,
                target_path,
                name,
                content,
                preview,
            } => {
                let mut writer = GnawTreeWriter::new(&file_path)?;
                let component_code = match content {
                    Some(c) => format!("{} {{\n    {}\n}}", name, c),
                    None => format!("{} {{}}\n", name),
                };
                let op = EditOperation::Insert {
                    parent_path: target_path.clone(),
                    position: 1,
                    content: component_code,
                };
                if preview {
                    let modified = writer.preview_edit(op)?;
                    print_diff(writer.get_source(), &modified);
                } else {
                    writer.edit(op)?;
                    println!("Successfully added component '{}' to {}", name, target_path);
                }
            }
            Commands::Undo { steps } => {
                Self::handle_undo(steps)?;
            }
            Commands::Redo { steps } => {
                Self::handle_redo(steps)?;
            }
            Commands::History { limit, format } => {
                Self::handle_history(limit, &format)?;
            }
            Commands::Restore {
                file_path,
                transaction_id,
                preview,
            } => {
                Self::handle_restore(&file_path, &transaction_id, preview)?;
            }
            Commands::SessionStart => {
                Self::handle_session_start()?;
            }
            Commands::Status => {
                Self::handle_status()?;
            }
            Commands::RestoreProject { timestamp, preview } => {
                Self::handle_restore_project(&timestamp, preview)?;
            }
            Commands::RestoreFiles {
                since,
                files,
                preview,
            } => {
                Self::handle_restore_files(&since, &files, preview)?;
            }
            Commands::RestoreSession {
                session_id,
                preview,
            } => {
                Self::handle_restore_session(&session_id, preview)?;
            }
        }
        Ok(())
    }

    fn handle_undo(steps: usize) -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let mut undo_manager = UndoRedoManager::new(&current_dir)?;

        let results = undo_manager.undo(steps)?;

        if results.is_empty() {
            println!("Nothing to undo");
            return Ok(());
        }

        for result in results {
            if result.success {
                println!("✓ Undone: {} ({})", result.message, result.transaction_id);
            } else {
                println!(
                    "✗ Failed to undo: {} ({})",
                    result.message, result.transaction_id
                );
            }
        }

        let state = undo_manager.get_state();
        println!(
            "\nUndo/Redo state: {} undo, {} redo available",
            state.undo_available, state.redo_available
        );

        Ok(())
    }

    fn handle_redo(steps: usize) -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let mut undo_manager = UndoRedoManager::new(&current_dir)?;

        let results = undo_manager.redo(steps)?;

        if results.is_empty() {
            println!("Nothing to redo");
            return Ok(());
        }

        for result in results {
            if result.success {
                println!("✓ Redone: {} ({})", result.message, result.transaction_id);
            } else {
                println!(
                    "✗ Failed to redo: {} ({})",
                    result.message, result.transaction_id
                );
            }
        }

        let state = undo_manager.get_state();
        println!(
            "\nUndo/Redo state: {} undo, {} redo available",
            state.undo_available, state.redo_available
        );

        Ok(())
    }

    fn handle_history(limit: usize, format: &str) -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let transaction_log = TransactionLog::load(&current_dir)?;

        let history = transaction_log.get_last_n_transactions(limit)?;

        match format {
            "json" => {
                let json = serde_json::to_string_pretty(&history)?;
                println!("{}", json);
            }
            "table" | _ => {
                if history.is_empty() {
                    println!("No transaction history found");
                    return Ok(());
                }

                println!(
                    "{:<20} {:<10} {:<30} {:<15} {}",
                    "Timestamp", "Operation", "File", "Node Path", "Description"
                );
                println!("{}", "=".repeat(90));

                for transaction in history.iter().rev() {
                    let timestamp = transaction.timestamp.format("%m-%d %H:%M:%S").to_string();
                    let operation = format!("{:?}", transaction.operation);
                    let file_name = transaction
                        .file_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    let node_path = transaction.node_path.as_deref().unwrap_or("N/A");

                    println!(
                        "{:<20} {:<10} {:<30} {:<15} {}",
                        timestamp, operation, file_name, node_path, transaction.description
                    );
                }
            }
        }

        Ok(())
    }

    fn handle_restore(file_path: &str, transaction_id: &str, preview: bool) -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let transaction_log = TransactionLog::load(&current_dir)?;

        let transaction = transaction_log
            .find_transaction(transaction_id)?
            .ok_or_else(|| anyhow::anyhow!("Transaction not found: {}", transaction_id))?;

        if preview {
            println!("Would restore {} to state from transaction:", file_path);
            println!("  ID: {}", transaction.id);
            println!(
                "  Timestamp: {}",
                transaction.timestamp.format("%Y-%m-%d %H:%M:%S")
            );
            println!("  Operation: {:?}", transaction.operation);
            println!("  Description: {}", transaction.description);
            println!("\nUse --no-preview to actually perform the restore");
        } else {
            // TODO: Implement actual restore logic
            println!("Restore functionality not yet implemented");
            println!(
                "Would restore {} using transaction {}",
                file_path, transaction_id
            );
        }

        Ok(())
    }

    fn handle_session_start() -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let mut transaction_log = TransactionLog::load(&current_dir)?;

        transaction_log.start_new_session()?;

        println!("✓ New session started");
        println!("Previous session history has been preserved");

        Ok(())
    }

    fn handle_status() -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let undo_manager = UndoRedoManager::new(&current_dir)?;

        let state = undo_manager.get_state();

        println!("GnawTreeWriter Status:");
        println!("=====================");
        println!("Undo operations available: {}", state.undo_available);
        println!("Redo operations available: {}", state.redo_available);

        if let Some(last_undo) = &state.last_undo {
            println!("Last undo transaction: {}", last_undo);
        }

        if let Some(last_redo) = &state.last_redo {
            println!("Last redo transaction: {}", last_redo);
        }

        // Show recent history
        let transaction_log = TransactionLog::load(&current_dir)?;
        let recent = transaction_log.get_last_n_transactions(5)?;

        if !recent.is_empty() {
            println!("\nRecent transactions:");
            for transaction in recent.iter().rev().take(3) {
                let timestamp = transaction.timestamp.format("%H:%M:%S").to_string();
                println!(
                    "  {} - {:?}: {}",
                    timestamp, transaction.operation, transaction.description
                );
            }
        }

        Ok(())
    }

    fn handle_restore_project(timestamp: &str, preview: bool) -> Result<()> {
        use chrono::DateTime;

        let current_dir = std::env::current_dir()?;
        let transaction_log = TransactionLog::load(&current_dir)?;

        // Parse timestamp
        let restore_to = DateTime::parse_from_rfc3339(timestamp)
            .or_else(|_| DateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S"))
            .or_else(|_| DateTime::parse_from_str(timestamp, "%Y-%m-%dT%H:%M:%S"))
            .map_err(|_| {
                anyhow::anyhow!("Invalid timestamp format. Use RFC3339 or YYYY-MM-DD HH:MM:SS")
            })?
            .with_timezone(&chrono::Utc);

        let plan = transaction_log.get_project_restoration_plan(restore_to)?;

        if !plan.has_changes() {
            println!(
                "No changes found since {}",
                restore_to.format("%Y-%m-%d %H:%M:%S UTC")
            );
            return Ok(());
        }

        if preview {
            println!("Project Restoration Plan:");
            println!("=========================");
            println!("{}", plan.get_summary());
            println!("\nFiles to be restored:");
            for file_plan in &plan.affected_files {
                println!(
                    "  {} ({} modifications since {})",
                    file_plan.file_path.display(),
                    file_plan.current_modifications_count,
                    restore_to.format("%Y-%m-%d %H:%M:%S")
                );
            }
            println!("\nUse --no-preview to perform the restoration");
        } else {
            println!("🚧 Project restoration not yet fully implemented");
            println!(
                "Would restore {} files to state at {}",
                plan.affected_files.len(),
                restore_to.format("%Y-%m-%d %H:%M:%S UTC")
            );
            // TODO: Implement actual multi-file restoration
        }

        Ok(())
    }

    fn handle_restore_files(since: &str, file_patterns: &[String], preview: bool) -> Result<()> {
        use chrono::DateTime;

        let current_dir = std::env::current_dir()?;
        let transaction_log = TransactionLog::load(&current_dir)?;

        // Parse timestamp
        let since_time = DateTime::parse_from_rfc3339(since)
            .or_else(|_| DateTime::parse_from_str(since, "%Y-%m-%d %H:%M:%S"))
            .or_else(|_| DateTime::parse_from_str(since, "%Y-%m-%dT%H:%M:%S"))
            .map_err(|_| {
                anyhow::anyhow!("Invalid timestamp format. Use RFC3339 or YYYY-MM-DD HH:MM:SS")
            })?
            .with_timezone(&chrono::Utc);

        let affected_files = transaction_log.get_affected_files_since(since_time)?;

        // Filter files by patterns (simplified - would need proper glob matching)
        let filtered_files: Vec<_> = if file_patterns.is_empty() {
            affected_files
        } else {
            affected_files
                .into_iter()
                .filter(|file| {
                    file_patterns.iter().any(|pattern| {
                        file.to_string_lossy().contains(pattern)
                            || file
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .contains(pattern)
                    })
                })
                .collect()
        };

        if filtered_files.is_empty() {
            println!(
                "No matching files found that were modified since {}",
                since_time.format("%Y-%m-%d %H:%M:%S UTC")
            );
            return Ok(());
        }

        if preview {
            println!("Files Restoration Plan:");
            println!("=======================");
            println!(
                "Restore {} files to state before {}",
                filtered_files.len(),
                since_time.format("%Y-%m-%d %H:%M:%S UTC")
            );
            println!("\nFiles to be restored:");
            for file in &filtered_files {
                println!("  {}", file.display());
            }
            println!("\nUse --no-preview to perform the restoration");
        } else {
            println!("🚧 File restoration not yet fully implemented");
            println!("Would restore {} files", filtered_files.len());
            // TODO: Implement actual file restoration
        }

        Ok(())
    }

    fn handle_restore_session(session_id: &str, preview: bool) -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let transaction_log = TransactionLog::load(&current_dir)?;

        let session_files = transaction_log.get_session_files(session_id)?;

        if session_files.is_empty() {
            println!("No files found for session: {}", session_id);
            return Ok(());
        }

        if preview {
            println!("Session Restoration Plan:");
            println!("=========================");
            println!("Restore all changes from session: {}", session_id);
            println!("Files affected in this session:");
            for file in &session_files {
                println!("  {}", file.display());
            }
            println!("\nUse --no-preview to perform the restoration");
        } else {
            println!("🚧 Session restoration not yet fully implemented");
            println!(
                "Would restore {} files from session {}",
                session_files.len(),
                session_id
            );
            // TODO: Implement actual session restoration
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
        if node.node_type != f {
            return;
        }
    }
    let indent = "  ".repeat(depth);
    println!(
        "{}{} [{}] (line {}-{})",
        indent, node.path, node.node_type, node.start_line, node.end_line
    );
}
