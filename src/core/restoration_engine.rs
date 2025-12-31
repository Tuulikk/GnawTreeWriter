use crate::core::transaction_log::{ProjectRestorationPlan, Transaction, TransactionLog};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};

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

        // Try hash-based restoration first
        if let Ok(result) = self.restore_by_hash(&transaction) {
            return Ok(result);
        }

        // Fallback to timestamp-based restoration
        self.restore_by_timestamp(&transaction)
    }

    /// Attempt restoration using hash matching
    fn restore_by_hash(&self, transaction: &Transaction) -> Result<PathBuf> {
        let target_hash = transaction
            .after_hash
            .as_ref()
            .ok_or_else(|| anyhow!("Transaction has no after_hash"))?;

        // Try to find backup by after_hash first
        if let Some(backup_file) = self.find_backup_by_content_hash(target_hash)? {
            return self.restore_from_backup(&transaction.file_path, &backup_file.path);
        }

        // Try to find the next transaction that has our after_hash as before_hash
        let next_transaction =
            self.find_next_transaction_for_file(&transaction.file_path, &transaction.timestamp)?;
        if let Some(next_tx) = next_transaction {
            if let Some(next_before_hash) = &next_tx.before_hash {
                if next_before_hash == target_hash {
                    if let Some(backup_file) = self.find_backup_by_content_hash(next_before_hash)? {
                        return self.restore_from_backup(&transaction.file_path, &backup_file.path);
                    }
                }
            }
        }

        Err(anyhow!("Hash-based restoration failed"))
    }

    /// Attempt restoration using timestamp matching
    fn restore_by_timestamp(&self, transaction: &Transaction) -> Result<PathBuf> {
        println!("🔄 Falling back to timestamp-based restoration");

        let backups = self.list_backup_files()?;
        let file_backups: Vec<_> = backups
            .into_iter()
            .filter(|b| b.original_file_path == transaction.file_path)
            .collect();

        if file_backups.is_empty() {
            return Err(anyhow!(
                "No backups found for file: {}",
                transaction.file_path.display()
            ));
        }

        // Find the backup closest to but after the transaction timestamp
        let best_backup = file_backups
            .iter()
            .filter(|b| b.timestamp >= transaction.timestamp)
            .min_by_key(|b| b.timestamp)
            .or_else(|| {
                // If no backup after transaction, use the latest before
                file_backups
                    .iter()
                    .filter(|b| b.timestamp <= transaction.timestamp)
                    .max_by_key(|b| b.timestamp)
            });

        match best_backup {
            Some(backup) => {
                println!("✅ Using timestamp-based backup: {}", backup.path.display());
                self.restore_from_backup(&transaction.file_path, &backup.path)
            }
            None => Err(anyhow!("No suitable backup found for transaction")),
        }
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

        // Use the transaction ID to restore to that state
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

    /// Find the next transaction for a file after a given timestamp
    fn find_next_transaction_for_file(
        &self,
        file_path: &PathBuf,
        after_time: &chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<Transaction>> {
        let file_history = self.transaction_log.get_file_history(file_path)?;

        Ok(file_history
            .into_iter()
            .filter(|t| t.timestamp > *after_time)
            .filter(|t| {
                matches!(
                    t.operation,
                    crate::core::transaction_log::OperationType::Edit
                        | crate::core::transaction_log::OperationType::Insert
                        | crate::core::transaction_log::OperationType::Delete
                )
            })
            .min_by_key(|t| t.timestamp))
    }

    /// Find backup file by content hash (delegates to core::backup)
    fn find_backup_by_content_hash(&self, content_hash: &str) -> Result<Option<BackupFile>> {
        if let Some(b) =
            crate::core::backup::find_backup_by_content_hash(&self.backup_dir, content_hash)?
        {
            return Ok(Some(BackupFile {
                path: b.path,
                timestamp: b.timestamp,
                original_file_path: b.original_file_path,
                content_hash: b.content_hash,
            }));
        }

        Ok(None)
    }

    /// List all backup files in the backup directory (delegates to core::backup)
    fn list_backup_files(&self) -> Result<Vec<BackupFile>> {
        let backups = crate::core::backup::list_backup_files(&self.backup_dir)?;
        Ok(backups
            .into_iter()
            .map(|b| BackupFile {
                path: b.path,
                timestamp: b.timestamp,
                original_file_path: b.original_file_path,
                content_hash: b.content_hash,
            })
            .collect())
    }

    // `parse_backup_file` removed. Use the helpers in `crate::core::backup`
    // (e.g. `crate::core::backup::parse_backup_file`) directly where needed.

    /// Restore file from a backup file (delegates to core::backup)
    fn restore_from_backup(&self, target_path: &Path, backup_path: &Path) -> Result<PathBuf> {
        // Note: core::backup::restore_from_backup expects (backup_path, target_path)
        crate::core::backup::restore_from_backup(backup_path, target_path)
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
