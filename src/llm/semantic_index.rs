use serde::{Serialize, Deserialize};
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEmbedding {
    pub file_path: String,
    pub node_path: String,
    pub content_preview: String,
    pub vector: Vec<f32>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SemanticIndex {
    pub entries: Vec<NodeEmbedding>,
}

pub struct SemanticIndexManager {
    storage_dir: PathBuf,
}

impl SemanticIndexManager {
    pub fn new(project_root: &Path) -> Self {
        let storage_dir = project_root.join(".gnawtreewriter_ai").join("index");
        if !storage_dir.exists() {
            let _ = fs::create_dir_all(&storage_dir);
        }
        Self { storage_dir }
    }

    pub fn save_index(&self, file_path: &str, entries: Vec<NodeEmbedding>) -> Result<()> {
        let file_hash = crate::core::transaction_log::calculate_content_hash(file_path);
        let save_path = self.storage_dir.join(format!("{}.json", file_hash));
        let data = serde_json::to_string_pretty(&entries)?;
        fs::write(save_path, data)?;
        Ok(())
    }

    pub fn load_project_index(&self) -> Result<SemanticIndex> {
        let mut index = SemanticIndex::default();
        if !self.storage_dir.exists() { return Ok(index); }

        for entry in fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let data = fs::read_to_string(path)?;
                if let Ok(mut entries) = serde_json::from_str::<Vec<NodeEmbedding>>(&data) {
                    index.entries.append(&mut entries);
                }
            }
        }
        Ok(index)
    }
}

impl SemanticIndex {
    pub fn search(&self, query_vector: &[f32], limit: usize) -> Vec<(&NodeEmbedding, f32)> {
        let mut results: Vec<(&NodeEmbedding, f32)> = self.entries.iter()
            .map(|entry| {
                let score = cosine_similarity(query_vector, &entry.vector);
                (entry, score)
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![0.0, 1.0];
        assert!(cosine_similarity(&a, &c).abs() < 1e-6);
    }
}