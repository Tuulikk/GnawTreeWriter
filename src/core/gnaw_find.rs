//! gnaw-find: Find AST nodes across project files

use crate::{GnawTreeWriter, TreeNode};
use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

/// gnaw-find: Search AST nodes across project files
pub fn find_nodes(
    pattern: Option<&str>,
    type_filter: Option<&str>,
    extensions: Option<&str>,
    text: Option<&str>,
    directory: Option<&str>,
    recursive: bool,
    max_results: usize,
) -> Result<Vec<FindResult>> {
    let current_dir = std::env::current_dir()?;
    let search_dir = directory.map(Path::new).unwrap_or_else(|| current_dir.as_path());

    let exts: Vec<&str> = extensions
        .map(|s| s.split(',').map(|s| s.trim()).collect())
        .unwrap_or_else(|| {
            vec![
                "rs", "py", "js", "ts", "tsx", "jsx", "go", "java", "c", "cpp", "h", "hpp", "cs", "php",
            ]
        });

    let mut files: Vec<std::path::PathBuf> = Vec::new();

    if recursive {
        for entry in WalkDir::new(search_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if exts.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                    // Skip common non-source dirs
                    let skip = path.components().any(|c| {
                        let s = c.as_os_str().to_string_lossy();
                        s == "target" || s == "node_modules" || s == ".git" || s.starts_with('.')
                    });
                    if !skip {
                        files.push(path.to_path_buf());
                    }
                }
            }
        }
    }

    if files.is_empty() {
        println!("No files found.");
        return Ok(Vec::new());
    }

    let mut results: Vec<FindResult> = Vec::new();

    for file_path in &files {
        let path_str = file_path.to_string_lossy();
        if let Ok(writer) = GnawTreeWriter::new(&path_str) {
            let tree = writer.analyze();

            if let Some(tf) = type_filter {
                collect_by_type(&tree, tf, &path_str, max_results, &mut results);
            } else if let Some(txt) = text {
                collect_by_text(&tree, txt, &path_str, max_results, &mut results);
            } else if let Some(pat) = pattern {
                collect_by_type(&tree, pat, &path_str, max_results, &mut results);
                collect_by_text(&tree, pat, &path_str, max_results, &mut results);
            }
        }
    }

    // Sort by file then line
    results.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
    results.truncate(max_results);

    Ok(results)
}

fn collect_by_type(tree: &TreeNode, filter: &str, file: &str, max: usize, results: &mut Vec<FindResult>) {
    if results.len() >= max {
        return;
    }
    if tree.node_type.to_lowercase().contains(&filter.to_lowercase()) {
        let name = tree.get_name().unwrap_or_else(|| "unnamed".to_string());
        results.push(FindResult {
            file: file.to_string(),
            node_type: tree.node_type.clone(),
            path: tree.path.clone(),
            name,
            line: tree.start_line,
        });
    }
    for child in &tree.children {
        collect_by_type(child, filter, file, max, results);
    }
}

fn collect_by_text(tree: &TreeNode, search: &str, file: &str, max: usize, results: &mut Vec<FindResult>) {
    if results.len() >= max {
        return;
    }
    if tree.content.to_lowercase().contains(&search.to_lowercase()) {
        let name = tree.get_name().unwrap_or_else(|| "unnamed".to_string());
        results.push(FindResult {
            file: file.to_string(),
            node_type: tree.node_type.clone(),
            path: tree.path.clone(),
            name,
            line: tree.start_line,
        });
    }
    for child in &tree.children {
        collect_by_text(child, search, file, max, results);
    }
}

#[derive(Serialize)]
pub struct FindResult {
    pub file: String,
    pub node_type: String,
    pub path: String,
    pub name: String,
    pub line: usize,
}

/// Format results as text output
pub fn format_results_text(results: &[FindResult], total: usize, max: usize) -> String {
    let mut output = String::new();
    let mut current_file = String::new();

    for r in results {
        if r.file != current_file {
            output.push_str(&format!("\n📄 {}\n", r.file));
            current_file = r.file.clone();
        }
        output.push_str(&format!("  {}:{} [{}] {}\n", r.line, r.path, r.node_type, r.name));
    }

    if total > max {
        output.push_str(&format!("\n(showing {} of {} matches)\n", max, total));
    }

    output
}

/// Format results as summary
pub fn format_results_summary(results: &[FindResult], total: usize) -> String {
    let mut by_file: HashMap<String, usize> = HashMap::new();
    for r in results {
        *by_file.entry(r.file.clone()).or_insert(0) += 1;
    }

    let mut output = format!("gnaw-find: {} files, {} matches\n", by_file.len(), total);
    for (file, count) in by_file.iter() {
        output.push_str(&format!("  {} [{}]\n", file, count));
    }

    output
}
