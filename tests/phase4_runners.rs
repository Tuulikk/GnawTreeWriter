//! Phase 4 tests — the self-running bits: staleness writes from the edit
//! path (decision #8: open-existing only, never create) and the explicit,
//! per-clone post-commit hook (P4: never auto-stages).

use gnawtreewriter::core::code_index::CodeIndexOptions;
use gnawtreewriter::core::code_index_db;
use gnawtreewriter::core::EditOperation;
use std::process::Command;

fn opts() -> CodeIndexOptions {
    CodeIndexOptions {
        extensions: vec!["rs".to_string()],
        ..Default::default()
    }
}

#[test]
fn mark_stale_upserts_one_row_per_file_and_refresh_drains() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".git"), "gitdir: nowhere\n").unwrap();
    std::fs::write(dir.path().join("lib.rs"), "pub fn a() -> u8 { 1 }\n").unwrap();
    code_index_db::generate_with_db(dir.path(), &opts()).unwrap();

    // Two marks for the same file → ONE queue row (upsert, decision #8).
    code_index_db::mark_stale(dir.path(), "lib.rs");
    code_index_db::mark_stale(dir.path(), "lib.rs");
    let st = code_index_db::store_status(dir.path()).unwrap();
    assert_eq!(st.queued, 1, "upsert must dedupe: {:?}", st);

    // A refresh drains the queue: everything walked is fresh afterwards.
    code_index_db::generate_with_db(dir.path(), &opts()).unwrap();
    let st = code_index_db::store_status(dir.path()).unwrap();
    assert_eq!(st.queued, 0, "refresh must drain: {:?}", st);
}

#[test]
fn mark_stale_never_creates_the_store() {
    let dir = tempfile::tempdir().unwrap();
    code_index_db::mark_stale(dir.path(), "lib.rs");
    assert!(
        !code_index_db::db_path(dir.path()).exists(),
        "decision #8: the edit path must never create the store"
    );
}

#[test]
fn real_edit_lands_in_the_queue() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("lib.rs");
    std::fs::write(dir.path().join(".git"), "gitdir: nowhere\n").unwrap();
    std::fs::write(&f, "pub fn a() -> u8 { 1 }\n").unwrap();
    code_index_db::generate_with_db(dir.path(), &opts()).unwrap();

    // A REAL edit through the core funnel — the choke point marks stale.
    let mut w = gnawtreewriter::GnawTreeWriter::new(f.to_str().unwrap()).unwrap();
    w.edit(
        EditOperation::Edit {
            node_path: "@fn:a".to_string(),
            content: "pub fn a() -> u8 { 2 }\n".to_string(),
        },
        false,
    )
    .unwrap();

    let st = code_index_db::store_status(dir.path()).unwrap();
    assert_eq!(st.queued, 1, "the edit must mark the file stale: {:?}", st);
}

#[test]
fn hook_install_is_idempotent_and_uninstall_restores() {
    let dir = tempfile::tempdir().unwrap();
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()
        .expect("git available for hook tests (isolated tempdir)");
    assert!(status.success());
    let bin = env!("CARGO_BIN_EXE_gnawtreewriter");

    let run_hook = |args: &[&str]| {
        Command::new(bin)
            .args(["code-index", dir.path().to_str().unwrap()])
            .args(args)
            .output()
            .expect("run code-index")
    };

    // Install → hook file exists with the marker + executable bit.
    assert!(run_hook(&["--install-hook"]).status.success());
    let hook = dir.path().join(".git").join("hooks").join("post-commit");
    let first = std::fs::read_to_string(&hook).unwrap();
    assert!(first.contains(">>> gnawtreewriter code-index >>>"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&hook).unwrap().permissions().mode() & 0o111,
            0o111,
            "hook must be executable"
        );
    }

    // Idempotent: second install must not duplicate the block.
    assert!(run_hook(&["--install-hook"]).status.success());
    let second = std::fs::read_to_string(&hook).unwrap();
    assert_eq!(
        first.matches(">>> gnawtreewriter code-index >>>").count(),
        second.matches(">>> gnawtreewriter code-index >>>").count()
    );

    // Uninstall removes the block (and a hook that held only the block).
    assert!(run_hook(&["--uninstall-hook"]).status.success());
    assert!(!hook.exists(), "block-only hook is removed with the block");
    assert!(
        run_hook(&["--uninstall-hook"]).status.success(),
        "uninstall without block is a polite no-op"
    );
}

#[test]
fn hook_install_preserves_existing_user_hook() {
    let dir = tempfile::tempdir().unwrap();
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success());

    let hooks = dir.path().join(".git").join("hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    let hook = hooks.join("post-commit");
    std::fs::write(&hook, "#!/bin/sh\necho user-logic\n").unwrap();

    let bin = env!("CARGO_BIN_EXE_gnawtreewriter");
    assert!(Command::new(bin)
        .args(["code-index", dir.path().to_str().unwrap(), "--install-hook"])
        .output()
        .unwrap()
        .status
        .success());

    let content = std::fs::read_to_string(&hook).unwrap();
    assert!(content.contains("user-logic"), "user logic preserved");
    assert!(content.contains(">>> gnawtreewriter code-index >>>"));

    // Uninstall strips OUR block, keeps THEIR logic.
    assert!(Command::new(bin)
        .args([
            "code-index",
            dir.path().to_str().unwrap(),
            "--uninstall-hook"
        ])
        .output()
        .unwrap()
        .status
        .success());
    let after = std::fs::read_to_string(&hook).unwrap();
    assert!(after.contains("user-logic"));
    assert!(!after.contains(">>> gnawtreewriter code-index >>>"));
}
