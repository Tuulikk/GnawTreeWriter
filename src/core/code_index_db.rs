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
pub const EXTRACTOR_VERSION: &str = "2";

const SCHEMA_VERSION: i32 = 4;
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

/// Schema v2 (T6 + Phase 3 foundation): knowledge-graph tables born in the
/// SAME db (consolidation rule — never a fifth index family). Relations
/// carry the honest target split: to_name always, to_id only on unique
/// resolution (FK-enforced); unresolved targets aggregate into symbols.
/// entities mirror the canonical gtw ids (qualified methods, #N suffix).
const SCHEMA_V2: &str = r#"
CREATE TABLE IF NOT EXISTS entities (
    id TEXT PRIMARY KEY,
    file TEXT NOT NULL,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    line INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_entities_file ON entities(file);
CREATE TABLE IF NOT EXISTS relations (
    id INTEGER PRIMARY KEY,
    from_id TEXT NOT NULL,
    to_name TEXT NOT NULL,
    to_id TEXT,
    relation_type TEXT NOT NULL,
    line INTEGER NOT NULL,
    file TEXT NOT NULL,
    FOREIGN KEY(to_id) REFERENCES entities(id)
);
CREATE INDEX IF NOT EXISTS idx_relations_file ON relations(file);
CREATE TABLE IF NOT EXISTS symbols (
    name TEXT NOT NULL,
    file TEXT NOT NULL,
    external INTEGER NOT NULL DEFAULT 0,
    occurrences INTEGER NOT NULL,
    PRIMARY KEY (name, file)
);
"#;

/// Schema v3 (Phase 3 completion): entities carry the GTW dot-path so the
/// DB graph can serve definitions/from_path — the read_node chain
/// (code-index → read_node → edit_node) survives the JSON deprecation.
const SCHEMA_V3: &str = r#"
ALTER TABLE entities ADD COLUMN node_path TEXT NOT NULL DEFAULT '';
"#;

/// Identity scheme version (D2/D6, phase 2 lock 2026-11-10): the revision
/// delta KEY algorithm. Any change here is a HISTORY BREAK — the next
/// refresh starts a new baseline (old deltas keep their meaning only
/// under the scheme that wrote them). Third component of the generation
/// key, next to schema and extractor versions.
pub(crate) const IDENTITY_SCHEME_VERSION: &str = "1";

/// Schema v4 (phase 2, proposal D1/D5/D7-D9): revision history —
/// append-only revision chain + per-file versions + AGGREGATED deltas
/// (relation_key = from_entity|type|target_key with count_before/after
/// per D8; callsite anchors live in the before/after JSON). Additive:
/// existing tables untouched; PRAGMA user_version 3 -> 4.
const SCHEMA_V4: &str = r#"
CREATE TABLE IF NOT EXISTS index_revisions (
    id INTEGER PRIMARY KEY,
    parent_id INTEGER REFERENCES index_revisions(id),
    git_rev TEXT NOT NULL,
    dirty INTEGER NOT NULL,
    source TEXT NOT NULL,
    identity_scheme_version TEXT NOT NULL,
    generation_key TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    status TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_revisions_parent ON index_revisions(parent_id);
CREATE TABLE IF NOT EXISTS revisions_tx_links (
    revision_id INTEGER NOT NULL REFERENCES index_revisions(id),
    transaction_id TEXT NOT NULL,
    PRIMARY KEY (revision_id, transaction_id)
);
CREATE TABLE IF NOT EXISTS file_versions (
    revision_id INTEGER NOT NULL REFERENCES index_revisions(id),
    path TEXT NOT NULL,
    before_hash TEXT,
    after_hash TEXT,
    parse_status TEXT NOT NULL,
    PRIMARY KEY (revision_id, path)
);
CREATE TABLE IF NOT EXISTS entity_changes (
    revision_id INTEGER NOT NULL REFERENCES index_revisions(id),
    entity_key TEXT NOT NULL,
    change_kind TEXT NOT NULL,
    before_json TEXT,
    after_json TEXT,
    PRIMARY KEY (revision_id, entity_key)
);
CREATE TABLE IF NOT EXISTS relation_changes (
    revision_id INTEGER NOT NULL REFERENCES index_revisions(id),
    relation_key TEXT NOT NULL,
    change_kind TEXT NOT NULL,
    count_before INTEGER NOT NULL DEFAULT 0,
    count_after INTEGER NOT NULL DEFAULT 0,
    before_json TEXT,
    after_json TEXT,
    resolution_status TEXT NOT NULL,
    PRIMARY KEY (revision_id, relation_key)
);
CREATE INDEX IF NOT EXISTS idx_relation_changes_key ON relation_changes(relation_key);
CREATE TABLE IF NOT EXISTS prose_annotations (
    id INTEGER PRIMARY KEY,
    anchor_kind TEXT NOT NULL,
    anchor_key TEXT NOT NULL,
    anchor_hash TEXT NOT NULL,
    body TEXT NOT NULL,
    producer TEXT NOT NULL,
    model TEXT,
    language TEXT NOT NULL,
    revision_id INTEGER REFERENCES index_revisions(id),
    generated_at INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'fresh',
    invalid_reason TEXT,
    suggested_key TEXT,
    UNIQUE (anchor_kind, anchor_key, producer, language)
);
CREATE INDEX IF NOT EXISTS idx_prose_status ON prose_annotations(status);
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
    /// stderr lines (root normalization, recovery mirror) — operational
    /// context. The artifact body carries only content-relevant notices
    /// (recovery), keeping the committed projection gate-stable
    /// (decisions #3, #10).
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
    migrate_connection(&conn)?;
    Ok(conn)
}

/// Schema setup shared by the file-backed open path and in-memory test
/// connections — one DDL truth for both (the coverage tests reuse the
/// exact production schema, not a copy).
pub(crate) fn migrate_connection(conn: &Connection) -> Result<()> {
    let v: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if v > SCHEMA_VERSION {
        anyhow::bail!(
            "code-index DB schema v{} is newer than this binary supports (v{}) — upgrade gnawtreewriter",
            v,
            SCHEMA_VERSION
        );
    }
    if v < 1 {
        conn.execute_batch(SCHEMA_V1)?;
    }
    if v < 2 {
        conn.execute_batch(SCHEMA_V2)?;
    }
    if v < 3 {
        // ALTER is NOT idempotent — stays version-gated (duplicate column
        // would abort otherwise).
        conn.execute_batch(SCHEMA_V3)?;
    }
    if v < 4 {
        conn.execute_batch(SCHEMA_V4)?;
    }
    // Phase 2 lock 7 correction (2026-11-10): CREATE-only batches are
    // idempotent (CREATE ... IF NOT EXISTS), so they run on EVERY open —
    // a store already at v4 that predates a later additive table (e.g.
    // prose_annotations, committed mid-increment by a commit-hook refresh
    // with an older binary) heals here instead of erroring downstream.
    conn.execute_batch(SCHEMA_V4)?;
    repair_prose_columns(conn)?;
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

/// Additive-column repair for pre-release v4 stores that carried the
/// prose table before later columns existed (0 rows, unreleased schema —
/// ALTER with DEFAULT is safe). Idempotent by PRAGMA table_info check.
fn repair_prose_columns(conn: &Connection) -> Result<()> {
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'prose_annotations'",
            [],
            |r| r.get::<_, i64>(0).map(|c| c > 0),
        )
        .unwrap_or(false);
    if !exists {
        return Ok(());
    }
    let cols: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(prose_annotations)")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
        rows.flatten().collect()
    };
    if !cols.iter().any(|c| c == "anchor_name") {
        conn.execute(
            "ALTER TABLE prose_annotations ADD COLUMN anchor_name TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    if !cols.iter().any(|c| c == "suggested_key") {
        conn.execute(
            "ALTER TABLE prose_annotations ADD COLUMN suggested_key TEXT",
            [],
        )?;
    }
    if !cols.iter().any(|c| c == "invalid_reason") {
        conn.execute(
            "ALTER TABLE prose_annotations ADD COLUMN invalid_reason TEXT",
            [],
        )?;
    }
    Ok(())
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

/// Open an EXISTING store — never creates it (decision #8: only refresh
/// may create). For query/doctor paths.
pub fn open_existing(root: &Path) -> Result<Connection> {
    let path = db_path(root);
    if !path.exists() {
        anyhow::bail!(
            "no code-index store at {} — run `code-index --db` (refresh) first",
            path.display()
        );
    }
    let conn = Connection::open(&path)?;
    conn.busy_timeout(Duration::from_millis(BUSY_TIMEOUT_MS))?;
    let v: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if v > SCHEMA_VERSION {
        anyhow::bail!(
            "code-index DB schema v{} is newer than this binary (v{}) — upgrade gnawtreewriter",
            v,
            SCHEMA_VERSION
        );
    }
    Ok(conn)
}

/// Doctor-facing store summary (read-only). None when no store exists.
#[derive(Debug)]
pub struct StoreStatus {
    pub last_refresh_rev: Option<String>,
    pub queued: usize,
    pub files: usize,
}

/// Read the store status; None when there is no store. Never creates one.
pub fn store_status(root: &Path) -> Option<StoreStatus> {
    let conn = open_existing(root).ok()?;
    let last_refresh_rev = meta_get(&conn, "last_refresh_rev").ok().flatten();
    let queued = conn
        .query_row("SELECT COUNT(*) FROM staleness_queue", [], |r| {
            r.get::<_, i64>(0)
        })
        .ok()? as usize;
    let files = conn
        .query_row("SELECT COUNT(*) FROM files", [], |r| r.get::<_, i64>(0))
        .ok()? as usize;
    Some(StoreStatus {
        last_refresh_rev,
        queued,
        files,
    })
}

/// Action-key (decision #1): identity of everything that affects extraction.
/// Presentation options (max_sig, json, output) are deliberately absent —
/// signatures are stored untruncated and truncated at render.
fn generation_key(opts: &CodeIndexOptions) -> String {
    let mut exts = opts.extensions.clone();
    exts.sort();
    exts.dedup();
    let raw = format!(
        "{SCHEMA_VERSION}|{EXTRACTOR_VERSION}|{IDENTITY_SCHEME_VERSION}|{}",
        exts.join(",")
    );
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
        conn.execute("DELETE FROM relations", [])?;
        conn.execute("DELETE FROM entities", [])?;
        conn.execute("DELETE FROM symbols", [])?;
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
        // T6/Phase 3: the knowledge graph rides the same parse — entities
        // + relations stored per file (same-file resolution inline; the
        // cross-file pass runs after the loop).
        let ent_index =
            crate::core::index_entities::extract_entities_from_tree(&tree, &source, &rel, true);
        let mut rel_index =
            crate::core::index_relations::extract_relations_from_tree(&tree, &source, &rel);
        crate::core::index_relations::resolve_relations(
            &mut rel_index.relations,
            &ent_index.entities,
        );
        store_entities_relations(conn, &rel, &ent_index.entities, &rel_index.relations)?;
        stats.files_parsed += 1;
    }

    for old in known.keys() {
        if !current.contains(old) {
            // Null out cross-file references BEFORE dropping the file's
            // entities (FK order), then clean all three tables — removed
            // files must not leak rows.
            conn.execute(
                "UPDATE relations SET to_id = NULL WHERE to_id IN (SELECT id FROM entities WHERE file = ?1)",
                params![old],
            )?;
            conn.execute("DELETE FROM relations WHERE file = ?1", params![old])?;
            conn.execute("DELETE FROM entities WHERE file = ?1", params![old])?;
            conn.execute("DELETE FROM files WHERE path = ?1", params![old])?;
            stats.files_removed += 1;
        }
    }

    // Cross-file resolution + symbol aggregation (T6/review #12): every
    // relation the per-file pass left NULL gets one shot against the FULL
    // entity set; what remains unresolved is a symbol, not a reference.
    resolve_unresolved_in_db(conn)?;
    conn.execute("DELETE FROM symbols", [])?;
    conn.execute_batch(
        "INSERT INTO symbols (name, file, external, occurrences)
         SELECT to_name, file, (to_name GLOB '*::*'), COUNT(*)
         FROM relations WHERE to_id IS NULL
         GROUP BY to_name, file;",
    )?;
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

/// Phase 2 skeleton (D5 lock (a)): record one COMPLETED revision — the
/// chain (parent = previous completed revision), git facts, identity
/// scheme version and per-file snapshots (after-only at this scaffolding
/// stage; before/after deltas arrive with the single-tx wrap where the
/// pre-refresh state is naturally in scope). Failed refreshes record
/// nothing yet — the dedicated failure path lands with the wrap.
pub(crate) fn record_revision(
    conn: &mut Connection,
    root: &Path,
    source: &str,
) -> Result<i64> {
    let (rev, dirty) = git_facts(root);
    let now = chrono::Utc::now().timestamp();
    let gk = meta_get(conn, "generation_key")?.unwrap_or_default();
    let parent: Option<i64> = conn
        .query_row(
            "SELECT id FROM index_revisions WHERE status = 'completed' ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .ok();
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO index_revisions (parent_id, git_rev, dirty, source, identity_scheme_version, generation_key, started_at, completed_at, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, 'completed')",
        params![parent, rev, dirty as i64, source, IDENTITY_SCHEME_VERSION, gk, now],
    )?;
    let rid = tx.last_insert_rowid();
    {
        let mut stmt = tx.prepare(
            "INSERT INTO file_versions (revision_id, path, before_hash, after_hash, parse_status)
             SELECT ?1, path, NULL, hash,
                    CASE WHEN warning IS NULL THEN 'ok' ELSE 'warning' END
             FROM files",
        )?;
        stmt.execute(params![rid])?;
    }
    tx.commit()?;
    Ok(rid)
}

/// Phase 2 prose layer (optional enrichment, ROADMAP 2026-11-10): the
/// producer path — local model, agents or background tools write CLAIMS
/// here. The index owns only the FACTS about those claims (anchor + hash +
/// status); prose never gates anything. Upsert on
/// (anchor_kind, anchor_key, producer, language): re-generating the same
/// claim resets it to fresh.
#[allow(clippy::too_many_arguments)]
pub fn upsert_prose(
    conn: &mut Connection,
    anchor_kind: &str,
    anchor_key: &str,
    anchor_hash: &str,
    anchor_name: &str,
    body: &str,
    producer: &str,
    model: Option<&str>,
    language: &str,
    revision_id: Option<i64>,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO prose_annotations (anchor_kind, anchor_key, anchor_hash, anchor_name, body, producer, model, language, revision_id, generated_at, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'fresh')
         ON CONFLICT(anchor_kind, anchor_key, producer, language) DO UPDATE SET
           body = excluded.body, anchor_hash = excluded.anchor_hash,
           anchor_name = excluded.anchor_name, model = excluded.model,
           revision_id = excluded.revision_id, generated_at = excluded.generated_at,
           status = 'fresh', invalid_reason = NULL, suggested_key = NULL",
        params![anchor_kind, anchor_key, anchor_hash, anchor_name, body, producer, model, language, revision_id, now],
    )?;
    Ok(())
}

/// Deterministic freshness sweep (index-side FACTS about prose claims —
/// the user-facing semantics: stale = "ej färsk" (content moved, identity
/// kept), invalid = "ej gällande" (anchor provably gone)). Runs after each
/// completed revision. Reports, never gates.
pub(crate) fn sweep_prose_freshness(conn: &mut Connection) -> Result<()> {
    let rows: Vec<(i64, String, String, String, String)> = {
        let mut stmt = conn.prepare(
            "SELECT id, anchor_kind, anchor_key, anchor_hash, anchor_name
             FROM prose_annotations WHERE status != 'invalid'",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?;
        rows.collect::<std::result::Result<_, _>>()?
    };
    // Owned strings — the successors lookup lives in a narrower scope than
    // the deferred updates (E0597 lesson: borrow the decision, own the data).
    let mut updates: Vec<(i64, &'static str, Option<String>, Option<String>)> = Vec::new();
    for (id, kind, key, anchor_hash, _anchor_name) in &rows {
        match kind.as_str() {
            "file" => {
                let h = conn.query_row(
                    "SELECT hash FROM files WHERE path = ?1",
                    params![key],
                    |r| r.get::<_, String>(0),
                );
                match h {
                    Ok(h) if h == *anchor_hash => {}
                    Ok(_) => updates.push((*id, "stale", Some("content_changed".to_string()), None)),
                    Err(_) => updates.push((*id, "invalid", Some("anchor_removed".to_string()), None)),
                }
            }
            "entity" => {
                let file = conn.query_row(
                    "SELECT file FROM entities WHERE id = ?1",
                    params![key],
                    |r| r.get::<_, String>(0),
                );
                match file {
                    Ok(f) => {
                        let h = conn.query_row(
                            "SELECT hash FROM files WHERE path = ?1",
                            params![f],
                            |r| r.get::<_, String>(0),
                        );
                        match h {
                            Ok(h) if h == *anchor_hash => {}
                            Ok(_) => {
                                updates.push((*id, "stale", Some("content_changed".to_string()), None))
                            }
                            Err(_) => {
                                updates.push((*id, "stale", Some("anchor_file_missing".to_string()), None))
                            }
                        }
                    }
                    Err(_) => {
                        // Anchor gone: a same-NAMED successor (bare name =
                        // last ':' segment of the id) becomes the suggested
                        // re-anchor when it is unique — never a guess.
                        let bare = key.rsplit(':').next().unwrap_or("");
                        let successors: Vec<String> = {
                            let mut stmt = conn.prepare("SELECT id FROM entities WHERE name = ?1")?;
                            let rows = stmt.query_map(params![bare], |r| r.get::<_, String>(0))?;
                            rows.flatten().collect()
                        };
                        if successors.len() == 1 {
                            updates.push((
                                *id,
                                "invalid",
                                Some("anchor_removed".to_string()),
                                Some(successors[0].clone()),
                            ));
                        } else {
                            updates.push((*id, "invalid", Some("anchor_removed".to_string()), None));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    for (id, status, reason, suggested) in &updates {
        conn.execute(
            "UPDATE prose_annotations SET status = ?1, invalid_reason = ?2, suggested_key = ?3 WHERE id = ?4",
            params![status, reason, suggested, id],
        )?;
    }
    Ok(())
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
    // stderr notices: operational context (never in the artifact — the
    // committed body must stay byte-stable for the --check gate).
    let mut notices = Vec::new();
    // artifact notices: content-relevant events (surface in the body).
    let mut artifact_notices = Vec::new();
    if index_root != root {
        notices.push(format!(
            "indexroten normaliserad till git-toplevel: {}",
            index_root.display()
        ));
    }
    let (mut conn, recovered) = open_or_recreate(&index_root)?;
    if recovered {
        let line = format!(
            "index-databasen var korrupt — ombyggd {} UTC",
            chrono::Utc::now().format("%Y-%m-%d %H:%M")
        );
        artifact_notices.push(line.clone());
        notices.push(line);
    }
    let stats = refresh(&mut conn, &index_root, opts)?;
    // Phase 2: one revision row per completed refresh — the chain
    // (parent = previous completed), git facts, identity version. Source
    // via GTW_CODE_INDEX_SOURCE (git_hook sets it in the installed hook).
    let source = std::env::var("GTW_CODE_INDEX_SOURCE")
        .unwrap_or_else(|_| "refresh_manual".to_string());
    if let Err(e) = record_revision(&mut conn, &index_root, &source) {
        notices.push(format!("revisionspost kunde inte skrivas: {e:#}"));
    }
    // Freshness sweep after the revision: prose claims are facts-checked
    // against the NEW store state (fresh / stale / invalid — D9-family
    // semantics, reports never gates).
    if let Err(e) = sweep_prose_freshness(&mut conn) {
        notices.push(format!("prosa-färskhet kunde inte uppdateras: {e:#}"));
    }
    let doc = render(&conn, &index_root, opts, &artifact_notices)?;
    Ok(DbRun {
        doc,
        stats,
        notices,
    })
}

/// Store one file's entities + relations (T6/Phase 3). from_id is plain
/// text (synthetic callsite ids are not entity rows); the FK lives on
/// to_id only. Entities insert before relations so same-file to_ids
/// satisfy the FK inside the transaction.
fn store_entities_relations(
    conn: &mut Connection,
    rel: &str,
    entities: &[crate::core::index_entities::Entity],
    relations: &[crate::core::index_relations::Relation],
) -> Result<()> {
    let tx = conn.transaction()?;
    // Stale cross-file references first: this file's entities are about
    // to be replaced — other files' relations pointing at them lose
    // their to_id (the after-loop pass re-resolves them). FK has no
    // cascade by design: a relation row belongs to its caller file.
    tx.execute(
        "UPDATE relations SET to_id = NULL WHERE to_id IN (SELECT id FROM entities WHERE file = ?1)",
        params![rel],
    )?;
    tx.execute("DELETE FROM entities WHERE file = ?1", params![rel])?;
    tx.execute("DELETE FROM relations WHERE file = ?1", params![rel])?;
    {
        let mut ent = tx.prepare(
            "INSERT INTO entities (id, file, kind, name, line, node_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        fn flatten<'a>(
            e: &'a crate::core::index_entities::Entity,
            out: &mut Vec<(&'a str, &'a str, &'a str, i64, &'a str)>,
        ) {
            out.push((
                e.id.as_str(),
                e.entity_type.as_str(),
                e.name.as_str(),
                e.line as i64,
                e.path.as_str(),
            ));
            for c in &e.children {
                flatten(c, out);
            }
        }
        for e in entities {
            let mut rows = Vec::new();
            flatten(e, &mut rows);
            for (id, kind, name, line, node_path) in rows {
                ent.execute(params![id, rel, kind, name, line, node_path])?;
            }
        }
        let mut rels = tx.prepare(
            "INSERT INTO relations (from_id, to_name, to_id, relation_type, line, file)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for x in relations {
            rels.execute(params![
                x.from,
                x.to_name,
                x.to_id,
                x.relation_type,
                x.line as i64,
                rel
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// One resolution shot for all still-unresolved relations against the
/// FULL entity set (cross-file edges). Same-file candidates win; a unique
/// candidate wins; ambiguous/external stay NULL (symbols, decision #12).
fn resolve_unresolved_in_db(conn: &Connection) -> Result<()> {
    // Delmål C (H4, 2026-11-10): implements resolves against kind=trait
    // only — same rule as the per-file pass (name collisions between
    // impl:Foo and trait:Foo must not leave the honest target unresolved).
    let mut by_name: std::collections::HashMap<String, Vec<(String, String, String)>> =
        Default::default();
    {
        let mut stmt = conn.prepare("SELECT name, file, id, kind FROM entities")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?;
        for row in rows {
            let (name, file, id, kind) = row?;
            by_name.entry(name).or_default().push((file, id, kind));
        }
    }
    let mut stmt = conn
        .prepare("SELECT id, to_name, file, relation_type FROM relations WHERE to_id IS NULL")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    let mut updates: Vec<(i64, String)> = Vec::new();
    for row in rows {
        let (rid, to_name, file, relation_type) = row?;
        let bare = to_name.rsplit("::").next().unwrap_or(&to_name).to_string();
        let mut cands = by_name.get(&bare).cloned().unwrap_or_default();
        if to_name.contains("::") {
            if let Some(extra) = by_name.get(&to_name) {
                cands.extend(extra.clone());
            }
        }
        if relation_type == "implements" {
            cands.retain(|(_, _, kind)| kind == "trait");
        }
        let same_file: Vec<&(String, String, String)> =
            cands.iter().filter(|(f, _, _)| f == &file).collect();
        let pick = match (same_file.len(), cands.len()) {
            (1, _) => Some(same_file[0].1.clone()),
            (0, 1) => Some(cands[0].1.clone()),
            _ => None,
        };
        if let Some(id) = pick {
            updates.push((rid, id));
        }
    }
    for (rid, id) in &updates {
        conn.execute(
            "UPDATE relations SET to_id = ?1 WHERE id = ?2",
            params![id, rid],
        )?;
    }
    Ok(())
}

/// Phase 4: record an edit in the staleness queue — open-existing ONLY
/// (decision #8: the edit path never creates the store), best-effort
/// (errors swallowed: an edit must never fail because of bookkeeping).
/// One row per file (upsert, latest event wins — decision #8's queue
/// semantics). Connection-per-call is fine at edit frequency (~µs local).
pub fn mark_stale(root: &Path, file_path: &str) {
    if !db_path(root).exists() {
        return;
    }
    let Ok(conn) = open_existing(root) else {
        return;
    };
    let rel = crate::core::code_index::canonical_rel_path(root, Path::new(file_path));
    let now = chrono::Utc::now().timestamp();
    let _ = conn.execute(
        "INSERT INTO staleness_queue (file, reason, from_rev, queued_by, queued_at)
         VALUES (?1, 'edit', NULL, 'gtw-edit', ?2)
         ON CONFLICT(file) DO UPDATE SET reason = 'edit', queued_at = ?2",
        params![rel, now],
    );
}

/// Content hash used by the store (md5 hex) — the ONE freshness truth,
/// shared with `files.hash`. Consumers (Motor2's map) must not invent their
/// own freshness signal: `mtime:size` both over-invalidated (a plain `touch`
/// forced a re-parse) and under-detected (same length within the same second
/// could slip through). Hashing the bytes answers the actual question.
pub fn content_hash(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(format!("{:x}", md5::compute(&bytes)))
}

/// Did the edit path mark this file stale? Open-existing only (decision #8):
/// no store → `false`, and the caller falls back to the content hash, which
/// is the durable truth either way. A `true` here may only make a consumer
/// recompute EARLIER, never later.
pub fn is_stale(root: &Path, file: &str) -> bool {
    if !db_path(root).exists() {
        return false;
    }
    let Ok(conn) = open_existing(root) else {
        return false;
    };
    let rel = crate::core::code_index::canonical_rel_path(root, Path::new(file));
    conn.query_row(
        "SELECT 1 FROM staleness_queue WHERE file = ?1",
        params![rel],
        |r| r.get::<_, i64>(0),
    )
    .map(|v| v == 1)
    .unwrap_or(false)
}

/// MCP/konsument-yta: alla filer som edit-vägen markerat stale (open-existing
/// only — ingen store → tom mängd, aldrig en skapad store).
pub fn stale_files(root: &Path) -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::new();
    if !db_path(root).exists() {
        return out;
    }
    let Ok(conn) = open_existing(root) else {
        return out;
    };
    if let Ok(mut stmt) = conn.prepare("SELECT file FROM staleness_queue ORDER BY file") {
        if let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(0)) {
            for row in rows.flatten() {
                out.insert(row);
            }
        }
    }
    out
}

/// Konsument-yta: entiteterna ur storen (kind/name/line per id), optionally
/// prefix-filtered on file. Consumers (Motor2's map) read facts HERE instead
/// of re-parsing the same files — one authority, no double parse.
pub fn load_entities(conn: &Connection, prefix: Option<&str>) -> Result<Vec<serde_json::Value>> {
    let mut stmt = conn.prepare(
        "SELECT id, file, kind, name, line
         FROM entities WHERE (?1 IS NULL OR file LIKE ?1 || '%')
         ORDER BY file, line",
    )?;
    let rows = stmt.query_map(params![prefix], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, String>(0)?,
            "file": r.get::<_, String>(1)?,
            "kind": r.get::<_, String>(2)?,
            "name": r.get::<_, String>(3)?,
            "line": r.get::<_, i64>(4)?,
        }))
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Konsument-yta: AST-hierarkin (`items`) med föräldra-koppling, så en
/// konsument kan uttrycka "impl äger metoder" i stället för att platta ut allt
/// till fil-nivå. `parent_*` är None för rot-entiteter.
pub fn load_items(conn: &Connection, prefix: Option<&str>) -> Result<Vec<serde_json::Value>> {
    let mut stmt = conn.prepare(
        "SELECT i.file, i.kind, i.name, i.line, p.kind, p.name, p.line
         FROM items i LEFT JOIN items p ON p.id = i.parent_id
         WHERE (?1 IS NULL OR i.file LIKE ?1 || '%')
         ORDER BY i.file, i.line",
    )?;
    let rows = stmt.query_map(params![prefix], |r| {
        Ok(serde_json::json!({
            "file": r.get::<_, String>(0)?,
            "kind": r.get::<_, String>(1)?,
            "name": r.get::<_, String>(2)?,
            "line": r.get::<_, i64>(3)?,
            "parent_kind": r.get::<_, Option<String>>(4)?,
            "parent_name": r.get::<_, Option<String>>(5)?,
            "parent_line": r.get::<_, Option<i64>>(6)?,
        }))
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// MCP query surface: relations for a path prefix.
pub fn load_relations(conn: &Connection, prefix: Option<&str>) -> Result<Vec<serde_json::Value>> {
    let mut stmt = conn.prepare(
        "SELECT from_id, to_name, to_id, relation_type, line, file
         FROM relations WHERE (?1 IS NULL OR file LIKE ?1 || '%')
         ORDER BY file, line",
    )?;
    let rows = stmt.query_map(params![prefix], |r| {
        Ok(serde_json::json!({
            "from": r.get::<_, String>(0)?,
            "to_name": r.get::<_, String>(1)?,
            "to_id": r.get::<_, Option<String>>(2)?,
            "relation_type": r.get::<_, String>(3)?,
            "line": r.get::<_, i64>(4)?,
            "file": r.get::<_, String>(5)?
        }))
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// MCP query surface: unresolved/external symbols with occurrence counts.
pub fn load_symbols(conn: &Connection) -> Result<Vec<serde_json::Value>> {
    let mut stmt = conn.prepare(
        "SELECT name, file, external, occurrences
         FROM symbols ORDER BY occurrences DESC, name",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(serde_json::json!({
            "name": r.get::<_, String>(0)?,
            "file": r.get::<_, String>(1)?,
            "external": r.get::<_, i64>(2)? != 0,
            "occurrences": r.get::<_, i64>(3)?
        }))
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
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