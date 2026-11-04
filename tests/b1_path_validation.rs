//! B1 tests — path freshness: receipts carry the new path + content hash,
//! and expect-declarations reject stale paths BEFORE writing
//! (GTW_RAPPORT_2026-11-01 B1 + the 2026-11-03 incident).

use gnawtreewriter::core::EditOperation;
use gnawtreewriter::GnawTreeWriter;

const SOURCE: &str =
    "/// Doc for alpha.\nfn alpha() -> u8 { 1 }\n\n/// Doc for beta.\nfn beta() -> u8 { 2 }\n";

fn setup(dir: &tempfile::TempDir) -> (std::path::PathBuf, GnawTreeWriter) {
    let f = dir.path().join("m.rs");
    std::fs::write(&f, SOURCE).unwrap();
    let w = GnawTreeWriter::new(f.to_str().unwrap()).unwrap();
    (f, w)
}

#[test]
fn receipt_carries_new_path_and_content_hash() {
    let dir = tempfile::tempdir().unwrap();
    let (f, mut w) = setup(&dir);
    w.edit(
        EditOperation::Edit {
            node_path: "1".to_string(),
            content: "/// Doc for alpha.\nfn alpha() -> u8 { 9 }\n".to_string(),
        },
        false,
    )
    .unwrap();

    let receipt = w.last_edit_receipt().expect("receipt present");
    // Content replacement keeps the ordinal position: the new path is
    // authoritative for the NEXT edit.
    assert_eq!(receipt.new_node_path, "1");
    assert!(
        !receipt.node_content_hash.is_empty(),
        "freshness hash present"
    );
    assert!(!f.to_str().unwrap().is_empty());
}

#[test]
fn expect_mismatch_rejects_before_writing() {
    let dir = tempfile::tempdir().unwrap();
    let (f, w) = setup(&dir);
    let before = std::fs::read_to_string(&f).unwrap();

    // The caller expects "beta" at path 1 — the node there is "alpha":
    // stale path, must reject BEFORE writing.
    let err = w
        .expect_node("1", Some("beta"), None)
        .expect_err("stale expectation must reject");
    assert!(err.to_string().contains("PATH STALE"), "{err}");
    assert_eq!(
        std::fs::read_to_string(&f).unwrap(),
        before,
        "rejected expectation writes nothing"
    );
}

#[test]
fn expect_match_passes_and_edit_lands() {
    let dir = tempfile::tempdir().unwrap();
    let (f, mut w) = setup(&dir);
    w.expect_node("1", Some("alpha"), Some("function_item"))
        .unwrap();
    w.edit(
        EditOperation::Edit {
            node_path: "1".to_string(),
            content: "/// Doc for alpha.\nfn alpha() -> u8 { 9 }\n".to_string(),
        },
        false,
    )
    .unwrap();
    assert!(std::fs::read_to_string(&f).unwrap().contains("{ 9 }"));
}

#[test]
fn the_2026_11_03_incident_is_caught_by_expect() {
    // The incident shape: doc + attribute precede the target; the caller
    // addresses the attribute path believing it is the struct. An
    // expect_name declaration catches it BEFORE the wrong write.
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("inc.rs");
    std::fs::write(
        &f,
        "/// Doc for alpha.\n#[derive(Clone)]\nfn alpha() -> u8 { 1 }\n",
    )
    .unwrap();
    let w = GnawTreeWriter::new(f.to_str().unwrap()).unwrap();
    // children: 0=doc, 1=attribute, 2=fn — path "1" is the ATTRIBUTE.
    let err = w
        .expect_node("1", Some("alpha"), None)
        .expect_err("attribute path must not satisfy expect_name=alpha");
    assert!(err.to_string().contains("PATH STALE"), "{err}");
}

#[test]
fn missing_path_is_a_clear_stale_path_error() {
    let dir = tempfile::tempdir().unwrap();
    let (_f, w) = setup(&dir);
    let err = w
        .expect_node("999.9", Some("anything"), None)
        .expect_err("missing path must error");
    assert!(err.to_string().contains("positional"), "{err}");
}
