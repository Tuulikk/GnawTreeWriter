/*
 * GnawTreeWriter - diff parser
 *
 * Parses unified diffs (git diff format) and converts them to batch operations.
 * Enables AI agents and users to provide diffs that can be previewed and applied
 * as atomic batch operations with full validation and rollback support.
 */

use anyhow::{anyhow, Result};
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::core::batch::BatchEdit;
use crate::core::Batch;
use crate::parser::{get_parser, TreeNode};

/// Represents a single hunk in a unified diff
#[derive(Debug, Clone)]
pub struct DiffHunk {
    /// File path that this hunk applies to
    pub file_path: PathBuf,
    /// Line number where old file hunk starts
    pub old_start: usize,
    /// Number of lines in old file hunk
    pub old_count: usize,
    /// Line number where new file hunk starts
    pub new_start: usize,
    /// Number of lines in new file hunk
    pub new_count: usize,
    /// Lines in this hunk (context, additions, deletions)
    pub lines: Vec<DiffLine>,
}

/// Individual line in a diff hunk
#[derive(Debug, Clone, PartialEq)]
pub enum DiffLine {
    /// Context line (unchanged, prefixed with space)
    Context(String),
    /// Addition line (added, prefixed with +)
    Addition(String),
    /// Deletion line (removed, prefixed with -)
    Deletion(String),
}

/// Result of parsing a complete unified diff
#[derive(Debug, Clone)]
pub struct ParsedDiff {
    /// All hunks found in the diff, grouped by file
    pub hunks: Vec<DiffHunk>,
    /// Metadata extracted from diff headers
    pub metadata: DiffMetadata,
}

/// Metadata extracted from unified diff headers
#[derive(Debug, Clone)]
pub struct DiffMetadata {
    /// Mapping of old file paths to new file paths
    pub file_renames: HashMap<PathBuf, PathBuf>,
    /// Original file paths mentioned in the diff
    pub files: Vec<PathBuf>,
}

/// Parse a unified diff string into ParsedDiff
pub fn parse_unified_diff(diff: &str) -> Result<ParsedDiff> {
    let mut hunks = Vec::new();
    let mut file_renames = HashMap::new();
    let mut files = Vec::new();

    // Regex patterns for diff headers
    let file_header_re = Regex::new(r"^--- ([^\s]+)")?;
    let new_file_header_re = Regex::new(r"^\+\+\+ ([^\s]+)")?;
    let hunk_header_re = Regex::new(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@")?;

    let lines: Vec<&str> = diff.lines().collect();
    let mut i = 0;

    let mut current_file: Option<PathBuf> = None;
    let mut old_file: Option<PathBuf> = None;

    while i < lines.len() {
        let line = lines[i];

        // Check for file header (--- old_file)
        if let Some(caps) = file_header_re.captures(line) {
            let file_path = normalize_path(&caps[1]);
            old_file = Some(file_path.clone());
            files.push(file_path);
        }
        // Check for new file header (+++ new_file)
        else if let Some(caps) = new_file_header_re.captures(line) {
            let new_path = normalize_path(&caps[1]);
            current_file = Some(new_path.clone());

            // Track file rename if we have both old and new
            if let Some(old) = &old_file {
                if old != &new_path {
                    file_renames.insert(old.clone(), new_path);
                }
            }
        }
        // Check for hunk header (@@ -a,b +c,d @@)
        else if let Some(caps) = hunk_header_re.captures(line) {
            if current_file.is_none() {
                return Err(anyhow!("Found hunk header without file header"));
            }

            let old_start: usize = caps[1].parse()?;
            let old_count: usize = caps
                .get(2)
                .map(|m| m.as_str().parse().unwrap_or(1))
                .unwrap_or(1);
            let new_start: usize = caps[3].parse()?;
            let new_count: usize = caps
                .get(4)
                .map(|m| m.as_str().parse().unwrap_or(1))
                .unwrap_or(1);

            let mut hunk_lines = Vec::new();
            i += 1;

            // Collect hunk lines until next hunk header or EOF
            while i < lines.len() && !lines[i].starts_with("@@") && !lines[i].starts_with("---") {
                let hunk_line = lines[i];
                if hunk_line.starts_with('+') && !hunk_line.starts_with("++") {
                    hunk_lines.push(DiffLine::Addition(hunk_line[1..].to_string()));
                } else if hunk_line.starts_with('-') && !hunk_line.starts_with("--") {
                    hunk_lines.push(DiffLine::Deletion(hunk_line[1..].to_string()));
                } else if hunk_line.starts_with(' ') || hunk_line.is_empty() {
                    hunk_lines.push(DiffLine::Context(hunk_line.to_string()));
                }
                i += 1;
            }
            i -= 1; // Back up one since we'll increment at loop end

            hunks.push(DiffHunk {
                file_path: current_file.clone().unwrap(),
                old_start,
                old_count,
                new_start,
                new_count,
                lines: hunk_lines,
            });
        }

        i += 1;
    }

    if hunks.is_empty() {
        return Err(anyhow!("No valid hunks found in diff"));
    }

    Ok(ParsedDiff {
        hunks,
        metadata: DiffMetadata {
            file_renames,
            files,
        },
    })
}

/// Convert a parsed diff to batch operations
///
/// For each hunk, this function:
/// 1. Parses the target file with the appropriate TreeSitter parser
/// 2. Finds the AST node at the hunk's line number
/// 3. Creates Edit or Insert operations with proper node paths
pub fn diff_to_batch(diff: &ParsedDiff) -> Result<Batch> {
    // Group hunks by file
    let mut file_hunks: HashMap<PathBuf, Vec<&DiffHunk>> = HashMap::new();
    for hunk in &diff.hunks {
        file_hunks.entry(hunk.file_path.clone()).or_default().push(hunk);
    }

    let mut batch = Batch::new();

    for (file_path, hunks) in file_hunks.into_iter() {
        let file_str = file_path.to_string_lossy().to_string();

        // Parse the file to get AST for line-to-node resolution
        let source = std::fs::read_to_string(&file_path)
            .map_err(|e| anyhow!("Failed to read {}: {}", file_path.display(), e))?;
        let parser = get_parser(&file_path)
            .map_err(|e| anyhow!("No parser available for {}: {}", file_path.display(), e))?;
        let tree = parser.parse(&source)
            .map_err(|e| anyhow!("Failed to parse {}: {}", file_path.display(), e))?;

        let mut operations = Vec::new();

        for hunk in hunks {
            let deletions: Vec<&DiffLine> = hunk
                .lines
                .iter()
                .filter(|l| matches!(l, DiffLine::Deletion(_)))
                .collect();
            let additions: Vec<&DiffLine> = hunk
                .lines
                .iter()
                .filter(|l| matches!(l, DiffLine::Addition(_)))
                .collect();

            if !deletions.is_empty() && !additions.is_empty() {
                // Edit: find the target node and replace it.
                // Strategy: find the node at the first deletion line,
                // then replace its entire content with the new additions.
                // We also include any trailing context that belongs to the node.
                let mut first_del_line = hunk.old_start;
                for line in &hunk.lines {
                    if matches!(line, DiffLine::Context(_)) {
                        first_del_line += 1;
                    } else {
                        break;
                    }
                }

                // Build new content from additions
                let mut new_parts: Vec<String> = additions
                    .iter()
                    .map(|l| match l {
                        DiffLine::Addition(s) => s.clone(),
                        _ => String::new(),
                    })
                    .collect();

                // If the old node had a closing delimiter (like '}' or ')') that
                // wasn't part of the deletion, we need to append it.
                let mut past_additions = false;
                for line in &hunk.lines {
                    match line {
                        DiffLine::Addition(_) => past_additions = true,
                        DiffLine::Context(s) if past_additions => {
                            let trimmed = s.trim();
                            if trimmed == "}" || trimmed == ")" || trimmed == "]" {
                                // Use indentation from the first addition line
                                let indent = additions
                                    .first()
                                    .and_then(|l| match l {
                                        DiffLine::Addition(s) => Some(s),
                                        _ => None,
                                    })
                                    .and_then(|s| s.chars().take_while(|c| *c == ' ' || *c == '\t').collect::<String>().into())
                                    .unwrap_or_default();
                                new_parts.push(format!("{}{}", indent, trimmed));
                                break;
                            }
                        }
                        _ => {}
                    }
                }

                let new_content = new_parts.join("\n");

                if !new_content.is_empty() {
                    // Find the best enclosing node at the deletion line
                    let node_path = resolve_line_to_node(&tree, first_del_line, &source);
                    operations.push(BatchEdit::Edit {
                        node_path,
                        content: new_content,
                    });
                }
            } else if !additions.is_empty() {
                // Pure insert: find parent node and compute correct position
                let content: String = additions
                    .iter()
                    .map(|l| match l {
                        DiffLine::Addition(s) => s.clone(),
                        _ => String::new(),
                    })
                    .collect::<Vec<String>>()
                    .join("\n");

                if !content.is_empty() {
                    let parent = tree.find_parent_at_line(hunk.new_start);
                    // Determine position: find which sibling index comes after the hunk line
                    let position = compute_insert_position(&tree, parent, hunk.new_start);
                    operations.push(BatchEdit::Insert {
                        parent_path: parent.path.clone(),
                        position,
                        content,
                    });
                }
            }
            // Pure deletions are not supported yet (would need Delete operation)
        }

        if !operations.is_empty() {
            batch = Batch::with_file(file_str, operations);
        }
    }

    Ok(batch)
}

/// Resolve a 1-based line number to the path of the best enclosing node for editing.
/// For diff edits, we want a meaningful container (function, class, statement),
/// not a leaf node (keyword, identifier). We walk up from the deepest match
/// to find the smallest node that has both a meaningful type and children.
fn resolve_line_to_node(tree: &TreeNode, line: usize, source: &str) -> String {
    let line_count = source.lines().count();
    let line = if line > line_count { line_count } else if line == 0 { 1 } else { line };

    // First find the deepest node at this line
    let deepest = match tree.find_node_at_line(line) {
        Some(n) => n,
        None => return tree.path.clone(),
    };

    // If the deepest node is a leaf-like type, find its enclosing
    // container by searching for the smallest ancestor that has children
    // and is a meaningful edit target.
    let nt = deepest.node_type.to_lowercase();
    let is_leaf = nt.contains("identifier")
        || nt.contains("keyword")
        || nt.contains(":")
        || nt.contains("operator")
        || nt.contains("literal")
        || nt.contains("string")
        || nt.contains("comment")
        || nt.contains("parameter")
        || deepest.children.is_empty();

    if is_leaf {
        // Walk up: find the first ancestor that is a scoped container
        // We need to search the tree for a node whose path is a prefix
        // of the deepest node's path.
        return find_editable_ancestor(tree, &deepest.path)
            .map(|n| n.path.clone())
            .unwrap_or_else(|| tree.path.clone());
    }

    deepest.path.clone()
}

/// Find the smallest ancestor of the node at `target_path` that is a
/// meaningful edit target (function, class, struct, statement, etc.).
/// Compute the insert position within a parent node based on which sibling
/// follows the given line number.
///
/// GTW position semantics:
///   0 = prepend (after opening brace or at top)
///   1 = append (at end of parent)
///   2 = after last ui_property (QML-specific)
///   3+ = after child at index (position - 3). So position N = after child N-3.
///
/// To insert before child at index `child_idx`, use position `child_idx + 3`.
fn compute_insert_position(root: &TreeNode, parent: &TreeNode, line: usize) -> usize {
    // Walk the parent's path to get the actual parent node with children
    let parent_path = &parent.path;
    let parent_node: &TreeNode = if parent_path.is_empty() || parent_path == "root" {
        root
    } else {
        let segments: Vec<&str> = parent_path.split('.').filter(|s| !s.is_empty()).collect();
        let mut node = root;
        for seg in segments {
            if let Ok(idx) = seg.parse::<usize>() {
                if let Some(child) = node.children.get(idx) {
                    node = child;
                } else {
                    break;
                }
            }
        }
        node
    };

    // Find the first child that starts at or after the target line.
    // We want to insert BEFORE that child.
    // GTW semantics: position 0 = prepend, position 1 = append,
    // position 3+N = after child N. So to insert BEFORE child at index i,
    // we use position (i + 2) = after child (i-1), with special case i=0 → position 0.
    for (i, child) in parent_node.children.iter().enumerate() {
        if child.start_line >= line {
            return if i == 0 { 0 } else { i + 2 };
        }
    }

    // No child starts after the line — append at end
    1
}

fn find_editable_ancestor<'a>(tree: &'a TreeNode, target_path: &str) -> Option<&'a TreeNode> {
    // Walk the path segments to find the node, then check each ancestor
    let segments: Vec<&str> = target_path.split('.').collect();
    let mut current = tree;
    let mut best: Option<&TreeNode> = None;

    for _seg in segments.iter() {
        let nt = current.node_type.to_lowercase();
        let is_meaningful = nt.contains("function")
            || nt.contains("method")
            || nt.contains("class")
            || nt.contains("struct")
            || nt.contains("enum")
            || nt.contains("impl")
            || nt.contains("trait")
            || nt.contains("mod")
            || nt.contains("statement")
            || nt.contains("block")
            || nt.contains("body")
            || nt.contains("declaration")
            || nt.contains("source_file")
            || nt.contains("program")
            || nt.contains("module");

        if is_meaningful && !current.children.is_empty() {
            best = Some(current);
        }

        // Navigate to the next child
        let seg_num: usize = _seg.parse().ok()?;
        current = current.children.get(seg_num)?;
    }

    best
}

/// Normalize file paths from diff headers (remove a/ or b/ prefix)
fn normalize_path(path: &str) -> PathBuf {
    let path = path.trim();
    let path = path.strip_prefix("a/").unwrap_or(path);
    let path = path.strip_prefix("b/").unwrap_or(path);
    PathBuf::from(path)
}

/// Read diff from file and parse it
pub fn parse_diff_file<P: AsRef<Path>>(diff_file: P) -> Result<ParsedDiff> {
    let content = std::fs::read_to_string(diff_file.as_ref())
        .map_err(|e| anyhow!("Failed to read diff file: {}", e))?;
    parse_unified_diff(&content)
}

/// Generate a preview description of what the diff will do
pub fn preview_diff(diff: &ParsedDiff) -> String {
    let mut preview = String::new();
    preview.push_str("=== Diff Preview ===\n\n");

    let mut file_hunks: HashMap<&PathBuf, Vec<&DiffHunk>> = HashMap::new();
    for hunk in &diff.hunks {
        file_hunks.entry(&hunk.file_path).or_default().push(hunk);
    }

    for (file_path, hunks) in file_hunks {
        preview.push_str(&format!("File: {}\n", file_path.display()));
        preview.push_str(&format!("  Hunks: {}\n", hunks.len()));

        let mut total_additions = 0;
        let mut total_deletions = 0;

        for hunk in hunks {
            let additions = hunk
                .lines
                .iter()
                .filter(|l| matches!(l, DiffLine::Addition(_)))
                .count();
            let deletions = hunk
                .lines
                .iter()
                .filter(|l| matches!(l, DiffLine::Deletion(_)))
                .count();
            total_additions += additions;
            total_deletions += deletions;

            preview.push_str(&format!(
                "  Hunk @ line {} (+{}, -{})\n",
                hunk.old_start, additions, deletions
            ));
        }

        preview.push_str(&format!(
            "  Total: +{}, -{} lines\n\n",
            total_additions, total_deletions
        ));
    }

    preview.push_str(&format!(
        "Total files affected: {}\n",
        diff.metadata.files.len()
    ));
    preview
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_diff() {
        let diff = r#"--- a/test.py
+++ b/test.py
@@ -1,3 +1,3 @@
 def foo():
-    return "old"
+    return "new"
     print("hello")
"#;

        let parsed = parse_unified_diff(diff).unwrap();
        assert_eq!(parsed.hunks.len(), 1);

        let hunk = &parsed.hunks[0];
        assert_eq!(hunk.file_path, PathBuf::from("test.py"));
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.new_start, 1);

        assert_eq!(hunk.lines.len(), 4);
        assert!(matches!(hunk.lines[0], DiffLine::Context(_)));
        assert!(matches!(hunk.lines[1], DiffLine::Deletion(_)));
        assert!(matches!(hunk.lines[2], DiffLine::Addition(_)));
        assert!(matches!(hunk.lines[3], DiffLine::Context(_)));
    }

    #[test]
    fn test_parse_multi_file_diff() {
        let diff = r#"--- a/file1.py
+++ b/file1.py
@@ -1,1 +1,1 @@
-old
+new
--- a/file2.py
+++ b/file2.py
@@ -5,1 +5,1 @@
-x
+y
"#;

        let parsed = parse_unified_diff(diff).unwrap();
        assert_eq!(parsed.hunks.len(), 2);
        assert_eq!(parsed.metadata.files.len(), 2);
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("a/file.txt"), PathBuf::from("file.txt"));
        assert_eq!(normalize_path("b/file.txt"), PathBuf::from("file.txt"));
        assert_eq!(normalize_path("file.txt"), PathBuf::from("file.txt"));
    }

    #[test]
    fn test_preview_diff() {
        let diff = r#"--- a/test.py
+++ b/test.py
@@ -1,2 +1,2 @@
 line1
-old line
+new line
"#;

        let parsed = parse_unified_diff(diff).unwrap();
        let preview = preview_diff(&parsed);

        assert!(preview.contains("test.py"));
        assert!(preview.contains("+1"));
        assert!(preview.contains("-1"));
    }

    #[test]
    fn test_diff_to_batch() {
        let diff = r#"--- a/test.py
+++ b/test.py
@@ -1,2 +1,2 @@
 line1
-old
+new
"#;

        let parsed = parse_unified_diff(diff).unwrap();
        let batch = diff_to_batch(&parsed);

        assert!(batch.is_ok());
    }
}
