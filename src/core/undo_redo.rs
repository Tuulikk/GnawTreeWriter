use crate::core::transaction_log::{OperationType, Transaction, TransactionLog};
use anyhow::{anyhow, Context, Result};

use std::fs;
use std::path::{Path, PathBuf};

/// Undo/Redo manager for transaction history
pub struct UndoRedoManager {
    transaction_log: TransactionLog,
    undo_stack: Vec<String>, // Transaction IDs
    redo_stack: Vec<String>, // Transaction IDs
    backup_dir: PathBuf,
}

impl UndoRedoManager {
    /// Create a new undo/redo manager
    pub fn new<P: AsRef<Path>>(project_root: P) -> Result<Self> {
        let backup_dir = project_root.as_ref().join(".gnawtreewriter_backups");

        // Ensure backup directory exists
        if !backup_dir.exists() {
            fs::create_dir_all(&backup_dir).context("Failed to create backup directory")?;
        }

        let transaction_log = TransactionLog::load(&project_root)?;

        Ok(Self {
            transaction_log,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            backup_dir,
        })
    }

    /// Record a new operation (clears redo stack)
    pub fn record_operation(&mut self, transaction_id: String) {
        self.undo_stack.push(transaction_id);
        self.redo_stack.clear(); // Clear redo stack when new operation is performed
    }

    /// Undo the last N operations
    pub fn undo(&mut self, steps: usize) -> Result<Vec<UndoRedoResult>> {
        let mut results = Vec::new();
        let steps_to_undo = std::cmp::min(steps, self.undo_stack.len());

        for _ in 0..steps_to_undo {
            if let Some(transaction_id) = self.undo_stack.pop() {
                let result = self.undo_single_transaction(&transaction_id)?;
                self.redo_stack.push(transaction_id);
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Redo the last N operations
    pub fn redo(&mut self, steps: usize) -> Result<Vec<UndoRedoResult>> {
        let mut results = Vec::new();
        let steps_to_redo = std::cmp::min(steps, self.redo_stack.len());

        for _ in 0..steps_to_redo {
            if let Some(transaction_id) = self.redo_stack.pop() {
                let result = self.redo_single_transaction(&transaction_id)?;
                self.undo_stack.push(transaction_id);
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Get the current undo/redo state
    pub fn get_state(&self) -> UndoRedoState {
        UndoRedoState {
            undo_available: self.undo_stack.len(),
            redo_available: self.redo_stack.len(),
            last_undo: self.undo_stack.last().cloned(),
            last_redo: self.redo_stack.last().cloned(),
        }
    }

    /// Get history of operations that can be undone
    pub fn get_undo_history(&self, limit: Option<usize>) -> Result<Vec<Transaction>> {
        let limit = limit.unwrap_or(self.undo_stack.len());
        let mut history = Vec::new();

        for transaction_id in self.undo_stack.iter().rev().take(limit) {
            if let Some(transaction) = self.transaction_log.find_transaction(transaction_id)? {
                history.push(transaction);
            }
        }

        Ok(history)
    }

    /// Get history of operations that can be redone
    pub fn get_redo_history(&self, limit: Option<usize>) -> Result<Vec<Transaction>> {
        let limit = limit.unwrap_or(self.redo_stack.len());
        let mut history = Vec::new();

        for transaction_id in self.redo_stack.iter().rev().take(limit) {
            if let Some(transaction) = self.transaction_log.find_transaction(transaction_id)? {
                history.push(transaction);
            }
        }

        Ok(history)
    }

    /// Undo a single transaction
    fn undo_single_transaction(&self, transaction_id: &str) -> Result<UndoRedoResult> {
        let transaction = self
            .transaction_log
            .find_transaction(transaction_id)?
            .ok_or_else(|| anyhow!("Transaction not found: {}", transaction_id))?;

        match transaction.operation {
            OperationType::Edit => self.undo_edit(&transaction),
            OperationType::Insert => self.undo_insert(&transaction),
            OperationType::Delete => self.undo_delete(&transaction),
            OperationType::AddProperty => self.undo_add_property(&transaction),
            OperationType::AddComponent => self.undo_add_component(&transaction),
            OperationType::Move => self.undo_move(&transaction),
            OperationType::Restore => self.undo_restore(&transaction),
            OperationType::SessionStart | OperationType::SessionEnd => Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: "Session marker - no action needed".to_string(),
            }),
        }
    }

    /// Redo a single transaction
    fn redo_single_transaction(&self, transaction_id: &str) -> Result<UndoRedoResult> {
        let transaction = self
            .transaction_log
            .find_transaction(transaction_id)?
            .ok_or_else(|| anyhow!("Transaction not found: {}", transaction_id))?;

        match transaction.operation {
            OperationType::Edit => self.redo_edit(&transaction),
            OperationType::Insert => self.redo_insert(&transaction),
            OperationType::Delete => self.redo_delete(&transaction),
            OperationType::AddProperty => self.redo_add_property(&transaction),
            OperationType::AddComponent => self.redo_add_component(&transaction),
            OperationType::Move => self.redo_move(&transaction),
            OperationType::Restore => self.redo_restore(&transaction),
            OperationType::SessionStart | OperationType::SessionEnd => Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: "Session marker - no action needed".to_string(),
            }),
        }
    }

    /// Undo an edit operation
    fn undo_edit(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        let backup_path = self.find_backup_by_hash(&transaction.before_hash)?;

        if let Some(backup_path) = backup_path {
            self.restore_from_backup(&transaction.file_path, &backup_path)?;

            Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!("Reverted edit: {}", transaction.description),
            })
        } else {
            Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: false,
                message: "Backup not found for undo operation".to_string(),
            })
        }
    }

    /// Redo an edit operation
    fn redo_edit(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        let backup_path = self.find_backup_by_hash(&transaction.after_hash)?;

        if let Some(backup_path) = backup_path {
            self.restore_from_backup(&transaction.file_path, &backup_path)?;

            Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!("Re-applied edit: {}", transaction.description),
            })
        } else {
            Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: false,
                message: "Backup not found for redo operation".to_string(),
            })
        }
    }

    /// Placeholder implementations for other operation types
    /// These would need to be implemented based on the specific backup format
    /// and restoration logic for each operation type

    fn undo_insert(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // TODO: Implement insert undo logic
        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Insert undo not yet implemented".to_string(),
        })
    }

    fn redo_insert(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // TODO: Implement insert redo logic
        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Insert redo not yet implemented".to_string(),
        })
    }

    fn undo_delete(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // TODO: Implement delete undo logic
        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Delete undo not yet implemented".to_string(),
        })
    }

    fn redo_delete(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // TODO: Implement delete redo logic
        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Delete redo not yet implemented".to_string(),
        })
    }

    fn undo_add_property(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // TODO: Implement add property undo logic
        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Add property undo not yet implemented".to_string(),
        })
    }

    fn redo_add_property(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // TODO: Implement add property redo logic
        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Add property redo not yet implemented".to_string(),
        })
    }

    fn undo_add_component(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // TODO: Implement add component undo logic
        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Add component undo not yet implemented".to_string(),
        })
    }

    fn redo_add_component(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // TODO: Implement add component redo logic
        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Add component redo not yet implemented".to_string(),
        })
    }

    fn undo_move(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // TODO: Implement move undo logic
        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Move undo not yet implemented".to_string(),
        })
    }

    fn redo_move(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // TODO: Implement move redo logic
        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Move redo not yet implemented".to_string(),
        })
    }

    fn undo_restore(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // TODO: Implement restore undo logic
        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Restore undo not yet implemented".to_string(),
        })
    }

    fn redo_restore(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // TODO: Implement restore redo logic
        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Restore redo not yet implemented".to_string(),
        })
    }

    /// Find backup file by content hash
    fn find_backup_by_hash(&self, hash: &Option<String>) -> Result<Option<PathBuf>> {
        let hash = match hash {
            Some(h) => h,
            None => return Ok(None),
        };

        // This is a simplified implementation
        // In reality, you'd need to scan the backup directory and match hashes
        // or maintain an index of hash -> backup file mappings

        let backup_files = fs::read_dir(&self.backup_dir)?;

        for entry in backup_files {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                // Check if filename contains the hash (simplified approach)
                if let Some(filename) = path.file_name() {
                    if filename.to_string_lossy().contains(hash) {
                        return Ok(Some(path));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Restore file from backup
    fn restore_from_backup(&self, target_path: &Path, backup_path: &Path) -> Result<()> {
        fs::copy(backup_path, target_path).context("Failed to restore file from backup")?;
        Ok(())
    }
}

/// Result of an undo/redo operation
#[derive(Debug, Clone)]
pub struct UndoRedoResult {
    pub transaction_id: String,
    pub operation: OperationType,
    pub file_path: PathBuf,
    pub success: bool,
    pub message: String,
}

/// Current state of undo/redo system
#[derive(Debug, Clone)]
pub struct UndoRedoState {
    pub undo_available: usize,
    pub redo_available: usize,
    pub last_undo: Option<String>,
    pub last_redo: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_undo_redo_manager() {
        let temp_dir = tempdir().unwrap();
        let manager = UndoRedoManager::new(temp_dir.path()).unwrap();

        let state = manager.get_state();
        assert_eq!(state.undo_available, 0);
        assert_eq!(state.redo_available, 0);
    }

    #[test]
    fn test_record_operation() {
        let temp_dir = tempdir().unwrap();
        let mut manager = UndoRedoManager::new(temp_dir.path()).unwrap();

        manager.record_operation("txn_123".to_string());

        let state = manager.get_state();
        assert_eq!(state.undo_available, 1);
        assert_eq!(state.redo_available, 0);
    }

    #[test]
    fn test_undo_redo_stacks() {
        let temp_dir = tempdir().unwrap();
        let mut manager = UndoRedoManager::new(temp_dir.path()).unwrap();

        // Record operations
        manager.record_operation("txn_1".to_string());
        manager.record_operation("txn_2".to_string());

        let state = manager.get_state();
        assert_eq!(state.undo_available, 2);
        assert_eq!(state.redo_available, 0);

        // Undo would move operations to redo stack
        // (actual undo logic would need proper backup files to test)
    }
}
