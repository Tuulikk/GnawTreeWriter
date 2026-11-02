//! code-index SQLite store — Phase 1 of `docs/CODE_INDEX_SQLITE_DESIGN.md`
//! (review 2026-11-02 decisions baked in).
//!
//! WHY: the committed markdown is a projection, not a query surface. The DB
//! (`.gnawtreewriter_ai/code_index.db`, gitignored) is the per-clone machine
//! layer: incremental upserts (parse only changed files) and a render path
//! byte-identical to the direct one (tested invariant, decision #5).
//!
//! Ownership rules that keep it honest:
//! - only `refresh` (via `--db`) creates the DB; other components may only
//!   open an existing one (decision #8);
//! - recovery deletes `db` + `-wal` + `-shm` and is surfaced as a notice,
//!   never silent (decision #3);
//! - reuse requires GENERATION identity, not just content (decision #1):
//!   extractor version and population (extensions) are part of the key,
//!   presentation options are not.

use crate::core::code_index::{
    canonical_rel_path, extract_items, git_facts, render_doc, CodeIndexOptions, FileIndex,
    IndexItem,
};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Bump on ANY change to item extraction output — part of the generation
/// key, so stale rows never survive an extractor change silently.
pub const EXTRACTOR_VERSION: &str = "1";

const SCHEMA_VERSION: i32 = 1;
const BUSY_TIMEOUT_MS: u64 = 5000;

const SCHEMA_V1: &str = r#"
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS files (
    path TEXT PRIMARY KEY,
    hash TEXT NOT NULL,
    lines INTEGER NOT NULL,
    warning TEXT,
    parsed_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS items (
    id INTEGER PRIMARY KEY,
    file TEXT NOT NULL REFERENCES files(path) ON DELETE CASCADE,
    parent_id INTEGER REFERENCES items(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    signature TEXT NOT NULL,
    line INTEGER NOT NULL,
    node_path TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_items_file ON items(file);
CREATE TABLE IF NOT EXISTS staleness_queue (
    file TEXT PRIMARY KEY,
    reason TEXT NOT NULL,
    from_rev TEXT,
    queued_by TEXT,
    queued_at INTEGER NOT NULL
);
"#;

pub struct RefreshStats {
    pub files_total: usize,
    pub files_parsed: usize,
    pub files_reused: usize,
    pub files_removed: usize,
    /// Full rebuild happened (generation change or corruption recovery).
    pub full_rebuild: bool,
}

pub struct DbRun {
    pub doc: String,
    pub stats: RefreshStats,
    /// Surfaced as trailing warning lines in the artifact (recovery, root
    /// normalization) — never silent (decisions #3, #10).
    pub notices: Vec<String>,
}

pub fn db_path(root: &Path) -> PathBuf {
    root.join(".gnawtreewriter_ai").join("code_index.db")
}

/// Resolve the index root: git toplevel when `root` is inside a work tree —
/// one clone, one DB, same population as the committed markdown
/// (decision #10). No git: the walked root as-is.
pub fn resolve_index_root(root: &Path) -> PathBuf {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|o| o.status.success());
    match out {
        Some(o) => {
            let top = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if top.is_empty() {
                root.to_path_buf()
            } else {
                PathBuf::from(top)
            }
        }
        None => root.to_path_buf(),
    }
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

fn open_and_migrate(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_millis(BUSY_TIMEOUT_MS))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    let v: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if v > SCHEMA_VERSION {
        anyhow::bail!(
            "code-index DB schema v{} is newer than this binary supports (v{}) — upgrade gnawtreewriter",
            v,
            SCHEMA_VERSION
        );
    }
    if v < SCHEMA_VERSION {
        conn.execute_batch(SCHEMA_V1)?;
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    }
    Ok(conn)
}

/// Open the DB, or on corruption rebuild from scratch. Recovery removes
/// `db` + `-wal` + `-shm` (decision #3); the flag lets callers surface it.
pub fn open_or_recreate(root: &Path) -> Result<(Connection, bool)> {
    let path = db_path(root);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).context("creating .gnawtreewriter_ai")?;
    }
    match open_and_migrate(&path) {
        Ok(conn) => Ok((conn, false)),
        Err(first) => {
            for p in [path.clone(), sidecar(&path, "-wal"), sidecar(&path, "-shm")] {
                let _ = std::fs::remove_file(&p);
            }
            let conn = open_and_migrate(&path).with_context(|| {
                format!("re-initializing code-index DB after corruption (first error: {first:#})")
            })?;
            Ok((conn, true))
        }
    }
}

/// Action-key (decision #1): identity of everything that affects extraction.
/// Presentation options (max_sig, json, output) are deliberately absent —
/// signatures are stored untruncated and truncated at render.
fn generation_key(opts: &CodeIndexOptions) -> String {
    let mut exts = opts.extensions.clone();
    exts.sort();
    exts.dedup();
    let raw = format!("{SCHEMA_VERSION}|{EXTRACTOR_VERSION}|{}", exts.join(","));
    format!("{:x}", md5::compute(raw.as_bytes()))
}

fn meta_get(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM meta WHERE key = ?1")?;
    let mut rows = stmt.query(params![key])?;
    Ok(match rows.next()? {
        Some(row) => Some(row.get(0)?),
        None => None,
    })
}

fn meta_set(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = ?2",
        params![key, value],
    )?;
    Ok(())
}

/// Replace one file's rows atomically (files row + item rows, cascades
/// clean the old ones).
fn upsert_file(
    conn: &mut Connection,
    rel: &str,
    hash: &str,
    lines: usize,
    warning: Option<&str>,
    items: &[IndexItem],
    now: i64,
) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM files WHERE path = ?1", params![rel])?;
    tx.execute(
        "INSERT INTO files (path, hash, lines, warning, parsed_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![rel, hash, lines, warning, now],
    )?;
    {
        let mut top = tx.prepare(
            "INSERT INTO items (file, parent_id, kind, name, signature, line, node_path)
             VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6)",
        )?;
        let mut child = tx.prepare(
            "INSERT INTO items (file, parent_id, kind, name, signature, line, node_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for item in items {
            top.execute(params![
                rel,
                item.category,
                item.name,
                item.sig,
                item.line as i64,
                item.node_path
            ])?;
            let parent_id = tx.last_insert_rowid();
            for m in &item.methods {
                child.execute(params![
                    rel,
                    parent_id,
                    m.category,
                    m.name,
                    m.sig,
                    m.line as i64,
                    m.node_path
                ])?;
            }
        }
    }
    tx.commit()?;
    Ok(())
}

/// Incremental refresh: parse only files whose content hash changed (or
/// everything, on generation change). Rows for removed files are deleted.
/// The staleness queue drains on success (decision #8). `meta` is written
/// LAST so an aborted refresh heals on the next run (decision #7).
pub fn refresh(
    conn: &mut Connection,
    root: &Path,
    opts: &CodeIndexOptions,
) -> Result<RefreshStats> {
    let want = generation_key(opts);
    let full_rebuild = meta_get(conn, "generation_key")?.as_deref() != Some(want.as_str());
    if full_rebuild {
        // Planned full re-upsert (generation changed). Rows are replaced per
        // file anyway; meta keeps the old generation until the pass
        // completes, so a mid-run abort is detected and healed next time.
        conn.execute("DELETE FROM staleness_queue", [])?;
        conn.execute("DELETE FROM files", [])?;
    }

    let exts: Vec<&str> = opts.extensions.iter().map(|s| s.as_str()).collect();
    let mut files = crate::core::file_walker::walk_source_files_filtered(root, &exts);
    if files.is_empty() {
        anyhow::bail!(
            "no source files found under {} (extensions: {}) — wrong root?",
            root.display(),
            exts.join(",")
        );
    }
    files.sort();

    let known: BTreeMap<String, String> = {
        let mut stmt = conn.prepare("SELECT path, hash FROM files")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        let mut m = BTreeMap::new();
        for row in rows {
            let (p, h) = row?;
            m.insert(p, h);
        }
        m
    };

    let now = chrono::Utc::now().timestamp();
    let (rev, _dirty) = git_facts(root);

    let mut stats = RefreshStats {
        files_total: files.len(),
        files_parsed: 0,
        files_reused: 0,
        files_removed: 0,
        full_rebuild,
    };

    let mut current: HashSet<String> = HashSet::new();
    for path in &files {
        let rel = canonical_rel_path(root, path);
        current.insert(rel.clone());
        let source =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let hash = format!("{:x}", md5::compute(source.as_bytes()));
        if !full_rebuild && known.get(&rel).map(String::as_str) == Some(hash.as_str()) {
            stats.files_reused += 1;
            continue;
        }
        let parser = crate::parser::get_parser(path)?;
        // Lenient parse — same READ-path discipline as the direct generator.
        let (res, warning) = parser.parse_lenient(&source);
        let tree = res.with_context(|| format!("parsing {}", path.display()))?;
        let warning_line = warning.map(|w| format!("{rel} ({} @ line {})", w.message, w.line));
        let lines = source.lines().count();
        let items = extract_items(&tree);
        upsert_file(
            conn,
            &rel,
            &hash,
            lines,
            warning_line.as_deref(),
            &items,
            now,
        )?;
        stats.files_parsed += 1;
    }

    for old in known.keys() {
        if !current.contains(old) {
            conn.execute("DELETE FROM files WHERE path = ?1", params![old])?;
            stats.files_removed += 1;
        }
    }

    // A completed refresh leaves nothing stale: every walked file is fresh.
    conn.execute("DELETE FROM staleness_queue", [])?;
    // Leave a checkpointed file behind so read-only consumers (mode=ro)
    // see a clean DB (decision #13). Best-effort.
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    meta_set(conn, "generation_key", &want)?;
    meta_set(conn, "extractor_version", EXTRACTOR_VERSION)?;
    meta_set(conn, "schema_version", &SCHEMA_VERSION.to_string())?;
    meta_set(conn, "gtw_version", env!("CARGO_PKG_VERSION"))?;
    meta_set(conn, "last_refresh_rev", &rev)?;
    meta_set(conn, "last_refresh_at", &chrono::Utc::now().to_rfc3339())?;
    Ok(stats)
}

/// Load the stored index back into the shared render shape. Items come back
/// in line order; methods reattach to their parent row. Orphaned children
/// (impossible in practice — cascades prevent them) are promoted, never
/// dropped: an index that hides items lies.
pub(crate) fn load_index(conn: &Connection) -> Result<Vec<FileIndex>> {
    let mut stmt = conn.prepare(
        "SELECT f.path, f.lines, i.id, i.parent_id, i.kind, i.name, i.signature, i.line, i.node_path
         FROM files f
         LEFT JOIN items i ON i.file = f.path
         ORDER BY f.path, i.line",
    )?;
    // NULL-tolerant mapper: a file with zero items still produces one row
    // with all-NULL item columns (LEFT JOIN).
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, Option<i64>>(2)?,
            r.get::<_, Option<i64>>(3)?,
            r.get::<_, Option<String>>(4)?,
            r.get::<_, Option<String>>(5)?,
            r.get::<_, Option<String>>(6)?,
            r.get::<_, Option<i64>>(7)?,
            r.get::<_, Option<String>>(8)?,
        ))
    })?;

    let mut out: Vec<FileIndex> = Vec::new();
    let mut by_file: HashMap<String, usize> = HashMap::new();
    let mut ids: HashMap<i64, (usize, usize)> = HashMap::new();
    for row in rows {
        let (path, lines, id, parent_id, kind, name, sig, line, node_path) = row?;
        let idx = match by_file.get(&path) {
            Some(i) => *i,
            None => {
                out.push(FileIndex {
                    path: path.clone(),
                    lines: lines as usize,
                    items: Vec::new(),
                });
                let idx = out.len() - 1;
                by_file.insert(path.clone(), idx);
                idx
            }
        };
        let Some(id) = id else { continue }; // LEFT JOIN: file with no items
        let item = IndexItem {
            category: kind.unwrap_or_default(),
            name: name.unwrap_or_default(),
            sig: sig.unwrap_or_default(),
            line: line.unwrap_or_default() as usize,
            node_path: node_path.unwrap_or_default(),
            methods: Vec::new(),
        };
        let target = match parent_id {
            None => None,
            Some(p) => ids.get(&p).copied(),
        };
        match target {
            Some((fi, ii)) => out[fi].items[ii].methods.push(item),
            None => {
                out[idx].items.push(item);
                ids.insert(id, (idx, out[idx].items.len() - 1));
            }
        }
    }
    Ok(out)
}

fn load_warnings(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT warning FROM files WHERE warning IS NOT NULL ORDER BY path")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Render the projection from the DB — byte-identical to the direct path
/// modulo stamp lines (tested invariant, decision #5).
pub fn render(
    conn: &Connection,
    root: &Path,
    opts: &CodeIndexOptions,
    notices: &[String],
) -> Result<String> {
    let indexed = load_index(conn)?;
    let warned = load_warnings(conn)?;
    let (rev, dirty) = git_facts(root);
    render_doc(&indexed, &warned, &rev, dirty, opts, root, notices)
}

/// The `--db` code path: resolve root, open (recovering if needed),
/// refresh incrementally, render from the store.
pub fn generate_with_db(root: &Path, opts: &CodeIndexOptions) -> Result<DbRun> {
    let index_root = resolve_index_root(root);
    let mut notices = Vec::new();
    if index_root != root {
        notices.push(format!(
            "indexroten normaliserad till git-toplevel: {}",
            index_root.display()
        ));
    }
    let (mut conn, recovered) = open_or_recreate(&index_root)?;
    if recovered {
        notices.push(format!(
            "index-databasen var korrupt — ombyggd {} UTC",
            chrono::Utc::now().format("%Y-%m-%d %H:%M")
        ));
    }
    let stats = refresh(&mut conn, &index_root, opts)?;
    let doc = render(&conn, &index_root, opts, &notices)?;
    Ok(DbRun {
        doc,
        stats,
        notices,
    })
}

/// P5: classify lock-class failures so the caller can degrade to the direct
/// path instead of failing the operation.
pub fn is_busy_error(err: &anyhow::Error) -> bool {
    for cause in err.chain() {
        if let Some(rusqlite::Error::SqliteFailure(err, _)) =
            cause.downcast_ref::<rusqlite::Error>()
        {
            if matches!(
                err.code,
                rusqlite::ffi::ErrorCode::DatabaseBusy | rusqlite::ffi::ErrorCode::DatabaseLocked
            ) {
                return true;
            }
        }
    }
    false
}
