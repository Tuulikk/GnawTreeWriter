//! Project state tracking for incremental indexing.
//!
//! Maintains a `.gnawtreewriter_state.json` file that records:
//! - Last analyzed git HEAD
//! - File content hashes (SHA256)
//! - Timestamp of last analysis
//!
//! Enables `diff_since` to detect changes without re-analyzing everything.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const STATE_FILE: &str = ".gnawtreewriter_state.json";

/// Project state for incremental tracking.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectState {
    /// ISO timestamp of last analysis
    pub last_analyzed: String,
    /// Git HEAD at last analysis
    pub git_head: String,
    /// File content hashes (relative path → SHA256 hex)
    pub file_hashes: HashMap<String, String>,
}

impl ProjectState {
    /// Load state from project root, or create empty if not found.
    pub fn load(project_root: &Path) -> Self {
        let state_path = project_root.join(STATE_FILE);
        if state_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&state_path) {
                if let Ok(state) = serde_json::from_str(&content) {
                    return state;
                }
            }
        }
        Self::default()
    }

    /// Save state to project root.
    pub fn save(&self, project_root: &Path) -> Result<()> {
        let state_path = project_root.join(STATE_FILE);
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&state_path, json)?;
        Ok(())
    }

    /// Update state with current git HEAD and file hashes.
    pub fn update(project_root: &Path) -> Result<Self> {
        let mut state = Self::load(project_root);

        // Get current git HEAD
        state.git_head = get_git_head(project_root);
        state.last_analyzed = chrono::Utc::now().to_rfc3339();

        // Compute file hashes for source files
        let files = crate::core::file_walker::walk_source_files_filtered(
            project_root,
            &["rs", "py", "js", "ts", "tsx", "jsx", "go", "java",
              "c", "cpp", "h", "hpp", "cs", "php", "rb", "swift", "kt"],
        );

        state.file_hashes.clear();
        for path in &files {
            if let Ok(rel) = path.strip_prefix(project_root) {
                if let Ok(content) = std::fs::read_to_string(path) {
                    let hash = format!("{:x}", md5::compute(content.as_bytes()));
                    state.file_hashes.insert(rel.to_string_lossy().to_string(), hash);
                }
            }
        }

        state.save(project_root)?;
        Ok(state)
    }

    /// Get the stored git HEAD.
    pub fn git_head(&self) -> &str {
        &self.git_head
    }

    /// Get the stored file hash for a path.
    pub fn file_hash(&self, path: &str) -> Option<&str> {
        self.file_hashes.get(path).map(|s| s.as_str())
    }
}

/// Get current git HEAD, or empty string if not a git repo.
fn get_git_head(project_root: &Path) -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project_root)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_state_load_save() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // First load should return default
        let state = ProjectState::load(root);
        assert!(state.git_head.is_empty());
        assert!(state.file_hashes.is_empty());

        // Save and reload
        let mut state = ProjectState::default();
        state.git_head = "abc123".to_string();
        state.last_analyzed = "2026-08-24T13:00:00Z".to_string();
        state.file_hashes.insert("src/main.rs".to_string(), "hash123".to_string());
        state.save(root).unwrap();

        let loaded = ProjectState::load(root);
        assert_eq!(loaded.git_head, "abc123");
        assert_eq!(loaded.file_hash("src/main.rs"), Some("hash123"));
        assert_eq!(loaded.file_hash("src/missing.rs"), None);
    }

    #[test]
    fn test_state_update() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Initialize git repo
        std::process::Command::new("git")
            .args(["init", root.to_str().unwrap()])
            .output()
            .ok();

        // Create a source file
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();

        let state = ProjectState::update(root).unwrap();

        // Should have git HEAD (may be empty if no commits)
        assert!(!state.last_analyzed.is_empty());
        assert!(state.file_hashes.contains_key("src/main.rs"));
    }

    #[test]
    fn test_state_nonexistent_file() {
        let dir = tempdir().unwrap();
        let state = ProjectState::load(dir.path());
        assert_eq!(state.file_hash("nonexistent.rs"), None);
    }
}
