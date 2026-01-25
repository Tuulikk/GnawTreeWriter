use serde::{Serialize, Deserialize};
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlfType {
    Auto,       // Automatically generated from tool use
    Intent,     // Explicitly stated goal
    Assumption, // Agent's assumption about the code
    Risk,       // Identified risk
    Outcome,    // Result of an operation
    Meta,       // Information about the session or project state
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlfEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub transaction_id: Option<String>,
    pub entry_type: AlfType,
    pub message: String,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
}

pub struct AlfManager {
    storage_path: PathBuf,
    entries: Vec<AlfEntry>,
}

impl AlfManager {
    pub fn load(project_root: &Path) -> Result<Self> {
        let ai_dir = project_root.join(".gnawtreewriter_ai");
        if !ai_dir.exists() {
            fs::create_dir_all(&ai_dir)?;
        }
        
        let storage_path = ai_dir.join("alf.json");
        let entries = if storage_path.exists() {
            let data = fs::read_to_string(&storage_path)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(Self { storage_path, entries })
    }

    pub fn log(&mut self, entry_type: AlfType, message: &str, txn_id: Option<String>) -> Result<String> {
        let id = format!("alf_{}", Utc::now().timestamp_micros());
        let entry = AlfEntry {
            id: id.clone(),
            timestamp: Utc::now(),
            transaction_id: txn_id,
            entry_type,
            message: message.to_string(),
            tags: Vec::new(),
            metadata: HashMap::new(),
        };

        self.entries.push(entry);
        self.save()?;
        Ok(id)
    }

    pub fn add_tag(&mut self, alf_id: &str, tag: &str) -> Result<()> {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == alf_id) {
            if !entry.tags.contains(&tag.to_string()) {
                entry.tags.push(tag.to_string());
            }
            self.save()?;
            Ok(())
        } else {
            anyhow::bail!("ALF entry not found: {}", alf_id)
        }
    }

    pub fn update_message(&mut self, alf_id: &str, new_message: &str) -> Result<()> {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == alf_id) {
            entry.message = new_message.to_string();
            self.save()?;
            Ok(())
        } else {
            anyhow::bail!("ALF entry not found: {}", alf_id)
        }
    }

    pub fn find_by_txn(&self, txn_id: &str) -> Option<&AlfEntry> {
        self.entries.iter().find(|e| e.transaction_id.as_deref() == Some(txn_id))
    }

    pub fn list(&self, limit: usize) -> Vec<AlfEntry> {
        self.entries.iter().rev().take(limit).cloned().collect()
    }

    fn save(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(&self.entries)?;
        fs::write(&self.storage_path, data).context("Failed to save ALF journal")?;
        Ok(())
    }
}
