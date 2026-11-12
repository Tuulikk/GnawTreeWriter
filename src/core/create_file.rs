//! Create a new file — the one write operation that is not an edit.
//!
//! Shared by the CLI (`scaffold`) and the MCP tool of the same name so the two
//! cannot drift: validation, the existing-file refusal and the transaction
//! entry live HERE, once.
//!
//! Logged as `OperationType::Insert` **for now**. That is itself the known
//! conflation — a created file is not an in-file insert, and its undo needs a
//! different path (see uppslag.md, 2026-11-11). The log entry exists so the
//! operation is at least recorded; `undo` cannot delete it yet.

use anyhow::{Context, Result};
use std::path::Path;

/// Where the new file's content comes from.
pub enum ContentSource<'a> {
    /// The structural DSL, e.g. `rust:mod(name:security, fn:validate)`.
    Schema(&'a str),
    /// Literal text.
    Literal(&'a str),
    /// Read from this file.
    File(&'a Path),
}

/// Create `path`. Refuses existing files, validates the content against the
/// real parser before writing, and records the operation.
pub fn create(path: &Path, source: ContentSource<'_>) -> Result<usize> {
    if path.exists() {
        anyhow::bail!(
            "File already exists: {}. Scaffold only creates NEW files — use `insert` or `edit` on it instead.",
            path.display()
        );
    }

    let code = match source {
        ContentSource::File(src) => std::fs::read_to_string(src)
            .with_context(|| format!("cannot read --from {}", src.display()))?,
        ContentSource::Literal(text) => text.to_string(),
        ContentSource::Schema(schema) => crate::core::ScaffoldEngine::new().generate(schema)?,
    };

    // Validate with the REAL parser before writing: a scaffold that does not
    // parse is a broken file with a friendly message.
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();
    if !ext.is_empty() {
        if let Ok(parser) = crate::parser::get_parser(Path::new(&format!("x.{ext}"))) {
            if let Err(e) = parser.parse(&code) {
                anyhow::bail!(
                    "scaffold rejected: the proposed content does not parse as {ext}: {e}\n\
                     Nothing was written."
                );
            }
        }
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, &code)?;

    let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    if let Ok(mut log) = crate::core::transaction_log::TransactionLog::new(&root) {
        use crate::core::transaction_log::{calculate_content_hash, OperationType};
        let mut meta = std::collections::HashMap::new();
        meta.insert("operation".to_string(), "scaffold".to_string());
        // Declared attribution: `mechanism` stays what actually invoked us
        // (cli/mcp), while `operation` above says WHAT was done — two different
        // questions that must not be collapsed (owner, 2026-11-12).
        crate::core::attribution::process_attribution().apply_to(&mut meta);
        // A created file is a content operation (head counts it), so the state it
        // creates needs a verifiable source like every other write.
        if let Ok(blob) =
            crate::core::backup::write_after_blob(&root, &path.display().to_string(), &code)
        {
            meta.insert("after_blob".to_string(), blob);
        }
        let _ = log.log_transaction(
            OperationType::CreateFile,
            path.to_path_buf(),
            None,
            None,
            Some(calculate_content_hash(&code)),
            format!(
                "scaffold: created {} ({} bytes)",
                path.display(),
                code.len()
            ),
            meta,
        );
    }

    Ok(code.len())
}
