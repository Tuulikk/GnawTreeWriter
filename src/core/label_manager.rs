use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use crate::core::transaction_log::calculate_content_hash;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelInfo {
    pub labels: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LabelStore {
    pub files: HashMap<String, HashMap<String, LabelInfo>>,
}

pub struct LabelManager {
    store_path: PathBuf,
    store: LabelStore,
}

impl LabelManager {
    pub fn load(project_root: &Path) -> Result<Self> {
        let store_path = project_root.join(".gnawtreewriter-labels.json");
        let store = if store_path.exists() {
            let content = fs::read_to_string(&store_path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            LabelStore::default()
        };

        Ok(Self { store_path, store })
    }

    pub fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.store)?;
        fs::write(&self.store_path, content)?;
        Ok(())
    }

    pub fn add_label(&mut self, file_path: &str, node_content: &str, label: &str) -> Result<()> {
        let hash = calculate_content_hash(node_content);
        let file_entry = self.store.files.entry(file_path.to_string()).or_default();
        let label_info = file_entry.entry(hash).or_insert_with(|| LabelInfo {
            labels: Vec::new(),
            metadata: HashMap::new(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        });

        if !label_info.labels.contains(&label.to_string()) {
            label_info.labels.push(label.to_string());
            label_info.last_updated = chrono::Utc::now().to_rfc3339();
        }

        self.save()
    }

    pub fn get_labels(&self, file_path: &str, node_content: &str) -> Vec<String> {
        let hash = calculate_content_hash(node_content);
        self.store.files.get(file_path)
            .and_then(|f| f.get(&hash))
            .map(|l| l.labels.clone())
            .unwrap_or_default()
    }

    pub fn remove_label(&mut self, file_path: &str, node_content: &str, label: &str) -> Result<bool> {
        let hash = calculate_content_hash(node_content);
        if let Some(file_entry) = self.store.files.get_mut(file_path) {
            if let Some(label_info) = file_entry.get_mut(&hash) {
                let initial_len = label_info.labels.len();
                label_info.labels.retain(|l| l != label);
                if label_info.labels.len() < initial_len {
                    self.save()?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
