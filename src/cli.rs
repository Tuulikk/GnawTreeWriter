use crate::core::{
    EditOperation, GnawTreeWriter, RestorationEngine, TransactionLog, UndoRedoManager,
};
use crate::parser::TreeNode;
use anyhow::Result;

use clap::{Parser, Subcommand};
use similar::{ChangeTag, TextDiff};

#[derive(Parser)]
#[command(name = "gnawtreewriter")]
#[command(about = "AI-native temporal code editor for tree-based editing")]
#[command(
    long_about = "GnawTreeWriter is a revolutionary tree-based code editor designed for AI-assisted development.\nIt provides temporal project management, multi-file restoration, and session-based rollback capabilities.\n\nQuick start: gnawtreewriter analyze <file> to see the structure, then edit specific nodes safely.\nFor help with specific commands, use: gnawtreewriter <command> --help"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse files and show their AST tree structure
    ///
    /// Shows the hierarchical structure of code files with node paths for editing.
    /// Perfect for understanding how to target specific parts of your code.
    ///
    /// Examples:
    ///   gnawtreewriter analyze app.py
    ///   gnawtreewriter analyze src/*.rs
    ///   gnawtreewriter analyze . --format summary
    Analyze {
        /// Files or directories to analyze (supports wildcards)
        paths: Vec<String>,
        #[arg(short, long, default_value = "json")]
        /// Output format: json, summary, or table
        format: String,
    },
    /// List all nodes in a file with their paths
    ///
    /// Shows every node in the file with dot-notation paths for precise editing.
    /// Use this to find the exact path for nodes you want to modify.
    ///
    /// Examples:
    ///   gnawtreewriter list app.py
    ///   gnawtreewriter list app.py --filter-type Property
    ///   gnawtreewriter list main.rs --filter-type function_item
    List {
        /// File to list nodes from
        file_path: String,
        #[arg(short, long)]
        /// Filter by node type (e.g., Property, function_item, class_definition)
        filter_type: Option<String>,
    },
    /// Show the content of a specific node
    ///
    /// Display the exact content of a node at the given path.
    /// Use 'list' command first to find available node paths.
    ///
    /// Examples:
    ///   gnawtreewriter show app.py "0.1"
    ///   gnawtreewriter show main.rs "0.2.1"
    Show {
        /// File containing the node
        file_path: String,
        /// Dot-notation path to the node (e.g., "0.1", "0.2.1")
        node_path: String,
    },
    /// Replace the content of a specific node
    ///
    /// Safely replace the content of a node with new code. The edit is validated
    /// for syntax correctness before being applied. A backup is automatically created.
    ///
    /// Examples:
    ///   gnawtreewriter edit app.py "0.1" 'def hello(): print("world")'
    ///   gnawtreewriter edit main.rs "0.2" 'fn main() { println!("Hello!"); }' --preview
    ///   gnawtreewriter edit style.css "0.1.0" 'color: blue;'
    Edit {
        /// File to edit
        file_path: String,
        /// Dot-notation path to the node (use 'list' to find paths)
        node_path: String,
        /// New content to replace the node with
        content: String,
        #[arg(short, long)]
        /// Preview changes without applying them
        preview: bool,
    },
    /// Insert new content into a parent node
    ///
    /// Add new code at a specific position within a parent node.
    /// Position meanings: 0=top, 1=bottom, 2=after properties (QML)
    /// Indentation is automatically detected and applied.
    ///
    /// Examples:
    ///   gnawtreewriter insert app.py "0" 1 'def new_function(): pass'
    ///   gnawtreewriter insert main.qml "0.1" 2 'width: 200'
    ///   gnawtreewriter insert style.css "0" 0 '/* New comment */'
    Insert {
        /// File to insert into
        file_path: String,
        /// Path to parent node where content will be inserted
        parent_path: String,
        /// Position: 0=top, 1=bottom, 2=after properties
        position: usize,
        /// Content to insert
        content: String,
        #[arg(short, long)]
        /// Preview changes without applying them
        preview: bool,
    },
    /// Undo recent edit operations
    ///
    /// Reverse your last edit operations using the transaction log.
    /// This works independently of git - it's session-based undo.
    ///
    /// Examples:
    ///   gnawtreewriter undo
    ///   gnawtreewriter undo --steps 3
    ///   gnawtreewriter undo -s 5
    Undo {
        #[arg(short, long, default_value = "1")]
        /// Number of operations to undo
        steps: usize,
    },
    /// Redo previously undone operations
    ///
    /// Re-apply operations that were undone with the undo command.
    /// Only works on operations that were undone in the current session.
    ///
    /// Examples:
    ///   gnawtreewriter redo
    ///   gnawtreewriter redo --steps 2
    ///   gnawtreewriter redo -s 3
    Redo {
        #[arg(short, long, default_value = "1")]
        /// Number of operations to redo
        steps: usize,
    },
    /// Show transaction history and recent operations
    ///
    /// Display a log of all edit operations with timestamps and descriptions.
    /// Essential for understanding what changed and when.
    ///
    /// Examples:
    ///   gnawtreewriter history
    ///   gnawtreewriter history --limit 20
    ///   gnawtreewriter history --format json
    History {
        #[arg(short, long, default_value = "10")]
        /// Number of recent transactions to show
        limit: usize,
        #[arg(short, long, default_value = "table")]
        /// Output format: table or json
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
    /// Debug hash calculation for troubleshooting
    DebugHash { content: String },
    /// Restore entire project to a specific point in time
    ///
    /// Revolutionary time-travel feature that restores all changed files
    /// to their state at a specific timestamp. Perfect for undoing AI agent sessions.
    ///
    /// Examples:
    ///   gnawtreewriter restore-project "2025-12-27T15:30:00Z" --preview
    ///   gnawtreewriter restore-project "2025-12-27T15:30:00"
    RestoreProject {
        /// Timestamp in ISO format (e.g., "2025-12-27T15:30:00Z")
        timestamp: String,
        #[arg(short, long)]
        /// Preview what would be restored without actually doing it
        preview: bool,
    },
    /// Restore specific files to state before a timestamp
    ///
    /// Selectively restore only certain files that were modified since a timestamp.
    /// Great for undoing changes to specific parts of your project.
    ///
    /// Examples:
    ///   gnawtreewriter restore-files --since "2025-12-27T16:00:00Z" --files "*.py"
    ///   gnawtreewriter restore-files -s "2025-12-27T16:00:00" -f "src/" --preview
    RestoreFiles {
        #[arg(short, long)]
        /// Only restore files modified since this timestamp
        since: String,
        #[arg(short, long)]
        /// File patterns to restore (e.g., "*.py", "src/")
        files: Vec<String>,
        #[arg(short, long)]
        /// Preview what would be restored
        preview: bool,
    },
    /// Undo all changes from a specific session
    ///
    /// Restore all files that were modified during a particular session.
    /// Perfect for undoing an entire AI agent workflow with one command.
    ///
    /// Examples:
    ///   gnawtreewriter restore-session "session_1766859069329812591" --preview
    ///   gnawtreewriter restore-session "session_1766859069329812591"
    RestoreSession {
        /// Session ID from history output
        session_id: String,
        #[arg(short, long)]
        /// Preview what would be restored
        preview: bool,
    },
    /// Delete a node
    Delete {
        file_path: String,
        node_path: String,
        #[arg(short, long)]
        preview: bool,
    },
    /// Add a property to a QML component
    ///
    /// QML-specific command to safely add properties at the correct location
    /// within a QML component. Handles proper positioning automatically.
    ///
    /// Examples:
    ///   gnawtreewriter add-property app.qml "0.1" width int 300
    ///   gnawtreewriter add-property main.qml "0" color string '"red"' --preview
    AddProperty {
        /// QML file to modify
        file_path: String,
        /// Path to QML component (use 'list' to find)
        target_path: String,
        /// Property name (e.g., "width", "height", "color")
        name: String,
        /// Property type (e.g., "int", "string", "bool")
        r#type: String,
        /// Property value (e.g., "300", '"red"', "true")
        value: String,
        #[arg(short, long)]
        /// Preview the addition
        preview: bool,
    },
    /// Add a child component to a QML component
    ///
    /// QML-specific command to add child components like Rectangle, Button, etc.
    /// Creates proper nesting structure automatically.
    ///
    /// Examples:
    ///   gnawtreewriter add-component app.qml "0" Rectangle
    ///   gnawtreewriter add-component main.qml "0.1" Button --content 'text: "Click me"'
    AddComponent {
        /// QML file to modify
        file_path: String,
        /// Path to parent component
        target_path: String,
        /// Component type (e.g., "Rectangle", "Button", "Text")
        name: String,
        #[arg(short, long)]
        /// Optional properties for the component
        content: Option<String>,
        #[arg(short, long)]
        /// Preview the addition
        preview: bool,
    },
    /// Show examples and common workflows
    ///
    /// Display practical examples for common tasks like editing functions,
    /// adding properties, or using time restoration features.
    ///
    /// Examples:
    ///   gnawtreewriter examples
    ///   gnawtreewriter examples --topic editing
    ///   gnawtreewriter examples --topic qml
    ///   gnawtreewriter examples --topic restoration
    Examples {
        #[arg(short, long)]
        /// Show examples for specific topic: editing, qml, restoration, workflow
        topic: Option<String>,
    },
    /// Interactive help wizard
    ///
    /// Start an interactive guide that walks you through common tasks.
    /// Perfect for first-time users or when you're not sure which command to use.
    ///
    /// Examples:
    ///   gnawtreewriter wizard
    ///   gnawtreewriter wizard --task editing
    ///   gnawtreewriter wizard --task restoration
    Wizard {
        #[arg(short, long)]
        /// Jump to specific task: first-time, editing, qml, restoration, troubleshooting
        task: Option<String>,
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
            Commands::Examples { topic } => {
                Self::handle_examples(topic.as_deref())?;
            }
            Commands::Wizard { task } => {
                Self::handle_wizard(task.as_deref())?;
            }
            Commands::DebugHash { content } => {
                Self::handle_debug_hash(&content)?;
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
            let engine = RestorationEngine::new(&current_dir)?;
            let result = engine.execute_project_restoration(&plan)?;
            result.print_summary();
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
            let engine = RestorationEngine::new(&current_dir)?;
            let result = engine.restore_files_before_timestamp(&filtered_files, since_time)?;
            result.print_summary();
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
            let engine = RestorationEngine::new(&current_dir)?;
            let result = engine.restore_session(session_id)?;
            result.print_summary();
        }

        Ok(())
    }

    fn handle_debug_hash(content: &str) -> Result<()> {
        use crate::core::calculate_content_hash;

        let hash = calculate_content_hash(content);
        println!("Content: {:?}", content);
        println!("Hash: {}", hash);

        // Also test with file content if it exists
        if std::path::Path::new(content).exists() {
            let file_content = std::fs::read_to_string(content)?;
            let file_hash = calculate_content_hash(&file_content);
            println!("File content: {:?}", file_content);
            println!("File hash: {}", file_hash);
        }

        Ok(())
    }

    fn handle_examples(topic: Option<&str>) -> Result<()> {
        match topic {
            Some("editing") => {
                println!("🔧 EDITING EXAMPLES");
                println!("==================");
                println!();
                println!("1. Basic workflow:");
                println!("   gnawtreewriter analyze app.py              # See structure");
                println!("   gnawtreewriter list app.py                 # Find node paths");
                println!("   gnawtreewriter edit app.py \"0.1\" 'new code' # Edit specific node");
                println!();
                println!("2. Safe editing with preview:");
                println!("   gnawtreewriter edit main.rs \"0.2\" 'fn main() {{}}' --preview");
                println!("   # Review the diff, then run without --preview");
                println!();
                println!("3. Insert new functions:");
                println!("   gnawtreewriter insert app.py \"0\" 1 'def helper(): return 42'");
                println!(
                    "   gnawtreewriter insert main.rs \"0\" 0 'use std::collections::HashMap;'"
                );
            }
            Some("qml") => {
                println!("⚛️  QML EXAMPLES");
                println!("===============");
                println!();
                println!("1. Add properties to components:");
                println!("   gnawtreewriter add-property app.qml \"0.1\" width int 300");
                println!("   gnawtreewriter add-property app.qml \"0.1\" color string '\"red\"'");
                println!();
                println!("2. Add child components:");
                println!("   gnawtreewriter add-component app.qml \"0\" Rectangle");
                println!("   gnawtreewriter add-component app.qml \"0.1\" Button --content 'text: \"Click\"'");
                println!();
                println!("3. Complex QML editing:");
                println!("   gnawtreewriter list app.qml --filter-type ui_property");
                println!("   gnawtreewriter edit app.qml \"0.2.1\" 'anchors.fill: parent'");
            }
            Some("restoration") => {
                println!("⏰ TIME RESTORATION EXAMPLES");
                println!("===========================");
                println!();
                println!("1. Project-wide time travel:");
                println!("   gnawtreewriter restore-project \"2025-12-27T15:30:00Z\" --preview");
                println!("   gnawtreewriter restore-project \"2025-12-27T15:30:00Z\"");
                println!();
                println!("2. Selective file restoration:");
                println!("   gnawtreewriter restore-files --since \"2025-12-27T16:00:00Z\" --files \"*.py\"");
                println!("   gnawtreewriter restore-files -s \"2025-12-27T16:00:00Z\" -f \"src/\"");
                println!();
                println!("3. Undo AI agent sessions:");
                println!("   gnawtreewriter history                      # Find session ID");
                println!("   gnawtreewriter restore-session \"session_123\" --preview");
                println!("   gnawtreewriter restore-session \"session_123\"");
            }
            Some("workflow") => {
                println!("🔄 COMMON WORKFLOWS");
                println!("==================");
                println!();
                println!("AI Agent Development Workflow:");
                println!("  1. gnawtreewriter session-start             # Start tracking");
                println!("  2. gnawtreewriter analyze src/*.py          # Understand structure");
                println!("  3. gnawtreewriter edit file.py \"0.1\" 'code'  # Make changes");
                println!("  4. gnawtreewriter history                    # Review what happened");
                println!("  5. gnawtreewriter restore-session \"id\"      # Undo if needed");
                println!();
                println!("Safe Refactoring Workflow:");
                println!("  1. gnawtreewriter status                     # Check current state");
                println!(
                    "  2. gnawtreewriter edit file.py \"0.1\" 'new' --preview  # Preview changes"
                );
                println!("  3. gnawtreewriter edit file.py \"0.1\" 'new'  # Apply if good");
                println!("  4. gnawtreewriter undo                       # Quick undo if needed");
            }
            _ => {
                println!("📚 GNAWTREEWRITER EXAMPLES");
                println!("=========================");
                println!();
                println!("Available example topics:");
                println!(
                    "  gnawtreewriter examples --topic editing      # Basic editing workflows"
                );
                println!("  gnawtreewriter examples --topic qml          # QML component editing");
                println!("  gnawtreewriter examples --topic restoration  # Time travel features");
                println!("  gnawtreewriter examples --topic workflow     # Complete workflows");
                println!();
                println!("Quick Start:");
                println!("  1. gnawtreewriter analyze <file>             # See file structure");
                println!("  2. gnawtreewriter edit <file> <path> 'code'  # Edit specific node");
                println!("  3. gnawtreewriter history                     # See what changed");
                println!();
                println!("For interactive guidance, try: gnawtreewriter wizard");
            }
        }
        Ok(())
    }

    fn handle_wizard(task: Option<&str>) -> Result<()> {
        match task {
            Some("first-time") => {
                println!("🧙 FIRST-TIME USER WIZARD");
                println!("=========================");
                println!();
                println!("Welcome to GnawTreeWriter! Let's get you started:");
                println!();
                println!("Step 1: Analyze a file to see its structure");
                println!("  Example: gnawtreewriter analyze app.py");
                println!("  This shows you the tree structure with node paths like '0.1', '0.2.1'");
                println!();
                println!("Step 2: Edit a specific node");
                println!(
                    "  Example: gnawtreewriter edit app.py \"0.1\" 'def hello(): print(\"world\")'"
                );
                println!("  Use the paths from step 1 to target exactly what you want to change");
                println!();
                println!("Step 3: Check what happened");
                println!("  Example: gnawtreewriter history");
                println!("  See all your changes with timestamps");
                println!();
                println!("💡 Pro tips:");
                println!("  • Always use --preview first to see changes safely");
                println!("  • Use 'gnawtreewriter list <file>' to see all available node paths");
                println!(
                    "  • Start a session with 'gnawtreewriter session-start' to group changes"
                );
            }
            Some("editing") => {
                println!("🔧 EDITING WIZARD");
                println!("================");
                println!();
                println!("What do you want to edit?");
                println!();
                println!("A) Edit existing code:");
                println!("   1. gnawtreewriter analyze <file>        # Find the node path");
                println!("   2. gnawtreewriter edit <file> <path> 'new code' --preview");
                println!("   3. Remove --preview to apply");
                println!();
                println!("B) Add new code:");
                println!("   1. gnawtreewriter list <file>           # Find parent node");
                println!("   2. gnawtreewriter insert <file> <parent> 1 'new code'");
                println!("   Position: 0=top, 1=bottom, 2=after properties");
                println!();
                println!("C) Delete code:");
                println!("   1. gnawtreewriter list <file>           # Find node to delete");
                println!("   2. gnawtreewriter delete <file> <path> --preview");
                println!("   3. Remove --preview to apply");
                println!();
                println!("Need help finding the right path? Try: gnawtreewriter list <file>");
            }
            Some("restoration") => {
                println!("⏰ TIME RESTORATION WIZARD");
                println!("==========================");
                println!();
                println!("What do you want to restore?");
                println!();
                println!("A) Undo recent changes:");
                println!("   gnawtreewriter undo                      # Last change");
                println!("   gnawtreewriter undo --steps 3            # Last 3 changes");
                println!();
                println!("B) Go back to specific time:");
                println!("   gnawtreewriter restore-project \"2025-12-27T15:30:00Z\" --preview");
                println!("   (Use ISO timestamp format)");
                println!();
                println!("C) Undo an AI agent session:");
                println!("   1. gnawtreewriter history                # Find session ID");
                println!("   2. gnawtreewriter restore-session <session-id> --preview");
                println!();
                println!("D) Restore specific files:");
                println!("   gnawtreewriter restore-files --since \"2025-12-27T16:00:00Z\" --files \"*.py\"");
                println!();
                println!("💡 Always use --preview first to see what will change!");
            }
            Some("troubleshooting") => {
                println!("🔍 TROUBLESHOOTING WIZARD");
                println!("========================");
                println!();
                println!("Common issues and solutions:");
                println!();
                println!("❌ \"Node not found at path\":");
                println!("   • Run: gnawtreewriter list <file>");
                println!("   • Check that path exists in current file state");
                println!("   • File might have changed - analyze again");
                println!();
                println!("❌ \"Validation failed\":");
                println!("   • Your new code has syntax errors");
                println!("   • Check quotes, brackets, and indentation");
                println!("   • Try smaller changes first");
                println!();
                println!("❌ \"Backup not found\":");
                println!("   • Some restoration operations need existing backups");
                println!("   • Check: ls .gnawtreewriter_backups/");
                println!("   • Use timestamp-based restoration as fallback");
                println!();
                println!("❌ Can't find the right node:");
                println!("   • Use: gnawtreewriter list <file> --filter-type <type>");
                println!("   • Try: gnawtreewriter analyze <file> for overview");
                println!("   • Look for node types like 'function_item', 'class_definition'");
                println!();
                println!("Still stuck? Check: https://github.com/Tuulikk/GnawTreeWriter/issues");
            }
            _ => {
                println!("🧙 GNAWTREEWRITER WIZARD");
                println!("=======================");
                println!();
                println!("What would you like help with?");
                println!();
                println!("Available wizards:");
                println!("  gnawtreewriter wizard --task first-time        # New user guide");
                println!("  gnawtreewriter wizard --task editing           # How to edit code");
                println!("  gnawtreewriter wizard --task restoration       # Time travel features");
                println!("  gnawtreewriter wizard --task troubleshooting   # Fix common problems");
                println!();
                println!("Quick help:");
                println!("  gnawtreewriter examples                        # See example commands");
                println!(
                    "  gnawtreewriter <command> --help                # Help for specific command"
                );
                println!(
                    "  gnawtreewriter --help                          # All available commands"
                );
                println!();
                println!("🎯 Most common first steps:");
                println!("  1. gnawtreewriter analyze <your-file>          # See structure");
                println!("  2. gnawtreewriter wizard --task first-time     # Detailed walkthrough");
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
