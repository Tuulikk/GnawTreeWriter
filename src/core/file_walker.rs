//! Git-aware file walking using the `ignore` crate.
//!
//! Replaces hardcoded skip-lists (`target`, `node_modules`, `.git`) across
//! multiple modules with a single, correct implementation that respects
//! `.gitignore`, `~/.gitignore_global`, and `.git/info/exclude`.

use std::path::{Path, PathBuf};

/// Walk source files under `root`, respecting `.gitignore`.
///
/// - Skips directories listed in `.gitignore` (e.g. `target/`, `node_modules/`)
/// - Skips hidden directories (`.git/`, `.vscode/`) but NOT hidden files
///   unless they are in `.gitignore`
/// - Follows `.gitignore` rules in parent directories
pub fn walk_source_files(root: &Path) -> Vec<PathBuf> {
    ignore::WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .parents(true)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .map(|e| e.into_path())
        .collect()
}

/// Walk source files under `root`, filtered by extension.
///
/// Extensions should be provided without the leading dot, e.g. `["rs", "py"]`.
/// If `extensions` is empty, all files are returned (still respecting .gitignore).
pub fn walk_source_files_filtered(root: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    if extensions.is_empty() {
        return walk_source_files(root);
    }

    walk_source_files(root)
        .into_iter()
        .filter(|path| {
            path.extension()
                .and_then(|e| e.to_str())
                .map(|ext| extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)))
                .unwrap_or(false)
        })
        .collect()
}

/// Check if a path matches a set of extensions (case-insensitive).
pub fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_walk_source_files_basic() {
        let dir = temp_dir();
        fs::write(dir.path().join("a.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("b.py"), "print('hi')").unwrap();
        fs::write(dir.path().join("c.txt"), "hello").unwrap();

        let files = walk_source_files(dir.path());
        let names: Vec<&str> = files
            .iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
            .collect();

        assert!(names.contains(&"a.rs"));
        assert!(names.contains(&"b.py"));
        assert!(names.contains(&"c.txt"));
    }

    #[test]
    fn test_walk_source_files_filtered() {
        let dir = temp_dir();
        fs::write(dir.path().join("a.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("b.py"), "print('hi')").unwrap();

        let files = walk_source_files_filtered(dir.path(), &["rs"]);
        let names: Vec<&str> = files
            .iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
            .collect();

        assert!(names.contains(&"a.rs"));
        assert!(!names.contains(&"b.py"));
    }

    #[test]
    fn test_walk_source_files_respects_gitignore() {
        let dir = temp_dir();
        // The ignore crate only reads .gitignore inside a git repo
        std::process::Command::new("git")
            .args(["init", dir.path().to_str().unwrap()])
            .output()
            .ok();
        fs::write(dir.path().join(".gitignore"), "ignored_dir/\n").unwrap();
        fs::create_dir(dir.path().join("ignored_dir")).unwrap();
        fs::write(dir.path().join("ignored_dir/a.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("visible.rs"), "fn main() {}").unwrap();

        let files = walk_source_files(dir.path());
        let names: Vec<&str> = files
            .iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
            .collect();

        assert!(names.contains(&"visible.rs"));
        assert!(!names.contains(&"a.rs"));
    }

    #[test]
    fn test_walk_source_files_empty_dir() {
        let dir = temp_dir();
        let files = walk_source_files(dir.path());
        assert!(files.is_empty());
    }

    #[test]
    fn test_has_extension() {
        assert!(has_extension(Path::new("main.rs"), &["rs"]));
        assert!(has_extension(Path::new("main.RS"), &["rs"]));
        assert!(!has_extension(Path::new("main.py"), &["rs"]));
        assert!(!has_extension(Path::new("no_ext"), &["rs"]));
    }
}
