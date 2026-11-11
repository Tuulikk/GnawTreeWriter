//! Level status for the index — **model-free**, and that is the point.
//!
//! The store has tiers with wildly different costs (measured 2026-11-11):
//!
//! | level | source | cost | reviewable without a model |
//! |---|---|---|---|
//! | L0 disk | filesystem | ms | yes |
//! | L1 AST | tree-sitter | ~40 ms per CHANGED file | **yes** |
//! | L2 graph | L1 | follows L1 | yes |
//! | L3 vectors | bge-base | model-bound (minutes) | coverage yes, ranking no |
//!
//! The rule this module enforces: **the cheap tier defines the expensive tier's
//! work queue** (L1's hash diff is exactly what L3 must re-embed), and the
//! expensive tier may never block a read — only report its coverage.
//!
//! `staleness_queue` alone cannot answer this: it is filled only by the GTW edit
//! path (`mark_stale`, `queued_by = 'gtw-edit'`), so an edit from any other
//! editor looked fresh forever.

use anyhow::Result;
use serde::Serialize;
use std::path::Path;

/// One tier's freshness, as a FACT with a denominator.
#[derive(Debug, Serialize)]
pub struct LevelStatus {
    /// `L1_ast` / `L3_vectors`.
    pub level: String,
    /// What produces it: "tree-sitter", "bge-base-en-v1.5", …
    pub source: String,
    /// Files this level knows about.
    pub total: usize,
    /// Files whose content no longer matches what the level recorded.
    pub stale: usize,
    /// Files the level has nothing for.
    pub uncovered: usize,
    /// Vectors pointing at files that no longer exist (should be 0).
    pub orphaned: usize,
    /// Unix seconds of the newest write this level recorded.
    pub updated_at: Option<i64>,
    /// Human sentence — never a guess.
    pub note: String,
}

#[derive(Debug, Serialize)]
pub struct LevelsReport {
    pub ast: LevelStatus,
    pub vectors: LevelStatus,
    /// One line for humans/agents, printed to stderr so `--json` stays pure.
    pub status_line: String,
}

/// Content fingerprint — the SAME function the vector store records, so the two
/// are comparable. (Not md5: the store uses `calculate_content_hash`, and
/// comparing two different hashes produced a false "0 % current" on this very
/// repo, 2026-11-11.)
fn content_fingerprint(text: &str) -> String {
    crate::core::transaction_log::calculate_content_hash(text)
}

/// md5 hex of a file's bytes — the algorithm `code_index.db` uses for
/// `files.hash`.
fn disk_hash(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(format!("{:x}", md5::compute(&bytes)))
}

/// The vector store's fingerprint for a file as it is on disk now.
fn disk_fingerprint(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|t| content_fingerprint(&t))
}

/// Measure all levels. Reads every indexed file — that IS the honest price of
/// asking "is this fresh?"; the store cannot know without comparing bytes.
pub fn measure(root: &Path) -> Result<LevelsReport> {
    // ── L1: what the AST store recorded vs what is on disk now ────────────
    let mut ast = LevelStatus {
        level: "L1_ast".to_string(),
        source: "tree-sitter".to_string(),
        total: 0,
        stale: 0,
        uncovered: 0,
        orphaned: 0,
        updated_at: None,
        note: String::new(),
    };
    let mut ast_paths: Vec<(String, String)> = Vec::new();
    if crate::core::code_index_db::db_path(root).exists() {
        let conn = crate::core::code_index_db::open_existing(root)?;
        let mut stmt = conn.prepare("SELECT path, hash, parsed_at FROM files")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?;
        for row in rows {
            let (path, hash, parsed_at) = row?;
            ast.total += 1;
            ast.updated_at = Some(ast.updated_at.map_or(parsed_at, |t| t.max(parsed_at)));
            if disk_hash(&root.join(&path)).as_deref() != Some(hash.as_str()) {
                ast.stale += 1;
            }
            ast_paths.push((path, hash));
        }
        ast.note = format!(
            "{} of {} indexed file(s) changed on disk since the last refresh — a refresh costs ~40 ms per changed file",
            ast.stale, ast.total
        );
    } else {
        ast.note = "no store — run `code-index <root> --db`".to_string();
    }

    let vectors = measure_vectors(root, &ast_paths);
    let current = vectors.total.saturating_sub(vectors.stale);
    // checked_div: total 0 (no vector store) yields 0 % instead of panicking.
    let pct = (current * 100).checked_div(vectors.total).unwrap_or(0);
    let status_line = format!(
        "L1 AST {}/{} stale · L3 vectors {}/{} current ({}%) [{} stale, {} uncovered, {} orphaned] · model {}",
        ast.stale,
        ast.total,
        current,
        vectors.total,
        pct,
        vectors.stale,
        vectors.uncovered,
        vectors.orphaned,
        vectors.source
    );

    Ok(LevelsReport {
        ast,
        vectors,
        status_line,
    })
}

/// L3: vectors compared by the hash recorded WITH each vector.
fn measure_vectors(root: &Path, ast_paths: &[(String, String)]) -> LevelStatus {
    let mut v = LevelStatus {
        level: "L3_vectors".to_string(),
        source: "not built".to_string(),
        total: 0,
        stale: 0,
        uncovered: 0,
        orphaned: 0,
        updated_at: None,
        note: String::new(),
    };
    let emb = embeddings_db(root);
    if !emb.exists() {
        v.note = "no vector store — `sense` satellite search has nothing to search; build with `ai index`".to_string();
        // A LEFTOVER store anywhere else in .gnawtreewriter_ai is worse than a
        // missing one: it exists, it opens, and it reports 0 % — a stale binary
        // (older layout) or a second component with a different idea of the
        // store path reads/writes there and the two never notice each other.
        // Measured 2026-11-11: a 0-byte `.gnawtreewriter_ai/embeddings.db`
        // appeared while the live store at `index/embeddings.db` held 276 files.
        let legacy = embeddings_legacy_stores(root);
        if !legacy.is_empty() {
            v.source = format!("not built (⚠ legacy store present: {})", legacy.join(", "));
        }
        return v;
    }
    let Ok(conn) =
        rusqlite::Connection::open_with_flags(&emb, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
    else {
        v.note = format!("vector store unreadable: {}", emb.display());
        return v;
    };
    if let Ok(name) = conn.query_row("SELECT model_name FROM model_info WHERE id = 1", [], |r| {
        r.get::<_, String>(0)
    }) {
        v.source = name;
    }
    let mut by_file: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Ok(mut stmt) =
        conn.prepare("SELECT file_path, MAX(file_hash) FROM embeddings GROUP BY file_path")
    {
        if let Ok(rows) =
            stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        {
            for row in rows.flatten() {
                by_file.insert(row.0, row.1);
            }
        }
    }
    v.total = by_file.len();
    for (path, recorded) in &by_file {
        match disk_fingerprint(&root.join(path)) {
            Some(now) if &now != recorded => v.stale += 1,
            Some(_) => {}
            None => v.orphaned += 1,
        }
    }
    // Files the AST store knows but the vector store never saw.
    v.uncovered = ast_paths
        .iter()
        .filter(|(p, _)| !by_file.contains_key(p))
        .count();
    v.note = format!(
        "{} file(s) embedded · {} stale · {} never embedded · {} vector(s) for files that no longer exist",
        v.total, v.stale, v.uncovered, v.orphaned
    );
    v
}

fn embeddings_db(root: &Path) -> std::path::PathBuf {
    root.join(".gnawtreewriter_ai")
        .join("index")
        .join("embeddings.db")
}

/// Vector stores sitting where vectors do NOT live — one directory up, or in
/// the store root. Named (not silently ignored) so a layout drift shows up as a
/// warning in the status line instead of a plausible-looking 0 %.
fn embeddings_legacy_stores(root: &Path) -> Vec<String> {
    let ai = root.join(".gnawtreewriter_ai");
    let mut found = Vec::new();
    for candidate in [
        ai.join("embeddings.db"),
        ai.join("index").join("index").join("embeddings.db"),
    ] {
        if candidate.exists() {
            found.push(
                candidate
                    .strip_prefix(root)
                    .unwrap_or(&candidate)
                    .display()
                    .to_string(),
            );
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A store in the WRONG directory is worse than no store: it exists, it
    /// opens, and it reports 0 % — a plausible number, so nobody investigates.
    /// Measured 2026-11-11: `.gnawtreewriter_ai/embeddings.db` (0 bytes) next to
    /// the live 276-file store at `.gnawtreewriter_ai/index/embeddings.db`.
    /// The layer must NAME the stray store instead of silently ignoring it.
    #[test]
    fn a_legacy_vector_store_is_named_not_silently_read() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let ai = root.join(".gnawtreewriter_ai");
        std::fs::create_dir_all(&ai).unwrap();
        std::fs::write(ai.join("embeddings.db"), b"").unwrap();

        let report = measure(root).unwrap();
        assert_eq!(report.vectors.total, 0, "the stray store holds no vectors");
        assert!(
            report.vectors.source.contains("legacy store present"),
            "stray store must be named, got: {}",
            report.vectors.source
        );
        assert!(report.vectors.source.contains("embeddings.db"));
        // And it rides into the status line, so `--coverage` cannot look green.
        assert!(report.status_line.contains("legacy store present"));
    }

    /// The invariant this module exists for: `stale` comes from COMPARING BYTES,
    /// so a file changed by ANY tool (not just GTW) shows up.
    #[test]
    fn a_tool_agnostic_change_is_visible_in_the_ast_level() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let ai = root.join(".gnawtreewriter_ai");
        std::fs::create_dir_all(&ai).unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        let file = root.join("src/a.rs");
        std::fs::write(&file, "fn a() {}\n").unwrap();

        let conn = rusqlite::Connection::open(ai.join("code_index.db")).unwrap();
        conn.execute_batch(
            "CREATE TABLE files (path TEXT PRIMARY KEY, hash TEXT NOT NULL, lines INTEGER NOT NULL, warning TEXT, parsed_at INTEGER NOT NULL);",
        )
        .unwrap();
        let h = disk_hash(&file).unwrap();
        conn.execute(
            "INSERT INTO files (path, hash, lines, parsed_at) VALUES ('src/a.rs', ?1, 1, 1000)",
            rusqlite::params![h],
        )
        .unwrap();
        drop(conn);

        let clean = measure(root).unwrap();
        assert_eq!(clean.ast.stale, 0, "an unchanged file must not be stale");

        // Change it WITHOUT GnawTreeWriter — the point of the whole test.
        std::fs::write(&file, "fn a() { let x = 1; }\n").unwrap();
        let dirty = measure(root).unwrap();
        assert_eq!(dirty.ast.stale, 1, "a printf/vim edit must show as stale");
        assert!(dirty.status_line.contains("L1 AST 1/1 stale"));
    }

    /// No vector store is a STATE, not an error — and it must say which.
    #[test]
    fn a_missing_vector_store_is_reported_not_silent() {
        let tmp = tempfile::tempdir().unwrap();
        let report = measure(tmp.path()).unwrap();
        assert_eq!(report.vectors.total, 0);
        assert!(report.vectors.note.contains("no vector store"));
    }
}
