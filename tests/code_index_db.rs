//! Phase 1 tests for the code-index SQLite store.
//!
//! Hermetic by construction: every test works in a tempfile::tempdir with a
//! `.git` MARKER file (not a real repo) — the repo-root-state lesson: tests
//! must never operate on the checkout's own transaction log or git roots.
//! Parity between the direct path and the DB projection is a tested
//! invariant (CODE_INDEX_SQLITE_DESIGN.md, review decision #5).

use gnawtreewriter::core::code_index::{generate_index, CodeIndexOptions};
use gnawtreewriter::core::code_index_db;
use std::path::Path;

fn opts() -> CodeIndexOptions {
    CodeIndexOptions {
        extensions: vec!["rs".to_string(), "py".to_string()],
        ..Default::default()
    }
}

fn make_repo(dir: &Path) {
    std::fs::create_dir_all(dir.join("src")).unwrap();
    // Marker, not a repo: `git rev-parse` fails -> root stays the walked
    // root (decision #10 fallback branch).
    std::fs::write(dir.join(".git"), "gitdir: nowhere\n").unwrap();
    std::fs::write(
        dir.join("src/lib.rs"),
        "pub struct Alpha { pub x: u16 }\n\nimpl Alpha {\n    pub fn new() -> Self { Self { x: 1 } }\n    fn hidden(&self) -> u16 { self.x }\n}\n\npub fn beta(a: u8) -> u8 { a }\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("src/tool.py"),
        "class Thing:\n    def run(self):\n        return 1\n\ndef top():\n    pass\n",
    )
    .unwrap();
    // A file with ZERO top-level items — LEFT JOIN NULL-row coverage (a
    // real GTW-repo run crashed on this before the mapper went tolerant).
    std::fs::write(
        dir.join("src/notes.rs"),
        "// only comments and a mod decl\nmod inner;\n",
    )
    .unwrap();
}

/// Strip lines that legitimately differ between two render invocations
/// (timestamp header in markdown, generated_at in JSON).
fn normalize(doc: &str) -> String {
    doc.lines()
        .filter(|l| !l.starts_with("> Genererad"))
        .filter(|l| !l.contains("\"generated_at_utc\""))
        .map(|l| format!("{l}\n"))
        .collect()
}

#[test]
fn db_render_matches_direct_markdown() {
    let dir = tempfile::tempdir().unwrap();
    make_repo(dir.path());
    let direct = generate_index(dir.path(), &opts()).unwrap();
    let run = code_index_db::generate_with_db(dir.path(), &opts()).unwrap();
    assert!(
        run.notices.is_empty(),
        "fresh DB should carry no notices: {:?}",
        run.notices
    );
    assert_eq!(
        normalize(&direct),
        normalize(&run.doc),
        "DB projection must match the direct render byte-for-byte (modulo stamps)"
    );
}

#[test]
fn db_render_matches_direct_json() {
    let dir = tempfile::tempdir().unwrap();
    make_repo(dir.path());
    let mut o = opts();
    o.json = true;
    let direct = generate_index(dir.path(), &o).unwrap();
    let run = code_index_db::generate_with_db(dir.path(), &o).unwrap();
    assert_eq!(
        normalize(&direct),
        normalize(&run.doc),
        "JSON projection must match the direct render"
    );
}

#[test]
fn incremental_refresh_reuses_and_reparses() {
    let dir = tempfile::tempdir().unwrap();
    make_repo(dir.path());

    let first = code_index_db::generate_with_db(dir.path(), &opts()).unwrap();
    assert_eq!(first.stats.files_parsed, 3, "first run parses everything");
    assert_eq!(first.stats.files_reused, 0);

    // Incremental identity: no changes -> byte-identical render, zero parses.
    let second = code_index_db::generate_with_db(dir.path(), &opts()).unwrap();
    assert_eq!(
        second.stats.files_parsed, 0,
        "unchanged files must be reused"
    );
    assert_eq!(second.stats.files_reused, 3);
    assert_eq!(
        normalize(&first.doc),
        normalize(&second.doc),
        "second run must render identically (incremental identity)"
    );

    // One change -> exactly that file re-parsed, the other reused.
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "pub struct Alpha { pub x: u16 }\n\npub fn gamma() -> u8 { 7 }\n",
    )
    .unwrap();
    let third = code_index_db::generate_with_db(dir.path(), &opts()).unwrap();
    assert_eq!(third.stats.files_parsed, 1);
    assert_eq!(third.stats.files_reused, 2);
    assert!(
        third.doc.contains("gamma"),
        "re-parse must surface new items"
    );
    assert!(
        !third.doc.contains("hidden"),
        "stale items from the old parse must be gone"
    );
}

#[test]
fn generation_key_covers_population_not_presentation() {
    let dir = tempfile::tempdir().unwrap();
    make_repo(dir.path());
    code_index_db::generate_with_db(dir.path(), &opts()).unwrap();

    // Presentation-only change: no invalidation, different render.
    let mut short = opts();
    short.max_sig = 8;
    let run = code_index_db::generate_with_db(dir.path(), &short).unwrap();
    assert_eq!(
        run.stats.files_parsed, 0,
        "max_sig must NOT invalidate rows"
    );
    assert!(
        run.doc.contains('…'),
        "short max_sig must truncate at render"
    );

    // Population change: generation shifts -> full rebuild.
    let mut rs_only = opts();
    rs_only.extensions = vec!["rs".to_string()];
    let run = code_index_db::generate_with_db(dir.path(), &rs_only).unwrap();
    assert!(run.stats.full_rebuild, "extensions change must rebuild");
    assert_eq!(run.stats.files_parsed, 2);
    assert!(!run.doc.contains("tool.py"), "excluded files must vanish");
}

#[test]
fn removed_files_are_pruned() {
    let dir = tempfile::tempdir().unwrap();
    make_repo(dir.path());
    code_index_db::generate_with_db(dir.path(), &opts()).unwrap();
    std::fs::remove_file(dir.path().join("src/tool.py")).unwrap();
    let run = code_index_db::generate_with_db(dir.path(), &opts()).unwrap();
    assert_eq!(run.stats.files_removed, 1);
    assert_eq!(run.stats.files_reused, 2);
    assert!(!run.doc.contains("tool.py"));
}

#[test]
fn corrupt_db_is_rebuilt_not_fatal() {
    let dir = tempfile::tempdir().unwrap();
    make_repo(dir.path());
    code_index_db::generate_with_db(dir.path(), &opts()).unwrap();

    // Garbage over the DB file -> recovery path (delete db + sidecars,
    // rebuild) must succeed AND surface a notice (never silent).
    std::fs::write(
        code_index_db::db_path(dir.path()),
        "not a sqlite file at all",
    )
    .unwrap();
    let run = code_index_db::generate_with_db(dir.path(), &opts()).unwrap();
    assert!(run.stats.full_rebuild, "recovery implies full rebuild");
    assert!(
        run.doc.contains("ombyggd"),
        "recovery must be visible in the artifact: {:?}",
        run.notices
    );
    assert!(run.doc.contains("beta"), "rebuilt index has content");
}
