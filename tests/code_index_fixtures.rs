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
#[test]
fn ast_observation_facit_keeps_syntax_separate_from_relation_and_store() {
    use gnawtreewriter::parser::TreeNode;

    const SOURCE: &str = r#"pub fn foo() {}
pub fn solo() {}
pub struct Obj;
impl Obj { pub fn foo(&self) {} }
pub mod module { pub fn foo() {} pub struct Thing; }
macro_rules! foo { () => {} }
pub fn observe(obj: Obj) {
    foo();
    module::foo();
    obj.foo();
    foo!();
    foo(); foo();
    solo();
}
pub mod nested {
    pub fn call() { super::foo(); }
}
pub struct Holder { pub item: module::Thing }
"#;
    fn collect<'a>(node: &'a TreeNode, kind: &str, out: &mut Vec<&'a TreeNode>) {
        if node.node_type == kind {
            out.push(node);
        }
        for child in &node.children {
            collect(child, kind, out);
        }
    }
    fn anchor(tree: &TreeNode, node: &TreeNode, source: &str, line: usize, occurrence: usize) {
        let text = source.lines().nth(line - 1).unwrap();
        let expected_col = text.match_indices(&node.content).nth(occurrence).unwrap().0 + 1;
        assert_eq!((node.start_line, node.start_col), (line, expected_col));
        assert_eq!(
            (node.end_line, node.end_col),
            (line, expected_col + node.content.chars().count())
        );
        assert!(!node.path.is_empty());
        assert_eq!(tree.find_path(&node.path).unwrap().content, node.content);
    }

    let (_dir, root) = build_fixture();
    let file = root.join("src/observations.rs");
    std::fs::write(&file, SOURCE).unwrap();
    let writer = gnawtreewriter::GnawTreeWriter::new(file.to_str().unwrap()).unwrap();
    let tree = writer.analyze();
    let rel_file = "src/observations.rs";
    let mut calls = Vec::new();
    collect(tree, "call_expression", &mut calls);
    let call = |line: usize, content: &str, occurrence: usize| {
        calls
            .iter()
            .copied()
            .filter(|n| n.start_line == line && n.content == content)
            .nth(occurrence)
            .unwrap()
    };
    let cases = [
        (8, "foo()", "identifier", "foo", &["foo"][..], "6.4.1.0"),
        (
            9,
            "module::foo()",
            "scoped_identifier",
            "module::foo",
            &["module", "foo"][..],
            "6.4.2.0",
        ),
        (
            10,
            "obj.foo()",
            "field_expression",
            "obj.foo",
            &["obj", "foo"][..],
            "6.4.3.0",
        ),
        (12, "foo()", "identifier", "foo", &["foo"][..], "6.4.5.0"),
        (
            16,
            "super::foo()",
            "scoped_identifier",
            "super::foo",
            &["super", "foo"][..],
            "7.3.1.4.1.0",
        ),
        (13, "solo()", "identifier", "solo", &["solo"][..], "6.4.7.0"),
    ];
    for (line, written_call, callee_kind, written_target, components, expected_path) in cases {
        let node = call(line, written_call, 0);
        anchor(tree, node, SOURCE, line, 0);
        assert_eq!(node.path, expected_path);
        let callee = node
            .children
            .iter()
            .find(|c| c.node_type == callee_kind)
            .unwrap();
        assert_eq!(callee.content, written_target);
        let actual_components: Vec<_> = callee
            .content
            .split([':', '.'])
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(actual_components, components);
    }
    let same_line: Vec<_> = calls
        .iter()
        .copied()
        .filter(|n| n.start_line == 12 && n.content == "foo()")
        .collect();
    assert_eq!(same_line.len(), 2);
    anchor(tree, same_line[0], SOURCE, 12, 0);
    anchor(tree, same_line[1], SOURCE, 12, 1);
    assert_eq!(same_line[0].path, "6.4.5.0");
    assert_eq!(same_line[1].path, "6.4.6.0");
    assert_ne!(same_line[0].path, same_line[1].path);

    let mut macros = Vec::new();
    collect(tree, "macro_invocation", &mut macros);
    let macro_call = macros.iter().copied().find(|n| n.start_line == 11).unwrap();
    anchor(tree, macro_call, SOURCE, 11, 0);
    assert_eq!(macro_call.content, "foo!()");
    assert_eq!(macro_call.path, "6.4.4.0");
    assert!(macro_call.children.iter().any(|c| c.content == "foo"));
    eprintln!(
        "AST macro {} at {}:{}-{} path={}",
        macro_call.content,
        macro_call.start_line,
        macro_call.start_col,
        macro_call.end_col,
        macro_call.path
    );

    let mut types = Vec::new();
    collect(tree, "scoped_type_identifier", &mut types);
    let field_type = types
        .iter()
        .copied()
        .find(|n| n.content == "module::Thing")
        .unwrap();
    anchor(tree, field_type, SOURCE, 18, 0);
    assert_eq!(
        field_type.content.split("::").collect::<Vec<_>>(),
        ["module", "Thing"]
    );
    assert_eq!(field_type.path, "8.3.1.3");

    let extraction =
        gnawtreewriter::core::index_relations::extract_relations_with_observations_from_tree(
            tree, SOURCE, rel_file,
        );
    let extracted = &extraction.index;
    let legacy =
        gnawtreewriter::core::index_relations::extract_relations_from_tree(tree, SOURCE, rel_file);
    assert_eq!(
        serde_json::to_value(&legacy).unwrap(),
        serde_json::to_value(extracted).unwrap(),
        "observations must not change legacy relations or summary"
    );
    use gnawtreewriter::core::index_relations::CallForm;
    let expected_observations = [
        (8, "foo", CallForm::Identifier, "foo", "6.4.1.0"),
        (9, "module::foo", CallForm::Qualified, "module", "6.4.2.0"),
        (10, "obj.foo", CallForm::Method, "foo", "6.4.3.0"),
        (11, "foo!", CallForm::Macro, "foo!", "6.4.4.0"),
        (12, "foo", CallForm::Identifier, "foo", "6.4.5.0"),
        (12, "foo", CallForm::Identifier, "foo", "6.4.6.0"),
        (13, "solo", CallForm::Identifier, "solo", "6.4.7.0"),
        (16, "super::foo", CallForm::Qualified, "foo", "7.3.1.4.1.0"),
    ];
    assert_eq!(
        extraction.call_observations.len(),
        expected_observations.len()
    );
    for (line, written, form, to_name, path) in expected_observations {
        let observation = extraction
            .call_observations
            .iter()
            .find(|o| o.anchor.node_path == path)
            .expect("one observation per AST call site");
        assert_eq!(observation.form, form);
        assert_eq!(observation.written_target, written);
        let anchor = &observation.anchor;
        assert_eq!(anchor.file, rel_file);
        let ast = tree.find_path(path).unwrap();
        assert_eq!(
            (
                anchor.start_line,
                anchor.start_col,
                anchor.end_line,
                anchor.end_col
            ),
            (ast.start_line, ast.start_col, ast.end_line, ast.end_col)
        );
        assert_eq!(anchor.start_line, line);
        let relation = &extracted.relations[observation.relation_index];
        assert_eq!(relation.relation_type, "calls");
        assert_eq!(relation.to_name, to_name);
        assert_eq!(relation.line, line);
        assert_eq!(relation.file, rel_file);
    }
    let same_line_observations: Vec<_> = extraction
        .call_observations
        .iter()
        .filter(|o| o.anchor.start_line == 12 && o.written_target == "foo")
        .collect();
    assert_eq!(same_line_observations.len(), 2);
    assert_ne!(
        same_line_observations[0].relation_index,
        same_line_observations[1].relation_index
    );
    assert_ne!(
        same_line_observations[0].anchor.start_col,
        same_line_observations[1].anchor.start_col
    );
    // The additive Rust data does not change the existing CLI/MCP JSON.
    assert!(serde_json::to_value(extracted)
        .unwrap()
        .get("call_observations")
        .is_none());
    let observed = |line: usize, relation_type: &str, to_name: &str| {
        extracted
            .relations
            .iter()
            .filter(|r| r.line == line && r.relation_type == relation_type && r.to_name == to_name)
            .collect::<Vec<_>>()
    };
    // get_name() returns the FIRST identifier of module::foo, but the last
    // identifier of super::foo (super is a keyword). Pin the inconsistency.
    for (line, name) in [(8, "foo"), (9, "module"), (10, "foo"), (16, "foo")] {
        assert_eq!(observed(line, "calls", name).len(), 1);
    }
    assert_eq!(observed(11, "calls", "foo!").len(), 1);
    assert_eq!(
        observed(12, "calls", "foo").len(),
        2,
        "two same-line calls remain two rows"
    );
    assert_eq!(observed(13, "calls", "solo").len(), 1);
    assert!(
        observed(13, "calls", "solo")[0].to_id.is_none(),
        "extraction observes the call before resolution"
    );
    assert!(observed(9, "calls", "module")[0].to_id.is_none());
    let type_rels = observed(18, "uses", "Thing");
    assert_eq!(type_rels.len(), 1);
    assert_eq!(
        observed(18, "uses", "module").len(),
        1,
        "scoped type emits a parent-name edge too"
    );
    let entities = gnawtreewriter::core::index_entities::extract_entities_from_tree(
        tree, SOURCE, rel_file, true,
    );
    let holder = entities
        .entities
        .iter()
        .find(|e| e.name == "Holder")
        .unwrap();
    assert_eq!(holder.entity_type, "struct");
    assert_eq!(holder.id, "gtw:src/observations.rs:struct:Holder");
    assert!(type_rels
        .iter()
        .all(|r| r.from == "gtw:src/observations.rs:function:Holder"));
    assert_ne!(
        type_rels[0].from, holder.id,
        "current extractor mislabels the struct source"
    );

    let (mut conn, _) = code_index_db::open_or_recreate(&root).unwrap();
    code_index_db::refresh(&mut conn, &root, &CodeIndexOptions::default()).unwrap();
    let stored = code_index_db::load_relations(&conn, Some(rel_file)).unwrap();
    let stored_at = |line: usize, kind: &str, name: &str| {
        stored
            .iter()
            .filter(|r| r["line"] == line && r["relation_type"] == kind && r["to_name"] == name)
            .collect::<Vec<_>>()
    };
    for (line, name) in [(8, "foo"), (9, "module"), (10, "foo"), (16, "foo")] {
        let rows = stored_at(line, "calls", name);
        assert_eq!(rows.len(), 1);
        if line == 9 {
            assert_eq!(
                rows[0]["to_id"], "gtw:src/observations.rs:mod:module",
                "qualified call currently resolves to the module, not its foo"
            );
        } else {
            assert!(
                rows[0]["to_id"].is_null(),
                "ambiguous foo targets stay unresolved"
            );
        }
        assert!(
            rows[0].get("node_path").is_none(),
            "AST path is not stored on relations"
        );
        assert!(
            rows[0].get("syntax_form").is_none(),
            "observed form is not stored"
        );
        assert!(
            rows[0].get("written_target").is_none(),
            "written target and its components are not stored"
        );
        assert!(
            rows[0].get("start_col").is_none(),
            "exact column span is not stored"
        );
    }
    assert_eq!(stored_at(11, "calls", "foo!").len(), 1);
    let same_line_stored = stored_at(12, "calls", "foo");
    assert_eq!(same_line_stored.len(), 2);
    assert_eq!(
        same_line_stored[0], same_line_stored[1],
        "two sites have distinct AST paths but identical exposed relation rows"
    );
    assert_eq!(
        stored_at(13, "calls", "solo")[0]["to_id"],
        "gtw:src/observations.rs:function:solo"
    );
    let stored_type = stored_at(18, "uses", "Thing");
    assert!(!stored_type.is_empty());
    assert_eq!(
        stored_type[0]["from"],
        "gtw:src/observations.rs:function:Holder"
    );
    assert_eq!(
        stored_type[0]["to_id"],
        "gtw:src/observations.rs:struct:Thing"
    );
    eprintln!("AST module::Thing -> extracted uses/Thing from function:Holder; actual source={}; DB has line but no AST path", holder.id);
}
