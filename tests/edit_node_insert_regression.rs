//! Node-addressing semantics — pins what an edit path TARGETS so future
//! path confusion is diagnosable as caller error, not tool error.
//!
//! Background (issue log 2026-11-03, closed as caller error — the B1
//! class): a batch derived paths from an earlier file state; after a
//! use-block insertion every path shifted one node. GTW replaced exactly
//! the ADDRESSED node (an attribute) — correct per addressing, wrong per
//! the caller's intent. Rules pinned here:
//!
//! 1. A path addresses EXACTLY one node — doc comments and attributes
//!    are separate sibling nodes, each with its own path.
//! 2. Editing an attribute/preceding node does NOT touch the function
//!    after it — and (F2) new content starting with `///` swallows the
//!    PRECEDING doc block.
//! 3. Path verification belongs to the caller: read_node the target
//!    (name/kind) before edit_node, re-query after structural edits.

use gnawtreewriter::core::EditOperation;
use gnawtreewriter::GnawTreeWriter;

const SOURCE: &str = "/// Doc for alpha.\n#[derive(Clone)]\nfn alpha() -> u8 { 1 }\n\n/// Doc for beta.\nfn beta() -> u8 { 2 }\n";

fn edit(path: &str, content: &str, dir: &tempfile::TempDir) -> String {
    let f = dir.path().join("m.rs");
    std::fs::write(&f, SOURCE).unwrap();
    let mut w = GnawTreeWriter::new(f.to_str().unwrap()).unwrap();
    w.edit(
        EditOperation::Edit {
            node_path: path.to_string(),
            content: content.to_string(),
        },
        false,
    )
    .unwrap();
    std::fs::read_to_string(&f).unwrap()
}

#[test]
fn attribute_path_replaces_attribute_only_and_f2_swallows_doc() {
    let dir = tempfile::tempdir().unwrap();
    // Path 1 = the ATTRIBUTE (children: 0=doc, 1=attribute, 2=fn, …).
    let out = edit(
        "1",
        "/// Doc for replacement_target.\nfn replacement_target() -> u8 { 9 }\n",
        &dir,
    );
    // F2: the new content's doc swallows the OLD doc line…
    assert!(
        !out.contains("/// Doc for alpha."),
        "old doc swallowed: {out}"
    );
    // …the attribute is replaced by the new fn…
    assert!(
        out.contains("fn replacement_target() -> u8 { 9 }"),
        "addressed node replaced: {out}"
    );
    // …and the function AFTER it is untouched — one alpha, no duplicates.
    assert_eq!(out.matches("fn alpha").count(), 1, "fn untouched: {out}");
    assert!(out.contains("fn beta"));
}

#[test]
fn function_path_replaces_function_only_and_keeps_doc_and_attribute() {
    let dir = tempfile::tempdir().unwrap();
    // Path 2 = the FUNCTION (doc and attribute keep their own paths).
    let out = edit("2", "fn alpha_v2() -> u8 { 9 }\n", &dir);
    assert!(out.contains("/// Doc for alpha."), "doc kept: {out}");
    assert!(out.contains("#[derive(Clone)]"), "attribute kept: {out}");
    assert!(
        out.contains("fn alpha_v2() -> u8 { 9 }"),
        "fn replaced: {out}"
    );
    assert_eq!(out.matches("fn alpha").count(), 1, "no duplicates: {out}");
    assert!(out.contains("fn beta"));
}

#[test]
fn struct_style_multi_item_content_replaces_exactly_the_addressed_node() {
    // The incident shape: doc + attribute precede the target; the
    // replacement content itself carries doc + attribute + struct (3
    // items). Addressing the TARGET replaces the target — the
    // preceding siblings stay.
    let dir = tempfile::tempdir().unwrap();
    let content =
        "/// Doc for replacement_target.\n#[derive(Clone)]\nfn replacement_target() -> u8 { 9 }\n";
    let out = edit("2", content, &dir);
    assert!(
        out.contains("fn replacement_target"),
        "target replaced: {out}"
    );
    assert_eq!(
        out.matches("fn replacement_target").count(),
        1,
        "exactly one replacement — no duplication: {out}"
    );
    // The addressed fn is gone (replaced); its doc + attribute stay.
    assert_eq!(out.matches("fn alpha").count(), 0, "alpha replaced: {out}");
    assert!(out.contains("#[derive(Clone)]"));
}
