/*
 * GnawTreeWriter - backup helpers
 *
 * Provides utilities to list, parse and restore JSON backup files created by
 * `GnawTreeWriter::create_backup()`. These helpers centralize backup-related
 * functionality so other modules (RestorationEngine, UndoRedoManager, tests)
 * can reuse a consistent implementation.
 */

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// Parsed metadata for a backup JSON file
#[derive(Debug, Clone)]
pub struct BackupFile {
    /// Full path to the backup JSON file
    pub path: PathBuf,
    /// Timestamp recorded in the backup file
    pub timestamp: DateTime<Utc>,
    /// Original source file path stored in the backup JSON
    pub original_file_path: PathBuf,
    /// Optional content hash of the source code (calculated at parse time)
    pub content_hash: Option<String>,
}

/// List all backup files found in `backup_dir`.
/// Only files with `.json` extension are considered. Parses each file and
/// returns a vector of `BackupFile` sorted by timestamp (newest first).
pub fn list_backup_files<P: AsRef<Path>>(backup_dir: P) -> Result<Vec<BackupFile>> {
    let backup_dir = backup_dir.as_ref();

    let mut backups = Vec::new();

    if !backup_dir.exists() {
        return Ok(backups);
    }

    let entries = fs::read_dir(backup_dir).context("Failed to read backup directory")?;

    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
            match parse_backup_file(&path) {
                Ok(b) => backups.push(b),
                Err(_) => {
                    // Skip files that fail to parse (non-critical for listing)
                    continue;
                }
            }
        }
    }

    // Sort by timestamp (newest first)
    backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(backups)
}

/// Parse a single backup JSON file and return a `BackupFile` with metadata
/// (timestamp, original_file_path, content_hash).
pub fn parse_backup_file<P: AsRef<Path>>(backup_path: P) -> Result<BackupFile> {
    let backup_path = backup_path.as_ref();

    let content = fs::read_to_string(backup_path).context(format!(
        "Failed to read backup file: {}",
        backup_path.display()
    ))?;

    let json: Value = serde_json::from_str(&content).context(format!(
        "Failed to parse backup JSON: {}",
        backup_path.display()
    ))?;

    // Extract file_path
    let original_file_path_str = json["file_path"]
        .as_str()
        .ok_or_else(|| anyhow!("Backup file missing 'file_path'"))?;

    // Extract timestamp
    let timestamp_str = json["timestamp"]
        .as_str()
        .ok_or_else(|| anyhow!("Backup file missing 'timestamp'"))?;
    let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
        .context("Failed to parse backup timestamp")?
        .with_timezone(&Utc);

    // Extract source_code (used for hash calculation)
    let source_code = json["source_code"]
        .as_str()
        .ok_or_else(|| anyhow!("Backup file missing 'source_code'"))?;
    let content_hash = Some(crate::core::calculate_content_hash(source_code));

    Ok(BackupFile {
        path: backup_path.to_path_buf(),
        timestamp,
        original_file_path: PathBuf::from(original_file_path_str),
        content_hash,
    })
}

/// Find the first backup whose content hash equals `content_hash`.
/// Returns the parsed `BackupFile` if found.
pub fn find_backup_by_content_hash<P: AsRef<Path>>(
    backup_dir: P,
    content_hash: &str,
) -> Result<Option<BackupFile>> {
    let backups = list_backup_files(backup_dir)?;
    for b in backups {
        if let Some(h) = &b.content_hash {
            if h == content_hash {
                return Ok(Some(b));
            }
        }
    }
    Ok(None)
}

/// Prefer a backup that matches both the content hash and the original file path.
/// If none found with both constraints, falls back to any backup matching the hash.
pub fn find_backup_by_content_hash_for_file<P: AsRef<Path>>(
    backup_dir: P,
    content_hash: &str,
    original_file: &Path,
) -> Result<Option<BackupFile>> {
    let backups = list_backup_files(backup_dir)?;
    // Prefer exact file match
    for b in &backups {
        if let Some(h) = &b.content_hash {
            if h == content_hash && b.original_file_path == original_file {
                return Ok(Some(b.clone()));
            }
        }
    }
    // Fallback: any matching hash
    for b in backups {
        if let Some(h) = &b.content_hash {
            if h == content_hash {
                return Ok(Some(b));
            }
        }
    }
    Ok(None)
}

/// Read the `source_code` field from a backup JSON and write it to `target_path`.
/// Returns the written `PathBuf` on success.
pub fn restore_from_backup<P: AsRef<Path>, Q: AsRef<Path>>(
    backup_path: P,
    target_path: Q,
) -> Result<PathBuf> {
    let backup_path = backup_path.as_ref();
    let target_path = target_path.as_ref();

    let backup_content = fs::read_to_string(backup_path).context(format!(
        "Failed to read backup file: {}",
        backup_path.display()
    ))?;

    let json: Value = serde_json::from_str(&backup_content).context(format!(
        "Failed to parse backup JSON: {}",
        backup_path.display()
    ))?;

    let source_code = json["source_code"]
        .as_str()
        .ok_or_else(|| anyhow!("Backup file missing 'source_code'"))?;

    fs::write(target_path, source_code).context(format!(
        "Failed to write restored file: {}",
        target_path.display()
    ))?;

    Ok(target_path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::tempdir;

    #[test]
    fn test_parse_backup_file() -> Result<()> {
        let tmp = tempdir()?;
        let backup_dir = tmp.path().join(".gnawtreewriter_backups");
        fs::create_dir_all(&backup_dir)?;

        let file_path = tmp.path().join("a.txt");
        let source_code = "hello world";

        let backup = serde_json::json!({
            "file_path": file_path.to_string_lossy(),
            "timestamp": Utc::now().to_rfc3339(),
            "tree": {},
            "source_code": source_code
        });

        let backup_path = backup_dir.join("b1.json");
        fs::write(&backup_path, serde_json::to_string_pretty(&backup)?)?;

        let parsed = parse_backup_file(&backup_path)?;
        assert_eq!(parsed.original_file_path, file_path);
        assert!(parsed.content_hash.is_some());
        assert_eq!(
            parsed.content_hash.unwrap(),
            crate::core::calculate_content_hash(source_code)
        );

        Ok(())
    }

    #[test]
    fn test_list_and_find_by_hash_and_restore() -> Result<()> {
        let tmp = tempdir()?;
        let backup_dir = tmp.path().join(".gnawtreewriter_backups");
        fs::create_dir_all(&backup_dir)?;

        let file_path = tmp.path().join("f.txt");

        let backup1 = serde_json::json!({
            "file_path": file_path.to_string_lossy(),
            "timestamp": Utc::now().to_rfc3339(),
            "tree": {},
            "source_code": "v1"
        });
        let backup2 = serde_json::json!({
            "file_path": file_path.to_string_lossy(),
            "timestamp": Utc::now().to_rfc3339(),
            "tree": {},
            "source_code": "v2"
        });

        let p1 = backup_dir.join("one.json");
        let p2 = backup_dir.join("two.json");
        fs::write(&p1, serde_json::to_string_pretty(&backup1)?)?;
        fs::write(&p2, serde_json::to_string_pretty(&backup2)?)?;

        let all = list_backup_files(&backup_dir)?;
        assert!(all.len() >= 2);

        let h = crate::core::calculate_content_hash("v2");
        let found = find_backup_by_content_hash(&backup_dir, &h)?;
        assert!(found.is_some());
        let b = found.unwrap();
        assert_eq!(b.content_hash.unwrap(), h);

        // Test restoring into a target file
        let target = tmp.path().join("out.txt");
        restore_from_backup(&p2, &target)?;
        let content = fs::read_to_string(&target)?;
        assert_eq!(content, "v2");

        Ok(())
    }
}
