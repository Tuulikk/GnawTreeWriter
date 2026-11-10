//! Phase 1 fixture suite (delmål B, CODE_INDEX_RELATION_HISTORY_PROPOSAL
//! 2026-11-10): a hermetic tempdir project (`.git`-marker — lesson #6,
//! never repo-root state) mirroring the hand-marked fixture at
//! `testprojekt/code-index-fixture`. Baseline facts are pinned AS MEASURED
//! post-C (extractor v2): the former holes H1/H2/H3/H4 flipped consciously
//! and now CONTRIBUTE rows, asserted here so a regression flips them back
//! loudly.

use gnawtreewriter::core::code_index::CodeIndexOptions;
use gnawtreewriter::core::code_index_coverage;
use gnawtreewriter::core::code_index_db;
use std::process::Command;

const LIB_RS: &str = r#"// fixture mirror: F1-F7 markers, see FACTS.md
pub fn helper(x: i32) -> i32 {
    x + 1
}

// [F1] plain_call / [F2] multiplicity_2_to_1 (D8)
pub fn alpha() -> i32 {
    let a = helper(1);
    let b = helper(2);
    a + b
}

// [F3] macro_invocation — H2 flipped: dbg! lands with a bang suffix
pub fn beta() -> i32 {
    let v = dbg!(7);
    v * 2
}

// [F4] qualified_path_call — H3 flipped: scoped-path calls are extracted
pub fn gamma() {
    crate::module_a::run();
}

pub mod module_a {
    // [F5] same_name_A — H3 flipped: super:: call extracted + resolved
    pub fn run() {
        super::helper(3);
    }
}

pub mod module_b {
    // [F6] same_name_B — H3 flipped
    pub fn run() {
        super::helper(4);
    }
}
"#;

const CALLER_RS: &str = r#"use crate::module_a::run;

// [F7] cross_file_ambiguous: bare run(), 2 candidates in OTHER files →
//      expect to_id NULL (ambiguous), never a guessed target.
pub fn call_run() {
    run();
}
"#;

const METHODS_RS: &str = r#"// [F8] impl + method entity / [F9] method_call — H1 flipped: extracted
//      and resolved against the impl-method entity.
pub struct Widget {
    pub name: String,
}

impl Widget {
    pub fn describe(&self) -> String {
        format!("widget {}", self.name)
    }
}

pub fn show() -> String {
    let w = Widget {
        name: "x".to_string(),
    };
    w.describe()
}

// [F10] trait_impl — H4 flipped: implements resolves against kind=trait.
pub trait Speaker {
    fn speak(&self) -> String;
}

pub struct Dog;

impl Speaker for Dog {
    fn speak(&self) -> String {
        "woff".to_string()
    }
}
"#;

/// Hermetic fixture project: tempdir + `.git` marker (lesson #6).
fn build_fixture() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().to_path_buf();
    let src = root.join("src");
    std::fs::create_dir_all(&src).expect("src dir");
    std::fs::write(src.join("lib.rs"), LIB_RS).expect("lib.rs");
    std::fs::write(src.join("caller.rs"), CALLER_RS).expect("caller.rs");
    std::fs::write(src.join("methods.rs"), METHODS_RS).expect("methods.rs");
    let ok = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&root)
        .status()
        .expect("git present")
        .success();
    assert!(ok, "git init must succeed");
    (dir, root)
}

#[test]
fn fixture_baseline_post_c() {
    let (_dir, root) = build_fixture();
    let (mut conn, _recovered) = code_index_db::open_or_recreate(&root).unwrap();
    let stats = code_index_db::refresh(&mut conn, &root, &CodeIndexOptions::default()).unwrap();
    assert_eq!((stats.files_total, stats.files_parsed), (3, 3));
    // A FRESH store has no generation_key yet — the first refresh is a
    // planned full re-upsert by design (decision #1).
    assert!(stats.full_rebuild);

    let rows = code_index_db::load_relations(&conn, None).unwrap();

    // Post-C calls inventory (MEASURED 2026-11-10, extractor v2):
    // helper×4 [F1+F2 plain + F5/F6 scoped, all bare-named `helper`] +
    // run×2 [F7 bare + F4 scoped, both honestly ambiguous] + describe
    // [F9→H1] + dbg! [F3→H2] + format! + to_string×2 = 11 rows, 5 resolved.
    let calls: Vec<_> = rows
        .iter()
        .filter(|r| r["relation_type"] == "calls")
        .collect();
    assert_eq!(calls.len(), 11, "post-C calls inventory (measured)");

    // [F2] D8 multiplicity + H3 flip together: FOUR helper rows — two
    // plain (alpha) and two scoped (super::helper from module_a::run /
    // module_b::run), all bare-named `helper` after the scoped extraction.
    // The two (alpha → helper) pairs are the callsite-level multiplicity
    // D8 pins.
    let helper_rows: Vec<_> = calls.iter().filter(|r| r["to_name"] == "helper").collect();
    assert_eq!(
        helper_rows.len(),
        4,
        "2 plain + 2 scoped, all bare-named helper"
    );
    let first_to = helper_rows[0]["to_id"].clone();
    assert!(first_to.is_string(), "helper resolves same-file");
    assert!(
        helper_rows.iter().all(|r| r["to_id"] == first_to),
        "all four helper rows resolve to the same entity"
    );

    // H3 flip, scoped subset: the super::helper rows come from the two
    // module-scope run() functions.
    let scoped: Vec<_> = calls
        .iter()
        .filter(|r| {
            r["to_name"] == "helper" && r["from"].as_str().unwrap_or("").contains(":function:run")
        })
        .collect();
    assert_eq!(scoped.len(), 2, "H3 flipped: super::helper rows exist");
    assert!(scoped.iter().all(|r| r["to_id"].is_string()));

    // H1 flip: method_call w.describe() resolves against the impl-method
    // entity (name-based candidate, same-file unique).
    let describe: Vec<_> = calls
        .iter()
        .filter(|r| r["to_name"] == "describe")
        .collect();
    assert_eq!(describe.len(), 1, "H1 flipped: method call extracted");
    assert!(describe[0]["to_id"].is_string(), "describe resolves");

    // H2 flip: macros land with a bang suffix and stay unresolved —
    // the honest unresolved_macro class (dbg! + format!).
    let macros: Vec<_> = calls
        .iter()
        .filter(|r| r["to_name"].as_str().unwrap_or("").ends_with('!'))
        .collect();
    assert_eq!(macros.len(), 2, "H2 flipped: dbg! + format!");
    assert!(macros.iter().all(|r| r["to_id"].is_null()));

    // Honest ambiguity unchanged: bare run() from caller.rs (cross-file,
    // 2 candidates) AND crate::module_a::run() (bare-name view sees two
    // same-file run entities) both stay NULL — never a guessed target.
    let run_rows: Vec<_> = calls.iter().filter(|r| r["to_name"] == "run").collect();
    assert_eq!(run_rows.len(), 2);
    assert!(run_rows.iter().all(|r| r["to_id"].is_null()));

    // Unmodeled stdlib methods stay honestly unresolved (to_string has no
    // entity) — a NULL, never a fake resolution.
    let to_string: Vec<_> = calls
        .iter()
        .filter(|r| r["to_name"] == "to_string")
        .collect();
    assert_eq!(to_string.len(), 2);
    assert!(to_string.iter().all(|r| r["to_id"].is_null()));

    // H4 flip: implements resolves against kind=trait only — the
    // impl:Speaker collision no longer hides the trait target.
    let impls: Vec<_> = rows
        .iter()
        .filter(|r| r["relation_type"] == "implements")
        .collect();
    assert_eq!(impls.len(), 1);
    assert!(
        impls[0]["to_id"].is_string(),
        "H4 flipped: implements resolves"
    );

    // The coverage harness reads the SAME store — measured post-C metrics.
    let report = code_index_coverage::measure(&conn).unwrap();
    let calls_cov = report
        .types
        .iter()
        .find(|t| t.relation_type == "calls")
        .expect("calls coverage row");
    assert_eq!((calls_cov.extracted, calls_cov.resolved), (11, 5));
    let macro_class = report
        .target_classes
        .iter()
        .find(|t| t.class == "unresolved_macro")
        .map(|t| t.count)
        .unwrap_or(0);
    assert_eq!(macro_class, 2, "H2 flipped: unresolved_macro class is live");
    let impl_cov = report
        .types
        .iter()
        .find(|t| t.relation_type == "implements")
        .expect("implements coverage row");
    assert_eq!((impl_cov.extracted, impl_cov.resolved), (1, 1));
}
