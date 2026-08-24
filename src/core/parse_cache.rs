//! Session-level cache for parsed source files.
//!
//! Avoids re-parsing the same file when multiple tools process it.

use crate::GnawTreeWriter;
use std::collections::HashMap;
use std::sync::Mutex;

/// Cache for parsed files within a session.
pub struct ParseCache {
    cache: Mutex<HashMap<String, (String, crate::parser::TreeNode)>>,
}

impl ParseCache {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Get cached source + tree, or parse and cache.
    pub fn get_or_parse(&self, file_path: &str) -> Option<(String, crate::parser::TreeNode)> {
        // Check cache first
        {
            if let Ok(cache) = self.cache.lock() {
                if let Some((source, tree)) = cache.get(file_path) {
                    return Some((source.clone(), tree.clone()));
                }
            }
        }

        // Parse and cache
        let writer = GnawTreeWriter::new(file_path).ok()?;
        let source = writer.get_source().to_string();
        let tree = writer.analyze().clone();

        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(file_path.to_string(), (source.clone(), tree.clone()));
        }

        Some((source, tree))
    }

    /// Clear the cache.
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }
    }

    /// Get cache size.
    pub fn len(&self) -> usize {
        self.cache.lock().map_or(0, |c| c.len())
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for ParseCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Global session cache.
static SESSION_CACHE: std::sync::LazyLock<ParseCache> = std::sync::LazyLock::new(ParseCache::new);

/// Get the global session cache.
pub fn session_cache() -> &'static ParseCache {
    &SESSION_CACHE
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_cache_parse_and_retrieve() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.rs");
        fs::write(&path, "fn main() {}").unwrap();

        let cache = ParseCache::new();
        let (source, tree) = cache.get_or_parse(path.to_str().unwrap()).unwrap();
        assert!(!source.is_empty());
        assert_eq!(tree.node_type, "source_file");

        // Second call should hit cache (same data)
        let (source2, tree2) = cache.get_or_parse(path.to_str().unwrap()).unwrap();
        assert_eq!(source, source2);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_clear() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.rs");
        fs::write(&path, "fn main() {}").unwrap();

        let cache = ParseCache::new();
        cache.get_or_parse(path.to_str().unwrap());
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_nonexistent_file() {
        let cache = ParseCache::new();
        assert!(cache.get_or_parse("/nonexistent/file.rs").is_none());
    }
}
