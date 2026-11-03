//! T6 tests — canonical entity/relation ID formats.
//!
//! Contract (review decision #12 + issue-log T6): IDs are
//! `gtw:{canonical_relpath}:{type}:{name}` — impl methods qualified
//! `Type::name`, deterministic `#N` disambiguation, paths POSIX +
//! toplevel-relative. Relations carry the honest target split:
//! `to_name` always, `to_id` only on unique resolution.

use gnawtreewriter::core::code_index_db;
use gnawtreewriter::core::index_entities::{canonical_entity_path, extract_entities_from_tree};
use gnawtreewriter::core::index_relations::{
    extract_relations_from_tree, resolve_relations, Relation,
};
use std::path::Path;

fn parse(code: &str) -> gnawtreewriter::parser::TreeNode {
    let parser = gnawtreewriter::parser::get_parser(Path::new("x.rs")).unwrap();
    parser.parse_lenient(code).0.unwrap()
}

#[test]
fn impl_methods_are_qualified_in_ids() {
    let code = r#"
struct A;
struct B;
impl A {
    pub fn new() -> Self { A }
}
impl B {
    pub fn new() -> Self { B }
}
"#;
    let tree = parse(code);
    let idx = extract_entities_from_tree(&tree, code, "src/ab.rs", true);
    let ids: Vec<&str> = idx
        .entities
        .iter()
        .flat_map(|e| e.children.iter().map(|c| c.id.as_str()))
        .collect();
    assert!(
        ids.iter().any(|i| i.ends_with(":function:A::new")),
        "A::new qualified: {ids:?}"
    );
    assert!(
        ids.iter().any(|i| i.ends_with(":function:B::new")),
        "B::new qualified: {ids:?}"
    );
    // The old bare form must be gone — it collided.
    assert!(
        !ids.iter().any(|i| i.ends_with(":function:new")),
        "bare method id must not exist: {ids:?}"
    );
}

#[test]
fn disambiguation_is_deterministic_by_line_order() {
    // Two impls of the SAME type with the same method — the later one
    // (by line) gets "#2".
    let code = r#"
struct S;
impl S {
    pub fn new() -> Self { S }
}
impl S {
    pub fn new() -> Self { S }
}
"#;
    let tree = parse(code);
    let idx = extract_entities_from_tree(&tree, code, "src/s.rs", true);
    let mut ids: Vec<String> = idx
        .entities
        .iter()
        .flat_map(|e| e.children.iter().map(|c| c.id.clone()))
        .collect();
    ids.sort();
    assert_eq!(
        ids,
        vec![
            "gtw:src/s.rs:function:S::new",
            "gtw:src/s.rs:function:S::new#2"
        ],
        "later duplicate gets #2: {ids:?}"
    );
    // Same input, same output (determinism principle).
    let tree2 = parse(code);
    let idx2 = extract_entities_from_tree(&tree2, code, "src/s.rs", true);
    let mut ids2: Vec<String> = idx2
        .entities
        .iter()
        .flat_map(|e| e.children.iter().map(|c| c.id.clone()))
        .collect();
    ids2.sort();
    assert_eq!(ids, ids2);
}

#[test]
fn entity_paths_are_canonical() {
    assert_eq!(canonical_entity_path("src/a.rs"), "src/a.rs");
    assert_eq!(canonical_entity_path("src\\win.rs"), "src/win.rs");
    // Absolute path OUTSIDE any git work tree (tempdir): POSIX-ized as-is.
    let dir = tempfile::tempdir().unwrap();
    let abs = dir.path().join("sub").join("x.rs");
    let want = format!("{}/sub/x.rs", dir.path().display());
    assert_eq!(canonical_entity_path(abs.to_str().unwrap()), want);
}

#[test]
fn relations_carry_honest_targets() {
    let code = r#"
use std::collections::HashMap;

pub struct Alpha;
impl Alpha {
    pub fn run(&self) -> u8 { helper() }
}

pub fn helper() -> u8 { 1 }
"#;
    let tree = parse(code);
    let ent = extract_entities_from_tree(&tree, code, "src/alpha.rs", true);
    let rels = extract_relations_from_tree(&tree, code, "src/alpha.rs");
    let RelationIndex { mut relations, .. } = rels;
    resolve_relations(&mut relations, &ent.entities);

    // Calls to same-file fn resolve to the full entity id…
    let calls: Vec<&Relation> = relations
        .iter()
        .filter(|r| r.relation_type == "calls")
        .collect();
    assert!(
        calls.iter().any(|r| r.to_name == "helper"
            && r.to_id.as_deref() == Some("gtw:src/alpha.rs:function:helper")),
        "same-file call must resolve: {calls:?}"
    );
    // …and the impl-internal caller is qualified.
    assert!(
        calls
            .iter()
            .any(|r| r.from.ends_with(":function:Alpha::run")),
        "caller scope qualified: {calls:?}"
    );
    // External module path stays unresolved — a symbol, not a reference.
    let imports: Vec<&Relation> = relations
        .iter()
        .filter(|r| r.relation_type == "imports")
        .collect();
    assert!(
        imports
            .iter()
            .any(|r| r.to_name.contains("HashMap") && r.to_id.is_none()),
        "external import must have to_id=None: {imports:?}"
    );
}

// Small local alias so the destructure above reads clean.
use gnawtreewriter::core::index_relations::RelationIndex;

#[test]
fn db_stores_graph_tables_and_resolves_cross_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".git"), "gitdir: nowhere\n").unwrap();
    std::fs::write(
        dir.path().join("lib.rs"),
        "use std::collections::HashMap;\npub fn shared() -> u8 { 1 }\npub fn uses_shared() -> u8 { shared() }\npub fn uses_missing() -> u8 { missing_fn() }\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("other.rs"),
        "pub fn other_calls() -> u8 { shared() }\n",
    )
    .unwrap();

    let opts = gnawtreewriter::core::code_index::CodeIndexOptions {
        extensions: vec!["rs".to_string()],
        ..Default::default()
    };
    let run = code_index_db::generate_with_db(dir.path(), &opts).unwrap();
    assert!(run.stats.files_parsed >= 2);

    // Cross-file edge: other.rs's call to shared() resolves against
    // lib.rs's entity (full-index resolution pass).
    let conn = code_index_db::open_existing(dir.path()).unwrap();
    let rels = code_index_db::load_relations(&conn, Some("other.rs")).unwrap();
    assert!(
        rels.iter().any(|r| {
            r["to_name"] == "shared"
                && r["to_id"] == serde_json::json!("gtw:lib.rs:function:shared")
        }),
        "cross-file resolution failed: {rels:?}"
    );
    // Symbols: unresolved targets aggregate — std-path imports are marked
    // external; the call to a non-existent fn stays unresolved.
    let symbols = code_index_db::load_symbols(&conn).unwrap();
    assert!(
        symbols
            .iter()
            .any(|s| s["name"] == "missing_fn" && s["external"] == false),
        "unresolved call expected as symbol: {symbols:?}"
    );
    assert!(
        symbols
            .iter()
            .any(|s| s["name"].as_str().unwrap_or("").contains("std::") && s["external"] == true),
        "external import expected as symbol: {symbols:?}"
    );
}
