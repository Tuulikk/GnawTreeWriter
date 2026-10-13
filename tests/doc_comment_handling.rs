//! F2/F3 regression tests (batt 2026-10-13, session 53a53545).
//!
//! F2: doc comments (///) are SIBLING nodes in tree-sitter — replacing or
//! deleting a definition must swallow the preceding /// block, otherwise the
//! old doc survives next to the new content's own doc (the duplication the
//! agent fought in the batt run).
//!
//! F3: numeric node paths are POSITIONS that shift after every edit.
//! `describe_node` must reveal what a path actually resolved to (path, type,
//! name) so a stale path hitting the wrong node is visible in the tool reply
//! instead of silently replacing a doc comment with unrelated code.
use gnawtreewriter::core::EditOperation;

fn make_file(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

const SRC: &str = "/// Konverterar ett heltal.\n///\n/// MEDVETEN BUGG: bara repetition.\npub fn till(n: u32) -> String {\n    \"I\".repeat(n as usize)\n}\n\npub fn annan() {}\n";

#[test]
fn edit_with_own_doc_swallows_the_old_doc_block() {
    let dir = temp_dir();
    let path = make_file(dir.path(), "lib.rs", SRC);
    let mut w = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
    w.edit(
        EditOperation::Edit {
            node_path: "@fn:till".to_string(),
            content: "/// Konverterar ett heltal (1..=3999).\npub fn till(n: u32) -> String {\n    String::new()\n}\n"
                .to_string(),
        },
        false,
    )
    .unwrap();

    let out = std::fs::read_to_string(&path).unwrap();
    let doc_lines = out.lines().filter(|l| l.trim_start().starts_with("///")).count();
    assert_eq!(doc_lines, 1, "exakt ETT doc-radsblock kvar: {out}");
    assert!(!out.contains("MEDVETEN BUGG"), "gammal doc ska bort: {out}");
    assert!(out.contains("Konverterar ett heltal (1..=3999)"), "ny doc ska finnas: {out}");
    assert!(out.contains("pub fn annan"), "omvärlden orörd: {out}");
}

#[test]
fn edit_without_own_doc_keeps_the_old_doc_block() {
    let dir = temp_dir();
    let path = make_file(dir.path(), "lib.rs", SRC);
    let mut w = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
    w.edit(
        EditOperation::Edit {
            node_path: "@fn:till".to_string(),
            content: "pub fn till(n: u32) -> String {\n    String::new()\n}\n".to_string(),
        },
        false,
    )
    .unwrap();

    let out = std::fs::read_to_string(&path).unwrap();
    assert!(out.contains("MEDVETEN BUGG"), "doc utan egen doc i nytt innehåll ska bevaras: {out}");
}

#[test]
fn delete_leaves_doc_to_the_caller_layer() {
    // Kontrakt (§45, en sanning per regel): core-radering tar ENDAST noden —
    /// doc-blocket ovanför ägs av Motor2:s F2d-kedja (attached_comment_paths).
    // Detta test låser kontraktet så ingen "hjälpsam" fix skapar dubbel ägarskap.
    let dir = temp_dir();
    let path = make_file(dir.path(), "lib.rs", SRC);
    let mut w = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
    w.edit(EditOperation::Delete { node_path: "@fn:till".to_string() }, false)
        .unwrap();

    let out = std::fs::read_to_string(&path).unwrap();
    assert!(!out.contains("pub fn till"), "noden borta: {out}");
    assert!(out.contains("MEDVETEN BUGG"), "doc-blocket lämnas till F2d-lagret: {out}");
    assert!(out.contains("pub fn annan"), "omvärlden orörd: {out}");
}

#[test]
fn describe_node_reveals_identity_of_the_resolved_target() {
    let dir = temp_dir();
    let path = make_file(dir.path(), "lib.rs", SRC);
    let w = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();

    // Identitets-form: @fn:till → funktionen, med namn-eko (F3:s signal).
    let (_p, t, n) = w.describe_node("@fn:till").expect("@fn:till ska resolve:a");
    assert!(t.contains("function"), "förväntad funktion, fick {t}");
    assert_eq!(n.as_deref(), Some("till"));

    // BATT-EVIDENSEN (53a53545): doc-kommentarer är numrerade syskon-noder —
    // "1" här är line_comment, INTE funktionen. En modell som återanvänt en
    // gammal numerisk path efter en edit träffar exakt sådana noder; eko:t
    // (typ+namn i svaret) är det som gör fel-träffen synlig i stället för tyst.
    let (_p2, t2, _n2) = w.describe_node("1").expect("nod 1 ska resolve:a");
    assert_eq!(t2, "line_comment", "doc-noder är egna numrerade syskon");

    assert!(w.describe_node("999").is_none(), "oresolverbar path = None (inte gissning)");
}
