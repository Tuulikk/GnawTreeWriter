//! Repro for the issue-log finding 2026-11-03: an edit whose target is
//! preceded by a doc comment + attribute INSERTS the new content instead
//! of replacing — the old definition survives as a duplicate (compile
//! error E0428 downstream, tool reports success).
//!
//! Bisection (2026-11-03, gnawtreewriter 0.22.0):
//! - doc comment only            → replace works
//! - doc comment + attribute     → DUPLICATES (this test)
//! Repro'd via CLI `edit` and via batch edit-ops on a comment-dense file;
//! same funnel (core preview_edit / edit_node_at_path, F2 doc-swallown
//! region). The region removes [doc..attribute] but leaves
//! [node.start_line..node.end_line] — insertion instead of replacement.
//!
//! FIX TARGET: the F2 region must span [region_start .. node.end_line]
//! inclusive. Unignore this test when fixed — it is the completion
//! criterion.

use gnawtreewriter::core::EditOperation;
use gnawtreewriter::GnawTreeWriter;

#[test]
#[ignore = "LIVE BUG (issue log 2026-11-03): edit on doc+attribute-preceded node inserts instead of replacing — old definition survives as duplicate"]
fn edit_replaces_node_with_doc_and_attribute() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("m.rs");
    std::fs::write(
        &f,
        "/// Doc for alpha.\n#[derive(Clone)]\nfn alpha() -> u8 { 1 }\n\n/// Doc for beta.\nfn beta() -> u8 { 2 }\n",
    )
    .unwrap();

    let mut w = GnawTreeWriter::new(f.to_str().unwrap()).unwrap();
    w.edit(
        EditOperation::Edit {
            node_path: "1".to_string(),
            content: "/// New doc for alpha.\nfn alpha() -> u8 { 9 }\n".to_string(),
        },
        false,
    )
    .unwrap();

    let out = std::fs::read_to_string(&f).unwrap();
    assert_eq!(
        out.matches("fn alpha").count(),
        1,
        "old alpha must be replaced, not duplicated:\n{out}"
    );
    assert!(out.contains("fn alpha() -> u8 { 9 }"), "new content present");
    assert!(out.contains("fn beta"), "unrelated fn untouched");
}
