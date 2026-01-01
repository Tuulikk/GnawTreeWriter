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
pub fn diff_to_batch(diff: &ParsedDiff) -> Result<Batch> {
    let mut file_operations: HashMap<PathBuf, Vec<BatchEdit>> = HashMap::new();

    for hunk in &diff.hunks {
        let file_path = &hunk.file_path;

        // Convert hunk to one or more batch edits
        // For simple line replacements, we can use Edit operation
        // For complex multi-line changes, we may need multiple operations

        // Find the actual content to replace by looking at deletions
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

        // Strategy: For now, we'll create a simple replace operation
        // In the future, we could do AST-aware conversion

        if !deletions.is_empty() {
            // Extract deleted content
            let _deleted_content: String = deletions
                .iter()
                .map(|l| match l {
                    DiffLine::Deletion(s) => s.as_str(),
                    _ => "",
                })
                .collect::<Vec<&str>>()
                .join("\n");

            // Find insertion point (typically the line before the first deletion)
            // For now, we use the old_start line number
            // TODO: This is a simplified approach. A more robust implementation
            // would use AST parsing to find the exact node to edit

            // Create an edit operation at the line level
            // We use line number as a node path for now
            let node_path = format!("line:{}", hunk.old_start);

            // Create the new content by combining context and additions
            let new_content: String = additions
                .iter()
                .map(|l| match l {
                    DiffLine::Addition(s) => s.as_str(),
                    _ => "",
                })
                .collect::<Vec<&str>>()
                .join("\n");

            if !new_content.is_empty() {
                let batch_edit = BatchEdit::Edit {
                    node_path,
                    content: new_content,
                };

                file_operations
                    .entry(file_path.clone())
                    .or_default()
                    .push(batch_edit);
            }
        } else if !additions.is_empty() {
            // Pure addition - use insert operation
            // Insert at the specified line
            let node_path = format!("line:{}", hunk.new_start);
            let content: String = additions
                .iter()
                .map(|l| match l {
                    DiffLine::Addition(s) => s.as_str(),
                    _ => "",
                })
                .collect::<Vec<&str>>()
                .join("\n");

            if !content.is_empty() {
                let batch_edit = BatchEdit::Insert {
                    parent_path: node_path,
                    position: 1, // After the line
                    content,
                };

                file_operations
                    .entry(file_path.clone())
                    .or_default()
                    .push(batch_edit);
            }
        }
    }

    // Convert to Batch structure
    let mut batch = Batch::new();
    if let Some((file_path, operations)) = file_operations.into_iter().next() {
        // Note: We're creating a separate Batch for each file
        // This is simplified - a real implementation might merge them
        batch = Batch::with_file(file_path.to_string_lossy().to_string(), operations);
        // For MVP, just handle first file
    }

    Ok(batch)
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
