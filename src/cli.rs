// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::core::{
    find_project_root, EditOperation, GnawTreeWriter, OperationType, RestorationEngine, TagManager,
    TransactionLog, UndoRedoManager,
};
use crate::parser::TreeNode;
use anyhow::{Context, Result};
use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Subcommand)]
/// Manage the MCP server (Model Context Protocol).
///
/// Use the `serve` subcommand to run a minimal JSON-RPC HTTP endpoint locally.
/// Examples:
///   gnawtreewriter mcp serve --addr 127.0.0.1:8080 --token secret
///   MCP_TOKEN=secret gnawtreewriter mcp serve --addr 0.0.0.0:8080
enum McpSubcommands {
    /// Start MCP server (JSON-RPC over HTTP).
    ///
    /// Options:
    ///   --addr <ADDR>    Address to bind (default: 127.0.0.1:8080)
    ///   --token <TOKEN>  Optional Bearer token for basic auth (can also be set via MCP_TOKEN)
    Serve {
        /// Address to bind (default: 127.0.0.1:8080)
        #[arg(long, default_value = "127.0.0.1:8080")]
        addr: String,
        #[arg(long)]
        /// Optional bearer token for basic auth. If omitted, `MCP_TOKEN` environment variable will be used.
        token: Option<String>,
    },
    /// Start MCP server over Stdio (Standard Input/Output).
    /// Recommended for local integration with Claude Desktop, Zed, or Gemini CLI.
    Stdio,
    /// Check MCP server status and list available tools.
    ///
    /// Options:
    ///   --url <URL>     Server URL (default: http://127.0.0.1:8080/)
    ///   --token <TOKEN>  Optional bearer token (can also be set via MCP_TOKEN)
    Status {
        /// Server URL (default: http://127.0.0.1:8080/)
        #[arg(long, default_value = "http://127.0.0.1:8080/")]
        url: String,
        #[arg(long)]
        /// Optional bearer token for basic auth. If omitted, `MCP_TOKEN` environment variable will be used.
        token: Option<String>,
    },
}

use similar::{ChangeTag, TextDiff};

#[derive(Parser)]
#[command(name = "gnawtreewriter")]
#[command(version)]
#[command(about = "AI-native temporal code editor for tree-based editing")]
#[command(
    long_about = "GnawTreeWriter is a revolutionary tree-based code editor designed for AI-assisted development.
It provides temporal project management, multi-file restoration, and session-based rollback capabilities.

Quick start: gnawtreewriter analyze <file> to see the structure, then edit specific nodes safely.
For help with specific commands, use: gnawtreewriter <command> --help"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(long, global = true)]
    /// Show what would happen without making any changes
    dry_run: bool,
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
    /// Files or directories to analyze (supports wildcards and recursive directory scanning)
    ///
    /// By design, directories require the --recursive flag for safety and clarity.
    /// This prevents accidental analysis of large directory trees.
    ///
    /// Examples:
    ///   gnawtreewriter analyze app.py
    ///   gnawtreewriter analyze src/*.rs
    ///   gnawtreewriter analyze src/ --recursive
    Analyze {
        /// Files or directories to analyze. Directories require --recursive flag
        paths: Vec<String>,
        #[arg(short, long, default_value = "json")]
        /// Output format: json, summary, or table
        format: String,
        #[arg(long)]
        /// Required flag to analyze directories (prevents accidental large scans)
        recursive: bool,
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
    ///   gnawtreewriter edit main.rs tag:my_function 'def updated(): print("Updated")' --preview
    Edit {
        /// File to edit
        file_path: String,
        /// Dot-notation path to the node (use 'list' to find paths)
        #[arg(required_unless_present = "tag")]
        node_path: Option<String>,
        #[arg(long)]
        /// Named reference (tag) for the target node
        tag: Option<String>,
        /// New content to replace the node with. Use "-" to read from stdin.
        #[arg(required_unless_present = "source_file")]
        content: Option<String>,
        /// Read content from a file instead of command line
        #[arg(long, conflicts_with = "content")]
        source_file: Option<String>,
        #[arg(short, long)]
        /// Preview changes without applying them
        preview: bool,
        #[arg(long)]
        /// Manually unescape \n sequences in the content (useful for some shells)
        unescape_newlines: bool,
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
        #[arg(required_unless_present = "tag")]
        parent_path: Option<String>,
        #[arg(long)]
        /// Named reference (tag) for the parent node
        tag: Option<String>,
        /// Position: 0=top, 1=bottom, 2=after properties
        position: usize,
        /// Content to insert. Use "-" to read from stdin.
        #[arg(required_unless_present = "source_file")]
        content: Option<String>,
        /// Read content from a file instead of command line
        #[arg(long, conflicts_with = "content")]
        source_file: Option<String>,
        #[arg(short, long)]
        /// Preview changes without applying them
        preview: bool,
        #[arg(long)]
        /// Manually unescape \n sequences in the content (useful for some shells)
        unescape_newlines: bool,
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
    /// Execute a batch of operations atomically from a JSON file
    ///
    /// Applies multiple edits/inserts/deletes atomically after in-memory validation.
    /// Example: `gnawtreewriter batch ops.json --preview`
    Batch {
        /// JSON file containing batch operations
        file: String,
        #[arg(short, long)]
        /// Preview changes without applying
        preview: bool,
    },
    /// Search nodes by text or name
    ///
    /// Find nodes containing a specific pattern or name.
    /// Examples:
    ///   gnawtreewriter search main.rs "println!"
    ///   gnawtreewriter search src/lib.rs "TreeNode"
    Search {
        /// File to search in
        file_path: String,
        /// Text pattern to search for
        pattern: String,
        #[arg(short, long)]
        /// Filter by node type (e.g., function_definition)
        filter_type: Option<String>,
        #[arg(short, long)]
        /// Maximum number of matches to show
        limit: Option<usize>,
    },
    /// Get a high-level skeletal view of a file
    ///
    /// Shows only definitions (functions, classes, structs) up to a certain depth.
    /// Examples:
    ///   gnawtreewriter skeleton main.rs
    ///   gnawtreewriter skeleton lib.rs --depth 3
    Skeleton {
        /// File to analyze
        file_path: String,
        #[arg(short, long, default_value = "2")]
        /// Maximum depth to show
        depth: usize,
    },
    /// Generate a semantic code quality report using AI
    ///
    /// Analyzes the file using ModernBERT to find structural anomalies or complexity.
    /// (Requires 'modernbert' feature to be enabled)
    SemanticReport {
        /// File to analyze
        file_path: String,
    },
    /// Convert a unified diff to a batch operation specification
    ///
    /// Parses a git diff format file and converts it to a batch JSON file.
    /// This allows AI agents and users to provide diffs that can be previewed and applied atomically.
    ///
    /// Examples:
    ///   gnawtreewriter diff-to-batch changes.patch
    ///   gnawtreewriter diff-to-batch changes.patch --output batch.json
    ///   gnawtreewriter diff-to-batch changes.patch --preview
    DiffToBatch {
        /// Diff file in unified format (git diff output)
        diff_file: String,
        #[arg(short, long)]
        /// Output JSON file for batch specification (default: batch.json)
        output: Option<String>,
        #[arg(short, long)]
        /// Preview the converted batch without writing to file
        preview: bool,
    },
    /// Restore file to a specific transaction state
    Restore {
        file_path: String,
        transaction_id: String,
        preview: bool,
    },
    /// Quick replace: simple search-and-replace in a file with preview and automatic backup
    QuickReplace {
        /// File to operate on
        file: String,
        /// Search pattern (literal string)
        search: String,
        /// Replacement text
        replace: String,
        #[arg(short, long)]
        /// Show preview but don't apply
        preview: bool,
        #[arg(long)]
        /// Manually unescape \n sequences (useful for some shells)
        unescape_newlines: bool,
    },
    /// Refactor: rename symbols across files with AST-aware renaming
    ///
    /// Revolutionary AST-based renaming that understands code structure.
    /// Rename functions, variables, classes with confidence - knows declarations from usages.
    /// Perfect for large refactorings where search-and-replace would be dangerous.
    ///
    /// Examples:
    ///   gnawtreewriter refactor rename myFunction newFunction app.py --preview
    ///   gnawtreewriter refactor rename MyClass NewClass src/ --recursive
    ///   gnawtreewriter refactor rename count increment main.rs
    Rename {
        /// Symbol name to rename (function, variable, class, etc.)
        symbol_name: String,
        /// New name for symbol
        new_name: String,
        /// Starting file or directory to search for symbol
        path: String,
        #[arg(short, long)]
        /// Recursively search in directory
        recursive: bool,
        #[arg(short, long)]
        /// Preview changes without applying them
        preview: bool,
    },
    /// Clone: duplicate code nodes or entire files
    ///
    /// Copy code structures (functions, classes, etc.) within or between files.
    /// Perfect for creating similar components or duplicating boilerplate code.
    ///
    /// Examples:
    ///   gnawtreewriter clone app.py "0.1" "0.2" --preview
    ///   gnawtreewriter clone src.rs "1.0" dest.rs "2.0"
    ///   gnawtreewriter clone main.py "0.1.2" utils.py "0.0" --preview
    Clone {
        /// Source file path
        source_file: String,
        /// Source node path (use 'list' to find paths)
        source_path: String,
        /// Target file path (can be same as source)
        #[arg(required_unless_present = "target_path")]
        target_file: Option<String>,
        /// Target node path where to insert cloned content
        #[arg(required_unless_present = "target_file")]
        target_path: Option<String>,
        #[arg(short, long)]
        /// Preview changes without applying them
        preview: bool,
    },
    /// Debug hash calculation for troubleshooting
    DebugHash { content: String },
    /// Start a new session (clears current session history)
    SessionStart,
    /// Show current undo/redo state
    Status,
    /// Manage the MCP server (Model Context Protocol).
    ///
    /// Examples:
    ///   gnawtreewriter mcp serve --addr 127.0.0.1:8080 --token secret
    ///   MCP_TOKEN=secret gnawtreewriter mcp serve --addr 0.0.0.0:8080
    Mcp {
        #[command(subcommand)]
        command: McpSubcommands,
    },
    /// Restore entire project to a specific point in time
    ///
    /// Revolutionary time-travel feature that restores all changed files
    /// to their state at a specific timestamp. Perfect for undoing AI agent sessions.
    ///
    /// Examples:
    ///   gnawtreewriter restore-project "2025-12-27T15:30:00Z" --preview
    ///   gnawtreewriter restore-project "2025-12-27T15:30:00"
    RestoreProject {
        /// Timestamp (e.g., "2025-12-27 15:30:00" for local, or RFC3339)
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
    ///   gnawtreewriter restore-files --since "2025-12-27 16:00:00" --files "*.py"
    ///   gnawtreewriter restore-files -s "2025-12-27T16:00:00Z" -f "src/" --preview
    RestoreFiles {
        #[arg(short, long)]
        /// Only restore files modified since this timestamp (Local or UTC)
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
        #[arg(required_unless_present = "tag")]
        node_path: Option<String>,
        #[arg(long)]
        /// Named reference (tag) for the target node
        tag: Option<String>,
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
    /// Manage named references (tags)
    ///
    /// Assign memorable names to node paths to make scripting robust to structural changes.
    /// Examples:
    ///   gnawtreewriter tag add main.rs "1.2.0" "my_function"
    ///   gnawtreewriter tag list main.rs
    ///   gnawtreewriter tag remove main.rs "my_function"
    Tag {
        #[command(subcommand)]
        command: TagSubcommands,
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
    ///   gnawtreewriter examples --topic batch
    Examples {
        #[arg(short, long)]
        /// Show examples for specific topic: editing, qml, restoration, workflow, batch
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
        /// Jump to specific task: first-time, editing, qml, restoration, batch, troubleshooting
        task: Option<String>,
    },
    /// Lint files and show issues with severity levels
    ///
    /// Analyze files for potential issues and coding standard violations.
    /// This is a convenience wrapper around analyze with issue detection.
    ///
    /// By design, directories require the --recursive flag for safety.
    ///
    /// Examples:
    ///   gnawtreewriter lint app.py
    ///   gnawtreewriter lint src/ --recursive
    ///   gnawtreewriter lint . --recursive --format json
    Lint {
        /// Files or directories to lint. Directories require --recursive flag
        paths: Vec<String>,
        #[arg(short, long, default_value = "text")]
        /// Output format: text or json
        format: String,
        #[arg(long)]
        /// Required flag to lint directories (prevents accidental large scans)
        recursive: bool,
    },
}

#[derive(Subcommand)]
enum TagSubcommands {
    /// Add a named reference to a tree node path
    Add {
        /// File containing the node
        file_path: String,
        /// Dot-notation path to the node (use 'list' to find paths)
        node_path: String,
        /// Name to assign to this path
        name: String,
        /// Force overwrite if tag exists
        #[arg(short, long)]
        force: bool,
    },
    /// List all named references for a file
    List {
        /// File to list tags for
        file_path: String,
    },
    /// Remove a named reference
    Remove {
        /// File containing the tag
        file_path: String,
        /// Tag name to remove
        name: String,
    },
    /// Rename an existing tag
    Rename {
        /// File containing the tag
        file_path: String,
        /// Existing tag name
        old_name: String,
        /// New tag name
        new_name: String,
        /// Force overwrite if target exists
        #[arg(short, long)]
        force: bool,
    },
}
impl Cli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::Analyze {
                paths,
                format: _fmt,
                recursive,
            } => {
                Self::handle_analyze(&paths, &_fmt, recursive)?;
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
                tag,
                content,
                source_file,
                preview,
                unescape_newlines,
            } => {
                let content = resolve_content(content, source_file, unescape_newlines)?;

                // Resolve target path from --tag flag, 'tag:<name>' positional, or explicit node_path
                let target_path = if let Some(tag_name) = tag {
                    let current_dir = std::env::current_dir()?;
                    let project_root = find_project_root(&current_dir);
                    let mgr = TagManager::load(&project_root)?;
                    mgr.get_path(&file_path, &tag_name).ok_or_else(|| {
                        anyhow::anyhow!("Tag '{}' not found for {}", tag_name, file_path)
                    })?
                } else if let Some(p) = node_path {
                    // Support inline 'tag:<name>' syntax in the positional node_path
                    if let Some(tag_name) = p.strip_prefix("tag:") {
                        let current_dir = std::env::current_dir()?;
                        let project_root = find_project_root(&current_dir);
                        let mgr = TagManager::load(&project_root)?;
                        mgr.get_path(&file_path, tag_name).ok_or_else(|| {
                            anyhow::anyhow!("Tag '{}' not found for {}", tag_name, file_path)
                        })?
                    } else {
                        p
                    }
                } else {
                    anyhow::bail!("Either node path or --tag must be specified for edit");
                };

                let mut writer = GnawTreeWriter::new(&file_path)?;
                let op = EditOperation::Edit {
                    node_path: target_path,
                    content,
                };
                if preview {
                    let modified = writer.preview_edit(op)?;
                    print_diff(writer.get_source(), &modified);
                } else {
                    writer.edit(op)?;
                    show_hint();
                }
            }
            Commands::Insert {
                file_path,
                parent_path,
                tag,
                position,
                content,
                source_file,
                preview,
                unescape_newlines,
            } => {
                let content = resolve_content(content, source_file, unescape_newlines)?;

                // Resolve parent path from --tag flag, 'tag:<name>' positional, or explicit parent_path
                let insert_parent = if let Some(tag_name) = tag {
                    let current_dir = std::env::current_dir()?;
                    let project_root = find_project_root(&current_dir);
                    let mgr = TagManager::load(&project_root)?;
                    mgr.get_path(&file_path, &tag_name).ok_or_else(|| {
                        anyhow::anyhow!("Tag '{}' not found for {}", tag_name, file_path)
                    })?
                } else if let Some(p) = parent_path {
                    if let Some(tag_name) = p.strip_prefix("tag:") {
                        let current_dir = std::env::current_dir()?;
                        let project_root = find_project_root(&current_dir);
                        let mgr = TagManager::load(&project_root)?;
                        mgr.get_path(&file_path, tag_name).ok_or_else(|| {
                            anyhow::anyhow!("Tag '{}' not found for {}", tag_name, file_path)
                        })?
                    } else {
                        p
                    }
                } else {
                    anyhow::bail!("Either parent path or --tag must be specified for insert");
                };

                let mut writer = GnawTreeWriter::new(&file_path)?;
                let op = EditOperation::Insert {
                    parent_path: insert_parent,
                    position,
                    content,
                };
                if preview {
                    let modified = writer.preview_edit(op)?;
                    print_diff(writer.get_source(), &modified);
                } else {
                    writer.edit(op)?;
                    show_hint();
                }
            }
            Commands::Delete {
                file_path,
                node_path,
                tag,
                preview,
            } => {
                // Resolve target path from --tag flag, 'tag:<name>' positional, or explicit node_path
                let target_path = if let Some(tag_name) = tag {
                    let current_dir = std::env::current_dir()?;
                    let project_root = find_project_root(&current_dir);
                    let mgr = TagManager::load(&project_root)?;
                    mgr.get_path(&file_path, &tag_name).ok_or_else(|| {
                        anyhow::anyhow!("Tag '{}' not found for {}", tag_name, file_path)
                    })?
                } else if let Some(p) = node_path {
                    if let Some(tag_name) = p.strip_prefix("tag:") {
                        let current_dir = std::env::current_dir()?;
                        let project_root = find_project_root(&current_dir);
                        let mgr = TagManager::load(&project_root)?;
                        mgr.get_path(&file_path, tag_name).ok_or_else(|| {
                            anyhow::anyhow!("Tag '{}' not found for {}", tag_name, file_path)
                        })?
                    } else {
                        p
                    }
                } else {
                    anyhow::bail!("Either node path or --tag must be specified for delete");
                };

                let mut writer = GnawTreeWriter::new(&file_path)?;
                let op = EditOperation::Delete {
                    node_path: target_path,
                };
                if preview {
                    let modified = writer.preview_edit(op)?;
                    print_diff(writer.get_source(), &modified);
                } else {
                    writer.edit(op)?;
                    show_hint();
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
                    show_hint();
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
                    Some(c) => format!(
                        "{} {{
    {}
}}",
                        name, c
                    ),
                    None => format!(
                        "{} {{}}
",
                        name
                    ),
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
                    show_hint();
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
            Commands::QuickReplace {
                file,
                search,
                replace,
                preview,
                unescape_newlines,
            } => {
                Self::handle_quick_replace(&file, &search, &replace, unescape_newlines, preview)?;
            }
            Commands::Rename {
                symbol_name,
                new_name,
                path,
                recursive,
                preview,
            } => {
                Self::handle_rename(&symbol_name, &new_name, &path, recursive, preview)?;
            }
            Commands::Clone {
                source_file,
                source_path,
                target_file,
                target_path,
                preview,
            } => {
                Self::handle_clone(
                    &source_file,
                    &source_path,
                    target_file.as_deref(),
                    target_path.as_deref(),
                    preview,
                )?;
            }
            Commands::SessionStart => {
                Self::handle_session_start()?;
            }
            Commands::Status => {
                Self::handle_status()?;
            }
            Commands::Mcp { command } => match command {
                McpSubcommands::Serve { addr, token } => {
                    #[cfg(not(feature = "mcp"))]
                    {
                        let _ = addr;
                        let _ = token;
                        let _ = std::env::var("MCP_TOKEN");
                        anyhow::bail!("MCP feature is not enabled. Recompile with --features mcp");
                    }
                    #[cfg(feature = "mcp")]
                    {
                        let token = token.or_else(|| std::env::var("MCP_TOKEN").ok());
                        crate::mcp::mcp_server::serve(&addr, token).await?;
                    }
                }
                McpSubcommands::Stdio => {
                    #[cfg(not(feature = "mcp"))]
                    {
                        anyhow::bail!("MCP feature is not enabled. Recompile with --features mcp");
                    }
                    #[cfg(feature = "mcp")]
                    {
                        crate::mcp::mcp_server::serve_stdio().await?;
                    }
                }
                McpSubcommands::Status { url, token } => {
                    #[cfg(not(feature = "mcp"))]
                    {
                        let _ = url;
                        let _ = token;
                        let _ = std::env::var("MCP_TOKEN");
                        anyhow::bail!("MCP feature is not enabled. Recompile with --features mcp");
                    }
                    #[cfg(feature = "mcp")]
                    {
                        let token = token.or_else(|| std::env::var("MCP_TOKEN").ok());
                        crate::mcp::mcp_server::status(&url, token).await?;
                    }
                }
            },
            Commands::Examples { topic } => {
                Self::handle_examples(topic.as_deref())?;
            }
            Commands::Wizard { task } => {
                Self::handle_wizard(task.as_deref())?;
            }
            Commands::Lint {
                paths,
                format,
                recursive,
            } => {
                Self::handle_lint(&paths, &format, recursive)?;
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
            Commands::Tag { command } => match command {
                TagSubcommands::Add {
                    file_path,
                    node_path,
                    name,
                    force,
                } => {
                    Self::handle_tag_add(&file_path, &node_path, &name, force)?;
                }
                TagSubcommands::List { file_path } => {
                    Self::handle_tag_list(&file_path)?;
                }
                TagSubcommands::Remove { file_path, name } => {
                    Self::handle_tag_remove(&file_path, &name)?;
                }
                TagSubcommands::Rename {
                    file_path,
                    old_name,
                    new_name,
                    force,
                } => {
                    Self::handle_tag_rename(&file_path, &old_name, &new_name, force)?;
                }
            },
            Commands::RestoreSession {
                session_id,
                preview,
            } => {
                Self::handle_restore_session(&session_id, preview)?;
            }
            Commands::Batch { file, preview } => {
                Self::handle_batch(&file, preview)?;
            }
            Commands::DiffToBatch {
                diff_file,
                output,
                preview,
            } => {
                Self::handle_diff_to_batch(&diff_file, output.as_deref(), preview)?;
            }
            Commands::Search { file_path, pattern, filter_type, limit } => {
                Self::handle_search(&file_path, &pattern, filter_type.as_deref(), limit)?;
            }
            Commands::Skeleton { file_path, depth } => {
                Self::handle_skeleton(&file_path, depth)?;
            }
            Commands::SemanticReport { file_path } => {
                Self::handle_semantic_report(&file_path).await?;
            }
        }
        Ok(())
    }

    fn handle_tag_add(file_path: &str, node_path: &str, name: &str, force: bool) -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);
        let mut mgr = TagManager::load(&project_root)?;

        // Validate node exists in file
        let writer = GnawTreeWriter::new(file_path)?;
        fn node_exists(tree: &TreeNode, path: &str) -> bool {
            if tree.path == path {
                return true;
            }
            for child in &tree.children {
                if node_exists(child, path) {
                    return true;
                }
            }
            false
        }
        if !node_exists(writer.analyze(), node_path) {
            anyhow::bail!("Path '{}' not found in {}", node_path, file_path);
        }

        mgr.add_tag(file_path, name, node_path, force)?;
        println!("✓ Tag '{}' added to {} -> {}", name, file_path, node_path);
        Ok(())
    }

    fn handle_tag_list(file_path: &str) -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);
        let mgr = TagManager::load(&project_root)?;
        let tags = mgr.list_tags(file_path);
        if tags.is_empty() {
            println!("No tags found for {}", file_path);
            return Ok(());
        }
        println!("Tags for {}:", file_path);
        for (name, path) in tags {
            println!("  {} -> {}", name, path);
        }
        Ok(())
    }

    fn handle_tag_remove(file_path: &str, name: &str) -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);
        let mut mgr = TagManager::load(&project_root)?;
        if mgr.remove_tag(file_path, name)? {
            println!("✓ Removed tag '{}' from {}", name, file_path);
        } else {
            println!("No tag '{}' found for {}", name, file_path);
        }
        Ok(())
    }

    fn handle_tag_rename(
        file_path: &str,
        old_name: &str,
        new_name: &str,
        force: bool,
    ) -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);
        let mut mgr = TagManager::load(&project_root)?;

        mgr.rename_tag(file_path, old_name, new_name, force)?;

        println!(
            "✓ Renamed tag '{}' -> '{}' in {}",
            old_name, new_name, file_path
        );
        Ok(())
    }

    fn handle_undo(steps: usize) -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);
        let mut undo_manager = UndoRedoManager::new(&project_root)?;

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
            "
Undo/Redo state: {} undo, {} redo available",
            state.undo_available, state.redo_available
        );

        Ok(())
    }

    fn handle_redo(steps: usize) -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);
        let mut undo_manager = UndoRedoManager::new(&project_root)?;

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
            "
Undo/Redo state: {} undo, {} redo available",
            state.undo_available, state.redo_available
        );

        Ok(())
    }

    fn handle_history(limit: usize, format: &str) -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);
        let transaction_log = TransactionLog::load(project_root)?;

        let history = transaction_log.get_last_n_transactions(limit)?;

        match format {
            "json" => {
                let json = serde_json::to_string_pretty(&history)?;
                println!("{}", json);
            }
            _ => {
                if history.is_empty() {
                    println!("No transaction history found");
                    return Ok(());
                }

                println!(
                    "{:<20} {:<10} {:<30} {:<15} Description",
                    "Timestamp", "Operation", "File", "Node Path"
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
        let project_root = find_project_root(&current_dir);
        let transaction_log = TransactionLog::load(project_root.clone())?;

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
            println!(
                "
Use --no-preview to actually perform the restore"
            );
        } else {
            // Perform the restore using the RestorationEngine
            let engine = RestorationEngine::new(&project_root)?;

            match engine.restore_file_to_transaction(transaction_id) {
                Ok(restored_path) => {
                    println!("✓ Restored: {}", restored_path.display());
                }
                Err(e) => {
                    println!("❌ Restore failed: {}", e);
                }
            }
        }

        Ok(())
    }

    fn handle_batch(file: &str, preview: bool) -> Result<()> {
        // Load and execute batch file; preview shows diffs, apply writes atomically
        let batch = crate::core::Batch::from_file(file)
            .with_context(|| format!("Failed to load batch file: {}", file))?;
        if preview {
            println!("{}", batch.preview_text()?);
        } else {
            batch.apply()?;
            println!("✓ Batch applied");
        }
        Ok(())
    }

    fn handle_diff_to_batch(diff_file: &str, output: Option<&str>, preview: bool) -> Result<()> {
        use crate::core::diff_parser::{diff_to_batch, parse_diff_file, preview_diff};

        // Parse the diff file
        let parsed = parse_diff_file(diff_file)
            .with_context(|| format!("Failed to parse diff file: {}", diff_file))?;

        // Show preview of what the diff contains
        println!("{}", preview_diff(&parsed));

        // Convert to batch operation
        let batch =
            diff_to_batch(&parsed).with_context(|| "Failed to convert diff to batch operation")?;

        if preview {
            // Show what the batch will do
            println!(
                "
=== Batch Preview ==="
            );
            println!("{}", batch.preview_text()?);
            println!(
                "
Use --no-preview to write batch file"
            );
            return Ok(());
        }

        // Determine output file path
        let output_file = output.unwrap_or("batch.json");

        // Write batch specification to JSON
        let batch_json = serde_json::to_string_pretty(&batch)?;
        std::fs::write(output_file, batch_json)
            .with_context(|| format!("Failed to write batch file: {}", output_file))?;

        println!("✓ Diff converted to batch specification: {}", output_file);
        println!(
            "  Apply with: gnawtreewriter batch {} --preview",
            output_file
        );

        Ok(())
    }

    fn handle_search(file_path: &str, pattern: &str, filter_type: Option<&str>, limit: Option<usize>) -> Result<()> {
        let writer = GnawTreeWriter::new(file_path)?;
        let tree = writer.analyze();
        let mut matches = Vec::new();

        fn find(n: &TreeNode, acc: &mut Vec<(String, String, String)>, p: &str, f: Option<&str>) {
            if n.content.contains(p) {
                if f.map_or(true, |filter| n.node_type == filter) {
                    let name = n.get_name().unwrap_or_else(|| "unnamed".to_string());
                    acc.push((n.path.clone(), n.node_type.clone(), name));
                }
            }
            for child in &n.children {
                find(child, acc, p, f);
            }
        }

        find(tree, &mut matches, pattern, filter_type);

        // Sort by relevance (node types containing "definition" or "item" first)
        matches.sort_by(|a, b| {
            let a_is_def = a.1.contains("definition") || a.1.contains("item");
            let b_is_def = b.1.contains("definition") || b.1.contains("item");
            b_is_def.cmp(&a_is_def)
        });

        let total_found = matches.len();
        if let Some(l) = limit {
            matches.truncate(l);
        }

        if matches.is_empty() {
            println!("No matches found for '{}' in {}", pattern, file_path);
        } else {
            println!("Found {} matches in {} (showing {}):", total_found, file_path, matches.len());
            for (path, node_type, name) in matches {
                println!("  {} [{}] '{}'", path, node_type, name);
            }
        }
        Ok(())
    }

    fn handle_skeleton(file_path: &str, max_depth: usize) -> Result<()> {
        let writer = GnawTreeWriter::new(file_path)?;
        let tree = writer.analyze();

        println!("Skeletal view of {} (max depth {}):", file_path, max_depth);

        fn build(n: &TreeNode, d: usize, md: usize) {
            if d > md {
                return;
            }
            let indent = "  ".repeat(d);
            let name = n.get_name().unwrap_or_default();
            println!("{}{} [{}] {}", indent, n.path, n.node_type, name);
            for child in &n.children {
                build(child, d + 1, md);
            }
        }

        build(tree, 0, max_depth);
        Ok(())
    }

    async fn handle_semantic_report(file_path: &str) -> Result<()> {
        #[cfg(feature = "modernbert")]
        {
            let current_dir = std::env::current_dir()?;
            let project_root = find_project_root(&current_dir);
            let mgr = crate::llm::ai_manager::AiManager::new(&project_root)?;

            println!("Generating semantic report for {}...", file_path);
            match mgr.generate_semantic_report(file_path).await {
                Ok(report) => {
                    println!("\nSemantic Report: {}", file_path);
                    println!("=====================================");
                    println!("Summary: {}", report.summary);
                    println!("\nFindings:");
                    for finding in report.findings {
                        println!("- [{}] {}: {}", finding.severity, finding.category, finding.message);
                    }
                }
                Err(e) => {
                    println!("Error generating report: {}", e);
                }
            }
        }
        #[cfg(not(feature = "modernbert"))]
        {
            let _ = file_path;
            println!("Error: 'modernbert' feature not enabled in this build.");
            println!("Please recompile with: cargo build --release --features modernbert");
        }
        Ok(())
    }

    fn handle_session_start() -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);
        let mut transaction_log = TransactionLog::load(project_root)?;

        transaction_log.start_new_session()?;

        println!("✓ New session started");
        println!("Previous session history has been preserved");

        Ok(())
    }

    fn handle_status() -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);
        let undo_manager = UndoRedoManager::new(&project_root)?;

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
        let transaction_log = TransactionLog::load(project_root)?;
        let recent = transaction_log.get_last_n_transactions(5)?;

        if !recent.is_empty() {
            println!(
                "
Recent transactions:"
            );
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
        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);
        let transaction_log = TransactionLog::load(project_root.clone())?;

        // Parse timestamp (supports Local and UTC/RFC3339)
        let restore_to = parse_user_timestamp(timestamp)?;

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
            println!(
                "
Files to be restored:"
            );
            for file_plan in &plan.affected_files {
                println!(
                    "  {} ({} modifications since {})",
                    file_plan.file_path.display(),
                    file_plan.current_modifications_count,
                    restore_to.format("%Y-%m-%d %H:%M:%S")
                );
            }
            println!(
                "
Use --no-preview to perform the restoration"
            );
        } else {
            let engine = RestorationEngine::new(&project_root)?;
            let result = engine.execute_project_restoration(&plan)?;
            result.print_summary();
        }

        Ok(())
    }

    fn handle_restore_files(since: &str, file_patterns: &[String], preview: bool) -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);
        let transaction_log = TransactionLog::load(project_root.clone())?;

        // Parse timestamp (supports Local and UTC/RFC3339)
        let since_time = parse_user_timestamp(since)?;

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
            println!(
                "
Files to be restored:"
            );
            for file in &filtered_files {
                println!("  {}", file.display());
            }
            println!(
                "
Use --no-preview to perform the restoration"
            );
        } else {
            let engine = RestorationEngine::new(&project_root)?;
            let result = engine.restore_files_before_timestamp(&filtered_files, since_time)?;
            result.print_summary();
        }

        Ok(())
    }

    fn handle_restore_session(session_id: &str, preview: bool) -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);
        let transaction_log = TransactionLog::load(project_root.clone())?;

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
            println!(
                "
Use --no-preview to perform the restoration"
            );
        } else {
            let engine = RestorationEngine::new(&project_root)?;
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
            Some("search") => {
                println!("🔍 SEARCH EXAMPLES");
                println!("==================");
                println!();
                println!("1. Find nodes by name:");
                println!("   gnawtreewriter search main.rs \"main\"");
                println!("   # Finds all nodes containing 'main'");
                println!();
                println!("2. Find nodes by pattern:");
                println!("   gnawtreewriter search app.py \"print\"");
                println!("   # Finds all print statements");
                println!();
                println!("3. Find specific patterns:");
                println!("   gnawtreewriter search src/lib.rs \"TreeNode\"");
                println!("   # Finds all references to TreeNode");
            }
            Some("skeleton") => {
                println!("🦴 SKELETON VIEW EXAMPLES");
                println!("==========================");
                println!();
                println!("1. High-level overview (default):");
                println!("   gnawtreewriter skeleton main.rs");
                println!("   # Shows top-level definitions");
                println!();
                println!("2. Custom depth:");
                println!("   gnawtreewriter skeleton src/lib.rs --depth 3");
                println!("   # Shows nested functions and methods");
                println!();
                println!("3. Compare structures:");
                println!("   gnawtreewriter skeleton file1.rs");
                println!("   gnawtreewriter skeleton file2.rs");
                println!("   # Easy visual comparison");
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
            Some("batch") => {
                println!("📦 BATCH OPERATIONS EXAMPLES");
                println!("==========================");
                println!();
                println!("1. Basic workflow:");
                println!("   gnawtreewriter batch update.json --preview");
                println!("   gnawtreewriter batch update.json");
                println!();
                println!("2. Batch JSON structure:");
                println!("   Format: See BATCH_USAGE.md for complete JSON format");
                println!("   Key components: description, operations array");
                println!();
                println!("3. Operation types:");
                println!("   - edit: Replace node content");
                println!("   - insert: Add new content (position: 0=top, 1=bottom, 2=after props)");
                println!("   - delete: Remove a node");
                println!();
                println!("4. Use with tags:");
                println!("   gnawtreewriter tag add app.qml \"1.1\" mainRect");
                println!("   # Use path '1.1' in batch operations");
                println!();
                println!("**Key Features:**");
                println!("  ✅ Atomic validation - All ops validated in-memory");
                println!("  ✅ Unified preview - See all changes before applying");
                println!("  ✅ Automatic rollback - Rollback on failure");
                println!("  ✅ Transaction logging - Each file logged separately");
                println!();
                println!("See BATCH_USAGE.md for complete documentation and examples.");
            }
            Some("quick") => {
                println!("⚡ QUICK-REPLACE EXAMPLES");
                println!("===========================");
                println!();
                println!("1. Text-based search and replace:");
                println!("   gnawtreewriter quick-replace app.py 'old_function' 'new_function' --preview");
                println!("   gnawtreewriter quick-replace app.py 'old_function' 'new_function'");
                println!();
                println!("2. Replace text patterns:");
                println!("   gnawtreewriter quick-replace main.rs \"println!(\\\"Hello\\\")\" \"println!(\\\"Hi\\\")\"");
                println!();
                println!("3. Safety features:");
                println!("   --preview: Show diff without applying changes");
                println!("   Automatic backup before apply");
                println!("   Parser validation for supported file types");
                println!("   Transaction logging for undo/redo");
                println!();
                println!("**Use Cases:**");
                println!("  ✅ Quick text replacements");
                println!("  ✅ Simple search-and-replace");
                println!("  ✅ Fast prototyping with preview");
                println!();
                println!("For AST-based editing, use 'edit' or 'insert' commands.");
            }
            Some("diff") => {
                println!("📝 DIFF-TO-BATCH EXAMPLES");
                println!("===========================");
                println!();
                println!("1. Convert unified diff to batch:");
                println!("   git diff > changes.patch");
                println!("   gnawtreewriter diff-to-batch changes.patch");
                println!();
                println!("2. Preview before conversion:");
                println!("   gnawtreewriter diff-to-batch changes.patch --preview");
                println!("   # Shows diff statistics and batch preview");
                println!();
                println!("3. Specify output file:");
                println!("   gnawtreewriter diff-to-batch changes.patch --output ops.json");
                println!();
                println!("4. Apply the batch:");
                println!("   gnawtreewriter batch ops.json --preview");
                println!("   gnawtreewriter batch ops.json");
                println!();
                println!("**Workflow:**");
                println!("  1. Generate diff (git diff, AI agent output, etc.)");
                println!("  2. Convert to batch with preview");
                println!("  3. Review batch preview");
                println!("  4. Apply with validation and rollback");
                println!();
                println!("**Features:**");
                println!("  ✅ Multi-file diff support");
                println!("  ✅ In-memory validation");
                println!("  ✅ Atomic rollback on failure");
                println!("  ✅ Transaction logging");
            }
            Some("ai") => {
                println!("🤖 LOCAL AI & ANALYSIS EXAMPLES");
                println!("===============================");
                println!();
                println!("1. Semantic Quality Report:");
                println!("   gnawtreewriter semantic-report src/main.rs");
                println!("   # Uses ModernBERT to find structural anomalies.");
                println!("   # Requires: --features modernbert at compile time");
                println!();
                println!("2. Structural Search:");
                println!("   gnawtreewriter search main.rs \"database connection\"");
                println!("   # Finds all nodes containing the pattern.");
                println!();
                println!("3. Hierarchical Skeleton:");
                println!("   gnawtreewriter skeleton src/lib.rs --depth 3");
                println!("   # High-level overview of classes and functions.");
                println!();
                println!("**Key Benefits:**");
                println!("  ✅ 100% Local - No data leaves your machine");
                println!("  ✅ Privacy First - Works offline with ModernBERT (for reports)");
                println!("  ✅ AST-Aware - Understands code structure, not just text");
                println!();
                println!("**Note:** 'semantic-report' requires the modernbert feature.");
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
                println!(
                    "  gnawtreewriter examples --topic batch        # Multi-file batch operations"
                );
                println!("  gnawtreewriter examples --topic quick        # Quick text search-and-replace");
                println!(
                    "  gnawtreewriter examples --topic diff         # Convert diffs to batch ops"
                );
                println!("  gnawtreewriter examples --topic ai           # AI and analysis features");
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
            Some("batch") => {
                println!("📦 BATCH OPERATIONS WIZARD");
                println!("=========================");
                println!();
                println!("Batch operations allow you to apply multiple changes atomically:");
                println!();
                println!("A) Create batch JSON from diff:");
                println!("   git diff > changes.patch");
                println!("   gnawtreewriter diff-to-batch changes.patch");
                println!();
                println!("B) Apply batch operations:");
                println!("   gnawtreewriter batch ops.json --preview");
                println!("   gnawtreewriter batch ops.json");
                println!();
                println!("C) Batch with tags:");
                println!("   gnawtreewriter tag add file.py \"0.1\" helper");
                println!("   # Use '0.1' in batch operations");
                println!();
                println!("💡 Perfect for:");
                println!("  • Multi-file refactoring");
                println!("  • AI agent workflows");
                println!("  • Coordinated changes");
            }
            Some("quick") => {
                println!("⚡ QUICK COMMAND WIZARD");
                println!("=======================");
                println!();
                println!("Quick command for fast, safe edits:");
                println!();
                println!("A) Node-edit mode:");
                println!("   gnawtreewriter quick file.py --node \"0.1.0\" --content 'new code' --preview");
                println!("   # Uses AST-based editing");
                println!();
                println!("B) Find/replace mode:");
                println!("   gnawtreewriter quick file.py --find 'old' --replace 'new' --preview");
                println!("   # Global text replacement");
                println!();
                println!("C) Apply changes:");
                println!("   gnawtreewriter quick file.py --node \"0.1.0\" --content 'new code'");
                println!("   # Creates backup, logs transaction");
                println!();
                println!("💡 Perfect for:");
                println!("  • Single-line edits");
                println!("  • Simple replacements");
                println!("  • Quick prototyping");
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
            }
            Some("ai") => {
                println!("🤖 LOCAL AI & ANALYSIS WIZARD");
                println!("==============================");
                println!();
                println!("Step 1: Semantic Quality Report");
                println!("  gnawtreewriter semantic-report src/main.rs");
                println!("  # Uses ModernBERT to find structural anomalies");
                println!("  # Requires: --features modernbert at compile time");
                println!();
                println!("Step 2: Search nodes by pattern");
                println!("  gnawtreewriter search main.rs \"database connection\"");
                println!("  # Finds all nodes containing the pattern");
                println!();
                println!("Step 3: Get skeletal overview");
                println!("  gnawtreewriter skeleton src/lib.rs --depth 3");
                println!("  # High-level overview of classes and functions");
                println!();
                println!("Step 4: Combine with editing");
                println!("  gnawtreewriter analyze <file>");
                println!("  gnawtreewriter search <file> \"pattern\"");
                println!("  gnawtreewriter edit <file> <path> 'code'");
                println!();
                println!("💡 Note: All AI features run 100% locally for privacy and speed.");
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
                println!("  gnawtreewriter wizard --task batch            # Multi-file operations");
                println!("  gnawtreewriter wizard --task quick            # Fast edits (text replace)");
                println!("  gnawtreewriter wizard --task ai               # AI and analysis features");
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

    fn handle_analyze(paths: &[String], format: &str, recursive: bool) -> Result<()> {
        let mut all_files = Vec::new();

        for path in paths {
            let path_buf = std::path::PathBuf::from(path);
            if path_buf.is_dir() {
                if recursive {
                    // Recursively find supported files
                    all_files.extend(Self::find_supported_files(&path_buf)?);
                } else {
                    return Err(anyhow::anyhow!(
                        "Directory '{}' requires --recursive flag for safety.

To analyze this directory: gnawtreewriter analyze {} --recursive
To analyze specific files: gnawtreewriter analyze {}/*.ext",
                        path,
                        path,
                        path
                    ));
                }
            } else {
                all_files.push(path.clone());
            }
        }

        if all_files.is_empty() {
            println!("No supported files found to analyze.");
            return Ok(());
        }

        let mut results = Vec::new();
        for file_path in &all_files {
            match GnawTreeWriter::new(file_path) {
                Ok(writer) => {
                    let tree = writer.analyze();
                    results.push(serde_json::to_value(tree)?);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to analyze {}: {}", file_path, e);
                }
            }
        }

        match format {
            "summary" => {
                println!("Analyzed {} files", results.len());
                for (i, result) in results.iter().enumerate() {
                    if let Some(file_path) = all_files.get(i) {
                        println!("File: {}", file_path);
                        if let Some(children) = result.get("children") {
                            if let Some(array) = children.as_array() {
                                println!("  Nodes: {}", array.len());
                            }
                        }
                    }
                }
            }
            _ => {
                println!("{}", serde_json::to_string_pretty(&results)?);
            }
        }
        Ok(())
    }

    fn find_supported_files(dir: &std::path::Path) -> Result<Vec<String>> {
        let mut files = Vec::new();
        let supported_extensions = vec![
            "py", "rs", "ts", "tsx", "js", "jsx", "php", "html", "htm", "qml", "go", "toml",
            "json", "yaml", "yml", "css", "md", "markdown", "txt", "xml", "svg", "xsl", "xsd",
            "rss", "atom",
        ];

        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    files.extend(Self::find_supported_files(&path)?);
                } else if let Some(ext) = path.extension() {
                    if let Some(ext_str) = ext.to_str() {
                        if supported_extensions.contains(&ext_str) {
                            if let Some(path_str) = path.to_str() {
                                files.push(path_str.to_string());
                            }
                        }
                    }
                }
            }
        }
        Ok(files)
    }

    fn handle_quick_replace(
        file: &str,
        search: &str,
        replace: &str,
        unescape_newlines: bool,
        preview: bool,
    ) -> Result<()> {
        use std::path::Path;

        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);
        let path = Path::new(file);

        // Read original content
        let original = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", file, e))?;

        // Process replacement text if unescape_newlines is set
        let replacement_text = if unescape_newlines {
            replace.replace(
                "\
", "
",
            )
        } else {
            replace.to_string()
        };

        // Prepare modified content (simple global replace)
        let modified = original.replace(search, &replacement_text);

        // Validate with parser (if available for file type)
        if let Err(e) = crate::parser::get_parser(path).and_then(|parser| parser.parse(&modified)) {
            return Err(anyhow::anyhow!(
                "Validation failed: {}. Change NOT applied.",
                e
            ));
        }

        if preview {
            println!("--- QuickReplace preview for: {}", file);
            print_diff(&original, &modified);
            println!(
                "
Use --no-preview to actually apply the change."
            );
            return Ok(());
        }

        // Apply: create backup, log transaction, write file
        let writer = GnawTreeWriter::new(file)?;
        writer.create_backup()?;

        let before_hash = crate::core::calculate_content_hash(&original);
        let after_hash = crate::core::calculate_content_hash(&modified);

        let mut tlog = TransactionLog::load(&project_root)?;
        let txid = tlog.log_transaction(
            OperationType::Edit,
            PathBuf::from(file),
            None,
            Some(before_hash),
            Some(after_hash),
            format!("Quick replace '{}' -> '{}'", search, replace),
            std::collections::HashMap::new(),
        )?;

        std::fs::write(path, modified)
            .map_err(|e| anyhow::anyhow!("Failed to write {}: {}", file, e))?;

        println!("✓ QuickReplace applied (txn {})", txid);

        Ok(())
    }

    fn handle_rename(
        symbol_name: &str,
        new_name: &str,
        path: &str,
        recursive: bool,
        preview: bool,
    ) -> Result<()> {
        use crate::core::{format_refactor_results, RefactorEngine};

        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);

        println!("🔄 Refactoring: rename '{}' -> '{}'", symbol_name, new_name);

        // Create refactor engine
        let engine = RefactorEngine::new(project_root.clone());

        if preview {
            println!(
                "--- Preview mode (dry run) ---
"
            );
            let results = engine.preview_rename(symbol_name, new_name, path, recursive)?;
            println!("{}", format_refactor_results(&results, true));
            println!(
                "
Use --no-preview to actually apply the rename."
            );
        } else {
            // Validate new name doesn't clash with reserved keywords
            // Check if path is a directory or file to determine language
            let path_buf = std::path::PathBuf::from(path);
            if path_buf.is_dir() {
                println!("⚠️  Recursive renaming in directory - will check multiple languages");
            } else if let Some(ext) = path_buf.extension() {
                let lang = match ext.to_str() {
                    Some("py") => "python",
                    Some("rs") => "rust",
                    Some("java") => "java",
                    Some("kt") | Some("kts") => "kotlin",
                    Some("cpp") | Some("hpp") => "cpp",
                    Some("c") | Some("h") => "c",
                    Some("go") => "go",
                    Some("js") | Some("jsx") => "javascript",
                    Some("ts") | Some("tsx") => "typescript",
                    Some("php") => "php",
                    Some("sh") | Some("bash") => "bash",
                    _ => "generic",
                };

                if !engine.validate_symbol_name(new_name, lang)? {
                    return Err(anyhow::anyhow!(
                        "Invalid symbol name: '{}' is a reserved keyword in {}",
                        new_name,
                        lang
                    ));
                }
            }

            // Perform the rename
            let results = engine.rename_symbol(symbol_name, new_name, path, recursive)?;
            println!("{}", format_refactor_results(&results, false));

            // Log transaction summary
            let total_renamed: usize = results.iter().map(|r| r.occurrences_renamed).sum();
            let mut tlog = TransactionLog::load(&project_root)?;

            for result in &results {
                if result.occurrences_renamed > 0 {
                    let _ = tlog.log_transaction(
                        OperationType::Edit,
                        result.file_path.clone(),
                        None,
                        None,
                        None,
                        format!(
                            "Rename '{}' -> '{}' ({} occurrences)",
                            symbol_name, new_name, result.occurrences_renamed
                        ),
                        std::collections::HashMap::new(),
                    );
                }
            }

            println!(
                "✓ Refactor complete: {} occurrences renamed across {} file(s)",
                total_renamed,
                results.len()
            );
        }

        Ok(())
    }

    fn handle_clone(
        source_file: &str,
        source_path: &str,
        target_file: Option<&str>,
        target_path: Option<&str>,
        preview: bool,
    ) -> Result<()> {
        use crate::parser::get_parser;

        // Determine target file (default to source file if not specified)
        let target_file_path = target_file.unwrap_or(source_file);

        // Read source file and parse
        let parser = get_parser(std::path::Path::new(source_file))?;
        let source_code = std::fs::read_to_string(source_file)
            .with_context(|| format!("Failed to read source file: {}", source_file))?;
        let source_tree = parser
            .parse(&source_code)
            .with_context(|| format!("Failed to parse source file: {}", source_file))?;

        // Find the source node to clone
        let source_node = Self::find_node_by_path(&source_tree, source_path)
            .ok_or_else(|| anyhow::anyhow!("Source node not found at path: {}", source_path))?;

        println!("🔄 Cloning node from {} [{}]", source_file, source_path);
        println!("  Node type: {}", source_node.node_type);
        println!(
            "  Lines: {}-{}",
            source_node.start_line, source_node.end_line
        );
        println!("  Content length: {} characters", source_node.content.len());

        // If no target path specified, we're doing a simple clone within same file
        if target_path.is_none() {
            return Err(anyhow::anyhow!(
                "Target path must be specified. Use: gnawtreewriter clone {} {} <target_file> <target_path>",
                source_file,
                source_path
            ));
        }

        let target_node_path = target_path.unwrap();

        // Clone operation: Insert the cloned content at target location
        let mut writer = GnawTreeWriter::new(target_file_path)?;
        let op = EditOperation::Insert {
            parent_path: target_node_path.to_string(),
            position: 1, // Insert at bottom of parent
            content: source_node.content.clone(),
        };

        if preview {
            let modified = writer.preview_edit(op)?;
            print_diff(writer.get_source(), &modified);
            println!(
                "
✓ Preview complete"
            );
            println!(
                "  Would clone to: {} [{}]",
                target_file_path, target_node_path
            );
            println!(
                "
Use without --preview to apply the clone"
            );
        } else {
            writer.edit(op)?;
            println!(
                "✓ Successfully cloned node to {} [{}]",
                target_file_path, target_node_path
            );
        }

        Ok(())
    }

    fn find_node_by_path<'a>(tree: &'a TreeNode, path: &str) -> Option<&'a TreeNode> {
        if tree.path == path {
            return Some(tree);
        }
        for child in &tree.children {
            if let Some(node) = Self::find_node_by_path(child, path) {
                return Some(node);
            }
        }
        None
    }

    fn handle_lint(paths: &[String], format: &str, recursive: bool) -> Result<()> {
        // For now, lint is a wrapper around analyze with issue detection
        // In the future, this could include actual linting rules

        let mut all_files = Vec::new();

        for path in paths {
            let path_buf = std::path::PathBuf::from(path);
            if path_buf.is_dir() {
                if recursive {
                    all_files.extend(Self::find_supported_files(&path_buf)?);
                } else {
                    return Err(anyhow::anyhow!(
                        "Directory '{}' requires --recursive flag for safety.

To lint this directory: gnawtreewriter lint {} --recursive
To lint specific files: gnawtreewriter lint {}/*.ext",
                        path,
                        path,
                        path
                    ));
                }
            } else {
                all_files.push(path.clone());
            }
        }

        if all_files.is_empty() {
            println!("No supported files found to lint.");
            return Ok(());
        }

        let mut issues = Vec::new();
        let mut total_files = 0;

        for file_path in &all_files {
            total_files += 1;
            match GnawTreeWriter::new(file_path) {
                Ok(_writer) => {
                    // For now, successful parsing means no syntax issues
                    // Future: Add actual linting rules here
                }
                Err(e) => {
                    issues.push(format!("{}:1:1 error {}", file_path, e));
                }
            }
        }

        match format {
            "json" => {
                let result = serde_json::json!({
                    "files_checked": total_files,
                    "issues_found": issues.len(),
                    "issues": issues
                });
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            _ => {
                if issues.is_empty() {
                    println!("✅ No issues found in {} files", total_files);
                } else {
                    println!(
                        "⚠️  Found {} issues in {} files:",
                        issues.len(),
                        total_files
                    );
                    for issue in issues {
                        println!("{}", issue);
                    }
                }
            }
        }
        Ok(())
    }
}

fn print_diff(old: &str, new: &str) {
    let diff = TextDiff::from_lines(old, new);
    println!("\x1b[1m--- Preview of changes ---\x1b[0m");
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => print!("\x1b[31m-{}\x1b[0m", change),
            ChangeTag::Insert => print!("\x1b[32m+{}\x1b[0m", change),
            ChangeTag::Equal => print!(" {}", change),
        };
    }
    println!("\x1b[1m--- End of preview ---\x1b[0m");
}

fn show_hint() {
    // Skip hints if GNAW_NO_HINTS is set
    if std::env::var("GNAW_NO_HINTS").is_ok() {
        return;
    }

    let hints = [
        "Use '-' as content to read from STDIN and avoid shell escaping issues.",
        "Mistake? Use 'gnawtreewriter undo' to revert your last change instantly.",
        "Use 'gnawtreewriter get_skeleton' for a fast overview of large files.",
        "You can target nodes by tag! Run 'gnawtreewriter tag --help' to learn more.",
        "Use 'gnawtreewriter analyze --format summary' for a high-level file overview.",
        "Searching for something? Try 'gnawtreewriter search_nodes' to find code patterns.",
        "Want to see code quality? Try 'gnawtreewriter get_semantic_report' (requires ModernBERT).",
    ];

    // Simple pseudo-random selection based on time
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let index = (nanos % hints.len() as u128) as usize;

    eprintln!("\x1b[2m[GnawTip]: {}\x1b[0m", hints[index]);
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
    let name_info = if let Some(name) = node.get_name() {
        format!(" '{}'", name)
    } else {
        String::new()
    };
    println!(
        "{}{} [{}] (line {}-{}){}",
        indent, node.path, node.node_type, node.start_line, node.end_line, name_info
    );
}

fn resolve_content(
    content: Option<String>,
    source_file: Option<String>,
    unescape_newlines: bool,
) -> Result<String> {
    let mut final_content = if let Some(path) = source_file {
        std::fs::read_to_string(path)?
    } else if let Some(c) = content {
        if c == "-" {
            use std::io::Read;
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer)?;
            buffer
        } else {
            c
        }
    } else {
        return Err(anyhow::anyhow!(
            "Either content or --source-file must be provided"
        ));
    };

    if unescape_newlines {
        final_content = final_content.replace(
            "\
", "
",
        );
    }

    Ok(final_content)
}

fn parse_user_timestamp(timestamp: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    use anyhow::Context;
    use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};

    // 1. Try RFC3339 (e.g., "2025-12-27T15:30:00Z" or "2025-12-27T16:30:00+01:00")
    if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp) {
        return Ok(dt.with_timezone(&Utc));
    }

    // 2. Try Naive formats (assume Local time)
    // We try common formats: "YYYY-MM-DD HH:MM:SS" and "YYYY-MM-DDTHH:MM:SS"
    let naive = NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%dT%H:%M:%S"))
        .context(
            "Invalid timestamp format.
Supported formats:
  - Local time: \"YYYY-MM-DD HH:MM:SS\"
  - RFC3339:    \"YYYY-MM-DDTHH:MM:SSZ\" (or with offset)",
        )?;

    // Convert Local Naive -> UTC
    // Local::from_local_datetime returns a LocalResult (None, Single, or Ambiguous)
    let local_dt = Local.from_local_datetime(&naive).single().ok_or_else(|| {
        anyhow::anyhow!("Ambiguous or invalid local time (e.g. during DST transition)")
    })?;

    Ok(local_dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::transaction_log::OperationType;
    use anyhow::Result;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::env;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_handle_restore_cli() -> Result<()> {
        let _guard = TEST_MUTEX.lock().unwrap();
        // Setup a temporary project root and current working directory
        let tmp = tempdir()?;
        let project_root = tmp.path();

        // Create .git directory to mark project root
        fs::create_dir(project_root.join(".git"))?;

        let orig_dir = env::current_dir()?;
        env::set_current_dir(project_root)?;

        // Create a file that will be restored
        let file_path = project_root.join("example.py");
        fs::write(&file_path, "original")?;

        // Create a backup that contains the 'modified' content (simulating a later backup)
        let backup_dir = project_root.join(".gnawtreewriter_backups");
        fs::create_dir_all(&backup_dir)?;
        let backup_file = backup_dir.join("backup_modified.json");
        let backup_json = serde_json::json!({
            "file_path": file_path.to_string_lossy(),
            "timestamp": Utc::now().to_rfc3339(),
            "tree": {},
            "source_code": "modified"
        });
        fs::write(&backup_file, serde_json::to_string_pretty(&backup_json)?)?;

        // Log a transaction that has after_hash matching the 'modified' backup
        let mut tlog = TransactionLog::load(project_root)?;
        let after_hash = crate::core::calculate_content_hash("modified");
        let before_hash = crate::core::calculate_content_hash("original");
        let txn_id = tlog.log_transaction(
            OperationType::Edit,
            file_path.clone(),
            None,
            Some(before_hash),
            Some(after_hash),
            "Edit for test".to_string(),
            HashMap::new(),
        )?;

        // Sanity: file is still the original pre-restore
        assert_eq!(fs::read_to_string(&file_path)?, "original");

        // Preview should not alter the file
        Cli::handle_restore(file_path.to_str().unwrap(), &txn_id, true)?;
        assert_eq!(fs::read_to_string(&file_path)?, "original");

        // Actual restore should replace file content with 'modified'
        Cli::handle_restore(file_path.to_str().unwrap(), &txn_id, false)?;
        assert_eq!(fs::read_to_string(&file_path)?, "modified");

        // Restore original cwd
        env::set_current_dir(orig_dir)?;

        Ok(())
    }

    #[test]
    fn test_quick_replace_preview() -> Result<()> {
        let _guard = TEST_MUTEX.lock().unwrap();
        let tmp = tempdir()?;
        let project_root = tmp.path();

        // Create .git directory to mark project root
        fs::create_dir(project_root.join(".git"))?;

        let orig_dir = std::env::current_dir()?;
        std::env::set_current_dir(project_root)?;

        let file_path = project_root.join("quick_preview.txt");
        fs::write(&file_path, "hello foo world")?;

        // Preview should not apply changes
        Cli::handle_quick_replace(file_path.to_str().unwrap(), "foo", "bar", false, true)?;
        assert_eq!(fs::read_to_string(&file_path)?, "hello foo world");

        std::env::set_current_dir(orig_dir)?;
        Ok(())
    }

    #[test]
    fn test_quick_replace_apply() -> Result<()> {
        let _guard = TEST_MUTEX.lock().unwrap();
        let tmp = tempdir()?;
        let project_root = tmp.path();

        // Create .git directory to mark project root
        fs::create_dir(project_root.join(".git"))?;

        let orig_dir = std::env::current_dir()?;
        std::env::set_current_dir(project_root)?;

        let file_path = project_root.join("quick_apply.txt");
        fs::write(&file_path, "hello foo world")?;

        // Apply should change the file, create a backup and log a transaction
        Cli::handle_quick_replace(file_path.to_str().unwrap(), "foo", "bar", false, false)?;

        // Verify file content changed
        assert_eq!(fs::read_to_string(&file_path)?, "hello bar world");

        // Verify backup was created and transaction was logged
        let backup_dir = project_root.join(".gnawtreewriter_backups");
        assert!(backup_dir.exists());

        let tlog = TransactionLog::load(project_root)?;
        let history = tlog.get_file_history(&file_path)?;
        assert!(!history.is_empty());

        // Verify a backup directory exists
        let backup_dir = project_root.join(".gnawtreewriter_backups");
        assert!(backup_dir.exists());

        // Verify there's at least one transaction for the file
        let tlog = TransactionLog::load(project_root)?;
        let history = tlog.get_file_history(&file_path)?;
        assert!(!history.is_empty());

        std::env::set_current_dir(orig_dir)?;
        Ok(())
    }
}
