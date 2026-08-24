//! Progressive exploration with zoom levels.
//!
//! Provides a map-like interface for navigating codebases:
//! - Level 0: Project overview (directories + token counts)
//! - Level 1: Directory contents (files + summaries)
//! - Level 2: File structure (signatures, compressed)
//! - Level 3: Full content

use crate::core::file_walker::walk_source_files_filtered;
use crate::core::token_count::estimate_code_tokens;
use crate::parser::TreeNode;
use crate::GnawTreeWriter;
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

/// Zoom level for exploration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum ZoomLevel {
    /// Project overview: directory tree + aggregated tokens.
    Overview = 0,
    /// Directory view: files with function/class summaries.
    Directory = 1,
    /// File view: signatures and compressed structure.
    File = 2,
    /// Full view: complete source code.
    Full = 3,
}

impl ZoomLevel {
    pub fn parse(s: &str) -> Self {
        match s {
            "0" | "overview" => ZoomLevel::Overview,
            "1" | "directory" | "dir" => ZoomLevel::Directory,
            "2" | "file" | "structure" => ZoomLevel::File,
            "3" | "full" | "source" => ZoomLevel::Full,
            _ => ZoomLevel::Overview,
        }
    }
}

/// A node in the exploration tree.
#[derive(Debug, Clone, Serialize)]
pub struct ExploreNode {
    /// Display name (file name or directory name).
    pub name: String,
    /// Full path relative to root.
    pub path: String,
    /// Node type: "directory", "file", "function", "struct", "enum", "impl"
    pub node_type: String,
    /// Token count (for files/dirs).
    pub tokens: usize,
    /// Line count (for files).
    pub lines: usize,
    /// Children nodes (for directories and files at overview/directory level).
    pub children: Vec<ExploreNode>,
    /// Drill-down hint: what command to run to zoom into this node.
    pub drill_down: String,
    /// Content (only at level 3).
    pub content: Option<String>,
}

/// Result of exploration.
#[derive(Debug, Clone, Serialize)]
pub struct ExploreResult {
    /// Current path being explored.
    pub path: String,
    /// Current zoom level.
    pub level: ZoomLevel,
    /// The exploration node.
    pub node: ExploreNode,
    /// Available zoom levels for this path.
    pub available_levels: Vec<ZoomLevel>,
}

/// Explore a path at a specific zoom level.
pub fn explore(root: &Path, target: &str, level: ZoomLevel) -> Result<ExploreResult> {
    let target_path = if target.is_empty() {
        root.to_path_buf()
    } else {
        root.join(target)
    };

    let available_levels = if target_path.is_dir() {
        vec![ZoomLevel::Overview, ZoomLevel::Directory]
    } else {
        vec![ZoomLevel::File, ZoomLevel::Full]
    };

    let actual_level = if available_levels.contains(&level) {
        level
    } else {
        // Fall back to the most appropriate level for this path type
        if target_path.is_dir() {
            ZoomLevel::Directory
        } else {
            ZoomLevel::File
        }
    };

    let node = match actual_level {
        ZoomLevel::Overview => explore_overview(root)?,
        ZoomLevel::Directory => explore_directory(&target_path)?,
        ZoomLevel::File => explore_file(&target_path)?,
        ZoomLevel::Full => explore_full(&target_path)?,
    };

    Ok(ExploreResult {
        path: target_path
            .strip_prefix(root)
            .unwrap_or(&target_path)
            .to_string_lossy()
            .to_string(),
        level: actual_level,
        node,
        available_levels,
    })
}

// ── Level 0: Project overview ────────────────────────────────

fn explore_overview(root: &Path) -> Result<ExploreNode> {
    let files = walk_source_files_filtered(
        root,
        &["rs", "py", "js", "ts", "tsx", "jsx", "go", "java",
          "c", "cpp", "h", "hpp", "cs", "php", "rb", "swift", "kt"],
    );

    // Build directory tree with aggregated stats
    let mut dir_map: std::collections::HashMap<String, DirStats> = std::collections::HashMap::new();

    for path in &files {
        let rel = path.strip_prefix(root).unwrap_or(path);
        let components: Vec<&str> = rel.components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();

        if components.is_empty() {
            continue;
        }

        // Accumulate stats for each directory level
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let tokens = estimate_code_tokens(&content);
        let lines = content.lines().count();

        // File entry
        let file_name = components.last().unwrap().to_string();
        let dir_path = if components.len() > 1 {
            components[..components.len()-1].join("/")
        } else {
            ".".to_string()
        };

        let entry = DirStats {
            tokens,
            lines,
            file_count: 1,
            files: vec![FileEntry {
                name: file_name,
                path: rel.to_string_lossy().to_string(),
                tokens,
                lines,
            }],
        };

        dir_map.entry(dir_path)
            .and_modify(|e| {
                e.tokens += tokens;
                e.lines += lines;
                e.file_count += 1;
                e.files.push(FileEntry {
                    name: entry.files[0].name.clone(),
                    path: entry.files[0].path.clone(),
                    tokens,
                    lines,
                });
            })
            .or_insert(entry);
    }

    // Build children nodes
    let mut children = Vec::new();
    let mut sorted_dirs: Vec<_> = dir_map.into_iter().collect();
    sorted_dirs.sort_by_key(|(_, stats)| std::cmp::Reverse(stats.tokens));

    for (dir, stats) in sorted_dirs {
        let name = if dir == "." { "root".to_string() } else { dir };
        children.push(ExploreNode {
            name: name.clone(),
            path: name.clone(),
            node_type: "directory".to_string(),
            tokens: stats.tokens,
            lines: stats.lines,
            children: stats.files.into_iter().map(|f| ExploreNode {
                name: f.name,
                path: f.path,
                node_type: "file".to_string(),
                tokens: f.tokens,
                lines: f.lines,
                children: vec![],
                drill_down: String::new(),
                content: None,
            }).collect(),
            drill_down: format!("explore --path \"{}\" --level 1", name),
            content: None,
        });
    }

    let total_tokens: usize = children.iter().map(|c| c.tokens).sum();
    let total_lines: usize = children.iter().map(|c| c.lines).sum();

    Ok(ExploreNode {
        name: "project".to_string(),
        path: ".".to_string(),
        node_type: "project".to_string(),
        tokens: total_tokens,
        lines: total_lines,
        children,
        drill_down: String::new(),
        content: None,
    })
}

// ── Level 1: Directory view ───────────────────────────────────

fn explore_directory(target: &Path) -> Result<ExploreNode> {
    let files = walk_source_files_filtered(
        target,
        &["rs", "py", "js", "ts", "tsx", "jsx", "go", "java",
          "c", "cpp", "h", "hpp", "cs", "php", "rb", "swift", "kt"],
    );

    let mut children = Vec::new();
    let mut total_tokens = 0usize;
    let mut total_lines = 0usize;

    for path in &files {
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let content = std::fs::read_to_string(path).unwrap_or_default();
        let tokens = estimate_code_tokens(&content);
        let lines = content.lines().count();
        total_tokens += tokens;
        total_lines += lines;

        // Get skeletal summary for this file
        let summary = get_file_summary(path);

        children.push(ExploreNode {
            name: name.clone(),
            path: name.clone(),
            node_type: "file".to_string(),
            tokens,
            lines,
            children: summary,
            drill_down: format!("explore --path \"{}\" --level 2", path.display()),
            content: None,
        });
    }

    // Sort by tokens descending
    children.sort_by_key(|c| std::cmp::Reverse(c.tokens));

    Ok(ExploreNode {
        name: target.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("directory")
            .to_string(),
        path: target.to_string_lossy().to_string(),
        node_type: "directory".to_string(),
        tokens: total_tokens,
        lines: total_lines,
        children,
        drill_down: String::new(),
        content: None,
    })
}

// ── Level 2: File structure (signatures) ─────────────────────

fn explore_file(target: &Path) -> Result<ExploreNode> {
    let writer = GnawTreeWriter::new(target.to_str().unwrap_or(""))?;
    let tree = writer.analyze();
    let source = writer.get_source();
    let tokens = estimate_code_tokens(source);
    let lines = source.lines().count();

    let mut children = Vec::new();
    extract_signatures(tree, &mut children, 0);

    let name = target.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    Ok(ExploreNode {
        name,
        path: target.to_string_lossy().to_string(),
        node_type: "file".to_string(),
        tokens,
        lines,
        children,
        drill_down: String::new(),
        content: None,
    })
}

fn extract_signatures(node: &TreeNode, children: &mut Vec<ExploreNode>, depth: usize) {
    if depth > 2 {
        return;
    }

    if matches!(
        node.node_type.as_str(),
        "function_item" | "function_definition" | "function_declaration"
            | "struct_item" | "struct_declaration"
            | "enum_item" | "enum_declaration"
            | "impl_item"
            | "trait_item" | "trait_declaration"
            | "class_declaration" | "class_definition"
            | "method_definition"
    ) {
        let name = node.get_name().unwrap_or_default();
        let node_type = match node.node_type.as_str() {
            s if s.contains("function") => "function",
            s if s.contains("struct") => "struct",
            s if s.contains("enum") => "enum",
            "impl_item" => "impl",
            s if s.contains("trait") || s.contains("interface") => "trait",
            s if s.contains("class") => "class",
            s if s.contains("method") => "method",
            _ => "other",
        }.to_string();

        let signature = node.content.lines().next().unwrap_or("").trim().to_string();
        let tokens = estimate_code_tokens(&node.content);
        let lines_count = node.content.lines().count();

        children.push(ExploreNode {
            name,
            path: node.path.clone(),
            node_type,
            tokens,
            lines: lines_count,
            children: vec![],
            drill_down: format!("read_node --path \"{}\"", node.path),
            content: Some(signature),
        });
    }

    for child in &node.children {
        extract_signatures(child, children, depth + 1);
    }
}

// ── Level 3: Full content ────────────────────────────────────

fn explore_full(target: &Path) -> Result<ExploreNode> {
    let writer = GnawTreeWriter::new(target.to_str().unwrap_or(""))?;
    let source = writer.get_source();
    let tokens = estimate_code_tokens(source);
    let lines = source.lines().count();

    let name = target.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    Ok(ExploreNode {
        name,
        path: target.to_string_lossy().to_string(),
        node_type: "file".to_string(),
        tokens,
        lines,
        children: vec![],
        drill_down: String::new(),
        content: Some(source.to_string()),
    })
}

// ── Helper types ─────────────────────────────────────────────

struct DirStats {
    tokens: usize,
    lines: usize,
    file_count: usize,
    files: Vec<FileEntry>,
}

struct FileEntry {
    name: String,
    path: String,
    tokens: usize,
    lines: usize,
}

/// Get a skeletal summary of a file (function/class names).
fn get_file_summary(path: &Path) -> Vec<ExploreNode> {
    if let Ok(writer) = GnawTreeWriter::new(path.to_str().unwrap_or("")) {
        let tree = writer.analyze();
        let mut children = Vec::new();
        extract_signatures(tree, &mut children, 0);
        children
    } else {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn setup_project() -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();

        std::process::Command::new("git")
            .args(["init", root.to_str().unwrap()])
            .output()
            .ok();

        fs::create_dir_all(root.join("src/auth")).unwrap();
        fs::create_dir_all(root.join("src/db")).unwrap();

        fs::write(root.join("src/auth/login.rs"),
            "pub fn validate_password(p: &str) -> bool { !p.is_empty() }\npub fn login(u: &str) -> Session { Session::new(u) }").unwrap();
        fs::write(root.join("src/auth/session.rs"),
            "pub struct Session { pub user: String }\nimpl Session { pub fn new(u: &str) -> Self { Session { user: u.to_string() } } }").unwrap();
        fs::write(root.join("src/db/users.rs"),
            "pub fn get_user(id: i32) -> User { todo!() }").unwrap();
        fs::write(root.join("src/main.rs"),
            "fn main() { login::validate_password(\"test\"); }").unwrap();

        (dir, root)
    }

    #[test]
    fn test_explore_overview() {
        let (_dir, root) = setup_project();
        let result = explore(&root, "", ZoomLevel::Overview).unwrap();

        assert_eq!(result.level, ZoomLevel::Overview);
        assert_eq!(result.node.node_type, "project");
        assert!(!result.node.children.is_empty());
        assert!(result.node.tokens > 0);
    }

    #[test]
    fn test_explore_directory() {
        let (_dir, root) = setup_project();
        let result = explore(&root, "src/auth", ZoomLevel::Directory).unwrap();

        assert_eq!(result.level, ZoomLevel::Directory);
        assert_eq!(result.node.node_type, "directory");
        assert!(result.node.children.len() >= 2); // login.rs, session.rs
    }

    #[test]
    fn test_explore_file() {
        let (_dir, root) = setup_project();
        let result = explore(&root, "src/auth/login.rs", ZoomLevel::File).unwrap();

        assert_eq!(result.level, ZoomLevel::File);
        assert_eq!(result.node.node_type, "file");
        assert!(!result.node.children.is_empty()); // Should have function signatures
        assert!(result.node.tokens > 0);
    }

    #[test]
    fn test_explore_full() {
        let (_dir, root) = setup_project();
        let result = explore(&root, "src/main.rs", ZoomLevel::Full).unwrap();

        assert_eq!(result.level, ZoomLevel::Full);
        assert!(result.node.content.is_some());
        assert!(result.node.content.unwrap().contains("fn main"));
    }

    #[test]
    fn test_explore_auto_level() {
        let (_dir, root) = setup_project();
        // Asking for Full on a directory should fall back to Directory
        let result = explore(&root, "src/auth", ZoomLevel::Full).unwrap();
        assert_eq!(result.level, ZoomLevel::Directory);
    }

    #[test]
    fn test_explore_available_levels() {
        let (_dir, root) = setup_project();
        let result = explore(&root, "", ZoomLevel::Overview).unwrap();
        assert!(result.available_levels.contains(&ZoomLevel::Overview));
        assert!(result.available_levels.contains(&ZoomLevel::Directory));

        let result = explore(&root, "src/main.rs", ZoomLevel::Full).unwrap();
        assert!(result.available_levels.contains(&ZoomLevel::File));
        assert!(result.available_levels.contains(&ZoomLevel::Full));
    }

    #[test]
    fn test_explore_file_drill_down() {
        let (_dir, root) = setup_project();
        let result = explore(&root, "src/auth", ZoomLevel::Directory).unwrap();

        // Each file should have a drill_down hint
        for child in &result.node.children {
            assert!(!child.drill_down.is_empty());
        }
    }

    #[test]
    fn test_explore_empty_project() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        std::process::Command::new("git")
            .args(["init", root.to_str().unwrap()])
            .output()
            .ok();

        let result = explore(root, "", ZoomLevel::Overview).unwrap();
        assert_eq!(result.node.tokens, 0);
        assert!(result.node.children.is_empty());
    }
}
