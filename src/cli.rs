// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::core::{
    find_project_root, EditOperation, GnawTreeWriter, OperationType, RestorationEngine, TagManager,
    TransactionLog, UndoRedoManager, visualizer::TreeVisualizer,
};
#[cfg(feature = "modernbert")]
use crate::llm::{GnawSenseBroker, SenseResponse};
use crate::parser::TreeNode;
use anyhow::{Context, Result};
use colored::Colorize;
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
    Analyze {
        paths: Vec<String>,
        #[arg(short, long, default_value = "json")]
        format: String,
        #[arg(long)]
        recursive: bool,
    },
    /// List all tree nodes for a file
    List {
        file_path: String,
        #[arg(short, long)]
        filter_type: Option<String>,
        #[arg(short, long, default_value = "100")]
        limit: usize,
        #[arg(short, long, default_value = "0")]
        offset: usize,
    },
    /// Show the content of a specific node
    Show {
        file_path: String,
        node_path: String,
    },
    /// Replace the content of a specific node
    Edit {
        file_path: String,
        #[arg(required_unless_present = "tag")]
        node_path: Option<String>,
        #[arg(long)]
        tag: Option<String>,
        #[arg(required_unless_present = "source_file")]
        content: Option<String>,
        #[arg(long, conflicts_with = "content")]
        source_file: Option<String>,
        #[arg(short, long)]
        preview: bool,
        #[arg(long)]
        unescape_newlines: bool,
        #[arg(long)]
        force: bool,
        #[arg(long, short = 'n')]
        narrative: Option<String>,
    },
    /// Insert new content into a parent node
    Insert {
        file_path: String,
        #[arg(required_unless_present = "tag")]
        parent_path: Option<String>,
        #[arg(long)]
        tag: Option<String>,
        position: usize,
        #[arg(required_unless_present = "source_file")]
        content: Option<String>,
        #[arg(long, conflicts_with = "content")]
        source_file: Option<String>,
        #[arg(short, long)]
        preview: bool,
        #[arg(long)]
        unescape_newlines: bool,
        #[arg(long, short = 'n')]
        narrative: Option<String>,
    },
    /// Undo recent edit operations
    Undo {
        #[arg(short, long, default_value = "1")]
        steps: usize,
    },
    /// Redo previously undone operations
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
    /// Execute a batch of operations
    Batch {
        file: String,
        #[arg(short, long)]
        preview: bool,
    },
    /// Search nodes by text or name
    Search {
        file_path: String,
        pattern: String,
        #[arg(short, long)]
        filter_type: Option<String>,
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Get a high-level skeletal view
    Skeleton {
        file_path: String,
        #[arg(short, long, default_value = "2")]
        depth: usize,
    },
    /// Comprehensive health check of the system
    Status,
    /// Generate a semantic code quality report
    SemanticReport {
        file_path: String,
    },
    /// Convert a unified diff to a batch
    DiffToBatch {
        diff_file: String,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long)]
        preview: bool,
    },
    /// Restore file to a specific transaction
    Restore {
        file_path: String,
        transaction_id: String,
        preview: bool,
    },
    /// Simple search-and-replace
    QuickReplace {
        file: String,
        search: String,
        replace: String,
        #[arg(short, long)]
        preview: bool,
        #[arg(long)]
        unescape_newlines: bool,
    },
    /// AST-aware renaming
    Rename {
        symbol_name: String,
        new_name: String,
        path: String,
        #[arg(short, long)]
        recursive: bool,
        #[arg(short, long)]
        preview: bool,
    },
    /// Clone code structures
    Clone {
        source_file: String,
        source_path: String,
        #[arg(required_unless_present = "target_path")]
        target_file: Option<String>,
        #[arg(required_unless_present = "target_file")]
        target_path: Option<String>,
        #[arg(short, long)]
        preview: bool,
    },
    /// Debug hash calculation
    DebugHash { content: String },
    /// Start a new session
    SessionStart {
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Manage the MCP server
    Mcp {
        #[command(subcommand)]
        command: McpSubcommands,
    },
    /// Restore entire project to a point in time
    RestoreProject {
        timestamp: String,
        #[arg(short, long)]
        preview: bool,
    },
    /// Restore specific files by timestamp
    RestoreFiles {
        #[arg(short, long)]
        since: String,
        #[arg(short, long)]
        files: Vec<String>,
        #[arg(short, long)]
        preview: bool,
    },
    /// Undo all changes from a session
    RestoreSession {
        session_id: String,
        #[arg(short, long)]
        preview: bool,
    },
    /// Delete a node
    Delete {
        file_path: String,
        #[arg(required_unless_present = "tag")]
        node_path: Option<String>,
        #[arg(long)]
        tag: Option<String>,
        #[arg(short, long)]
        preview: bool,
    },
    /// Add a property to a QML component
    AddProperty {
        file_path: String,
        target_path: String,
        name: String,
        r#type: String,
        value: String,
        #[arg(short, long)]
        preview: bool,
    },
    /// Add a child component to a QML component
    AddComponent {
        file_path: String,
        target_path: String,
        name: String,
        #[arg(short, long)]
        content: Option<String>,
        #[arg(short, long)]
        preview: bool,
    },
    /// Manage named references (tags)
    Tag {
        #[command(subcommand)]
        command: TagSubcommands,
    },
    /// Show examples and common workflows
    Examples {
        #[arg(short, long)]
        topic: Option<String>,
    },
    /// Interactive help wizard
    Wizard {
        #[arg(short, long)]
        task: Option<String>,
    },
    /// Lint files
    Lint {
        paths: Vec<String>,
        #[arg(short, long, default_value = "text")]
        format: String,
        #[arg(long)]
        recursive: bool,
    },
    /// Search for code semantically
    Sense {
        query: String,
        file: Option<PathBuf>,
        #[arg(long)]
        deep: bool,
    },
    /// Semantically insert code
    SenseInsert {
        file: PathBuf,
        anchor: String,
        content: String,
        #[arg(long, default_value = "after")]
        intent: String,
        #[arg(long)]
        preview: bool,
    },
    /// Scaffold a new file
    Scaffold {
        file_path: PathBuf,
        #[arg(long)]
        schema: String,
    },
    /// AI tools
    Ai {
        #[command(subcommand)]
        command: AiSubcommands,
    },
    /// Agentic Logging Framework
    Alf {
        message: Option<String>,
        #[arg(long, default_value = "writer")]
        actor: String,
        #[arg(long)]
        txn: Option<String>,
        #[arg(long, default_value = "intent")]
        kind: String,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        list: bool,
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Show version information
    Version,
    /// Generate a project blueprint
    Blueprint {
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Semantic edit
    SemanticEdit {
        file_path: String,
        query: String,
        #[arg(required_unless_present = "source_file")]
        content: Option<String>,
        #[arg(long, conflicts_with = "content")]
        source_file: Option<String>,
        #[arg(long, short = 'n')]
        narrative: Option<String>,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum AiSubcommands {
    /// Setup AI models (downloads required files)
    Setup {
        #[arg(long)]
        /// Force re-download even if already present
        force: bool,
    },
    /// Show AI status and installed models
    Status,
    /// Index the entire project for semantic search
    Index {
        /// Directory to index (defaults to project root)
        path: Option<PathBuf>,
    },
    /// Generate an engineering report of recent structural changes
    Report {
        /// Number of recent entries to include
        #[arg(short, long, default_value = "10")]
        limit: usize,
        /// Path to save the report (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
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
                limit,
                offset,
            } => {
                let writer = GnawTreeWriter::new(&file_path)?;
                list_nodes(&file_path, writer.analyze(), filter_type.as_deref(), limit, offset);
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
                force,
                narrative,
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

                let content = resolve_content(content, source_file, unescape_newlines)?;
                let mut writer = GnawTreeWriter::new(&file_path)?;
                
                // Capture old node for visual diff
                let old_node = writer.analyze().clone().find_path(&target_path).cloned();

                let op = EditOperation::Edit {
                    node_path: target_path.clone(),
                    content,
                };
                if preview {
                    let modified = writer.preview_edit(op)?;
                    print_diff(writer.get_source(), &modified);
                } else {
                    writer.edit(op, force)?;
                    Self::show_visual_diff(&writer, &target_path, old_node.as_ref(), narrative.as_deref());
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
                narrative,
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
                    parent_path: insert_parent.clone(),
                    position,
                    content,
                };
                if preview {
                    let modified = writer.preview_edit(op)?;
                    print_diff(writer.get_source(), &modified);
                } else {
                    writer.edit(op, false)?;
                    Self::show_visual_pulse(&writer, &insert_parent, narrative.as_deref());
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
                    writer.edit(op, false)?;
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
                    writer.edit(op, false)?;
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
                    writer.edit(op, false)?;
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
            Commands::SessionStart { name } => {
                Self::handle_session_start(name)?;
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
            Commands::Status => {
                Self::handle_health_check().await?;
            }
            Commands::SemanticReport { file_path } => {
                Self::handle_semantic_report(&file_path).await?;
            }
            Commands::Sense { query, file, deep } => {
                Self::handle_sense(&query, file.as_ref().and_then(|p| p.to_str()), deep).await?;
            }
            Commands::SenseInsert { file, anchor, content, intent, preview } => {
                Self::handle_sense_insert(file, anchor, content, intent, preview).await?;
            }
            Commands::Scaffold { file_path, schema } => {
                Self::handle_scaffold(&file_path, &schema)?;
            }
            Commands::Ai { command } => match command {
                AiSubcommands::Setup { force } => {
                    Self::handle_ai_setup(force).await?;
                }
                AiSubcommands::Status => {
                    Self::handle_ai_status()?;
                }
                AiSubcommands::Index { path } => {
                    Self::handle_ai_index(path).await?;
                }
                AiSubcommands::Report { limit, output } => {
                    Self::handle_ai_report(limit, output).await?;
                }
            },
                
            Commands::Alf { message, actor, txn, kind, tag, id, list, limit } => {
            Self::handle_alf(message, actor, txn, kind, tag, id, list, limit)?;
                }
            
            Commands::Version => {
Self::handle_version()?;
            }
            Commands::Blueprint { output } => {
                Self::handle_blueprint(output.as_deref())?;
            }
            Commands::SemanticEdit {
                file_path,
                query,
                content,
                source_file,
                narrative,
                force,
            } => {
                Self::handle_semantic_edit(&file_path, &query, content, source_file, narrative, force).await?;
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
            if n.content.contains(p) && f.is_none_or(|filter| n.node_type == filter) {
                let name = n.get_name().unwrap_or_else(|| "unnamed".to_string());
                acc.push((n.path.clone(), n.node_type.clone(), name));
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
            for (path, node_type, name) in &matches {
                println!("  {} [{}] '{}'", path, node_type, name);
            }

            if let Some((path, _, _)) = matches.first() {
                println!("\n💡 [GnawTip]: To edit the first match surgically, use:");
                println!("   gnawtreewriter edit {} {} -", file_path, path);
            }
        }
        Ok(())
    }

    fn handle_skeleton(file_path: &str, max_depth: usize) -> Result<()> {
        let writer = GnawTreeWriter::new(file_path)?;
        let tree = writer.analyze();

        println!("Skeletal view of {} (max depth {}):", file_path, max_depth);

        fn build(n: &TreeNode, d: usize, md: usize, count: &mut usize) {
            if d > md || *count >= 500 {
                return;
            }
            *count += 1;
            let indent = "  ".repeat(d);
            let name = n.get_name().unwrap_or_default();
            println!("{}{} [{}] {}", indent, n.path, n.node_type, name);
            if *count == 500 {
                println!("... (limit of 500 nodes reached)");
                return;
            }
            for child in &n.children {
                build(child, d + 1, md, count);
            }
        }

        let mut count = 0;
        build(tree, 0, max_depth, &mut count);
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

    async fn handle_sense(query: &str, file_path: Option<&str>, deep: bool) -> Result<()> {
        #[cfg(feature = "modernbert")]
        {
            let current_dir = std::env::current_dir()?;
            let project_root = find_project_root(&current_dir);
            let broker = GnawSenseBroker::new(&project_root)?;

            println!("🧠 GnawSense is thinking about: \"{}\"...", query);
            let response = broker.sense(query, file_path).await?;

            match response {
                SenseResponse::Satelite { matches } => {
                    println!("\n🛰️ Satelite View: I found these relevant areas in the project:");
                    for (i, m) in matches.iter().enumerate() {
                        println!("  {}. {} (score: {:.2})", i + 1, m.file_path, m.score);
                    }
                    println!("\nTip: Use `gnawtreewriter sense \"{}\" --file {}` to zoom in.", query, matches[0].file_path);
                }
                SenseResponse::Zoom { file_path, nodes, impact } => {
                    println!("\n🔍 Zoom View: Relevant nodes in {}:", file_path);
                    for (i, n) in nodes.iter().enumerate() {
                        println!("  {}. [{}] (score: {:.2})", i + 1, n.path, n.score);
                        if deep || i == 0 {
                            println!("     \"{}\"", n.preview.replace("\n", " "));
                        }
                    }

                    if let Some(matches) = impact {
                        println!("\n⚠️  Impact Alert: This logic appears to be used in:");
                        for m in matches {
                            println!("  🔗 {} (node path: {})", m.file_path, m.node_path);
                        }
                    }
                }
            }
        }
        #[cfg(not(feature = "modernbert"))]
        {
            let _ = (query, file_path, deep);
            println!("Error: 'modernbert' feature not enabled in this build.");
            println!("Please recompile with: cargo build --release --features modernbert");
        }
        Ok(())
    }

    async fn handle_sense_insert(
        file: PathBuf,
        anchor: String,
        content: String,
        intent: String,
        preview: bool,
    ) -> Result<()> {
        #[cfg(feature = "modernbert")]
        {
            let current_dir = std::env::current_dir()?;
            let project_root = find_project_root(&current_dir);
            let broker = GnawSenseBroker::new(&project_root)?;
            let file_path = file.to_str().ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;

            println!("🧠 GnawSense is searching for anchor: \"{}\"...", anchor);
            let proposal = broker.propose_edit(&anchor, file_path, &intent).await?;
            
            println!("📍 Found anchor at {} (confidence: {:.2})", proposal.anchor_path, proposal.confidence);
            println!("🔧 Action: {} at {} position {}", proposal.suggested_op, proposal.parent_path, proposal.position);

            let mut writer = GnawTreeWriter::new(file_path)?;
            let op = EditOperation::Insert {
                parent_path: proposal.parent_path,
                position: proposal.position,
                content,
            };

            if preview {
                let modified = writer.preview_edit(op)?;
                println!("\n--- Preview of Semantic Insertion ---");
                print_diff(writer.get_source(), &modified);
            } else {
                writer.edit(op, false)?;
                println!("✓ Successfully inserted code semantically.");
            }
        }
        #[cfg(not(feature = "modernbert"))]
        {
            let _ = (file, anchor, content, intent, preview);
            println!("Error: 'modernbert' feature not enabled.");
        }
        Ok(())
    }

    fn handle_version() -> Result<()> {
        println!("GnawTreeWriter v{}", env!("CARGO_PKG_VERSION"));
        Ok(())
    }

    fn handle_blueprint(output_path: Option<&str>) -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);
        let engine = crate::core::blueprint::BlueprintEngine::new(&project_root);
        let blueprint = engine.generate()?;
        
        if let Some(path) = output_path {
            let content = if path.ends_with(".md") {
                engine.render_to_markdown(&blueprint)
            } else {
                // Fallback to simple text rendering if not .md
                // For now we'll just use markdown as it's the requested format
                engine.render_to_markdown(&blueprint)
            };
            std::fs::write(path, content)?;
            println!("✓ Blueprint saved to {}", path);
        } else {
            engine.render_to_terminal(&blueprint);
        }
        Ok(())
    }

    async fn handle_semantic_edit(
        file_path: &str,
        query: &str,
        content: Option<String>,
        source_file: Option<String>,
        narrative: Option<String>,
        force: bool,
    ) -> Result<()> {
        #[cfg(feature = "modernbert")]
        {
            let current_dir = std::env::current_dir()?;
            let project_root = find_project_root(&current_dir);
            let broker = GnawSenseBroker::new(&project_root)?;
            
            println!("🧠 GnawSense is searching for: \"{}\" in {}...", query, file_path);
            let response = broker.sense(query, Some(file_path)).await?;

            if let SenseResponse::Zoom { nodes, .. } = response {
                if let Some(best_node) = nodes.first() {
                    println!("📍 Found best match at node path: {} (score: {:.2})", best_node.path, best_node.score);
                    
                    let content = resolve_content(content, source_file, false)?;
                    let mut writer = GnawTreeWriter::new(file_path)?;
                    
                    // Capture old state for visual diff
                    let old_node = writer.analyze().find_path(&best_node.path).cloned();

                    let op = EditOperation::Edit {
                        node_path: best_node.path.clone(),
                        content,
                    };
                    
                    writer.edit(op, force)?;
                    Self::show_visual_diff(&writer, &best_node.path, old_node.as_ref(), narrative.as_deref());
                    println!("✓ Successfully edited node: {}", best_node.path);
                    return Ok(());
                }
            }
            anyhow::bail!("Could not find a semantic match for '{}' in {}", query, file_path);
        }
        #[cfg(not(feature = "modernbert"))]
        {
            let _ = (file_path, query, content, source_file, narrative, force);
            anyhow::bail!("ModernBERT feature not enabled. Semantic features require --features modernbert.");
        }
    }

    fn handle_scaffold(file_path: &PathBuf, schema: &str) -> Result<()> {
        use crate::core::ScaffoldEngine;
        use std::fs;

        if file_path.exists() {
            anyhow::bail!("File already exists: {}. Scaffolding only works for new files.", file_path.display());
        }

        let engine = ScaffoldEngine::new();
        let code = engine.generate(schema)?;

        // Create parent directories if they don't exist
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(file_path, code)?;
        println!("✓ Successfully scaffolded new file: {}", file_path.display());
        println!("You can now use `sense-insert` to fill in the implementation.");

        Ok(())
    }

    fn handle_alf(
        message: Option<String>,
        actor: String,
        txn: Option<String>,
        kind: String,
        tag: Option<String>,
        id: Option<String>,
        list: bool,
        limit: usize,
    ) -> Result<()> {
        use crate::core::alf::{AlfManager, AlfType};
        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);
        let mut alf = AlfManager::load(&project_root)?;
        alf.set_actor(&actor);

        if list {
            let entries = alf.list(limit);
            println!("\n📓 Agentic Logging Framework (ALF) - Recent Entries:");
            for e in entries {
                let kind_str = match e.entry_type {
                    AlfType::Auto => "🤖 AUTO",
                    AlfType::Intent => "🎯 INTENT",
                    AlfType::Assumption => "🤔 ASSUMPTION",
                    AlfType::Risk => "⚠️ RISK",
                    AlfType::Outcome => "✅ OUTCOME",
                    AlfType::Meta => "ℹ️ META",
                };
                let txn_str = e.transaction_id.as_ref().map(|t| format!(" [txn:{}]", &t[..8])).unwrap_or_default();
                let actor_str = format!(" @{}", e.actor);
                println!("- [{}] {}{}{}: {}", e.timestamp.format("%H:%M:%S"), kind_str, actor_str, txn_str, e.message);
                if !e.tags.is_empty() {
                    println!("  tags: {}", e.tags.join(", "));
                }
                println!("  ID: {}", e.id);
            }
            return Ok(());
        }

        if let Some(tag_name) = tag {
            let target_id = id.ok_or_else(|| anyhow::anyhow!("--id is required when tagging"))?;
            alf.add_tag(&target_id, &tag_name)?;
            println!("✓ Tag '{}' added to entry {}", tag_name, target_id);
            return Ok(());
        }

        if let Some(msg) = message {
            let alf_type = match kind.to_lowercase().as_str() {
                "intent" => AlfType::Intent,
                "assumption" => AlfType::Assumption,
                "risk" => AlfType::Risk,
                "outcome" => AlfType::Outcome,
                "meta" => AlfType::Meta,
                _ => AlfType::Intent,
            };

            let entry_id = alf.log(alf_type, &msg, txn)?;
            println!("✓ Logged to ALF: {}", entry_id);
        }

        Ok(())
    }

    async fn handle_ai_setup(force: bool) -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);
        let mgr = crate::llm::ai_manager::AiManager::new(&project_root)?;
        
        println!("🚀 Setting up AI models in {}...", project_root.display());
        if let Err(e) = mgr.setup(crate::llm::ai_manager::AiModel::ModernBert, crate::llm::ai_manager::DeviceType::Cpu, force).await {
            println!("\n⚠️  {}", "Automatic setup failed.".bold().red());
            println!("Error: {}", e);
            println!("\n💡 [The Helpful Guard]: You can download the model manually using these commands:");
            println!("   mkdir -p .gnawtreewriter_ai/models/modernbert");
            println!("   curl -L https://huggingface.co/answerdotai/ModernBERT-base/resolve/main/config.json -o .gnawtreewriter_ai/models/modernbert/config.json");
            println!("   curl -L https://huggingface.co/answerdotai/ModernBERT-base/resolve/main/tokenizer.json -o .gnawtreewriter_ai/models/modernbert/tokenizer.json");
            println!("   curl -L https://huggingface.co/answerdotai/ModernBERT-base/resolve/main/model.safetensors -o .gnawtreewriter_ai/models/modernbert/model.safetensors");
            return Err(e);
        }
        println!("✨ AI models setup successfully.");
        Ok(())
    }

    fn handle_ai_status() -> Result<()> {
        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);
        let mgr = crate::llm::ai_manager::AiManager::new(&project_root)?;
        let status = mgr.get_status()?;
        
        println!("\n🧠 GnawTreeWriter AI Status");
        println!("===========================");
        println!("ModernBERT: {}", if status.modern_bert_installed { "✅ Installed".green() } else { "❌ Not found (run 'ai setup')".red() });
        println!("Cache Dir:  {}", status.cache_dir.display());
        println!("Device:     CPU");
        println!();
        Ok(())
    }

    async fn handle_ai_index(path: Option<PathBuf>) -> Result<()> {
        #[cfg(feature = "modernbert")]
        {
            use crate::llm::ProjectIndexer;
            let current_dir = std::env::current_dir()?;
            let project_root = find_project_root(&current_dir);
            let target_path = path.unwrap_or(project_root.clone());

            println!("🚀 Starting project-wide semantic indexing...");
            println!("📂 Target: {}", target_path.display());
            
            let indexer = ProjectIndexer::new(&project_root)?;
            let total = indexer.index_all(&target_path).await?;
            
            println!("✨ Successfully indexed {} files.", total);
            println!("You can now use `gnawtreewriter sense \"<query>\"` without a file context to search the entire project.");
        }
        #[cfg(not(feature = "modernbert"))]
        {
            let _ = path;
            println!("Error: 'modernbert' feature not enabled in this build.");
        }
        Ok(())
    }

    async fn handle_ai_report(limit: usize, output: Option<PathBuf>) -> Result<()> {
        use crate::core::report::ReportEngine;
        let current_dir = std::env::current_dir()?;
        let project_root = find_project_root(&current_dir);
        
        let engine = ReportEngine::new();
        let markdown = engine.generate_markdown_report(&project_root, limit)?;

        if let Some(path) = output {
            std::fs::write(&path, &markdown)?;
            println!("✓ Engineering report saved to: {}", path.display());
        } else {
            println!("{}", markdown);
        }

        Ok(())
    }

        fn handle_session_start(name: Option<String>) -> Result<()> {

            let current_dir = std::env::current_dir()?;

            let project_root = find_project_root(&current_dir);

            let mut transaction_log = TransactionLog::load(project_root)?;

            transaction_log.start_new_session(name)?;

            println!(

                "✓ New session started: {}",

                transaction_log.get_current_session_id()

            );

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
        let transaction_log = TransactionLog::load(&project_root)?;

        // ALIAS LOOKUP: Check if the session_id is actually a human-readable alias
        let alias_file = project_root.join(".gnawtreewriter_aliases.json");
        let actual_id = if alias_file.exists() {
            let data = std::fs::read_to_string(alias_file)?;
            let aliases: std::collections::HashMap<String, String> = serde_json::from_str(&data).unwrap_or_default();
            aliases.get(session_id).cloned().unwrap_or_else(|| session_id.to_string())
        } else {
            session_id.to_string()
        };

        if actual_id != session_id {
            println!("🔍 Alias found: '{}' -> {}", session_id, actual_id);
        }

        if preview {
             println!("Would restore session {}...", actual_id);
             let files = transaction_log.get_session_files(&actual_id)?;
             if files.is_empty() {
                 println!("No files affected in this session.");
             } else {
                 println!("Files to be restored:");
                 for f in files {
                     println!(" - {}", f.display());
                 }
             }
             println!("\nUse --no-preview to perform restoration.");
             return Ok(());
        }

        let restoration_engine = RestorationEngine::new(&project_root)?;
        let result = restoration_engine.restore_session(&actual_id)?;

        result.print_summary();
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
                println!("   gnawtreewriter edit app.py "0.1" 'new code' # Edit specific node");
                println!();
                println!("2. Surgical Inline Editing (v0.9.1+):");
                println!("   gnawtreewriter edit main.rs "1.2.3" 'new_var' # Change just one variable");
                println!("   # The editor now preserves surrounding code on the same line!");
                println!();
                println!("3. Safe editing with preview:");
                println!("   gnawtreewriter edit main.rs "0.2" 'fn main() {{}}' --preview");
                println!("   # Review the diff, then run without --preview");
                println!();
                println!("4. Insert new functions:");
                println!("   gnawtreewriter insert app.py "0" 1 'def helper(): return 42'");
                println!(
                    "   gnawtreewriter insert main.rs "0" 0 'use std::collections::HashMap;'"
                );
            }
            Some("precision") => {
                println!("🎯 SURGICAL PRECISION (v0.9.1)");
                println!("==============================");
                println!();
                println!("GnawTreeWriter v0.9.1 introduces inline editing support.");
                println!("Earlier versions would replace entire lines, but now you can");
                println!("target specific nodes within a line (like a single parameter).");
                println!();
                println!("1. Edit a single parameter:");
                println!("   gnawtreewriter edit src/lib.rs "1.2.3.5" 'new_param_name'");
                println!();
                println!("2. Pedagogical Validation:");
                println!("   If you make a syntax error, the editor now provides");
                println!("   language-specific tips to help you fix it.");
            }
            Some("search") => {
                println!("🔍 SEARCH EXAMPLES");
                println!("==================");
                println!();
                println!("1. Find nodes by name:");
                println!("   gnawtreewriter search main.rs "main"");
                println!("   # Finds all nodes containing 'main'");
                println!();
                println!("2. Find nodes by pattern:");
                println!("   gnawtreewriter search app.py "print"");
                println!("   # Finds all print statements");
                println!();
                println!("3. Find specific patterns:");
                println!("   gnawtreewriter search src/lib.rs "TreeNode"");
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
                println!("   gnawtreewriter add-property app.qml "0.1" width int 300");
                println!("   gnawtreewriter add-property app.qml "0.1" color string '"red"'");
                println!();
                println!("2. Add child components:");
                println!("   gnawtreewriter add-component app.qml "0" Rectangle");
                println!("   gnawtreewriter add-component app.qml "0.1" Button --content 'text: "Click"'");
                println!();
                println!("3. Complex QML editing:");
                println!("   gnawtreewriter list app.qml --filter-type ui_property");
                println!("   gnawtreewriter edit app.qml "0.2.1" 'anchors.fill: parent'");
            }
            Some("restoration") => {
                println!("⏰ TIME RESTORATION EXAMPLES");
                println!("===========================");
                println!();
                println!("1. Project-wide time travel:");
                println!("   gnawtreewriter restore-project "2025-12-27T15:30:00Z" --preview");
                println!("   gnawtreewriter restore-project "2025-12-27T15:30:00Z"");
                println!();
                println!("2. Selective file restoration:");
                println!("   gnawtreewriter restore-files --since "2025-12-27T16:00:00Z" --files "*.py"");
                println!("   gnawtreewriter restore-files -s "2025-12-27T16:00:00Z" -f "src/"");
                println!();
                println!("3. Undo AI agent sessions:");
                println!("   gnawtreewriter history                      # Find session ID");
                println!("   gnawtreewriter restore-session "session_123" --preview");
                println!("   gnawtreewriter restore-session "session_123"");
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
                println!("   gnawtreewriter tag add app.qml "1.1" mainRect");
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
                println!("   gnawtreewriter quick-replace main.rs "println!("Hello")" "println!("Hi")"");
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
                println!("🤖 GNAWSENSE: AI-POWERED NAVIGATION & ACTION");
                println!("============================================");
                println!();
                println!("1. Semantic Search (Project-wide):");
                println!("   gnawtreewriter sense "how is file backup handled?"");
                println!("   # Uses ModernBERT to find relevant files semantically.");
                println!();
                println!("2. Semantic Zoom (Within file):");
                println!("   gnawtreewriter sense "where is the database connection?" src/db.rs");
                println!("   # Finds specific functions or classes by meaning.");
                println!();
                println!("3. Agentic Journaling (ALF):");
                println!("   gnawtreewriter alf "Refactoring for scalability" --kind intent");
                println!("   gnawtreewriter alf --list                                   # See history");
                println!();
                println!("4. Engineering Reports:");
                println!("   gnawtreewriter ai report --limit 5                          # Show recent work");
                println!("   gnawtreewriter ai report --output docs/evolution.md         # Save to file");
                println!();
                println!("5. Semantic Insertion (The magic!):");
                println!("   gnawtreewriter sense-insert main.rs "the main function" "println!("Init...");" --preview");
                println!("   # Inserts code near a landmark without needing paths.");
                println!();
                println!("**Key Benefits:**");
                println!("  ✅ 100% Local - Powered by ModernBERT (requires modernbert feature)");
                println!("  ✅ Precision - Bridges the gap between intent and AST structure");
                println!("  ✅ Agent-Friendly - Allows AI agents to navigate autonomously");
            }
            Some("scaffolding") => {
                println!("🏗️  STRUCTURAL SCAFFOLDING EXAMPLES");
                println!("================================");
                println!();
                println!("1. Create a new Rust module:");
                println!("   gnawtreewriter scaffold src/auth.rs --schema "rust:mod(name:security, fn:validate)"");
                println!();
                println!("2. Create a Python class:");
                println!("   gnawtreewriter scaffold model.py --schema "python:class(name:User, fn:save)"");
                println!();
                println!("3. Combined workflow:");
                println!("   # Step 1: Scaffold the file structure");
                println!("   # Step 2: Use sense-insert to fill in the logic");
                println!();
                println!("**Why Scaffolding?**");
                println!("  ✅ Valid Syntax - Files are correct from the first byte");
                println!("  ✅ AST Landmarks - Creates anchors for GnawSense to find");
                println!("  ✅ Consistency - Enforces structural patterns");
            }
            Some("workflow") => {
                println!("🔄 COMMON WORKFLOWS");
                println!("==================");
                println!();
                println!("AI Agent Development Workflow:");
                println!("  1. gnawtreewriter session-start             # Start tracking");
                println!("  2. gnawtreewriter analyze src/*.py          # Understand structure");
                println!("  3. gnawtreewriter edit file.py "0.1" 'code'  # Make changes");
                println!("  4. gnawtreewriter history                    # Review what happened");
                println!("  5. gnawtreewriter restore-session "id"      # Undo if needed");
                println!();
                println!("Safe Refactoring Workflow:");
                println!("  1. gnawtreewriter status                     # Check current state");
                println!(
                    "  2. gnawtreewriter edit file.py "0.1" 'new' --preview  # Preview changes"
                );
                println!("  3. gnawtreewriter edit file.py "0.1" 'new'  # Apply if good");
                println!("  4. gnawtreewriter undo                       # Quick undo if needed");
            }
            Some("handbook") => {
                println!("📖 THE GNAWTREE ARCHITECT HANDBOOK");
                println!("================================");
                println!();
                println!("1. SETUP: Prepare your local AI");
                println!("   gnawtreewriter ai setup --model modernbert");
                println!("   gnawtreewriter ai index                     # Map the project");
                println!();
                println!("2. UNDERSTAND: Find your target");
                println!("   gnawtreewriter sense "how does X work?"     # Semantic search");
                println!("   gnawtreewriter skeleton <file>              # Structural overview");
                println!("   gnawtreewriter list <file>                  # Get exact node paths");
                println!();
                println!("3. MODIFY: Edit with surgical precision");
                println!("   gnawtreewriter edit <file> <path> 'code'    # Standard edit");
                println!("   gnawtreewriter edit <file> <path> @file.txt # Safe injection");
                println!("   gnawtreewriter sense-insert <file> "anchor" 'code'");
                println!();
                println!("4. SAFETY: The Guardian is watching");
                println!("   Always use --preview first to verify changes.");
                println!("   Use 'gnawtreewriter undo' if anything goes wrong.");
                println!("   Massive deletions will be BLOCKED by The Guardian.");
                println!("   v0.9.1+ includes inline precision and helpful syntax tips.");
                println!();
                println!("5. REPORT: Document your progress");
                println!("   gnawtreewriter alf "My intent" --kind intent");
                println!("   gnawtreewriter ai report --limit 5          # Generate evidence");
                println!();
                println!("Tip: Combine commands for speed, e.g., index then sense!");
            }
            _ => {
                println!("📚 GNAWTREEWRITER EXAMPLES");
                println!("=========================");
                println!();
                println!("Available example topics:");
                println!(
                    "  gnawtreewriter examples --topic editing      # Basic editing workflows"
                );
                println!(
                    "  gnawtreewriter examples --topic precision    # Surgical inline editing (v0.9.1)"
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
                println!("  gnawtreewriter examples --topic handbook     # Consolidated handbook");
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
                println!("Step 2: Edit a specific node with Surgical Precision (v0.9.1+)");
                println!(
                    "  Example: gnawtreewriter edit app.py "0.1" 'def hello(): print("world")'"
                );
                println!("  Paths can target large blocks OR small inline nodes like a single parameter.");
                println!("  GnawTreeWriter preserves the rest of the line automatically!");
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
                println!("A) Edit existing code (Surgical Precision):");
                println!("   1. gnawtreewriter analyze <file>        # Find the node path");
                println!("   2. gnawtreewriter edit <file> <path> 'new code' --preview");
                println!("   # Note: You can target tiny nodes within a line (inline nodes).");
                println!();
                println!("B) Add new code:");
                println!("   1. gnawtreewriter list <file>           # Find parent node");
                println!("   2. gnawtreewriter insert <file> <parent> 1 'new code'");
                println!("   Position: 0=top, 1=bottom, 2=after properties");
                println!();
                println!("C) Delete code:");
                println!("   1. gnawtreewriter list <file>           # Find node to delete");
                println!("   2. gnawtreewriter delete <file> <path> --preview");
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
                println!("   gnawtreewriter restore-project "2025-12-27T15:30:00Z" --preview");
                println!("   (Use ISO timestamp format)");
                println!();
                println!("C) Undo an AI agent session:");
                println!("   1. gnawtreewriter history                # Find session ID");
                println!("   2. gnawtreewriter restore-session <session-id> --preview");
                println!();
                println!("D) Restore specific files:");
                println!("   gnawtreewriter restore-files --since "2025-12-27T16:00:00Z" --files "*.py"");
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
                println!("   gnawtreewriter tag add file.py "0.1" helper");
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
                println!("   gnawtreewriter quick file.py --node "0.1.0" --content 'new code' --preview");
                println!("   # Uses AST-based editing");
                println!();
                println!("B) Find/replace mode:");
                println!("   gnawtreewriter quick file.py --find 'old' --replace 'new' --preview");
                println!("   # Global text replacement");
                println!();
                println!("C) Apply changes:");
                println!("   gnawtreewriter quick file.py --node "0.1.0" --content 'new code'");
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
                println!("❌ "Node not found at path":");
                println!("   • Run: gnawtreewriter list <file>");
                println!("   • Check that path exists in current file state");
                println!("   • File might have changed - analyze again");
                println!();
                println!("❌ "Validation failed":");
                println!("   • Your new code has syntax errors");
                println!("   • Read the Tip provided by the editor (v0.9.1+)");
                println!("   • Check for missing semicolons, brackets, or indentation");
                println!("   • Try smaller changes first");
                println!();
                println!("❌ "Backup not found":");
                println!("   • Some restoration operations need existing backups");
                println!("   • Check: ls .gnawtreewriter_backups/");
                println!("   • Use timestamp-based restoration as fallback");
                println!();
                println!("❌ Can't find the right node:");
                println!("   • Use 'gnawtreewriter search <file> "text"' to find by content");
                println!("   • Use 'gnawtreewriter skeleton <file>' for a high-level view");
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
                println!("  gnawtreewriter search main.rs "database connection"");
                println!("  # Finds all nodes containing the pattern");
                println!();
                println!("Step 3: Get skeletal overview");
                println!("  gnawtreewriter skeleton src/lib.rs --depth 3");
                println!("  # High-level overview of classes and functions");
                println!();
                println!("Step 4: Combine with editing");
                println!("  gnawtreewriter analyze <file>");
                println!("  gnawtreewriter search <file> "pattern"");
                println!("  gnawtreewriter edit <file> <path> 'code'");
                println!();
                println!("💡 Note: All AI features run 100% locally for privacy and speed.");
                println!("   • Use: gnawtreewriter list <file> --filter-type <type>");
                println!("   • Try: gnawtreewriter analyze <file> for overview");
                println!("   • Look for node types like 'function_item', 'class_definition'");
                println!();
                println!("Still stuck? Check: https://github.com/gnawSoftware/GnawTreeWriter/issues");
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

    fn show_visual_pulse(writer: &GnawTreeWriter, focus_path: &str, narrative: Option<&str>) {
        let viz = TreeVisualizer::new(5, true);
        
        eprintln!("\n┌──────────────────────────────────────────┐");
        eprintln!("│ 🛠️  {} {:<30} │", "Operation:".bold(), "Structural Update");
        eprintln!("│ 📍 {} {:<30} │", "Target:".bold(), focus_path);
        eprintln!("│ ✨ {} {:<30} │", "Status:".bold(), "Syntax Validated ✅");
        eprintln!("└──────────────────────────────────────────┘");

        if let Some(n) = narrative {
            eprintln!("\n🎙️  {}", "Narrative:".bold().cyan());
            eprintln!("   \"{}\"", n.italic());
        }

        eprintln!("\n{}", "Structure Context:".bold());
        eprintln!("{}", viz.generate_sparkline(writer.analyze()));
        eprintln!("{}", viz.render_with_diff(writer.analyze(), focus_path, None));
    }

    fn show_visual_diff(writer: &GnawTreeWriter, focus_path: &str, old_node: Option<&TreeNode>, narrative: Option<&str>) {
        let viz = TreeVisualizer::new(5, true);
        
        let total_lines = writer.get_source().lines().count();
        let new_node = writer.analyze().find_path(focus_path);
        
        let old_lines_count = old_node.map(|n| n.content.lines().count()).unwrap_or(0);
        let new_lines_count = new_node.map(|n| n.content.lines().count()).unwrap_or(0);
        
        let removed_preview = if let Some(node) = old_node {
            let first_line = node.content.lines().next().unwrap_or("").trim();
            if first_line.len() > 25 {
                format!("{}...", &first_line[..22])
            } else {
                first_line.to_string()
            }
        } else {
            "None (New Insertion)".to_string()
        };

        let efficiency = if total_lines > 0 {
            let saved = total_lines.saturating_sub(new_lines_count);
            (saved * 100 / total_lines).min(100)
        } else {
            100
        };

        let target_desc = if let Some(n) = new_node {
            format!("{} [{}]", focus_path, n.node_type)
        } else {
            focus_path.to_string()
        };

        eprintln!("\n┌──────────────────────────────────────────┐");
        eprintln!("│ 🛠️  {} {:<30} │", "Operation:".bold(), "Surgical Edit");
        eprintln!("│ 📍 {} {:<30} │", "Target:".bold(), target_desc);
        eprintln!("│ 🗑️  {} {:<30} │", "Removed:".bold(), format!("\"{}\"", removed_preview));
        eprintln!("│ 📝 {} -{} / +{} lines              │", "Changes:".bold(), old_lines_count, new_lines_count);
        if total_lines > 5 {
            eprintln!("│ 📊 {} {:<23} % │", "Efficiency:".bold(), efficiency);
        } else {
            eprintln!("│ ✨ {} {:<30} │", "Precision:".bold(), "Surgical");
        }
        eprintln!("└──────────────────────────────────────────┘");

        if let Some(n) = narrative {
            eprintln!("\n🎙️  {}", "Narrative:".bold().cyan());
            eprintln!("   \"{}\"", n.italic());
        }

        eprintln!("\n{}", "Structure Context:".bold());
        eprintln!("{}", viz.generate_sparkline(writer.analyze()));
        eprintln!("{}", viz.render_with_diff(writer.analyze(), focus_path, old_node));
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

        // VALIDATION: Try to parse the modified code in memory before saving
        let validation_path = Path::new(file);
        if let Err(e) = crate::parser::get_parser(validation_path).and_then(|parser| Ok(parser.parse(&modified)?)) {
            println!("Validation failed: The proposed edit would result in invalid syntax.\nError: {}\n\nChange was NOT applied.", e);
            return Ok(());
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
            writer.edit(op, false)?;
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
    
        async fn handle_health_check() -> Result<()> {
            use colored::*;
            use std::process::Command;
    
            println!("
    {}", "🛡️  GnawTreeWriter System Health Check".bold().bright_white());
            println!("{}", "=====================================".bright_black());
    
            // 1. Environment & Paths
            let current_dir = std::env::current_dir()?;
            let project_root = find_project_root(&current_dir);
            let is_git = project_root.join(".git").exists();
    
            println!("
    {}", "📍 Environment".bold());
            println!("  CWD:          {}", current_dir.display().to_string().cyan());
            println!("  Project Root: {}", project_root.display().to_string().cyan());
            println!("  Git Status:   {}", if is_git { "✅ Repository Found".green() } else { "⚠️  Not a Git Repo (Precision may suffer)".yellow() });
    
            // 3. Transactions & Undo State
            println!("\n    {}", "🔄 Transactions & Undo State".bold());
            let undo_manager = crate::core::undo_redo::UndoRedoManager::new(&project_root)?;
            let state = undo_manager.get_state();
            let transaction_log = crate::core::transaction_log::TransactionLog::load(project_root.clone())?;
            let recent = transaction_log.get_last_n_transactions(5)?;

            println!("  Undo Available: {}", if state.undo_available > 0 { format!("{} steps", state.undo_available).green() } else { "0".bright_black() });
            println!("  Redo Available: {}", if state.redo_available > 0 { format!("{} steps", state.redo_available).green() } else { "0".bright_black() });

            if let Some(last_undo) = &state.last_undo {
                println!("  Last Action:   {}", last_undo.cyan());
            }

            if !recent.is_empty() {
                println!("  Recent History:");
                for transaction in recent.iter().rev().take(3) {
                    let timestamp = transaction.timestamp.format("%H:%M:%S").to_string();
                    println!("    • {} [{:?}] {}", timestamp.bright_black(), transaction.operation, transaction.description);
                }
            } else {
                println!("  Recent History: {}", "No transactions recorded yet".bright_black());
            }
    
            // 2. AI Engine (GnawSense & HRM2)
            #[cfg(feature = "modernbert")]
            {
                println!("\n    {}", "🧠 GnawSense AI Ecosystem".bold().bright_magenta());
                let mgr = crate::llm::ai_manager::AiManager::new(&project_root)?;
                let status = mgr.get_status()?;
                
                println!("  Engine:       {}", "✅ ModernBERT (Semantic Core)".green());
                println!("  Reasoning:    {}", "✅ HRM2 (Hierarchical Relational Model)".green().bold());
                println!("  Cache:        {}", status.cache_dir.display().to_string().cyan());
                
                let model_dir = status.cache_dir.join("modernbert");
                let c = model_dir.join("config.json").exists();
                let t = model_dir.join("tokenizer.json").exists();
                let w = model_dir.join("model.safetensors").exists();
    
                print!("  Model Files:  ");
                if c && t && w {
                    println!("{}", "✅ All components found".green());
                } else {
                    let mut missing = Vec::new();
                    if !c { missing.push("config.json"); }
                    if !t { missing.push("tokenizer.json"); }
                    if !w { missing.push("model.safetensors"); }
                    println!("{} {}", "❌ Missing:".red(), missing.join(", ").red());
                    println!("                {}", "Run 'gnawtreewriter ai setup' to fix.".italic().bright_black());
                }
    
                // Test load attempt (fast check)
                match mgr.load_model(crate::llm::AiModel::ModernBert, crate::llm::DeviceType::Cpu) {
                    Ok(_) => println!("  Runtime:      {}", "✅ AI Services ready for GnawSense operations".green()),
                    Err(e) => println!("  Runtime:      {} {}", "❌ Load failed:".red(), e.to_string().red()),
                }
            }
            #[cfg(not(feature = "modernbert"))]
            {
                println!("\n    {}", "🧠 GnawSense AI Ecosystem".bold());
                println!("  Status:       {}", "❌ Disabled".red());
                println!("  Note:         {}", "Recompile with --features modernbert to enable semantic intelligence.".italic().bright_black());
            }
    
            // 3. MCP Link (Gemini CLI Integration)
            println!("
    {}", "🔗 MCP Link".bold());
            let home = std::env::var("HOME").unwrap_or_default();
            let mcp_config_path = std::path::PathBuf::from(&home).join(".gemini/antigravity/mcp_config.json");
            
            if mcp_config_path.exists() {
                match std::fs::read_to_string(&mcp_config_path) {
                    Ok(content) => {
                        if content.contains("gnawtreewriter") {
                            println!("  Config:       {}", "✅ Registered in Gemini CLI".green());
                        } else {
                            println!("  Config:       {}", "⚠️  Found config but gnawtreewriter is missing".yellow());
                        }
                    }
                    Err(_) => println!("  Config:       {}", "❌ Config exists but is unreadable".red()),
                }
            } else {
                println!("  Config:       {}", "❌ Not found (~/.gemini/antigravity/mcp_config.json)".red());
            }
    
            // 4. Backend Daemon (GnawGuard)
            println!("
    {}", "🛡️  Backend Daemon".bold());
            let guard_check = Command::new("pgrep").arg("-f").arg("gnaw-guard").output();
            match guard_check {
                Ok(output) if !output.stdout.is_empty() => {
                    println!("  GnawGuard:    {}", "✅ Running in background".green());
                }
                _ => {
                    println!("  GnawGuard:    {}", "⚪ Not detected (Optional)".bright_black());
                }
            }
    
            println!("
    {}", "✨ Summary".bold());
            println!("  System is ready for agentic surgical operations.");
            println!();
    
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

    fn list_nodes(file_path: &str, tree: &TreeNode, filter_type: Option<&str>, limit: usize, offset: usize) {
        let mut all_nodes_meta = Vec::new();

        fn collect(n: &TreeNode, filter: Option<&str>, acc: &mut Vec<(String, String, String)>) {
            if filter.is_none() || filter.unwrap() == n.node_type {
                acc.push((
                    n.path.clone(),
                    n.node_type.clone(),
                    n.get_name().unwrap_or_else(|| "unnamed".to_string()),
                ));
            }
            for child in &n.children {
                collect(child, filter, acc);
            }
        }

        collect(tree, filter_type, &mut all_nodes_meta);
        let total_count = all_nodes_meta.len();
        
        let target_nodes: Vec<_> = all_nodes_meta.into_iter().skip(offset).take(limit).collect();

        if target_nodes.is_empty() {
            println!("No nodes found matching criteria (Total: {}, Offset: {})", total_count, offset);
            return;
        }

        if offset > 0 || total_count > limit {
            println!("--- Showing {} nodes (offset {}, total {}) ---", target_nodes.len(), offset, total_count);
        }

        for (path, node_type, name) in &target_nodes {
            println!("  {} [{}] {}", path, node_type, name);
        }

        if let Some((path, _, name)) = target_nodes.first() {
            if path != "" { // Don't suggest editing the source_file root directly usually
                println!("\n💡 [GnawTip]: To edit a node (e.g. '{}'), use:", name);
                println!("   gnawtreewriter edit {} {} -", file_path, path);
            }
        }
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
        } else if let Some(file_path) = c.strip_prefix('@') {
            // SAFE INJECTION: Read content from a file path starting with @
            std::fs::read_to_string(file_path)
                .with_context(|| format!("Failed to read content from file: {}", file_path))?
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