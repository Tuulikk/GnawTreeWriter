//! Project packaging for AI context.
//!
//! Combines git-aware file walking, token counting, and code compression
//! to produce a single AI-optimized output of an entire project.

use crate::core::compress::compress_source;
use crate::core::file_walker::walk_source_files_filtered;
use crate::core::token_count::estimate_code_tokens;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Output format for pack command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackFormat {
    /// Markdown with tree structure and code blocks.
    Markdown,
    /// JSON with per-file metadata.
    Json,
    /// Plain text with markers.
    Plain,
}

impl PackFormat {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => PackFormat::Json,
            "plain" | "text" => PackFormat::Plain,
            _ => PackFormat::Markdown,
        }
    }
}

/// Options for packing a project.
#[derive(Debug, Clone)]
pub struct PackOptions {
    /// Output format.
    pub format: PackFormat,
    /// Whether to compress function bodies.
    pub compress: bool,
    /// Whether to include token counts.
    pub tokens: bool,
    /// File extensions to include (empty = all supported).
    pub include_extensions: Vec<String>,
    /// Additional ignore patterns (supplements .gitignore).
    pub ignore_patterns: Vec<String>,
    /// Custom instructions to include in output.
    pub instructions: Option<String>,
    /// Output file path (None = stdout).
    pub output: Option<String>,
}

impl Default for PackOptions {
    fn default() -> Self {
        PackOptions {
            format: PackFormat::Markdown,
            compress: false,
            tokens: true,
            include_extensions: vec![],
            ignore_patterns: vec![],
            instructions: None,
            output: None,
        }
    }
}

/// Result of packing a project.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PackResult {
    /// The packed output content.
    pub content: String,
    /// Total files processed.
    pub file_count: usize,
    /// Total original tokens.
    pub total_tokens: usize,
    /// Total compressed tokens (if compression enabled).
    pub compressed_tokens: usize,
    /// Per-file information.
    pub files: Vec<PackFileInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PackFileInfo {
    pub path: String,
    pub tokens: usize,
    pub lines: usize,
}

/// Pack a project into an AI-optimized format.
pub fn pack_project(root: &Path, options: &PackOptions) -> Result<PackResult> {
    let ext_refs: Vec<&str> = options
        .include_extensions
        .iter()
        .map(|s| s.as_str())
        .collect();

    let files = walk_source_files_filtered(root, &ext_refs);

    // Find the actual project root (where .git or .gnawtreewriter_session.json is)
    let project_root = crate::core::find_project_root(root);

    // Filter out files matching ignore patterns
    let files: Vec<PathBuf> = files
        .into_iter()
        .filter(|path| {
            let path_str = path.to_string_lossy();
            !options.ignore_patterns.iter().any(|pat| path_str.contains(pat))
        })
        .collect();

    let mut pack_files = Vec::new();
    let mut total_tokens = 0usize;
    let mut compressed_tokens = 0usize;
    let mut file_entries: Vec<(String, String)> = Vec::new(); // (path, content)

    for path in &files {
        // Get path relative to project root for display
        let rel_path = path
            .strip_prefix(&project_root)
            .or_else(|_| path.strip_prefix(root))
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        // If strip_prefix gave empty (single file case), use the filename
        let rel_path = if rel_path.is_empty() {
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        } else {
            rel_path
        };

        if let Ok(content) = std::fs::read_to_string(path) {
            let tokens = estimate_code_tokens(&content);
            total_tokens += tokens;

            let lines = content.lines().count();

            pack_files.push(PackFileInfo {
                path: rel_path.clone(),
                tokens,
                lines,
            });

            file_entries.push((rel_path, content));
        }
    }

    let content = match options.format {
        PackFormat::Markdown => format_markdown(&file_entries, options),
        PackFormat::Json => format_json(&file_entries, options)?,
        PackFormat::Plain => format_plain(&file_entries, options),
    };

    if options.compress {
        // Re-count compressed tokens from the final output
        compressed_tokens = estimate_code_tokens(&content);
    } else {
        compressed_tokens = total_tokens;
    }

    Ok(PackResult {
        content,
        file_count: pack_files.len(),
        total_tokens,
        compressed_tokens,
        files: pack_files,
    })
}

fn format_markdown(files: &[(String, String)], options: &PackOptions) -> String {
    let mut output = String::new();

    // Header with metadata
    output.push_str("# Project Context\n\n");

    if let Some(ref instructions) = options.instructions {
        output.push_str("## Instructions\n\n");
        output.push_str(instructions);
        output.push_str("\n\n");
    }

    // Tree structure overview
    output.push_str("## Structure\n\n");
    output.push_str("```\n");
    let mut prev_dir = String::new();
    for (path, _) in files {
        let dir = Path::new(path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");
        if dir != prev_dir {
            if !dir.is_empty() {
                output.push_str(&format!("{}/\n", dir));
            }
            prev_dir = dir.to_string();
        }
        let name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);
        let indent = if dir.is_empty() { "" } else { "  " };
        output.push_str(&format!("{}{}\n", indent, name));
    }
    output.push_str("```\n\n");

    // Summary table
    output.push_str("## Summary\n\n");
    output.push_str("| File | Tokens | Lines |\n");
    output.push_str("|------|--------|-------|\n");
    for (path, content) in files {
        let tokens = estimate_code_tokens(content);
        let lines = content.lines().count();
        output.push_str(&format!("| {} | {} | {} |\n", path, tokens, lines));
    }
    output.push_str("\n");

    // File contents
    output.push_str("## Files\n\n");

    for (path, content) in files {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("text");

        let display_content = if options.compress {
            match crate::parser::get_parser(Path::new(path)) {
                Ok(parser) => {
                    if let Ok(tree) = parser.parse(content) {
                        let compressed = compress_source(content, &tree);
                        compressed.code
                    } else {
                        content.clone()
                    }
                }
                Err(_) => content.clone(),
            }
        } else {
            content.clone()
        };

        let tokens = if options.tokens {
            let t = estimate_code_tokens(&display_content);
            format!(" ({} tokens)", t)
        } else {
            String::new()
        };

        let lines = display_content.lines().count();
        output.push_str(&format!("### {}{} [{} lines]\n\n", path, tokens, lines));
        output.push_str(&format!("```{}\n{}\n```\n\n", ext, display_content));
    }

    output
}

fn format_json(files: &[(String, String)], options: &PackOptions) -> Result<String> {
    let mut file_array = Vec::new();
    let mut total_lines = 0usize;

    for (path, content) in files {
        let display_content = if options.compress {
            match crate::parser::get_parser(Path::new(path)) {
                Ok(parser) => {
                    if let Ok(tree) = parser.parse(content) {
                        let compressed = compress_source(content, &tree);
                        compressed.code
                    } else {
                        content.clone()
                    }
                }
                Err(_) => content.clone(),
            }
        } else {
            content.clone()
        };

        let tokens = estimate_code_tokens(&display_content);
        let lines = content.lines().count();
        total_lines += lines;

        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("text");

        file_array.push(serde_json::json!({
            "path": path,
            "content": display_content,
            "tokens": tokens,
            "lines": lines,
            "language": ext,
        }));
    }

    let total_tokens: usize = file_array
        .iter()
        .filter_map(|f| f.get("tokens").and_then(|t| t.as_u64()).map(|t| t as usize))
        .sum();

    // Build language breakdown
    let mut language_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for f in &file_array {
        if let Some(lang) = f.get("language").and_then(|l| l.as_str()) {
            *language_counts.entry(lang.to_string()).or_insert(0) += 1;
        }
    }

    let output = serde_json::json!({
        "instructions": options.instructions,
        "file_count": files.len(),
        "total_tokens": total_tokens,
        "total_lines": total_lines,
        "languages": language_counts,
        "compress": options.compress,
        "files": file_array,
    });

    Ok(serde_json::to_string_pretty(&output)?)
}

fn format_plain(files: &[(String, String)], options: &PackOptions) -> String {
    let mut output = String::new();

    output.push_str("=== PROJECT CONTEXT ===\n\n");

    if let Some(ref instructions) = options.instructions {
        output.push_str(&format!("Instructions: {}\n\n", instructions));
    }

    for (path, content) in files {
        let display_content = if options.compress {
            match crate::parser::get_parser(Path::new(path)) {
                Ok(parser) => {
                    if let Ok(tree) = parser.parse(content) {
                        let compressed = compress_source(content, &tree);
                        compressed.code
                    } else {
                        content.clone()
                    }
                }
                Err(_) => content.clone(),
            }
        } else {
            content.clone()
        };

        let tokens = if options.tokens {
            format!(" [{} tokens]", estimate_code_tokens(&display_content))
        } else {
            String::new()
        };

        output.push_str(&format!("--- {}{} ---\n", path, tokens));
        output.push_str(&display_content);
        output.push_str("\n\n");
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn setup_project() -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();

        // Initialize git repo for .gitignore to work
        std::process::Command::new("git")
            .args(["init", root.to_str().unwrap()])
            .output()
            .ok();

        // Create src directory
        fs::create_dir_all(root.join("src")).unwrap();

        // Create source files
        fs::write(
            root.join("src/main.rs"),
            "fn main() {\n    println!(\"hello\");\n}",
        )
        .unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}",
        )
        .unwrap();
        fs::write(root.join("README.md"), "# My Project\n\nA test project.")
            .unwrap();

        // Create a file that should be ignored
        fs::write(root.join(".gitignore"), "target/\n").unwrap();
        fs::create_dir(root.join("target")).unwrap();
        fs::write(root.join("target/debug.bin"), "binary").unwrap();

        (dir, root)
    }

    #[test]
    fn test_pack_markdown_default() {
        let (_dir, root) = setup_project();
        let options = PackOptions::default();
        let result = pack_project(&root, &options).unwrap();

        assert!(result.file_count >= 2); // At least src files
        assert!(result.total_tokens > 0);
        assert!(result.content.contains("# Project Context"));
        assert!(result.content.contains("fn main()"));
    }

    #[test]
    fn test_pack_respects_gitignore() {
        let (_dir, root) = setup_project();
        let options = PackOptions::default();
        let result = pack_project(&root, &options).unwrap();

        // Should NOT contain target/debug.bin
        assert!(!result.content.contains("debug.bin"),
            "Should respect .gitignore. Got:\n{}", result.content);
    }

    #[test]
    fn test_pack_with_compression() {
        let (_dir, root) = setup_project();
        let options = PackOptions {
            compress: true,
            ..Default::default()
        };
        let result = pack_project(&root, &options).unwrap();

        assert!(result.content.contains("⋮----"),
            "Should contain compression placeholder. Got:\n{}", result.content);
        // Note: compressed_tokens may be higher than total_tokens due to markdown formatting
        // The real test is that compression placeholders exist
    }

    #[test]
    fn test_pack_with_instructions() {
        let (_dir, root) = setup_project();
        let options = PackOptions {
            instructions: Some("Focus on the main function".to_string()),
            ..Default::default()
        };
        let result = pack_project(&root, &options).unwrap();

        assert!(result.content.contains("Focus on the main function"));
    }

    #[test]
    fn test_pack_json_format() {
        let (_dir, root) = setup_project();
        let options = PackOptions {
            format: PackFormat::Json,
            ..Default::default()
        };
        let result = pack_project(&root, &options).unwrap();

        let json: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert!(json.get("files").is_some());
        assert!(json.get("total_tokens").is_some());
    }

    #[test]
    fn test_pack_plain_format() {
        let (_dir, root) = setup_project();
        let options = PackOptions {
            format: PackFormat::Plain,
            ..Default::default()
        };
        let result = pack_project(&root, &options).unwrap();

        assert!(result.content.contains("=== PROJECT CONTEXT ==="));
    }

    #[test]
    fn test_pack_with_extension_filter() {
        let (_dir, root) = setup_project();
        let options = PackOptions {
            include_extensions: vec!["rs".to_string()],
            ..Default::default()
        };
        let result = pack_project(&root, &options).unwrap();

        // Should contain Rust files but not markdown
        assert!(result.content.contains("fn main()"));
        assert!(!result.content.contains("# My Project"));
    }

    #[test]
    fn test_pack_empty_project() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        std::process::Command::new("git")
            .args(["init", root.to_str().unwrap()])
            .output()
            .ok();

        // Create a .gitignore to exclude everything except source files
        fs::write(root.join(".gitignore"), ".git/\n*.txt\n").unwrap();
        fs::write(root.join("notes.txt"), "not a source file").unwrap();

        let options = PackOptions::default();
        let result = pack_project(root, &options).unwrap();

        // Should find .gitignore (it's a text file) but not notes.txt
        assert!(result.file_count <= 1,
            "Should find at most 1 file, found: {}", result.file_count);
        assert!(result.total_tokens >= 0);
    }
}
