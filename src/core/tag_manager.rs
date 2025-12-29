use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Tag file structure serialized to TOML:
///
/// [files."<file_path>"]
/// tags."tag_name" = "1.2.0"
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TagsFile {
    pub files: HashMap<String, FileTags>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct FileTags {
    pub tags: HashMap<String, String>,
}

/// A simple manager for named references (tags) that map (file -> tag name -> node path).
///
/// Tags are stored in a TOML file at the project root:
/// `.gnawtreewriter-tags.toml`
#[derive(Debug, Clone)]
pub struct TagManager {
    tag_file: PathBuf,
    tags: TagsFile,
}

impl TagManager {
    /// Path to the default tags file inside a project root
    pub fn default_tags_path<P: AsRef<Path>>(project_root: P) -> PathBuf {
        project_root.as_ref().join(".gnawtreewriter-tags.toml")
    }

    /// Load TagManager from a project root. If the tags file does not exist,
    /// an empty TagManager is returned (no tags).
    pub fn load<P: AsRef<Path>>(project_root: P) -> Result<Self> {
        let tag_file = Self::default_tags_path(project_root);
        if !tag_file.exists() {
            return Ok(Self {
                tag_file,
                tags: TagsFile::default(),
            });
        }

        let content = fs::read_to_string(&tag_file)
            .with_context(|| format!("Failed to read tags file: {}", tag_file.display()))?;

        let tags: TagsFile =
            toml::from_str(&content).context("Failed to parse tags file as TOML")?;

        Ok(Self { tag_file, tags })
    }

    /// Persist current tags to disk.
    pub fn save(&self) -> Result<()> {
        let toml =
            toml::to_string_pretty(&self.tags).context("Failed to serialize tags to TOML")?;
        fs::write(&self.tag_file, toml)
            .with_context(|| format!("Failed to write tags to {}", self.tag_file.display()))?;
        Ok(())
    }

    /// Add a tag for `file_path` mapping `name` -> `node_path`.
    ///
    /// If a tag with the same name already exists for the file and `force` is false,
    /// an error is returned. If `force` is true the tag will be overwritten.
    pub fn add_tag(
        &mut self,
        file_path: &str,
        name: &str,
        node_path: &str,
        force: bool,
    ) -> Result<()> {
        let file_entry = self
            .tags
            .files
            .entry(file_path.to_string())
            .or_insert_with(FileTags::default);

        if file_entry.tags.contains_key(name) && !force {
            anyhow::bail!(
                "Tag '{}' already exists for file '{}'. Use --force to overwrite.",
                name,
                file_path
            );
        }

        file_entry
            .tags
            .insert(name.to_string(), node_path.to_string());

        self.save()?;
        Ok(())
    }

    /// Remove a tag for `file_path`. Returns `Ok(true)` if the tag existed and was removed.
    /// Returns `Ok(false)` if the tag didn't exist.
    pub fn remove_tag(&mut self, file_path: &str, name: &str) -> Result<bool> {
        if let Some(file_entry) = self.tags.files.get_mut(file_path) {
            if file_entry.tags.remove(name).is_some() {
                // if the file has no more tags, remove the file entry
                if file_entry.tags.is_empty() {
                    self.tags.files.remove(file_path);
                }
                self.save()?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Get the node path for a given tag name in a file, if present.
    pub fn get_path(&self, file_path: &str, name: &str) -> Option<String> {
        self.tags
            .files
            .get(file_path)
            .and_then(|ft| ft.tags.get(name).cloned())
    }

    /// List all tags for a given file as a vector of (name, node_path) tuples.
    /// The result is sorted by tag name for deterministic output.
    pub fn list_tags(&self, file_path: &str) -> Vec<(String, String)> {
        if let Some(ft) = self.tags.files.get(file_path) {
            let mut v: Vec<(String, String)> = ft
                .tags
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            v.sort_by(|a, b| a.0.cmp(&b.0));
            v
        } else {
            Vec::new()
        }
    }

    /// Check whether a tag exists for a file.
    pub fn tag_exists(&self, file_path: &str, name: &str) -> bool {
        self.tags
            .files
            .get(file_path)
            .map(|ft| ft.tags.contains_key(name))
            .unwrap_or(false)
    }

    /// Return the path to the tags file tracked by this manager.
    pub fn tags_file_path(&self) -> &Path {
        &self.tag_file
    }

    /// Get all tags across all files (for debugging or bulk operations).
    pub fn all_tags(&self) -> HashMap<String, HashMap<String, String>> {
        self.tags
            .files
            .iter()
            .map(|(f, ft)| (f.clone(), ft.tags.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn add_list_remove_tag() -> Result<()> {
        let tmp = tempdir()?;
        let project_root = tmp.path();

        // Start with empty manager
        let mut mgr = TagManager::load(project_root)?;
        assert!(mgr.list_tags("file.rs").is_empty());

        // Add tag
        mgr.add_tag("file.rs", "my_function", "1.2.0", false)?;
        assert_eq!(
            mgr.get_path("file.rs", "my_function"),
            Some("1.2.0".to_string())
        );

        // List tags
        let list = mgr.list_tags("file.rs");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0], ("my_function".to_string(), "1.2.0".to_string()));

        // Remove tag
        assert!(mgr.remove_tag("file.rs", "my_function")?);
        assert!(mgr.list_tags("file.rs").is_empty());

        Ok(())
    }

    #[test]
    fn persistence_round_trip() -> Result<()> {
        let tmp = tempdir()?;
        let project_root = tmp.path();

        let mut mgr = TagManager::load(project_root)?;
        mgr.add_tag("foo.txt", "root", "0", false)?;
        mgr.add_tag("foo.txt", "helper", "0.1", false)?;

        // Load again from disk
        let mgr2 = TagManager::load(project_root)?;
        let tags = mgr2.list_tags("foo.txt");
        assert_eq!(tags.len(), 2);
        assert_eq!(mgr2.get_path("foo.txt", "root"), Some("0".to_string()));

        Ok(())
    }

    #[test]
    fn add_tag_conflict() -> Result<()> {
        let tmp = tempdir()?;
        let project_root = tmp.path();

        let mut mgr = TagManager::load(project_root)?;
        mgr.add_tag("a.rs", "t", "0", false)?;
        let res = mgr.add_tag("a.rs", "t", "1", false);
        assert!(res.is_err());
        // Force overwrite should succeed
        mgr.add_tag("a.rs", "t", "1", true)?;
        assert_eq!(mgr.get_path("a.rs", "t"), Some("1".to_string()));

        Ok(())
    }
}
