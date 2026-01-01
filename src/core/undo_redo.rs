use crate::core::transaction_log::{OperationType, Transaction, TransactionLog};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;

use std::fs;
use std::path::{Path, PathBuf};

/// Undo/Redo manager for transaction history
pub struct UndoRedoManager {
    transaction_log: TransactionLog,
    undo_stack: Vec<String>, // Transaction IDs
    redo_stack: Vec<String>, // Transaction IDs
    backup_dir: PathBuf,
    project_root: PathBuf,
}

impl UndoRedoManager {
    /// Create a new undo/redo manager
    pub fn new<P: AsRef<Path>>(project_root: P) -> Result<Self> {
        let project_root = project_root.as_ref().to_path_buf();
        let backup_dir = project_root.join(".gnawtreewriter_backups");

        // Ensure backup directory exists
        if !backup_dir.exists() {
            fs::create_dir_all(&backup_dir).context("Failed to create backup directory")?;
        }

        let transaction_log = TransactionLog::load(&project_root)?;

        // Populate undo stack from transaction log history
        let mut undo_stack = Vec::new();
        let history = transaction_log.get_full_history()?;

        for transaction in history {
            // Only add reversible operations to the undo stack
            if matches!(
                transaction.operation,
                OperationType::Edit
                    | OperationType::Insert
                    | OperationType::Delete
                    | OperationType::AddProperty
                    | OperationType::AddComponent
                    | OperationType::Move
                    | OperationType::Restore
            ) {
                undo_stack.push(transaction.id);
            }
        }

        Ok(Self {
            transaction_log,
            undo_stack,
            redo_stack: Vec::new(),
            backup_dir,
            project_root,
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
        let backup_path =
            self.find_backup_by_hash(&transaction.before_hash, Some(&transaction.file_path))?;

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
        let backup_path =
            self.find_backup_by_hash(&transaction.after_hash, Some(&transaction.file_path))?;

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
        // Try to restore using before_hash first
        if let Some(backup_path) =
            self.find_backup_by_hash(&transaction.before_hash, Some(&transaction.file_path))?
        {
            self.restore_from_backup(&transaction.file_path, &backup_path)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!("Reverted insert: {}", transaction.description),
            });
        }

        // Fallback: find the last transaction before this one and restore to that state
        if let Some(prev_tx) = self
            .transaction_log
            .get_last_transaction_before(&transaction.file_path, transaction.timestamp)?
        {
            let engine = crate::core::RestorationEngine::new(&self.project_root)?;
            if let Ok(_path) = engine.restore_file_to_transaction(&prev_tx.id) {
                return Ok(UndoRedoResult {
                    transaction_id: transaction.id.clone(),
                    operation: transaction.operation.clone(),
                    file_path: transaction.file_path.clone(),
                    success: true,
                    message: format!(
                        "Undo Insert: restored to previous transaction {}",
                        prev_tx.id
                    ),
                });
            }
        }

        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Insert undo failed: no suitable backup or previous state found".to_string(),
        })
    }

    fn redo_insert(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // Try to restore using after_hash
        if let Some(backup_path) =
            self.find_backup_by_hash(&transaction.after_hash, Some(&transaction.file_path))?
        {
            self.restore_from_backup(&transaction.file_path, &backup_path)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!("Re-applied insert: {}", transaction.description),
            });
        }

        // Fallback: find the next transaction that represents the after state
        if let Some(next_tx) =
            self.find_next_transaction_for_file(&transaction.file_path, &transaction.timestamp)?
        {
            let engine = crate::core::RestorationEngine::new(&self.project_root)?;
            engine.restore_file_to_transaction(&next_tx.id)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!(
                    "Re-applied insert by restoring to transaction {}",
                    next_tx.id
                ),
            });
        }

        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Backup not found for redo operation".to_string(),
        })
    }

    fn undo_delete(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // Try to restore using before_hash
        if let Some(backup_path) =
            self.find_backup_by_hash(&transaction.before_hash, Some(&transaction.file_path))?
        {
            self.restore_from_backup(&transaction.file_path, &backup_path)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!("Reverted delete: {}", transaction.description),
            });
        }

        // Fallback: restore to last transaction before this one
        if let Some(prev_tx) = self
            .transaction_log
            .get_last_transaction_before(&transaction.file_path, transaction.timestamp)?
        {
            let engine = crate::core::RestorationEngine::new(&self.project_root)?;
            engine.restore_file_to_transaction(&prev_tx.id)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!("Reverted delete by restoring to transaction {}", prev_tx.id),
            });
        }

        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Backup not found for undo operation".to_string(),
        })
    }

    fn redo_delete(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // Try to restore using after_hash
        if let Some(backup_path) =
            self.find_backup_by_hash(&transaction.after_hash, Some(&transaction.file_path))?
        {
            self.restore_from_backup(&transaction.file_path, &backup_path)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!("Re-applied delete: {}", transaction.description),
            });
        }

        // Fallback: find next transaction representing after state
        if let Some(next_tx) =
            self.find_next_transaction_for_file(&transaction.file_path, &transaction.timestamp)?
        {
            let engine = crate::core::RestorationEngine::new(&self.project_root)?;
            engine.restore_file_to_transaction(&next_tx.id)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!(
                    "Re-applied delete by restoring to transaction {}",
                    next_tx.id
                ),
            });
        }

        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Backup not found for redo operation".to_string(),
        })
    }

    fn undo_add_property(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // Try to restore using before_hash
        if let Some(backup_path) =
            self.find_backup_by_hash(&transaction.before_hash, Some(&transaction.file_path))?
        {
            self.restore_from_backup(&transaction.file_path, &backup_path)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!("Reverted add property: {}", transaction.description),
            });
        }

        // Fallback: restore to last transaction before this one
        if let Some(prev_tx) = self
            .transaction_log
            .get_last_transaction_before(&transaction.file_path, transaction.timestamp)?
        {
            let engine = crate::core::RestorationEngine::new(&self.project_root)?;
            engine.restore_file_to_transaction(&prev_tx.id)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!(
                    "Reverted add property by restoring to transaction {}",
                    prev_tx.id
                ),
            });
        }

        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Backup not found for undo operation".to_string(),
        })
    }

    fn redo_add_property(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // Try to restore using after_hash
        if let Some(backup_path) =
            self.find_backup_by_hash(&transaction.after_hash, Some(&transaction.file_path))?
        {
            self.restore_from_backup(&transaction.file_path, &backup_path)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!("Re-applied add property: {}", transaction.description),
            });
        }

        // Fallback: find next transaction representing after state
        if let Some(next_tx) =
            self.find_next_transaction_for_file(&transaction.file_path, &transaction.timestamp)?
        {
            let engine = crate::core::RestorationEngine::new(&self.project_root)?;
            engine.restore_file_to_transaction(&next_tx.id)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!(
                    "Re-applied add property by restoring to transaction {}",
                    next_tx.id
                ),
            });
        }

        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Backup not found for redo operation".to_string(),
        })
    }

    fn undo_add_component(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // Try to restore using before_hash
        if let Some(backup_path) =
            self.find_backup_by_hash(&transaction.before_hash, Some(&transaction.file_path))?
        {
            self.restore_from_backup(&transaction.file_path, &backup_path)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!("Reverted add component: {}", transaction.description),
            });
        }

        // Fallback: restore to last transaction before this one
        if let Some(prev_tx) = self
            .transaction_log
            .get_last_transaction_before(&transaction.file_path, transaction.timestamp)?
        {
            let engine = crate::core::RestorationEngine::new(&self.project_root)?;
            engine.restore_file_to_transaction(&prev_tx.id)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!(
                    "Reverted add component by restoring to transaction {}",
                    prev_tx.id
                ),
            });
        }

        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Backup not found for undo operation".to_string(),
        })
    }

    fn redo_add_component(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // Try to restore using after_hash
        if let Some(backup_path) =
            self.find_backup_by_hash(&transaction.after_hash, Some(&transaction.file_path))?
        {
            self.restore_from_backup(&transaction.file_path, &backup_path)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!("Re-applied add component: {}", transaction.description),
            });
        }

        // Fallback: find next transaction representing after state
        if let Some(next_tx) =
            self.find_next_transaction_for_file(&transaction.file_path, &transaction.timestamp)?
        {
            let engine = crate::core::RestorationEngine::new(&self.project_root)?;
            engine.restore_file_to_transaction(&next_tx.id)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!(
                    "Re-applied add component by restoring to transaction {}",
                    next_tx.id
                ),
            });
        }

        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Backup not found for redo operation".to_string(),
        })
    }

    fn undo_move(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // Try to restore using before_hash
        if let Some(backup_path) =
            self.find_backup_by_hash(&transaction.before_hash, Some(&transaction.file_path))?
        {
            self.restore_from_backup(&transaction.file_path, &backup_path)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!("Reverted move: {}", transaction.description),
            });
        }

        // Fallback: restore to last transaction before this one
        if let Some(prev_tx) = self
            .transaction_log
            .get_last_transaction_before(&transaction.file_path, transaction.timestamp)?
        {
            let engine = crate::core::RestorationEngine::new(&self.project_root)?;
            engine.restore_file_to_transaction(&prev_tx.id)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!("Reverted move by restoring to transaction {}", prev_tx.id),
            });
        }

        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Backup not found for undo operation".to_string(),
        })
    }

    fn redo_move(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // Try to restore using after_hash
        if let Some(backup_path) =
            self.find_backup_by_hash(&transaction.after_hash, Some(&transaction.file_path))?
        {
            self.restore_from_backup(&transaction.file_path, &backup_path)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!("Re-applied move: {}", transaction.description),
            });
        }

        // Fallback: find next transaction representing after state
        if let Some(next_tx) =
            self.find_next_transaction_for_file(&transaction.file_path, &transaction.timestamp)?
        {
            let engine = crate::core::RestorationEngine::new(&self.project_root)?;
            engine.restore_file_to_transaction(&next_tx.id)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!("Re-applied move by restoring to transaction {}", next_tx.id),
            });
        }

        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Backup not found for redo operation".to_string(),
        })
    }

    fn undo_restore(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // Try to restore using before_hash
        if let Some(backup_path) =
            self.find_backup_by_hash(&transaction.before_hash, Some(&transaction.file_path))?
        {
            self.restore_from_backup(&transaction.file_path, &backup_path)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!("Reverted restore: {}", transaction.description),
            });
        }

        // Fallback: restore to last transaction before this one
        if let Some(prev_tx) = self
            .transaction_log
            .get_last_transaction_before(&transaction.file_path, transaction.timestamp)?
        {
            let engine = crate::core::RestorationEngine::new(&self.project_root)?;
            engine.restore_file_to_transaction(&prev_tx.id)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!(
                    "Reverted restore by restoring to transaction {}",
                    prev_tx.id
                ),
            });
        }

        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Backup not found for undo operation".to_string(),
        })
    }

    fn redo_restore(&self, transaction: &Transaction) -> Result<UndoRedoResult> {
        // Try to restore using after_hash
        if let Some(backup_path) =
            self.find_backup_by_hash(&transaction.after_hash, Some(&transaction.file_path))?
        {
            self.restore_from_backup(&transaction.file_path, &backup_path)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!("Re-applied restore: {}", transaction.description),
            });
        }

        // Fallback: find next transaction representing after state
        if let Some(next_tx) =
            self.find_next_transaction_for_file(&transaction.file_path, &transaction.timestamp)?
        {
            let engine = crate::core::RestorationEngine::new(&self.project_root)?;
            engine.restore_file_to_transaction(&next_tx.id)?;

            return Ok(UndoRedoResult {
                transaction_id: transaction.id.clone(),
                operation: transaction.operation.clone(),
                file_path: transaction.file_path.clone(),
                success: true,
                message: format!(
                    "Re-applied restore by restoring to transaction {}",
                    next_tx.id
                ),
            });
        }

        Ok(UndoRedoResult {
            transaction_id: transaction.id.clone(),
            operation: transaction.operation.clone(),
            file_path: transaction.file_path.clone(),
            success: false,
            message: "Backup not found for redo operation".to_string(),
        })
    }

    /// Find next transaction for a file after a given timestamp
    fn find_next_transaction_for_file(
        &self,
        file_path: &PathBuf,
        after_time: &DateTime<Utc>,
    ) -> Result<Option<Transaction>> {
        let file_history = self.transaction_log.get_file_history(file_path)?;

        Ok(file_history
            .into_iter()
            .filter(|t| t.timestamp > *after_time)
            .filter(|t| {
                matches!(
                    t.operation,
                    OperationType::Edit | OperationType::Insert | OperationType::Delete
                )
            })
            .min_by_key(|t| t.timestamp))
    }

    /// Find backup file by content hash
    /// If `file` is provided, prefer backups that were created for that file.
    fn find_backup_by_hash(
        &self,
        hash: &Option<String>,
        file: Option<&Path>,
    ) -> Result<Option<PathBuf>> {
        let target_hash = match hash {
            Some(h) => h,
            None => return Ok(None),
        };

        if !self.backup_dir.exists() {
            return Ok(None);
        }

        // Prefer a backup that matches both the content hash and the file (if provided).
        if let Some(f) = file {
            if let Some(b) = crate::core::backup::find_backup_by_content_hash_for_file(
                &self.backup_dir,
                target_hash,
                f,
            )? {
                return Ok(Some(b.path));
            }
        }

        // Fallback: find any backup matching the content hash
        if let Some(b) =
            crate::core::backup::find_backup_by_content_hash(&self.backup_dir, target_hash)?
        {
            return Ok(Some(b.path));
        }

        Ok(None)
    }

    /// Restore file from backup
    fn restore_from_backup(&self, target_path: &Path, backup_path: &Path) -> Result<()> {
        let backup_content = fs::read_to_string(backup_path).context(format!(
            "Failed to read backup file: {}",
            backup_path.display()
        ))?;

        let json: Value =
            serde_json::from_str(&backup_content).context("Failed to parse backup JSON")?;

        let source_code = json["source_code"]
            .as_str()
            .ok_or_else(|| anyhow!("Backup file missing source_code"))?;

        fs::write(target_path, source_code).context(format!(
            "Failed to write restored file: {}",
            target_path.display()
        ))?;

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
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::fs;
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
    fn test_find_backup_by_hash_and_restore_file() -> Result<()> {
        let tmp = tempdir()?;
        let file_path = tmp.path().join("example.txt");
        fs::write(&file_path, "after")?;

        let backup_dir = tmp.path().join(".gnawtreewriter_backups");
        fs::create_dir_all(&backup_dir)?;

        let backup_path = backup_dir.join("test_backup.json");
        let backup_json = serde_json::json!({
            "file_path": file_path.to_string_lossy(),
            "timestamp": Utc::now().to_rfc3339(),
            "tree": {},
            "source_code": "original content"
        });

        fs::write(&backup_path, serde_json::to_string_pretty(&backup_json)?)?;

        let manager = UndoRedoManager::new(tmp.path())?;
        let hash = crate::core::calculate_content_hash("original content");

        let found = manager.find_backup_by_hash(&Some(hash.clone()), Some(&file_path))?;
        assert!(found.is_some());

        manager.restore_from_backup(&file_path, &found.unwrap())?;
        let content = fs::read_to_string(&file_path)?;
        assert_eq!(content, "original content");

        Ok(())
    }

    #[test]
    fn test_undo_redo_insert_flow() -> Result<()> {
        let tmp = tempdir()?;
        let file_path = tmp.path().join("a.txt");

        // File initially at 'after content'
        fs::write(&file_path, "after content")?;

        let backup_dir = tmp.path().join(".gnawtreewriter_backups");
        fs::create_dir_all(&backup_dir)?;

        let before_backup = backup_dir.join("before.json");
        let before_json = serde_json::json!({
            "file_path": file_path.to_string_lossy(),
            "timestamp": Utc::now().to_rfc3339(),
            "tree": {},
            "source_code": "original"
        });
        fs::write(&before_backup, serde_json::to_string_pretty(&before_json)?)?;

        let after_backup = backup_dir.join("after.json");
        let after_json = serde_json::json!({
            "file_path": file_path.to_string_lossy(),
            "timestamp": Utc::now().to_rfc3339(),
            "tree": {},
            "source_code": "after content"
        });
        fs::write(&after_backup, serde_json::to_string_pretty(&after_json)?)?;

        let txn = Transaction {
            id: "txn_1".to_string(),
            timestamp: Utc::now(),
            operation: OperationType::Insert,
            file_path: file_path.clone(),
            node_path: None,
            before_hash: Some(crate::core::calculate_content_hash("original")),
            after_hash: Some(crate::core::calculate_content_hash("after content")),
            description: "Insert test".to_string(),
            session_id: "session_1".to_string(),
            metadata: HashMap::new(),
        };

        let manager = UndoRedoManager::new(tmp.path())?;

        // Ensure current file is 'after content'
        assert_eq!(fs::read_to_string(&file_path)?, "after content");

        // Undo should restore to 'original'
        let undo_res = manager.undo_insert(&txn)?;
        assert!(undo_res.success, "Undo failed: {}", undo_res.message);
        assert_eq!(fs::read_to_string(&file_path)?, "original");

        // Redo should restore back to 'after content'
        let redo_res = manager.redo_insert(&txn)?;
        assert!(redo_res.success, "Redo failed: {}", redo_res.message);
        assert_eq!(fs::read_to_string(&file_path)?, "after content");

        Ok(())
    }

    #[test]
    fn test_undo_fallback_uses_previous_transaction() -> Result<()> {
        let tmp = tempdir()?;
        let file_path = tmp.path().join("fallback.txt");

        // Current file content (we'll simulate "after" state)
        fs::write(&file_path, "after_state")?;

        // Create backup for the previous state "prev_state"
        let backup_dir = tmp.path().join(".gnawtreewriter_backups");
        fs::create_dir_all(&backup_dir)?;

        let prev_backup = backup_dir.join("prev.json");
        let prev_json = serde_json::json!({
            "file_path": file_path.to_string_lossy(),
            "timestamp": Utc::now().to_rfc3339(),
            "tree": {},
            "source_code": "prev_state"
        });
        fs::write(&prev_backup, serde_json::to_string_pretty(&prev_json)?)?;

        // Log a previous transaction whose after_hash matches "prev_state"
        let mut tlog = TransactionLog::load(tmp.path())?;
        let _prev_id = tlog.log_transaction(
            OperationType::Edit,
            file_path.clone(),
            None,
            Some(crate::core::calculate_content_hash("older")),
            Some(crate::core::calculate_content_hash("prev_state")),
            "prev tx".to_string(),
            std::collections::HashMap::new(),
        )?;

        // Create a transaction to undo that has no before_hash (so we must fallback)
        let txn = Transaction {
            id: "fake_undo".to_string(),
            timestamp: Utc::now() + chrono::Duration::seconds(1),
            operation: OperationType::Insert,
            file_path: file_path.clone(),
            node_path: None,
            before_hash: None,
            after_hash: Some(crate::core::calculate_content_hash("after_state")),
            description: "Fake insert".to_string(),
            session_id: "s1".to_string(),
            metadata: std::collections::HashMap::new(),
        };

        let manager = UndoRedoManager::new(tmp.path())?;

        // Undo should find the previous transaction and restore 'prev_state'
        let res = manager.undo_insert(&txn)?;
        assert!(res.success, "{}", res.message);
        assert_eq!(fs::read_to_string(&file_path)?, "prev_state");

        Ok(())
    }

    #[test]
    fn test_redo_fallback_uses_next_transaction() -> Result<()> {
        let tmp = tempdir()?;
        let file_path = tmp.path().join("fallback2.txt");

        // Start file in original state
        fs::write(&file_path, "original")?;

        // Create backup for the next transaction (holds 'next_state')
        let backup_dir = tmp.path().join(".gnawtreewriter_backups");
        fs::create_dir_all(&backup_dir)?;

        let next_backup = backup_dir.join("next.json");
        let next_json = serde_json::json!({
            "file_path": file_path.to_string_lossy(),
            "timestamp": Utc::now().to_rfc3339(),
            "tree": {},
            "source_code": "next_state"
        });
        fs::write(&next_backup, serde_json::to_string_pretty(&next_json)?)?;

        // Log a next transaction whose before_hash matches 'next_state'
        let mut tlog = TransactionLog::load(tmp.path())?;
        let _next_id = tlog.log_transaction(
            OperationType::Edit,
            file_path.clone(),
            None,
            Some(crate::core::calculate_content_hash("next_state")),
            Some(crate::core::calculate_content_hash("later_state")),
            "next tx".to_string(),
            std::collections::HashMap::new(),
        )?;

        // Create a transaction to redo (it has no after_hash, so redo should find next tx)
        let txn = Transaction {
            id: "fake_redo".to_string(),
            timestamp: Utc::now() - chrono::Duration::seconds(10),
            operation: OperationType::Insert,
            file_path: file_path.clone(),
            node_path: None,
            before_hash: Some(crate::core::calculate_content_hash("original")),
            after_hash: None,
            description: "Fake redo".to_string(),
            session_id: "s1".to_string(),
            metadata: std::collections::HashMap::new(),
        };

        let manager = UndoRedoManager::new(tmp.path())?;

        let res = manager.redo_insert(&txn)?;
        assert!(res.success, "{}", res.message);

        // After redo, file should be restored to 'next_state'
        assert_eq!(fs::read_to_string(&file_path)?, "next_state");

        Ok(())
    }
}
