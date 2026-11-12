use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

/// Represents a single transaction in the log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub operation: OperationType,
    pub file_path: PathBuf,
    pub node_path: Option<String>,
    pub before_hash: Option<String>,
    pub after_hash: Option<String>,
    pub description: String,
    pub session_id: String,
    pub metadata: HashMap<String, String>,
}

impl Transaction {
    /// Attribution as declared in `metadata` (see `core::attribution`).
    /// Unknown stays unknown: no contributors means an empty list, never a guess.
    pub fn attribution(&self) -> crate::core::attribution::Attribution {
        crate::core::attribution::Attribution::from_metadata(&self.metadata)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OperationType {
    Edit,
    Insert,
    Delete,
    AddProperty,
    AddComponent,
    Move,
    Restore,
    /// A transaction was REVERTED by `undo` (2026-11-12).
    ///
    /// Distinct from `Restore` on purpose: `Restore` says "content was replaced
    /// by a stored state" and need not name a single transaction; `Undo` says
    /// "THIS transaction is no longer applied". The reverted id lives in
    /// `metadata["reverted_txn"]`, with `from_hash`/`to_hash` beside it.
    ///
    /// These events are what make undo survive a process boundary: the journal
    /// stays append-only, and the applied set is DERIVED by folding it
    /// (`TransactionLog::applied_transactions`). Nothing is rewritten.
    Undo,
    /// A previously undone transaction is applied again (`redo`).
    /// Carries `metadata["restored_txn"]`.
    Redo,
    /// A FILE was created (scaffold). Filesystem scope — its undo is
    /// `undo --files`, never the in-file undo path (owner's distinction,
    /// 2026-11-11: reverting content and deleting a file are different
    /// operations with different risk).
    CreateFile,
    SessionStart,
    SessionEnd,
}

/// Transaction log manager
pub struct TransactionLog {
    log_file: PathBuf,
    session_id: String,
    current_session: Vec<Transaction>,
    session_id_file: PathBuf,
}

impl TransactionLog {
    pub fn get_current_session_id(&self) -> &str {
        &self.session_id
    }

    /// Create a new transaction log
    pub fn new<P: AsRef<Path>>(project_root: P) -> Result<Self> {
        let log_file = project_root.as_ref().join(".gnawtreewriter_session.json");
        let session_id_file = project_root.as_ref().join(".gnawtreewriter_session_id");
        let session_id = generate_session_id();

        // Save session_id to file for persistence
        std::fs::write(&session_id_file, &session_id)?;

        let mut log = Self {
            log_file,
            session_id: session_id.clone(),
            current_session: Vec::new(),
            session_id_file,
        };

        // Log session start - this will add it to both current_session and log file
        log.log_transaction(
            OperationType::SessionStart,
            PathBuf::from("session"),
            None,
            None,
            None,
            "Session started".to_string(),
            HashMap::new(),
        )?;

        Ok(log)
    }

    /// Load existing transaction log
    pub fn load<P: AsRef<Path>>(project_root: P) -> Result<Self> {
        let log_file = project_root.as_ref().join(".gnawtreewriter_session.json");
        let session_id_file = project_root.as_ref().join(".gnawtreewriter_session_id");

        if !log_file.exists() {
            return Self::new(project_root);
        }

        // Try to load existing session_id from file
        let session_id = if session_id_file.exists() {
            std::fs::read_to_string(&session_id_file).unwrap_or_else(|_| generate_session_id())
        } else {
            generate_session_id()
        };

        // Load current session transactions from log file
        let full_history = Self::load_full_history_from_file(&log_file)?;
        let current_session: Vec<Transaction> = full_history
            .into_iter()
            .filter(|t| t.session_id == session_id)
            .collect();

        Ok(Self {
            log_file,
            session_id,
            current_session,
            session_id_file,
        })
    }

    /// Ensure a session exists (for implicit session creation)
    fn ensure_session_exists(&mut self) -> Result<()> {
        if self.current_session.is_empty() {
            // Auto-start a default session
            // This allows edits without explicit session-start command
            self.session_id = generate_session_id();

            // Save session_id to file for persistence
            std::fs::write(&self.session_id_file, &self.session_id)?;

            // Create and log SessionStart transaction
            let transaction = Transaction {
                id: generate_transaction_id(),
                timestamp: Utc::now(),
                operation: OperationType::SessionStart,
                file_path: PathBuf::from("session"),
                node_path: None,
                before_hash: None,
                after_hash: None,
                description: "Default session auto-started".to_string(),
                session_id: self.session_id.clone(),
                metadata: HashMap::new(),
            };

            self.current_session.push(transaction.clone());
            self.append_to_log(&transaction)?;
        }
        Ok(())
    }

    /// Log a new transaction
    #[allow(clippy::too_many_arguments)]
    pub fn log_transaction(
        &mut self,
        operation: OperationType,
        file_path: PathBuf,
        node_path: Option<String>,
        before_hash: Option<String>,
        after_hash: Option<String>,
        description: String,
        metadata: HashMap<String, String>,
    ) -> Result<String> {
        // Ensure we have an active session (auto-start default session if needed)
        // Skip for session operations to avoid infinite recursion
        if !matches!(
            operation,
            OperationType::SessionStart | OperationType::SessionEnd
        ) {
            self.ensure_session_exists()?;
        }

        let transaction = Transaction {
            id: generate_transaction_id(),
            timestamp: Utc::now(),
            operation,
            file_path,
            node_path,
            before_hash,
            after_hash,
            description,
            session_id: self.session_id.clone(),
            metadata,
        };

        let transaction_id = transaction.id.clone();

        // Add to current session
        self.current_session.push(transaction.clone());

        // Append to log file
        self.append_to_log(&transaction)?;

        Ok(transaction_id)
    }

    /// Get transaction by ID (wrapper around find_transaction for easier Result handling)
    pub fn get_transaction(&self, transaction_id: &str) -> Result<Transaction> {
        self.find_transaction(transaction_id)?
            .ok_or_else(|| anyhow::anyhow!("Transaction not found: {}", transaction_id))
    }

    /// Get transaction history for current session
    pub fn get_session_history(&self) -> &[Transaction] {
        &self.current_session
    }

    /// Get full transaction history from file
    pub fn get_full_history(&self) -> Result<Vec<Transaction>> {
        Self::load_full_history_from_file(&self.log_file)
    }
    /// Fold the journal into the set of transactions that are STILL APPLIED.
    ///
    /// This is the single definition of "what counts now", and it is derived —
    /// never stored as truth (owner's rule, 2026-11-12: the head file is a
    /// projection of the journal, not a second authority).
    ///
    /// Rules, in journal order:
    /// * a content operation (edit/insert/delete/restore/...) is a CANDIDATE
    /// * `Undo` with `metadata["reverted_txn"]` removes that id from the set
    /// * `Redo` with `metadata["restored_txn"]` puts it back
    /// * everything else (session markers, an unknown/missing id) is ignored —
    ///   an unreadable event leaves the set alone instead of guessing, which is
    ///   why the return also names the events that could not be interpreted.
    ///
    /// Return order is journal order, i.e. the order `undo` must walk backwards.
    pub fn applied_transactions(history: &[Transaction]) -> (Vec<String>, Vec<String>) {
        let mut applied: Vec<String> = Vec::new();
        let mut unreadable: Vec<String> = Vec::new();

        for t in history {
            match &t.operation {
                OperationType::Undo => match t.metadata.get("reverted_txn") {
                    Some(id) => applied.retain(|a| a != id),
                    None => unreadable.push(t.id.clone()),
                },
                OperationType::Redo => match t.metadata.get("restored_txn") {
                    // Re-appending keeps the original relative order: a redo of
                    // an older txn must not jump above a newer one, because
                    // undo walks the journal backwards.
                    Some(id) => {
                        if !applied.iter().any(|a| a == id) {
                            let anchor =
                                history.iter().position(|h| &h.id == id).and_then(|orig| {
                                    let orig_ts = history.get(orig).map(|h| h.timestamp);
                                    applied.iter().position(|a| {
                                        history
                                            .iter()
                                            .find(|h| &h.id == a)
                                            .map(|h| h.timestamp > orig_ts.unwrap_or(h.timestamp))
                                            .unwrap_or(true)
                                    })
                                });
                            match anchor {
                                Some(pos) => applied.insert(pos, id.clone()),
                                None => applied.push(id.clone()),
                            }
                        }
                    }
                    None => unreadable.push(t.id.clone()),
                },
                op if op.is_content_operation() => applied.push(t.id.clone()),
                _ => {}
            }
        }
        (applied, unreadable)
    }

    /// The REDO set: transactions that were undone and not re-applied since,
    /// oldest-first (so `pop()` yields the newest undo, which is what redo does).
    ///
    /// A new content operation clears the list — the standard rule: editing
    /// after an undo abandons the branch you undid away from. Derived from the
    /// journal, so redo also survives a process boundary.
    pub fn redo_candidates(history: &[Transaction]) -> Vec<String> {
        let mut redoable: Vec<String> = Vec::new();
        for t in history {
            match &t.operation {
                OperationType::Undo => {
                    if let Some(id) = t.metadata.get("reverted_txn") {
                        if !redoable.iter().any(|r| r == id) {
                            redoable.push(id.clone());
                        }
                    }
                }
                OperationType::Redo => {
                    if let Some(id) = t.metadata.get("restored_txn") {
                        redoable.retain(|r| r != id);
                    }
                }
                op if op.is_content_operation() => redoable.clear(),
                _ => {}
            }
        }
        redoable
    }

    /// Load full history from a log file (helper method)
    fn load_full_history_from_file(log_file: &PathBuf) -> Result<Vec<Transaction>> {
        if !log_file.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(log_file).context("Failed to open transaction log file")?;
        let reader = BufReader::new(file);

        let mut transactions = Vec::new();

        for line in reader.lines() {
            let line = line.context("Failed to read line from log file")?;
            if line.trim().is_empty() {
                continue;
            }

            let transaction: Transaction =
                serde_json::from_str(&line).context("Failed to parse transaction from log")?;
            transactions.push(transaction);
        }

        Ok(transactions)
    }

    /// Get transactions for a specific file
    pub fn get_file_history<P: AsRef<Path>>(&self, file_path: P) -> Result<Vec<Transaction>> {
        let full_history = self.get_full_history()?;
        let target_path = file_path.as_ref();

        Ok(full_history
            .into_iter()
            .filter(|t| t.file_path == target_path)
            .collect())
    }

    /// Get transactions since a specific timestamp
    pub fn get_history_since(&self, since: DateTime<Utc>) -> Result<Vec<Transaction>> {
        let full_history = self.get_full_history()?;

        Ok(full_history
            .into_iter()
            .filter(|t| t.timestamp >= since)
            .collect())
    }

    /// Get transactions within a time range
    pub fn get_history_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Transaction>> {
        let full_history = self.get_full_history()?;

        Ok(full_history
            .into_iter()
            .filter(|t| t.timestamp >= start && t.timestamp <= end)
            .collect())
    }

    /// Get all files affected in a time range
    pub fn get_affected_files_since(&self, since: DateTime<Utc>) -> Result<Vec<PathBuf>> {
        let transactions = self.get_history_since(since)?;
        let mut files: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

        for transaction in transactions {
            if matches!(
                transaction.operation,
                OperationType::Edit | OperationType::Insert | OperationType::Delete
            ) {
                files.insert(transaction.file_path);
            }
        }

        Ok(files.into_iter().collect())
    }

    /// Get all files affected in a session
    pub fn get_session_files(&self, session_id: &str) -> Result<Vec<PathBuf>> {
        let full_history = self.get_full_history()?;
        let mut files: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

        for transaction in full_history {
            if transaction.session_id == session_id
                && matches!(
                    transaction.operation,
                    OperationType::Edit | OperationType::Insert | OperationType::Delete
                )
            {
                files.insert(transaction.file_path);
            }
        }

        Ok(files.into_iter().collect())
    }

    /// Find the last transaction for a file before a specific timestamp
    pub fn get_last_transaction_before(
        &self,
        file_path: &PathBuf,
        before: DateTime<Utc>,
    ) -> Result<Option<Transaction>> {
        let file_history = self.get_file_history(file_path)?;

        Ok(file_history
            .into_iter()
            .filter(|t| t.timestamp < before)
            .filter(|t| {
                matches!(
                    t.operation,
                    OperationType::Edit | OperationType::Insert | OperationType::Delete
                )
            })
            .max_by_key(|t| t.timestamp))
    }

    /// Get project restoration plan for a specific timestamp
    pub fn get_project_restoration_plan(
        &self,
        restore_to: DateTime<Utc>,
    ) -> Result<ProjectRestorationPlan> {
        let affected_files = self.get_affected_files_since(restore_to)?;
        let mut file_plans = Vec::new();

        for file_path in affected_files {
            if let Some(last_transaction) =
                self.get_last_transaction_before(&file_path, restore_to)?
            {
                file_plans.push(FileRestorationPlan {
                    file_path: file_path.clone(),
                    target_transaction_id: last_transaction.id,
                    target_hash: last_transaction.after_hash.clone(),
                    current_modifications_count: self
                        .count_modifications_since(&file_path, restore_to)?,
                });
            }
        }

        Ok(ProjectRestorationPlan {
            restore_to_timestamp: restore_to,
            affected_files: file_plans,
            total_transactions_to_revert: self.count_transactions_since(restore_to)?,
        })
    }

    /// Count modifications to a file since a timestamp
    fn count_modifications_since(
        &self,
        file_path: &PathBuf,
        since: DateTime<Utc>,
    ) -> Result<usize> {
        let file_history = self.get_file_history(file_path)?;
        Ok(file_history
            .into_iter()
            .filter(|t| t.timestamp >= since)
            .filter(|t| {
                matches!(
                    t.operation,
                    OperationType::Edit | OperationType::Insert | OperationType::Delete
                )
            })
            .count())
    }

    /// Count total transactions since a timestamp
    fn count_transactions_since(&self, since: DateTime<Utc>) -> Result<usize> {
        let transactions = self.get_history_since(since)?;
        Ok(transactions
            .into_iter()
            .filter(|t| {
                matches!(
                    t.operation,
                    OperationType::Edit | OperationType::Insert | OperationType::Delete
                )
            })
            .count())
    }

    /// Find transaction by ID
    pub fn find_transaction(&self, transaction_id: &str) -> Result<Option<Transaction>> {
        // Check current session first
        for transaction in &self.current_session {
            if transaction.id == transaction_id {
                return Ok(Some(transaction.clone()));
            }
        }

        // Search full history
        let full_history = self.get_full_history()?;
        for transaction in full_history {
            if transaction.id == transaction_id {
                return Ok(Some(transaction));
            }
        }

        Ok(None)
    }

    /// Get the last N transactions
    pub fn get_last_n_transactions(&self, n: usize) -> Result<Vec<Transaction>> {
        let full_history = self.get_full_history()?;

        Ok(full_history.into_iter().rev().take(n).rev().collect())
    }

    /// Start a new session (clears current session, keeps history)
    pub fn start_new_session(&mut self, name: Option<String>) -> Result<()> {
        if !self.current_session.is_empty() {
            let _ = self.log_transaction(
                OperationType::SessionEnd,
                PathBuf::from("session"),
                None,
                None,
                None,
                format!(
                    "Session ended with {} operations",
                    self.current_session.len()
                ),
                HashMap::new(),
            );
        }

        self.session_id = generate_session_id();
        self.current_session.clear();

        let _ = std::fs::write(&self.session_id_file, &self.session_id);

        if let Some(alias) = name {
            let project_root = self.log_file.parent().unwrap_or(Path::new("."));
            let alias_file = project_root.join(".gnawtreewriter_aliases.json");
            let mut aliases: HashMap<String, String> = if alias_file.exists() {
                let data = std::fs::read_to_string(&alias_file)?;
                serde_json::from_str(&data).unwrap_or_default()
            } else {
                HashMap::new()
            };
            aliases.insert(alias.clone(), self.session_id.clone());
            let data = serde_json::to_string_pretty(&aliases)?;
            let _ = std::fs::write(alias_file, data);
            println!(
                "✓ Session alias created: '{}' -> {}",
                alias, self.session_id
            );
        }

        self.log_transaction(
            OperationType::SessionStart,
            PathBuf::from("session"),
            None,
            None,
            None,
            "New session started".to_string(),
            HashMap::new(),
        )?;

        Ok(())
    }

    /// Export history as JSON
    pub fn export_history(&self, format: ExportFormat) -> Result<String> {
        let history = self.get_full_history()?;

        match format {
            ExportFormat::Json => serde_json::to_string_pretty(&history)
                .context("Failed to serialize history to JSON"),
            ExportFormat::JsonCompact => serde_json::to_string(&history)
                .context("Failed to serialize history to compact JSON"),
        }
    }

    /// Private method to append transaction to log file
    fn append_to_log(&self, transaction: &Transaction) -> Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
            .context("Failed to open log file for writing")?;

        let mut writer = BufWriter::new(file);
        let json_line =
            serde_json::to_string(transaction).context("Failed to serialize transaction")?;

        writeln!(writer, "{}", json_line).context("Failed to write transaction to log")?;

        writer.flush().context("Failed to flush log file")?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    JsonCompact,
}

/// Plan for restoring multiple files to a specific point in time
#[derive(Debug, Clone)]
pub struct ProjectRestorationPlan {
    pub restore_to_timestamp: DateTime<Utc>,
    pub affected_files: Vec<FileRestorationPlan>,
    pub total_transactions_to_revert: usize,
}

/// Plan for restoring a single file
#[derive(Debug, Clone)]
pub struct FileRestorationPlan {
    pub file_path: PathBuf,
    pub target_transaction_id: String,
    pub target_hash: Option<String>,
    pub current_modifications_count: usize,
}

impl ProjectRestorationPlan {
    /// Check if this restoration would affect any files
    pub fn has_changes(&self) -> bool {
        !self.affected_files.is_empty()
    }

    /// Get summary of what will be restored
    pub fn get_summary(&self) -> String {
        format!(
            "Restore {} files to state at {}, reverting {} transactions",
            self.affected_files.len(),
            self.restore_to_timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            self.total_transactions_to_revert
        )
    }

    /// Get list of files that will be affected
    pub fn get_file_list(&self) -> Vec<&PathBuf> {
        self.affected_files.iter().map(|f| &f.file_path).collect()
    }
}

/// Generate a unique session ID
fn generate_session_id() -> String {
    format!("session_{}", Utc::now().timestamp_nanos_opt().unwrap_or(0))
}

/// Generate a unique transaction ID
fn generate_transaction_id() -> String {
    format!("txn_{}", Utc::now().timestamp_nanos_opt().unwrap_or(0))
}

/// Utility function to calculate content hash
pub fn calculate_content_hash(content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

impl OperationType {
    /// True for the operations that change a file's CONTENT and are therefore
    /// reversible from a content blob. Session markers and the `Undo`/`Redo`
    /// events themselves are deliberately excluded: reactions are not things
    /// that can be undone as if they were edits.
    pub fn is_content_operation(&self) -> bool {
        matches!(
            self,
            OperationType::Edit
                | OperationType::Insert
                | OperationType::Delete
                | OperationType::AddProperty
                | OperationType::AddComponent
                | OperationType::Move
                | OperationType::Restore
                | OperationType::CreateFile
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_transaction_log() {
        let temp_dir = tempdir().unwrap();
        let log = TransactionLog::new(temp_dir.path()).unwrap();

        assert_eq!(log.current_session.len(), 1); // SessionStart
        assert!(matches!(
            log.current_session[0].operation,
            OperationType::SessionStart
        ));
    }

    #[test]
    fn test_log_transaction() {
        let temp_dir = tempdir().unwrap();
        let mut log = TransactionLog::new(temp_dir.path()).unwrap();

        let transaction_id = log
            .log_transaction(
                OperationType::Edit,
                PathBuf::from("test.py"),
                Some("0.1".to_string()),
                Some("hash1".to_string()),
                Some("hash2".to_string()),
                "Test edit".to_string(),
                HashMap::new(),
            )
            .unwrap();

        assert_eq!(log.current_session.len(), 2); // SessionStart + Edit
        assert!(!transaction_id.is_empty());
    }

    #[test]
    fn test_find_transaction() {
        let temp_dir = tempdir().unwrap();
        let mut log = TransactionLog::new(temp_dir.path()).unwrap();

        let transaction_id = log
            .log_transaction(
                OperationType::Edit,
                PathBuf::from("test.py"),
                Some("0.1".to_string()),
                Some("hash1".to_string()),
                Some("hash2".to_string()),
                "Test edit".to_string(),
                HashMap::new(),
            )
            .unwrap();

        let found = log.find_transaction(&transaction_id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, transaction_id);
    }

    #[test]
    fn test_content_hash() {
        let hash1 = calculate_content_hash("def test(): pass");
        let hash2 = calculate_content_hash("def test(): pass");
        let hash3 = calculate_content_hash("def other(): pass");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
