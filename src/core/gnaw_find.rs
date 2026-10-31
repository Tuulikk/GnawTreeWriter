//! gnaw-find: Find AST nodes across project files

use crate::core::file_walker::walk_source_files_filtered;
use crate::{GnawTreeWriter, TreeNode};
use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

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
    let search_dir = directory
        .map(Path::new)
        .unwrap_or_else(|| current_dir.as_path());

    let exts: Vec<&str> = extensions
        .map(|s| s.split(',').map(|s| s.trim()).collect())
        .unwrap_or_else(|| {
            vec![
                "rs", "py", "js", "ts", "tsx", "jsx", "go", "java", "c", "cpp", "h", "hpp", "cs",
                "php",
            ]
        });

    let mut files: Vec<std::path::PathBuf> = Vec::new();

    if recursive {
        files = walk_source_files_filtered(search_dir, &exts);
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
                collect_by_type(tree, tf, &path_str, max_results, &mut results);
            } else if let Some(txt) = text {
                collect_by_text(tree, txt, &path_str, max_results, &mut results);
            } else if let Some(pat) = pattern {
                collect_by_type(tree, pat, &path_str, max_results, &mut results);
                collect_by_text(tree, pat, &path_str, max_results, &mut results);
            }
        }
    }

    // Sort by file then line
    results.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
    results.truncate(max_results);

    Ok(results)
}

fn collect_by_type(
    tree: &TreeNode,
    filter: &str,
    file: &str,
    max: usize,
    results: &mut Vec<FindResult>,
) {
    if results.len() >= max {
        return;
    }
    if tree
        .node_type
        .to_lowercase()
        .contains(&filter.to_lowercase())
    {
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

fn collect_by_text(
    tree: &TreeNode,
    search: &str,
    file: &str,
    max: usize,
    results: &mut Vec<FindResult>,
) {
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
        output.push_str(&format!(
            "  {}:{} [{}] {}\n",
            r.line, r.path, r.node_type, r.name
        ));
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

/// A lexical (non-embedding) fallback hit for the semantic channels (T5).
#[derive(Debug)]
pub struct LexicalHit {
    pub file: String,
    pub path: String,
    pub kind: String,
    pub line: usize,
    pub preview: String,
}

/// Tokenized query terms for the lexical degrade: lowercase, split on
/// whitespace/punctuation, at least 3 chars (skips "to", "is", "a").
fn lexical_terms(query: &str) -> Vec<String> {
    query
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .filter(|t| t.len() >= 3)
        .map(|t| t.to_ascii_lowercase())
        .collect()
}

/// Degraded search for sense/search_semantic when the embedding model or
/// project index is missing (issue log 2026-10-18/23, T5): answers with
/// name / first-line text matches instead of dying on a cryptic model
/// error. HONEST by contract: these are lexical matches with NO semantic
/// ranking — callers MUST name the degradation in their replies.
/// `file` (zoom mode) searches one file; `None` (satellite mode) scans
/// project source files (walk, cap 400 files, stops early at enough hits).
pub fn lexical_degrade(
    root: &std::path::Path,
    file: Option<&str>,
    query: &str,
    max: usize,
) -> Result<Vec<LexicalHit>> {
    let terms = lexical_terms(query);
    if terms.is_empty() {
        return Ok(Vec::new());
    }
    let exts: Vec<&str> = vec![
        "rs", "py", "js", "ts", "tsx", "jsx", "go", "java", "c", "cpp", "h", "hpp", "php", "zig",
        "css", "qml",
    ];
    let files: Vec<std::path::PathBuf> = match file {
        Some(f) => vec![std::path::PathBuf::from(f)],
        None => walk_source_files_filtered(root, &exts)
            .into_iter()
            .take(400)
            .collect(),
    };

    let mut hits: Vec<LexicalHit> = Vec::new();
    let budget = max.max(1) * 4;
    for file_path in &files {
        if hits.len() >= budget {
            break;
        }
        let path_str = file_path.to_string_lossy().to_string();
        let Ok(writer) = GnawTreeWriter::new(&path_str) else {
            continue;
        };
        let tree = writer.analyze();
        collect_lexical(tree, &terms, &path_str, budget, &mut hits);
    }
    hits.truncate(max);
    Ok(hits)
}

/// Post-order walker so the DEEPEST node owns a hit: container nodes
/// always contain their children's text, so top-down matching would seed
/// every hit with ancestors (the same noise a 1 121-node file root gives
/// list_nodes). A node whose NAME matches wins outright; otherwise a node
/// with no matching descendant may claim a hit via its FIRST content line.
/// Returns how many hits this subtree produced.
fn collect_lexical(
    tree: &TreeNode,
    terms: &[String],
    file: &str,
    max: usize,
    results: &mut Vec<LexicalHit>,
) -> usize {
    let mut sub = 0usize;
    for child in &tree.children {
        sub += collect_lexical(child, terms, file, max, results);
    }
    if results.len() >= max {
        return sub;
    }
    // Never seed the file root as a hit: children own the relevance, and
    // source_file behavior + get_name can alias the query (found via the
    // satellite regression test: get_name on a source_file may return
    // its first function-s name).
    if tree.node_type == "source_file" {
        return sub;
    }
    let name_hit = tree
        .get_name()
        .map(|n| {
            let n = n.to_lowercase();
            terms.iter().any(|t| n.contains(t.as_str()))
        })
        .unwrap_or(false);
    if name_hit {
        let name = tree.get_name().unwrap_or_else(|| "unnamed".to_string());
        results.push(LexicalHit {
            file: file.to_string(),
            path: tree.path.clone(),
            kind: tree.node_type.clone(),
            line: tree.start_line,
            preview: format!("{} — {}", name, first_line_preview(&tree.content)),
        });
        return sub + 1;
    }
    if sub == 0 {
        let first = first_line_preview(&tree.content).to_lowercase();
        if !first.is_empty() && terms.iter().any(|t| first.contains(t.as_str())) {
            results.push(LexicalHit {
                file: file.to_string(),
                path: tree.path.clone(),
                kind: tree.node_type.clone(),
                line: tree.start_line,
                preview: first_line_preview(&tree.content),
            });
            return sub + 1;
        }
    }
    sub
}

/// Char-safe preview (lesson: byte-slices panicked on å/ä/ö — chars().take
/// is the pattern). Empty content yields an empty preview.
fn first_line_preview(content: &str) -> String {
    let first = content
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty())
        .unwrap_or("");
    first.chars().take(96).collect()
}

#[cfg(test)]
mod lexical_degrade_tests {
    use super::*;

    fn write(dir: &std::path::Path, name: &str, src: &str) -> String {
        let p = dir.join(name);
        std::fs::write(&p, src).unwrap();
        p.to_string_lossy().to_string()
    }

    /// T5: satellite degrade — name hits across files, no model needed,
    /// and container nodes must never seed hits (deepest node owns them).
    #[test]
    fn lexical_degrade_satellite_finds_name_hits() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "alpha.rs",
            "fn complete_task(slot: usize) {\n    // marks the slot done\n}\n\nfn other() {}\n",
        );
        write(
            dir.path(),
            "beta.rs",
            "fn alpha_beta() {}\nstruct unrelated_thing;\n",
        );

        let hits = lexical_degrade(dir.path(), None, "complete task", 5).unwrap();
        assert!(
            hits.iter()
                .any(|h| h.file.ends_with("alpha.rs") && h.kind == "function_item"),
            "expected complete_task as an fn hit, got: {:?}",
            hits
        );
        let leaked: Vec<&LexicalHit> = hits
            .iter()
            .filter(|h| h.path.is_empty() || h.kind == "source_file")
            .collect();
        assert!(leaked.is_empty(), "root/container hit leaked: {:?}", leaked);
    }

    /// Short or noise-only queries return nothing — an empty satellite
    /// degrade stays honest instead of pretending.
    #[test]
    fn lexical_degrade_short_or_empty_query_is_no_hits() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "one.rs", "fn abc() {}\n");
        assert!(lexical_degrade(dir.path(), None, "is a", 5)
            .unwrap()
            .is_empty());
        assert!(lexical_degrade(dir.path(), None, "?!", 5)
            .unwrap()
            .is_empty());
    }

    /// Zoom degrade: a leaf's first content line can claim the hit (e.g.
    /// a comment that answers the query) when no name matches.
    #[test]
    fn lexical_degrade_zoom_hits_first_line_of_leaf() {
        let dir = tempfile::tempdir().unwrap();
        let a = write(
            dir.path(),
            "leaf.rs",
            "// handles graceful degradation\nfn unrelated_name() {}\n",
        );
        let hits = lexical_degrade(dir.path(), Some(&a), "graceful degradation", 5).unwrap();
        assert!(
            hits.iter().any(|h| h.file.ends_with("leaf.rs")),
            "expected a zoom hit from first-line text, got: {:?}",
            hits
        );
    }
}
