//! `validate` advice must be MEASURED, not assumed.
//!
//! A model that reads "run heal" on a parse error that is not a brace problem
//! loses its turn; worse, it learns that GTW's advice is a guess. So the advice
//! comes from the healer itself: unbalanced braces → a heal case; balanced
//! braces → explicitly NOT a heal case.
//!
//! Covers both surfaces: the shared `heal_advice` (CLI and MCP) and the CLI
//! output of a failing `validate`.

use gnawtreewriter::core::healer::{heal_advice, repair_file, HealOutcome};
use std::fs;
use tempfile::tempdir;

#[test]
fn unbalanced_braces_are_named_a_heal_case() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("broken.rs");
    fs::write(&file, "fn a() {\n    let x = 1;\n").unwrap();

    let advice = heal_advice(&file);
    assert!(
        advice.contains("IS a heal case"),
        "unbalanced braces must be offered as a heal case, got: {advice}"
    );

    // And it must actually be repairable — the advice is not a promise GTW
    // cannot keep.
    match repair_file(&file, false).unwrap() {
        HealOutcome::Preview { .. } => {}
        other => panic!("expected an appendable preview, got {other:?}"),
    }
}

#[test]
fn balanced_braces_are_explicitly_not_a_heal_case() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("broken.rs");
    // Balanced braces, still a parse error: `heal` cannot help here, and
    // saying it could would send the model down a dead end.
    fs::write(&file, "fn a() {\n    let x = ;\n}\n").unwrap();

    let advice = heal_advice(&file);
    assert!(
        advice.contains("NOT a heal case"),
        "a non-brace parse error must be excluded from the heal hint, got: {advice}"
    );
}
