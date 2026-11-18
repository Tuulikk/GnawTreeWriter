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
        .filter(|l| !l.starts_with("> "))
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
fn new_competing_definition_reconsiders_existing_cross_file_call() {
    let incremental = tempfile::tempdir().unwrap();
    let full = tempfile::tempdir().unwrap();
    for root in [incremental.path(), full.path()] {
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join(".git"), "gitdir: nowhere\n").unwrap();
        std::fs::write(
            root.join("src/caller.rs"),
            "pub fn invoke() { target(); steady(); }\n",
        )
        .unwrap();
        std::fs::write(
            root.join("src/one.rs"),
            "pub fn target() {}\npub fn steady() {}\n",
        )
        .unwrap();
    }

    let (mut inc_db, _) = code_index_db::open_or_recreate(incremental.path()).unwrap();
    code_index_db::refresh(&mut inc_db, incremental.path(), &opts()).unwrap();
    let before = code_index_db::load_relations(&inc_db, None).unwrap();
    let call = |rows: &[serde_json::Value], name: &str| {
        rows.iter()
            .find(|r| {
                r["file"] == "src/caller.rs"
                    && r["to_name"] == name
                    && r["relation_type"] == "calls"
            })
            .unwrap()["to_id"]
            .clone()
    };
    let original_target = call(&before, "target");
    let original_steady = call(&before, "steady");
    assert!(original_target.is_string(), "initial target must resolve");
    assert!(original_steady.is_string(), "control call must resolve");

    for root in [incremental.path(), full.path()] {
        std::fs::write(root.join("src/two.rs"), "pub fn target() {}\n").unwrap();
    }
    let stats = code_index_db::refresh(&mut inc_db, incremental.path(), &opts()).unwrap();
    assert_eq!(stats.files_parsed, 1, "only the new definition is parsed");
    assert_eq!(
        stats.files_reused, 2,
        "caller and original definition are reused"
    );
    let after_incremental = code_index_db::load_relations(&inc_db, None).unwrap();
    assert_eq!(call(&after_incremental, "steady"), original_steady);

    let (mut full_db, _) = code_index_db::open_or_recreate(full.path()).unwrap();
    code_index_db::refresh(&mut full_db, full.path(), &opts()).unwrap();
    let after_full = code_index_db::load_relations(&full_db, None).unwrap();
    assert!(
        call(&after_full, "target").is_null(),
        "fresh rebuild sees ambiguity"
    );
    assert_eq!(call(&after_full, "steady"), original_steady);

    let mut incremental_rows: Vec<_> = after_incremental
        .into_iter()
        .map(|r| r.to_string())
        .collect();
    let mut full_rows: Vec<_> = after_full.into_iter().map(|r| r.to_string()).collect();
    incremental_rows.sort();
    full_rows.sort();
    assert_eq!(
        incremental_rows, full_rows,
        "incremental and full relation graphs must agree"
    );
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
#[test]
fn removed_definitions_reconsider_ambiguous_and_resolved_calls() {
    let incremental = tempfile::tempdir().unwrap();
    let one_removed = tempfile::tempdir().unwrap();
    let both_removed = tempfile::tempdir().unwrap();
    let prepare = |root: &Path, include_one: bool, include_two: bool| {
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join(".git"), "gitdir: nowhere\n").unwrap();
        std::fs::write(
            root.join("src/caller.rs"),
            "pub fn invoke() { target(); steady(); }\n",
        )
        .unwrap();
        std::fs::write(root.join("src/stable.rs"), "pub fn steady() {}\n").unwrap();
        if include_one {
            std::fs::write(root.join("src/one.rs"), "pub fn target() {}\n").unwrap();
        }
        if include_two {
            std::fs::write(root.join("src/two.rs"), "pub fn target() {}\n").unwrap();
        }
    };
    prepare(incremental.path(), true, true);
    prepare(one_removed.path(), false, true);
    prepare(both_removed.path(), false, false);

    let rows = |root: &Path| {
        let (mut conn, _) = code_index_db::open_or_recreate(root).unwrap();
        code_index_db::refresh(&mut conn, root, &opts()).unwrap();
        code_index_db::load_relations(&conn, None).unwrap()
    };
    let target = |relations: &[serde_json::Value], name: &str| {
        relations
            .iter()
            .find(|r| {
                r["file"] == "src/caller.rs"
                    && r["to_name"] == name
                    && r["relation_type"] == "calls"
            })
            .unwrap()["to_id"]
            .clone()
    };
    let normalized = |relations: Vec<serde_json::Value>| {
        let mut values: Vec<_> = relations.into_iter().map(|r| r.to_string()).collect();
        values.sort();
        values
    };

    let (mut conn, _) = code_index_db::open_or_recreate(incremental.path()).unwrap();
    code_index_db::refresh(&mut conn, incremental.path(), &opts()).unwrap();
    let initial = code_index_db::load_relations(&conn, None).unwrap();
    assert!(
        target(&initial, "target").is_null(),
        "two definitions are ambiguous"
    );
    let stable = target(&initial, "steady");
    assert!(stable.is_string());

    std::fs::remove_file(incremental.path().join("src/one.rs")).unwrap();
    let stats = code_index_db::refresh(&mut conn, incremental.path(), &opts()).unwrap();
    assert_eq!(stats.files_removed, 1);
    assert_eq!(stats.files_parsed, 0, "unchanged callers must be reused");
    let after_one = code_index_db::load_relations(&conn, None).unwrap();
    assert_eq!(
        target(&after_one, "target"),
        serde_json::json!("gtw:src/two.rs:function:target")
    );
    assert_eq!(target(&after_one, "steady"), stable);
    assert_eq!(normalized(after_one), normalized(rows(one_removed.path())));

    std::fs::remove_file(incremental.path().join("src/two.rs")).unwrap();
    let stats = code_index_db::refresh(&mut conn, incremental.path(), &opts()).unwrap();
    assert_eq!(stats.files_removed, 1);
    assert_eq!(stats.files_parsed, 0);
    let after_both = code_index_db::load_relations(&conn, None).unwrap();
    assert!(
        target(&after_both, "target").is_null(),
        "removed target must be rejected"
    );
    assert_eq!(target(&after_both, "steady"), stable);
    assert_eq!(
        normalized(after_both),
        normalized(rows(both_removed.path()))
    );
}
#[test]
fn unchanged_refresh_keeps_targets_without_relation_updates() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join(".git"), "gitdir: nowhere\n").unwrap();
    std::fs::write(
        root.join("src/caller.rs"),
        "pub fn invoke() { target(); }\n",
    )
    .unwrap();
    std::fs::write(root.join("src/target.rs"), "pub fn target() {}\n").unwrap();

    let (mut conn, _) = code_index_db::open_or_recreate(root).unwrap();
    code_index_db::refresh(&mut conn, root, &opts()).unwrap();
    let before = code_index_db::load_relations(&conn, None).unwrap();
    assert_eq!(before.len(), 1);
    assert!(before[0]["to_id"].is_string());

    conn.execute_batch(
        "CREATE TEMP TABLE relation_update_audit (n INTEGER);
         CREATE TEMP TRIGGER count_relation_updates AFTER UPDATE OF to_id ON relations
         BEGIN INSERT INTO relation_update_audit(n) VALUES (1); END;",
    )
    .unwrap();
    let changes_before: i64 = conn
        .query_row("SELECT total_changes()", [], |r| r.get(0))
        .unwrap();
    let stats = code_index_db::refresh(&mut conn, root, &opts()).unwrap();
    let changes_after: i64 = conn
        .query_row("SELECT total_changes()", [], |r| r.get(0))
        .unwrap();
    let target_updates: i64 = conn
        .query_row("SELECT COUNT(*) FROM relation_update_audit", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(stats.files_parsed, 0);
    assert_eq!(stats.files_reused, 2);
    assert_eq!(target_updates, 0, "unchanged target must not be rewritten");
    assert_eq!(code_index_db::load_relations(&conn, None).unwrap(), before);
    eprintln!(
        "no-op refresh: {} total SQLite changes, {} relation target updates",
        changes_after - changes_before,
        target_updates
    );
}
