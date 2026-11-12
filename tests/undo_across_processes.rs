//! The process boundary is the bug this file exists for.
//!
//! Before 2026-11-12 two separate `undo` invocations reverted the SAME
//! transaction, because the undo stack was rebuilt from a flat list of content
//! operations and nothing recorded that one had already been reverted. These
//! tests run the REAL binary in SEPARATE processes (no shared memory), which is
//! the only honest way to prove the fix.

use std::path::Path;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_gnawtreewriter");

fn run(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let out = Command::new(BIN)
        .args(args)
        .current_dir(dir)
        .output()
        .expect("spawn gnawtreewriter");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// Hermetic project: a `.git` marker pins the root (AGENTS #6 — a stray
/// session file in a shared parent must not capture the tempdir).
fn project() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".git")).unwrap();
    let file = dir.path().join("sample.rs");
    std::fs::write(&file, "fn a() {\n    1\n}\n").unwrap();
    (dir, file)
}

#[test]
fn undo_in_separate_processes_walks_backwards() {
    let (dir, file) = project();
    let original = std::fs::read_to_string(&file).unwrap();

    let (ok, out, _) = run(
        dir.path(),
        &[
            "quick-insert",
            "sample.rs",
            "--after",
            "^fn a",
            "// one",
            "--unique",
        ],
    );
    assert!(ok, "insert one: {out}");
    let (ok, out, _) = run(
        dir.path(),
        &[
            "quick-insert",
            "sample.rs",
            "--after",
            "^fn a",
            "// two",
            "--unique",
        ],
    );
    assert!(ok, "insert two: {out}");

    let after_two = std::fs::read_to_string(&file).unwrap();
    assert!(after_two.contains("// one") && after_two.contains("// two"));

    // PROCESS 1 undoes the newest edit.
    let (ok, out, _) = run(dir.path(), &["undo"]);
    assert!(ok, "undo #1: {out}");
    let after_undo1 = std::fs::read_to_string(&file).unwrap();
    assert!(after_undo1.contains("// one"), "// one must survive: {out}");
    assert!(
        !after_undo1.contains("// two"),
        "// two must be gone: {out}"
    );

    // PROCESS 2 must go ONE STEP FURTHER, not repeat the same undo.
    let (ok, out, _) = run(dir.path(), &["undo"]);
    assert!(ok, "undo #2: {out}");
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        original,
        "second process must revert the FIRST edit too: {out}"
    );

    // And the journal says so in the open: two Undo events naming their target.
    let log = std::fs::read_to_string(dir.path().join(".gnawtreewriter_session.json")).unwrap();
    assert_eq!(
        log.matches("\"operation\":\"Undo\"").count(),
        2,
        "two Undo events expected"
    );
    assert_eq!(
        log.matches("\"reverted_txn\"").count(),
        2,
        "each Undo names what it reverted"
    );
}

#[test]
fn redo_in_a_third_process_reapplies_and_is_then_undoable_again() {
    let (dir, file) = project();

    let (ok, out, _) = run(
        dir.path(),
        &[
            "quick-insert",
            "sample.rs",
            "--after",
            "^fn a",
            "// keep",
            "--unique",
        ],
    );
    assert!(ok, "{out}");
    let (ok, out, _) = run(dir.path(), &["undo"]);
    assert!(ok, "{out}");
    assert!(!std::fs::read_to_string(&file).unwrap().contains("// keep"));

    // A FRESH process must still know what can be redone (derived from the log).
    let (ok, out, err) = run(dir.path(), &["redo"]);
    assert!(ok, "{out}{err}");
    assert!(
        std::fs::read_to_string(&file).unwrap().contains("// keep"),
        "redo in a new process must re-apply the undone edit: {out}{err}"
    );

    // …and the re-applied edit is undoable again.
    let (ok, out, _) = run(dir.path(), &["undo"]);
    assert!(ok, "{out}");
    assert!(!std::fs::read_to_string(&file).unwrap().contains("// keep"));

    let log = std::fs::read_to_string(dir.path().join(".gnawtreewriter_session.json")).unwrap();
    assert_eq!(log.matches("\"operation\":\"Redo\"").count(), 1);
    assert_eq!(log.matches("\"restored_txn\"").count(), 1);
}

#[test]
fn the_newest_state_has_a_verifiable_content_source() {
    let (dir, file) = project();
    let (ok, out, _) = run(
        dir.path(),
        &[
            "quick-insert",
            "sample.rs",
            "--after",
            "^fn a",
            "// newest",
            "--unique",
        ],
    );
    assert!(ok, "{out}");

    // The journal names an after-blob for the state the edit CREATED (owner's
    // rule #3: a hash is not storage — the pre-edit blob cannot cover this).
    let log = std::fs::read_to_string(dir.path().join(".gnawtreewriter_session.json")).unwrap();
    let after_blob = log
        .split("\"after_blob\":")
        .nth(1)
        .and_then(|s| s.split('"').nth(1))
        .expect("an after_blob must be recorded for the newest state");
    let blob = dir.path().join(".gnawtreewriter_backups").join(after_blob);
    let body = std::fs::read_to_string(&blob).expect("the named blob must exist");
    // Parse instead of substring-matching: the blob is compact JSON, so
    // newlines are escaped and a raw `contains` would compare the wrong thing.
    let json: serde_json::Value = serde_json::from_str(&body).expect("blob is JSON");
    let stored = json["source_code"].as_str().expect("source_code");
    let on_disk = std::fs::read_to_string(&file).unwrap();
    assert_eq!(
        stored, on_disk,
        "the blob must hold exactly the bytes now on disk (verifiable source)"
    );
    assert!(json.get("tree").is_none(), "v2 blob: source only, no AST");
}

#[test]
fn an_external_write_is_visible_as_a_discrepancy_not_repaired() {
    let (dir, file) = project();
    let (ok, out, _) = run(
        dir.path(),
        &[
            "quick-insert",
            "sample.rs",
            "--after",
            "^fn a",
            "// gtw",
            "--unique",
        ],
    );
    assert!(ok, "{out}");

    // Someone else writes the file: GTW must be able to SEE it.
    std::fs::write(&file, "fn a() {\n    999\n}\n").unwrap();

    let root = dir.path().to_path_buf();
    let (head, _stale) = gnawtreewriter::core::state_head::load(&root).unwrap();
    let d = gnawtreewriter::core::state_head::verify(&head, &root);
    assert!(
        d.iter()
            .any(|x| format!("{x:?}").contains("WorkingTreeMismatch")),
        "an external write must be reported: {d:?}"
    );
    // Never silently repaired.
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "fn a() {\n    999\n}\n",
        "verify must not write"
    );
}
