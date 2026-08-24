//! Project statistics and analysis for AI context planning.

use crate::core::file_walker::walk_source_files_filtered;
use crate::core::token_count::estimate_code_tokens;
use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

/// Language stats for the project.
#[derive(Debug, Clone, Serialize)]
pub struct LanguageStats {
    pub name: String,
    pub file_count: usize,
    pub total_tokens: usize,
    pub total_lines: usize,
    pub avg_tokens_per_file: usize,
}

/// File info sorted by token count.
#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
    pub path: String,
    pub tokens: usize,
    pub lines: usize,
    pub compressible: bool,
}

/// Full project statistics.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectStats {
    pub total_files: usize,
    pub total_tokens: usize,
    pub total_lines: usize,
    pub languages: Vec<LanguageStats>,
    pub largest_files: Vec<FileInfo>,
    pub compression_estimate: CompressionEstimate,
    pub context_windows: Vec<ContextWindowInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompressionEstimate {
    pub current_tokens: usize,
    pub compressed_estimate: usize,
    pub reduction_estimate: f64,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextWindowInfo {
    pub name: String,
    pub max_tokens: usize,
    pub fits: bool,
    pub utilization: f64,
}

/// Analyze a project directory and return statistics.
pub fn analyze_project(root: &Path) -> Result<ProjectStats> {
    let files = walk_source_files_filtered(
        root,
        &["rs", "py", "js", "ts", "tsx", "jsx", "go", "java",
          "c", "cpp", "h", "hpp", "cs", "php", "rb", "swift", "kt"],
    );

    let mut lang_map: HashMap<String, (usize, usize, usize)> = HashMap::new(); // (count, tokens, lines)
    let mut all_files: Vec<FileInfo> = Vec::new();
    let mut total_tokens = 0usize;
    let mut total_lines = 0usize;

    for path in &files {
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("other")
            .to_lowercase();

        let tokens = estimate_code_tokens(&content);
        let lines = content.lines().count();
        total_tokens += tokens;
        total_lines += lines;

        let entry = lang_map.entry(ext.clone()).or_insert((0, 0, 0));
        entry.0 += 1;
        entry.1 += tokens;
        entry.2 += lines;

        // Check if file is compressible (has function bodies)
        let compressible = path.extension().and_then(|e| e.to_str()).is_some_and(|ext| {
            matches!(ext, "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "go" | "java" | "c" | "cpp" | "kt" | "swift")
        });

        all_files.push(FileInfo {
            path: rel_path,
            tokens,
            lines,
            compressible,
        });
    }

    // Sort by tokens descending, take top 15
    all_files.sort_by_key(|f| std::cmp::Reverse(f.tokens));
    let largest_files: Vec<FileInfo> = all_files.into_iter().take(15).collect();

    // Build language stats
    let mut languages: Vec<LanguageStats> = lang_map
        .into_iter()
        .map(|(name, (count, tokens, lines))| LanguageStats {
            name,
            file_count: count,
            total_tokens: tokens,
            total_lines: lines,
            avg_tokens_per_file: tokens / count.max(1),
        })
        .collect();
    languages.sort_by(|a, b| b.total_tokens.cmp(&a.total_tokens));

    // Compression estimate (rough: 70% of compressible tokens can be saved)
    let compressible_tokens: usize = languages.iter()
        .filter(|l| matches!(l.name.as_str(), "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "go" | "java" | "c" | "cpp" | "kt" | "swift"))
        .map(|l| l.total_tokens)
        .sum();
    let compressed_estimate = total_tokens - (compressible_tokens as f64 * 0.65) as usize;
    let reduction_pct = if total_tokens > 0 {
        1.0 - (compressed_estimate as f64 / total_tokens as f64)
    } else {
        0.0
    };

    let recommendation = if reduction_pct > 0.5 {
        format!("High compression potential ({:.0}% reduction). Use --compress for ~{} tokens.", reduction_pct * 100.0, compressed_estimate)
    } else if reduction_pct > 0.2 {
        format!("Moderate compression potential ({:.0}% reduction).", reduction_pct * 100.0)
    } else {
        "Low compression potential — files are mostly declarations.".to_string()
    };

    let compression_estimate = CompressionEstimate {
        current_tokens: total_tokens,
        compressed_estimate,
        reduction_estimate: reduction_pct,
        recommendation,
    };

    // Context window analysis
    let context_windows = vec![
        ("GPT-3.5 Turbo (4k)", 4096),
        ("GPT-3.5 Turbo (16k)", 16384),
        ("GPT-4 (8k)", 8192),
        ("GPT-4 (32k)", 32768),
        ("GPT-4 Turbo (128k)", 128000),
        ("Claude 3 Haiku (200k)", 200000),
    ];

    let context_windows: Vec<ContextWindowInfo> = context_windows
        .into_iter()
        .map(|(name, max)| ContextWindowInfo {
            name: name.to_string(),
            max_tokens: max,
            fits: total_tokens <= max,
            utilization: total_tokens as f64 / max as f64,
        })
        .collect();

    Ok(ProjectStats {
        total_files: files.len(),
        total_tokens,
        total_lines,
        languages,
        largest_files,
        compression_estimate,
        context_windows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn setup_project() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();

        std::process::Command::new("git")
            .args(["init", root.to_str().unwrap()])
            .output()
            .ok();

        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/main.rs"),
            "fn main() {\n    let x = 1;\n    println!(\"{}\", x);\n}",
        ).unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\npub fn sub(a: i32, b: i32) -> i32 {\n    a - b\n}",
        ).unwrap();
        fs::write(
            root.join("src/utils.py"),
            "def greet(name):\n    return f\"Hello, {name}!\"\n\ndef farewell(name):\n    return f\"Goodbye, {name}!\"\n",
        ).unwrap();

        (dir, root)
    }

    #[test]
    fn test_analyze_project_basic() {
        let (_dir, root) = setup_project();
        let stats = analyze_project(&root).unwrap();

        assert!(stats.total_files >= 3);
        assert!(stats.total_tokens > 0);
        assert!(stats.total_lines > 0);
    }

    #[test]
    fn test_analyze_project_languages() {
        let (_dir, root) = setup_project();
        let stats = analyze_project(&root).unwrap();

        assert!(stats.languages.iter().any(|l| l.name == "rs"));
        assert!(stats.languages.iter().any(|l| l.name == "py"));
    }

    #[test]
    fn test_analyze_project_largest_files() {
        let (_dir, root) = setup_project();
        let stats = analyze_project(&root).unwrap();

        assert!(!stats.largest_files.is_empty());
        // lib.rs has 2 functions, should have more tokens than main.rs
        let lib_tokens = stats.largest_files.iter().find(|f| f.path.contains("lib.rs")).map(|f| f.tokens).unwrap_or(0);
        let main_tokens = stats.largest_files.iter().find(|f| f.path.contains("main.rs")).map(|f| f.tokens).unwrap_or(0);
        assert!(lib_tokens >= main_tokens);
    }

    #[test]
    fn test_analyze_project_compression_estimate() {
        let (_dir, root) = setup_project();
        let stats = analyze_project(&root).unwrap();

        assert!(stats.compression_estimate.reduction_estimate > 0.0);
        assert!(stats.compression_estimate.compressed_estimate < stats.total_tokens);
        assert!(!stats.compression_estimate.recommendation.is_empty());
    }

    #[test]
    fn test_analyze_project_context_windows() {
        let (_dir, root) = setup_project();
        let stats = analyze_project(&root).unwrap();

        assert!(!stats.context_windows.is_empty());
        // Small project should fit in 4k window
        assert!(stats.context_windows.iter().any(|w| w.fits));
    }

    #[test]
    fn test_analyze_project_empty() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        std::process::Command::new("git")
            .args(["init", root.to_str().unwrap()])
            .output()
            .ok();
        fs::write(root.join(".gitignore"), ".git/\n").unwrap();

        let stats = analyze_project(root).unwrap();
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.total_tokens, 0);
    }
}
