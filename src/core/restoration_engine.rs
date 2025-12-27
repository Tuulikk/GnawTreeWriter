use crate::core::transaction_log::{ProjectRestorationPlan, Transaction, TransactionLog};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Engine for executing file and project restoration operations
pub struct RestorationEngine {
    project_root: PathBuf,
    backup_dir: PathBuf,
    transaction_log: TransactionLog,
}

#[derive(Debug, Clone)]
pub struct RestorationResult {
    pub restored_files: Vec<PathBuf>,
    pub failed_files: Vec<(PathBuf, String)>,
    pub total_files: usize,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct BackupFile {
    pub path: PathBuf,
    pub timestamp: DateTime<Utc>,
    pub original_file_path: PathBuf,
    pub content_hash: Option<String>,
}

impl RestorationEngine {
    /// Create a new restoration engine
    pub fn new<P: AsRef<Path>>(project_root: P) -> Result<Self> {
        let project_root = project_root.as_ref().to_path_buf();
        let backup_dir = project_root.join(".gnawtreewriter_backups");
        let transaction_log = TransactionLog::load(&project_root)?;

        if !backup_dir.exists() {
            fs::create_dir_all(&backup_dir).context("Failed to create backup directory")?;
        }

        Ok(Self {
            project_root,
            backup_dir,
            transaction_log,
        })
    }

    /// Execute a project restoration plan
    pub fn execute_project_restoration(
        &self,
        plan: &ProjectRestorationPlan,
    ) -> Result<RestorationResult> {
        let mut restored_files = Vec::new();
        let mut failed_files = Vec::new();

        println!(
            "🔄 Starting project restoration to {}",
            plan.restore_to_timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );

        for file_plan in &plan.affected_files {
            match self.restore_file_to_transaction(&file_plan.target_transaction_id) {
                Ok(restored_path) => {
                    restored_files.push(restored_path.clone());
                    println!("✅ Restored: {}", restored_path.display());
                }
                Err(e) => {
                    let error_msg = format!("Failed to restore: {}", e);
                    failed_files.push((file_plan.file_path.clone(), error_msg.clone()));
                    println!(
                        "❌ Failed to restore {}: {}",
                        file_plan.file_path.display(),
                        error_msg
                    );
                }
            }
        }

        let success = failed_files.is_empty();

        if success {
            println!(
                "🎉 Project restoration completed successfully! Restored {} files",
                restored_files.len()
            );
        } else {
            println!(
                "⚠️  Project restoration completed with {} errors out of {} files",
                failed_files.len(),
                plan.affected_files.len()
            );
        }

        Ok(RestorationResult {
            restored_files,
            failed_files,
            total_files: plan.affected_files.len(),
            success,
        })
    }

    /// Restore a single file to the state after a specific transaction
    pub fn restore_file_to_transaction(&self, transaction_id: &str) -> Result<PathBuf> {
        // Find the transaction
        let transaction = self
            .transaction_log
            .find_transaction(transaction_id)?
            .ok_or_else(|| anyhow!("Transaction not found: {}", transaction_id))?;

        // Get the target hash (after_hash for the transaction)
        let target_hash = transaction
            .after_hash
            .as_ref()
            .ok_or_else(|| anyhow!("Transaction has no after_hash: {}", transaction_id))?;

        // Find backup file with matching content hash
        let backup_file = self
            .find_backup_by_content_hash(target_hash)?
            .ok_or_else(|| anyhow!("Backup not found for hash: {}", target_hash))?;

        // Restore from backup
        self.restore_from_backup(&transaction.file_path, &backup_file.path)
    }

    /// Restore multiple files to their state before a specific timestamp
    pub fn restore_files_before_timestamp(
        &self,
        files: &[PathBuf],
        before_time: DateTime<Utc>,
    ) -> Result<RestorationResult> {
        let mut restored_files = Vec::new();
        let mut failed_files = Vec::new();

        println!(
            "🔄 Restoring {} files to state before {}",
            files.len(),
            before_time.format("%Y-%m-%d %H:%M:%S UTC")
        );

        for file_path in files {
            match self.restore_file_before_timestamp(file_path, before_time) {
                Ok(restored_path) => {
                    restored_files.push(restored_path.clone());
                    println!("✅ Restored: {}", restored_path.display());
                }
                Err(e) => {
                    let error_msg = format!("Failed to restore: {}", e);
                    failed_files.push((file_path.clone(), error_msg.clone()));
                    println!(
                        "❌ Failed to restore {}: {}",
                        file_path.display(),
                        error_msg
                    );
                }
            }
        }

        let success = failed_files.is_empty();

        if success {
            println!(
                "🎉 Files restoration completed successfully! Restored {} files",
                restored_files.len()
            );
        } else {
            println!(
                "⚠️  Files restoration completed with {} errors out of {} files",
                failed_files.len(),
                files.len()
            );
        }

        Ok(RestorationResult {
            restored_files,
            failed_files,
            total_files: files.len(),
            success,
        })
    }

    /// Restore a single file to its state before a specific timestamp
    pub fn restore_file_before_timestamp(
        &self,
        file_path: &PathBuf,
        before_time: DateTime<Utc>,
    ) -> Result<PathBuf> {
        // Find last transaction for this file before the timestamp
        let transaction = self
            .transaction_log
            .get_last_transaction_before(file_path, before_time)?
            .ok_or_else(|| {
                anyhow!(
                    "No transaction found for file {} before {}",
                    file_path.display(),
                    before_time.format("%Y-%m-%d %H:%M:%S")
                )
            })?;

        // Use the after_hash of that transaction as our target state
        self.restore_file_to_transaction(&transaction.id)
    }

    /// Restore all files affected in a specific session
    pub fn restore_session(&self, session_id: &str) -> Result<RestorationResult> {
        let session_files = self.transaction_log.get_session_files(session_id)?;

        if session_files.is_empty() {
            return Ok(RestorationResult {
                restored_files: Vec::new(),
                failed_files: Vec::new(),
                total_files: 0,
                success: true,
            });
        }

        println!("🔄 Restoring session: {}", session_id);
        println!("Files to restore: {}", session_files.len());

        // For session restoration, we want to find the state of each file
        // just before the session started
        let session_transactions = self.get_session_transactions(session_id)?;
        let session_start_time = session_transactions
            .iter()
            .map(|t| t.timestamp)
            .min()
            .ok_or_else(|| anyhow!("Session has no transactions"))?;

        self.restore_files_before_timestamp(&session_files, session_start_time)
    }

    /// Get all transactions for a specific session
    fn get_session_transactions(&self, session_id: &str) -> Result<Vec<Transaction>> {
        let full_history = self.transaction_log.get_full_history()?;
        Ok(full_history
            .into_iter()
            .filter(|t| t.session_id == session_id)
            .collect())
    }

    /// Find backup file by content hash
    fn find_backup_by_content_hash(&self, content_hash: &str) -> Result<Option<BackupFile>> {
        let backups = self.list_backup_files()?;

        for backup in backups {
            if let Some(hash) = &backup.content_hash {
                if hash == content_hash {
                    return Ok(Some(backup));
                }
            }
        }

        Ok(None)
    }

    /// List all backup files in the backup directory
    fn list_backup_files(&self) -> Result<Vec<BackupFile>> {
        let mut backups = Vec::new();

        if !self.backup_dir.exists() {
            return Ok(backups);
        }

        let entries = fs::read_dir(&self.backup_dir).context("Failed to read backup directory")?;

        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(backup_file) = self.parse_backup_file(&path) {
                    backups.push(backup_file);
                }
            }
        }

        // Sort by timestamp (newest first)
        backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(backups)
    }

    /// Parse a backup file and extract metadata
    fn parse_backup_file(&self, backup_path: &Path) -> Result<BackupFile> {
        let content = fs::read_to_string(backup_path).context(format!(
            "Failed to read backup file: {}",
            backup_path.display()
        ))?;

        let json: Value = serde_json::from_str(&content).context(format!(
            "Failed to parse backup JSON: {}",
            backup_path.display()
        ))?;

        // Extract file path from backup
        let original_file_path = json["file_path"]
            .as_str()
            .ok_or_else(|| anyhow!("Backup file missing file_path"))?;

        // Extract timestamp
        let timestamp_str = json["timestamp"]
            .as_str()
            .ok_or_else(|| anyhow!("Backup file missing timestamp"))?;
        let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
            .context("Failed to parse timestamp")?
            .with_timezone(&Utc);

        // Calculate content hash from source code
        let source_code = json["source_code"]
            .as_str()
            .ok_or_else(|| anyhow!("Backup file missing source_code"))?;
        let content_hash = Some(crate::core::calculate_content_hash(source_code));

        Ok(BackupFile {
            path: backup_path.to_path_buf(),
            timestamp,
            original_file_path: PathBuf::from(original_file_path),
            content_hash,
        })
    }

    /// Restore file from a backup file
    fn restore_from_backup(&self, target_path: &Path, backup_path: &Path) -> Result<PathBuf> {
        // Read backup file
        let backup_content = fs::read_to_string(backup_path).context(format!(
            "Failed to read backup file: {}",
            backup_path.display()
        ))?;

        let json: Value =
            serde_json::from_str(&backup_content).context("Failed to parse backup JSON")?;

        // Extract source code
        let source_code = json["source_code"]
            .as_str()
            .ok_or_else(|| anyhow!("Backup file missing source_code"))?;

        // Write to target file
        fs::write(target_path, source_code).context(format!(
            "Failed to write restored file: {}",
            target_path.display()
        ))?;

        Ok(target_path.to_path_buf())
    }

    /// Get restoration statistics
    pub fn get_restoration_stats(&self) -> Result<RestorationStats> {
        let backups = self.list_backup_files()?;
        let history = self.transaction_log.get_full_history()?;

        // Group backups by file
        let mut files_with_backups = std::collections::HashSet::new();
        for backup in &backups {
            files_with_backups.insert(&backup.original_file_path);
        }

        Ok(RestorationStats {
            total_backup_files: backups.len(),
            total_transactions: history.len(),
            files_with_backups: files_with_backups.len(),
            oldest_backup: backups.last().map(|b| b.timestamp),
            newest_backup: backups.first().map(|b| b.timestamp),
        })
    }
}

#[derive(Debug, Clone)]
pub struct RestorationStats {
    pub total_backup_files: usize,
    pub total_transactions: usize,
    pub files_with_backups: usize,
    pub oldest_backup: Option<DateTime<Utc>>,
    pub newest_backup: Option<DateTime<Utc>>,
}

impl RestorationResult {
    pub fn print_summary(&self) {
        if self.success {
            println!("✅ Restoration completed successfully!");
            println!("   Restored files: {}", self.restored_files.len());
        } else {
            println!("⚠️  Restoration completed with errors:");
            println!("   Successful: {}", self.restored_files.len());
            println!("   Failed: {}", self.failed_files.len());

            if !self.failed_files.is_empty() {
                println!("\nFailed files:");
                for (file, error) in &self.failed_files {
                    println!("   ❌ {}: {}", file.display(), error);
                }
            }
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_files == 0 {
            1.0
        } else {
            self.restored_files.len() as f64 / self.total_files as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_restoration_engine() {
        let temp_dir = tempdir().unwrap();
        let engine = RestorationEngine::new(temp_dir.path()).unwrap();

        assert!(engine.backup_dir.exists());
        assert_eq!(engine.project_root, temp_dir.path());
    }

    #[test]
    fn test_restoration_result_success_rate() {
        let result = RestorationResult {
            restored_files: vec![PathBuf::from("file1.py"), PathBuf::from("file2.py")],
            failed_files: vec![(PathBuf::from("file3.py"), "error".to_string())],
            total_files: 3,
            success: false,
        };

        assert_eq!(result.success_rate(), 2.0 / 3.0);
    }
}
