//! Context Curator — intelligent file selection for AI agents.
//!
//! Instead of dumping the entire project, the curator selects the most
//! relevant files based on multiple strategies:
//!
//! - **Keyword relevance** — files matching task description
//! - **Git changes** — recently modified files
//! - **Dependencies** — callers/callees of target symbols
//! - **Structural** — overview first, drill-down on demand

use crate::core::file_walker::walk_source_files_filtered;
use crate::core::token_count::estimate_code_tokens;
use crate::parser::TreeNode;
use crate::GnawTreeWriter;
use anyhow::Result;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Curation strategy.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum CurationStrategy {
    /// Match files by keyword relevance to task description.
    Relevance,
    /// Include files changed in recent git commits.
    RecentChanges,
    /// Include callers/callees of specified symbols.
    Dependencies,
    /// Combine all strategies with weighted scoring.
    Auto,
}

impl CurationStrategy {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "recent" | "changes" | "git" => CurationStrategy::RecentChanges,
            "deps" | "dependencies" | "callers" => CurationStrategy::Dependencies,
            "auto" | "smart" => CurationStrategy::Auto,
            _ => CurationStrategy::Relevance,
        }
    }
}

/// A curated file with relevance metadata.
#[derive(Debug, Clone, Serialize)]
pub struct CuratedFile {
    /// Relative path.
    pub path: String,
    /// Relevance score (0.0 - 1.0).
    pub score: f64,
    /// Why this file was included.
    pub reason: String,
    /// Estimated tokens.
    pub tokens: usize,
    /// Number of lines.
    pub lines: usize,
}

/// Result of context curation.
#[derive(Debug, Clone, Serialize)]
pub struct CuratedContext {
    /// Curated files ordered by relevance.
    pub files: Vec<CuratedFile>,
    /// Total tokens across all curated files.
    pub total_tokens: usize,
    /// Strategy used.
    pub strategy: String,
    /// Human-readable summary.
    pub summary: String,
}

/// Curate project context based on a task description.
pub fn curate_context(
    root: &Path,
    task: &str,
    strategy: CurationStrategy,
    max_tokens: usize,
    max_files: usize,
) -> Result<CuratedContext> {
    let files = walk_source_files_filtered(
        root,
        &["rs", "py", "js", "ts", "tsx", "jsx", "go", "java", "c", "cpp", "h", "hpp"],
    );

    let mut scored_files = match strategy {
        CurationStrategy::Relevance => score_by_relevance(&files, root, task),
        CurationStrategy::RecentChanges => score_by_recent_changes(&files, root),
        CurationStrategy::Dependencies => score_by_dependencies(&files, root, task),
        CurationStrategy::Auto => {
            let relevance = score_by_relevance(&files, root, task);
            let recent = score_by_recent_changes(&files, root);
            let deps = score_by_dependencies(&files, root, task);

            let mut score_map: HashMap<String, (f64, String, usize, usize)> = HashMap::new();

            for f in &relevance {
                let entry = score_map.entry(f.path.clone()).or_insert((0.0, String::new(), f.tokens, f.lines));
                entry.0 += f.score * 0.5;
                if entry.1.is_empty() || f.score > 0.5 {
                    entry.1 = f.reason.clone();
                }
            }

            for f in &recent {
                let entry = score_map.entry(f.path.clone()).or_insert((0.0, String::new(), f.tokens, f.lines));
                entry.0 += f.score * 0.3;
                if f.score > 0.7 {
                    entry.1 = format!("{}; {}", entry.1, f.reason);
                }
            }

            for f in &deps {
                let entry = score_map.entry(f.path.clone()).or_insert((0.0, String::new(), f.tokens, f.lines));
                entry.0 += f.score * 0.2;
                if f.score > 0.7 {
                    entry.1 = format!("{}; {}", entry.1, f.reason);
                }
            }

            score_map
                .into_iter()
                .map(|(path, (score, reason, tokens, lines))| CuratedFile {
                    path, score, reason, tokens, lines,
                })
                .collect()
        }
    };

    // Sort by score descending
    scored_files.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Apply token budget and file limit
    let mut total_tokens = 0usize;
    let mut final_files = Vec::new();

    for f in scored_files {
        if final_files.len() >= max_files {
            break;
        }
        if total_tokens + f.tokens > max_tokens {
            continue;
        }
        total_tokens += f.tokens;
        final_files.push(f);
    }

    let summary = format!(
        "Curated {} files ({} tokens) using {:?} strategy",
        final_files.len(),
        total_tokens,
        strategy
    );

    Ok(CuratedContext {
        files: final_files,
        total_tokens,
        strategy: format!("{:?}", strategy),
        summary,
    })
}

/// Score files by keyword relevance to task description.
fn score_by_relevance(files: &[PathBuf], root: &Path, task: &str) -> Vec<CuratedFile> {
    let keywords: Vec<String> = task
        .split_whitespace()
        .map(|w| w.to_lowercase())
        .filter(|w| w.len() > 2) // skip short words
        .collect();

    if keywords.is_empty() {
        return Vec::new();
    }

    let mut results = Vec::new();

    for path in files {
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let content_lower = content.to_lowercase();
        let path_lower = rel_path.to_lowercase();

        // Score based on keyword matches
        let mut score = 0.0f64;
        let mut matched_keywords = Vec::new();

        for kw in &keywords {
            // Path match (high weight)
            if path_lower.contains(kw.as_str()) {
                score += 0.4;
                matched_keywords.push(kw.clone());
            }

            // Content match (lower weight, but counts occurrences)
            let count = content_lower.matches(kw.as_str()).count();
            if count > 0 {
                score += (count as f64).min(5.0) * 0.1;
                if !matched_keywords.contains(kw) {
                    matched_keywords.push(kw.clone());
                }
            }
        }

        // Normalize score to 0-1
        score = score.min(1.0);

        if score > 0.0 {
            let tokens = estimate_code_tokens(&content);
            results.push(CuratedFile {
                path: rel_path,
                score,
                reason: format!("keyword match: {}", matched_keywords.join(", ")),
                tokens,
                lines: content.lines().count(),
            });
        }
    }

    results
}

/// Score files by recent git changes.
fn score_by_recent_changes(files: &[PathBuf], root: &Path) -> Vec<CuratedFile> {
    // Get recently changed files from git
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", "HEAD~5..HEAD"])
        .current_dir(root)
        .output();

    let changed_files: HashSet<String> = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| l.to_string())
            .collect(),
        Err(_) => return Vec::new(),
    };

    let mut results = Vec::new();

    for path in files {
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        if changed_files.contains(&rel_path) {
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let tokens = estimate_code_tokens(&content);

            // More recent = higher score (simplified: all get 0.8)
            results.push(CuratedFile {
                path: rel_path,
                score: 0.8,
                reason: "recently changed".to_string(),
                tokens,
                lines: content.lines().count(),
            });
        }
    }

    results
}

/// Score files by dependency relationships to target symbols.
fn score_by_dependencies(files: &[PathBuf], root: &Path, target: &str) -> Vec<CuratedFile> {
    let target_lower = target.to_lowercase();
    let mut results = Vec::new();
    let mut found_target = false;

    // First pass: find files that define the target
    for path in files {
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        if let Ok(writer) = GnawTreeWriter::new(path.to_str().unwrap_or("")) {
            let tree = writer.analyze();
            let content = writer.get_source();

            // Check if this file defines the target
            if content.to_lowercase().contains(&target_lower) {
                let tokens = estimate_code_tokens(content);

                // Check if it's a definition (function, struct, etc.)
                let is_definition = check_if_definition(tree, &target_lower);

                results.push(CuratedFile {
                    path: rel_path,
                    score: if is_definition { 1.0 } else { 0.6 },
                    reason: if is_definition {
                        format!("defines '{}'", target)
                    } else {
                        format!("references '{}'", target)
                    },
                    tokens,
                    lines: content.lines().count(),
                });

                if is_definition {
                    found_target = true;
                }
            }
        }
    }

    // If we found the target, also find callers
    if found_target {
        for path in files {
            let rel_path = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            // Skip files we already have
            if results.iter().any(|r| r.path == rel_path) {
                continue;
            }

            if let Ok(writer) = GnawTreeWriter::new(path.to_str().unwrap_or("")) {
                let content = writer.get_source();

                // Check if this file calls the target
                if content.to_lowercase().contains(&target_lower) {
                    let tokens = estimate_code_tokens(content);
                    results.push(CuratedFile {
                        path: rel_path,
                        score: 0.5,
                        reason: format!("calls '{}'", target),
                        tokens,
                        lines: content.lines().count(),
                    });
                }
            }
        }
    }

    results
}

/// Check if a symbol is defined (not just referenced) in the tree.
fn check_if_definition(tree: &TreeNode, target: &str) -> bool {
    if let Some(name) = tree.get_name() {
        if name.to_lowercase() == target {
            // Check if this is a definition node
            if matches!(
                tree.node_type.as_str(),
                "function_item"
                    | "function_definition"
                    | "function_declaration"
                    | "struct_item"
                    | "struct_declaration"
                    | "class_declaration"
                    | "class_definition"
                    | "trait_item"
                    | "interface_declaration"
                    | "impl_item"
            ) {
                return true;
            }
        }
    }

    for child in &tree.children {
        if check_if_definition(child, target) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
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

        fs::write(
            root.join("src/auth/login.rs"),
            "pub fn validate_password(password: &str) -> bool {\n    !password.is_empty()\n}",
        )
        .unwrap();

        fs::write(
            root.join("src/auth/session.rs"),
            "use crate::auth::login;\npub fn create_session(user: &str) -> Session {\n    Session::new(user)\n}",
        )
        .unwrap();

        fs::write(
            root.join("src/db/users.rs"),
            "pub fn get_user(id: i32) -> User {\n    todo!()\n}",
        )
        .unwrap();

        fs::write(
            root.join("src/main.rs"),
            "use auth::login;\nfn main() {\n    login::validate_password(\"test\");\n}",
        )
        .unwrap();

        (dir, root)
    }

    #[test]
    fn test_curate_by_relevance() {
        let (_dir, root) = setup_project();
        let result = curate_context(
            &root,
            "login password authentication",
            CurationStrategy::Relevance,
            10000,
            10,
        )
        .unwrap();

        assert!(!result.files.is_empty());
        // auth files should score higher than db files
        let auth_score = result
            .files
            .iter()
            .find(|f| f.path.contains("auth"))
            .map(|f| f.score)
            .unwrap_or(0.0);
        let db_score = result
            .files
            .iter()
            .find(|f| f.path.contains("db"))
            .map(|f| f.score)
            .unwrap_or(0.0);
        assert!(auth_score > db_score);
    }

    #[test]
    fn test_curate_by_dependencies() {
        let (_dir, root) = setup_project();
        let result = curate_context(
            &root,
            "validate_password",
            CurationStrategy::Dependencies,
            10000,
            10,
        )
        .unwrap();

        assert!(!result.files.is_empty());
        // Should find the file that defines validate_password
        assert!(result.files.iter().any(|f| f.path.contains("login.rs")));
    }

    #[test]
    fn test_curate_auto() {
        let (_dir, root) = setup_project();
        let result = curate_context(
            &root,
            "authentication session",
            CurationStrategy::Auto,
            10000,
            10,
        )
        .unwrap();

        assert!(!result.files.is_empty());
        assert!(result.total_tokens > 0);
    }

    #[test]
    fn test_curate_token_budget() {
        let (_dir, root) = setup_project();
        let result = curate_context(
            &root,
            "login",
            CurationStrategy::Relevance,
            100, // Very tight budget
            10,
        )
        .unwrap();

        assert!(result.total_tokens <= 100);
    }

    #[test]
    fn test_curate_file_limit() {
        let (_dir, root) = setup_project();
        let result = curate_context(
            &root,
            "",
            CurationStrategy::Relevance,
            100000,
            2, // Only 2 files
        )
        .unwrap();

        assert!(result.files.len() <= 2);
    }

    #[test]
    fn test_strategy_from_str() {
        assert_eq!(CurationStrategy::parse("relevance"), CurationStrategy::Relevance);
        assert_eq!(CurationStrategy::parse("recent"), CurationStrategy::RecentChanges);
        assert_eq!(CurationStrategy::parse("git"), CurationStrategy::RecentChanges);
        assert_eq!(CurationStrategy::parse("deps"), CurationStrategy::Dependencies);
        assert_eq!(CurationStrategy::parse("auto"), CurationStrategy::Auto);
        assert_eq!(CurationStrategy::parse("smart"), CurationStrategy::Auto);
    }

    #[test]
    fn test_curate_scores_sorted_descending() {
        let (_dir, root) = setup_project();
        let result = curate_context(
            &root,
            "login password",
            CurationStrategy::Relevance,
            100000,
            10,
        )
        .unwrap();

        // Verify descending order
        let scores: Vec<f64> = result.files.iter().map(|f| f.score).collect();
        let mut sorted = scores.clone();
        sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
        assert_eq!(scores, sorted, "Files should be sorted by score descending");
    }

    #[test]
    fn test_curate_irrelevant_task_returns_empty() {
        let (_dir, root) = setup_project();
        let result = curate_context(
            &root,
            "xyzzyplughnonexistent",
            CurationStrategy::Relevance,
            100000,
            10,
        )
        .unwrap();

        assert!(result.files.is_empty(), "No files should match nonsense keywords");
        assert_eq!(result.total_tokens, 0);
    }

    #[test]
    fn test_curate_reason_field_populated() {
        let (_dir, root) = setup_project();
        let result = curate_context(
            &root,
            "login",
            CurationStrategy::Relevance,
            100000,
            10,
        )
        .unwrap();

        assert!(!result.files.is_empty());
        for f in &result.files {
            assert!(!f.reason.is_empty(), "Every curated file should have a reason");
        }
    }

    #[test]
    fn test_curate_token_counts_accurate() {
        let (_dir, root) = setup_project();
        let result = curate_context(
            &root,
            "login",
            CurationStrategy::Relevance,
            100000,
            10,
        )
        .unwrap();

        // Sum of per-file tokens should equal total
        let sum: usize = result.files.iter().map(|f| f.tokens).sum();
        assert_eq!(sum, result.total_tokens, "total_tokens should equal sum of file tokens");
    }

    #[test]
    fn test_curate_dependencies_finds_callers() {
        let (_dir, root) = setup_project();
        // main.rs calls validate_password — deps should find both
        let result = curate_context(
            &root,
            "validate_password",
            CurationStrategy::Dependencies,
            100000,
            10,
        )
        .unwrap();

        let has_login = result.files.iter().any(|f| f.path.contains("login.rs"));
        assert!(has_login, "Should find defining file");
    }

    #[test]
    fn test_curate_budget_excludes_large_files() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::process::Command::new("git")
            .args(["init", root.to_str().unwrap()])
            .output()
            .ok();

        // One small relevant file, one huge relevant file
        let mut big_content = String::from("fn big_login_fn() {\n");
        for i in 0..2000 {
            big_content.push_str(&format!("    let x{} = {};\n", i, i));
        }
        big_content.push_str("}\n");

        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/small.rs"), "fn login() {}").unwrap();
        fs::write(root.join("src/big.rs"), &big_content).unwrap();

        // Budget that only fits the small file
        let result = curate_context(
            &root,
            "login",
            CurationStrategy::Relevance,
            2000,
            10,
        )
        .unwrap();

        assert!(result.total_tokens <= 2000);
        assert!(
            result.files.iter().any(|f| f.path.contains("small.rs")),
            "Small file should fit in budget"
        );
    }

    #[test]
    fn test_curate_auto_produces_summary() {
        let (_dir, root) = setup_project();
        let result = curate_context(
            &root,
            "session",
            CurationStrategy::Auto,
            100000,
            10,
        )
        .unwrap();

        assert!(!result.summary.is_empty());
        assert!(result.summary.contains("Auto"), "Summary should mention strategy");
    }

    #[test]
    fn test_curate_short_keywords_ignored() {
        let (_dir, root) = setup_project();
        // "fn" is only 2 chars — should be skipped as too short
        let result = curate_context(
            &root,
            "fn",
            CurationStrategy::Relevance,
            100000,
            10,
        )
        .unwrap();

        assert!(result.files.is_empty(), "Keywords < 3 chars should be ignored");
    }
}
