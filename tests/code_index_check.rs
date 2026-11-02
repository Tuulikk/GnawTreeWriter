//! Phase 2 tests for the code-index content-hash gate (`--check`).
//!
//! Hermetic: tempdir repos, never the checkout itself. The gate is DB-free
//! by construction (P6) — every test exercises the direct path only.

use gnawtreewriter::core::code_index::{check_index, generate_index, CodeIndexOptions};
use std::path::Path;

fn opts() -> CodeIndexOptions {
    CodeIndexOptions {
        extensions: vec!["rs".to_string()],
        ..Default::default()
    }
}

fn make_repo(dir: &Path) -> std::path::PathBuf {
    std::fs::write(dir.join(".git"), "gitdir: nowhere\n").unwrap();
    std::fs::write(
        dir.join("lib.rs"),
        "pub struct Alpha { pub x: u16 }\n\npub fn beta(a: u8) -> u8 { a }\n",
    )
    .unwrap();
    let artifact = dir.join("code-index.md");
    std::fs::write(&artifact, generate_index(dir, &opts()).unwrap()).unwrap();
    artifact
}

#[test]
fn fresh_artifact_passes() {
    let dir = tempfile::tempdir().unwrap();
    let artifact = make_repo(dir.path());
    let res = check_index(dir.path(), &artifact, &opts(), false).unwrap();
    assert!(res.ok, "fresh artifact must pass: {:?}", res.failures);
    assert!(!res.fixed);
}

#[test]
fn hand_edit_fails_fast_tier() {
    let dir = tempfile::tempdir().unwrap();
    let artifact = make_repo(dir.path());
    let doc = std::fs::read_to_string(&artifact).unwrap();
    // Tamper with the BODY only — the header's Body-hash stays old.
    let tampered = doc.replace("pub fn beta(a: u8) -> u8", "pub fn EVIL(a: u8) -> u8");
    std::fs::write(&artifact, tampered).unwrap();
    let res = check_index(dir.path(), &artifact, &opts(), false).unwrap();
    assert!(!res.ok);
    assert!(
        res.failures.iter().any(|f| f.contains("Body-hash header")),
        "fast tier must catch the hand-edit: {:?}",
        res.failures
    );
}

#[test]
fn stale_artifact_fails_full_tier() {
    let dir = tempfile::tempdir().unwrap();
    let artifact = make_repo(dir.path());
    // Change the source AFTER the artifact was written — staleness.
    std::fs::write(
        dir.path().join("lib.rs"),
        "pub struct Alpha { pub x: u16 }\n\npub fn gamma() -> u8 { 9 }\n",
    )
    .unwrap();
    let res = check_index(dir.path(), &artifact, &opts(), false).unwrap();
    assert!(!res.ok);
    assert!(
        res.failures.iter().any(|f| f.contains("stale")),
        "full tier must catch staleness: {:?}",
        res.failures
    );
}

#[test]
fn fix_rewrites_and_then_passes() {
    let dir = tempfile::tempdir().unwrap();
    let artifact = make_repo(dir.path());
    std::fs::write(
        dir.path().join("lib.rs"),
        "pub struct Alpha { pub x: u16 }\n\npub fn gamma() -> u8 { 9 }\n",
    )
    .unwrap();
    let res = check_index(dir.path(), &artifact, &opts(), true).unwrap();
    assert!(!res.ok, "fix reports what was wrong");
    assert!(res.fixed, "fix must rewrite the artifact");
    // A subsequent check (no fix) must now pass — the gate verifies its
    // own repair (re-scan verification).
    let again = check_index(dir.path(), &artifact, &opts(), false).unwrap();
    assert!(
        again.ok,
        "after --fix the gate must pass: {:?}",
        again.failures
    );
    assert!(std::fs::read_to_string(&artifact)
        .unwrap()
        .contains("gamma"));
}

#[test]
fn legacy_artifact_without_hash_header_is_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let artifact = make_repo(dir.path());
    // Strip the Body-hash line — artifact predates the gate.
    let doc = std::fs::read_to_string(&artifact).unwrap();
    let legacy = doc
        .lines()
        .filter(|l| !l.starts_with("> Body-hash:"))
        .map(|l| format!("{l}\n"))
        .collect::<String>();
    std::fs::write(&artifact, legacy).unwrap();
    let res = check_index(dir.path(), &artifact, &opts(), false).unwrap();
    assert!(!res.ok);
    assert!(
        res.failures.iter().any(|f| f.contains("predates the gate")),
        "{:?}",
        res.failures
    );
}
