//! Regression (2026-11-10): the position-blind parsers made `edit_node`
//! replace WHOLE LINES, and the CLI then crashed rendering the result.
//!
//! Verified against the shipped 0.24.0 binary before the fix:
//!
//! | format   | before                                                          |
//! |----------|-----------------------------------------------------------------|
//! | markdown | editing a paragraph also swallowed the following blank line      |
//! | markdown | editing a heading DELETED its `#` (span = line, content = text)  |
//! | css      | editing a declaration value replaced the whole rule line         |
//! | yaml     | edit refused by Duplex validation (nodes had line 1, col 0-0)    |
//! | toml     | same as yaml                                                     |
//! | any leaf | `TreeVisualizer::generate_sparkline` divided by zero AFTER the   |
//! |          | write, so the caller saw a crash on a success                    |
//!
//! Contract: editing one node must change exactly that node's bytes and leave
//! every other byte alone. The tests assert the FULL resulting file text, not
//! a substring, because "contains the new value" is what let the corruption
//! through in the first place.
use gnawtreewriter::core::EditOperation;
use gnawtreewriter::parser::TreeNode;

/// Deepest node whose content is exactly `needle`.
fn deepest_with_content<'a>(node: &'a TreeNode, needle: &str) -> Option<&'a TreeNode> {
    let mut best: Option<&TreeNode> = if node.content == needle {
        Some(node)
    } else {
        None
    };
    for child in &node.children {
        if let Some(found) = deepest_with_content(child, needle) {
            best = Some(found);
        }
    }
    best
}

fn edit_file(name: &str, source: &str, needle: &str, replacement: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    std::fs::write(&path, source).unwrap();

    let mut w = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
    let node_path = deepest_with_content(w.analyze(), needle)
        .unwrap_or_else(|| panic!("no node with content {needle:?} in {name}"))
        .path
        .clone();

    w.edit(
        EditOperation::Edit {
            node_path,
            content: replacement.to_string(),
        },
        false,
    )
    .expect("surgical edit must succeed (and must not panic)");

    std::fs::read_to_string(&path).unwrap()
}

#[test]
fn markdown_edit_keeps_heading_marker_and_blank_lines() {
    let src = "# Rubrik A\n\nEtt stycke.\n\n## Underrubrik\n\n- punkt ett\n";
    let want = "# Rubrik A\n\nNytt stycke.\n\n## Underrubrik\n\n- punkt ett\n";
    // Before the fix a paragraph edit consumed the blank line after it.
    assert_eq!(edit_file("a.md", src, "Ett stycke.", "Nytt stycke."), want);
}

#[test]
fn markdown_edit_of_heading_text_preserves_hash() {
    let src = "# Rubrik A\n\nEtt stycke.\n";
    let want = "# Ny rubrik\n\nEtt stycke.\n";
    // The heading's *text* is what an agent edits; the marker is syntax.
    assert_eq!(edit_file("b.md", src, "Rubrik A", "Ny rubrik"), want);
}

#[test]
fn css_edit_of_a_value_touches_only_that_value() {
    let src = "body { color: red; }\n";
    let want = "body { color: green; }\n";
    // Before the fix the whole rule line was replaced by "green".
    assert_eq!(edit_file("c.css", src, "red", "green"), want);
}

#[test]
fn yaml_edit_of_a_value_touches_only_that_value() {
    let src = "namn: demo\nversion: 1\n";
    let want = "namn: demo2\nversion: 1\n";
    // Before the fix: "The proposed edit broke the AST" — never applicable.
    assert_eq!(edit_file("d.yaml", src, "demo", "demo2"), want);
}

#[test]
fn toml_edit_of_a_value_touches_only_that_value() {
    let src = "namn = \"demo\"\nversion = 1\n";
    let want = "namn = \"demo2\"\nversion = 1\n";
    // `pair` nodes let us edit the value without touching the key.
    assert_eq!(edit_file("e.toml", src, "\"demo\"", "\"demo2\""), want);
}

#[test]
fn json_edit_stays_valid_and_goes_through_the_semantic_path() {
    // JSON is deliberately still on the interim path (J1): it navigates by
    // MAPPING KEY, so paths are "namn", not numeric. Swapping in
    // tree-sitter-json changes every JSON path and must be done together with
    // that function — this test pins today's contract so the swap cannot
    // happen by accident.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("f.json");
    std::fs::write(&path, "{\"namn\": \"demo\", \"tal\": 1}\n").unwrap();

    let mut w = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
    w.edit(
        EditOperation::Edit {
            node_path: "namn".to_string(),
            content: "demo2".to_string(),
        },
        false,
    )
    .expect("JSON edit must succeed");

    let text = std::fs::read_to_string(&path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("must stay valid JSON");
    assert_eq!(parsed["namn"], "demo2");
    assert_eq!(parsed["tal"], 1);
}

/// The added grammars must parse through the registry — an arm pointing at the
/// wrong struct (or a filename falling through to `TextParser`, which also
/// produces children) would otherwise only show up at edit time.
#[test]
fn added_grammars_parse_through_the_registry() {
    let cases = [
        (
            "build.ps1",
            "function Get-Foo {\n    param([int]$x)\n    return $x\n}\n",
            "program",
        ),
        (
            "CMakeLists.txt",
            "cmake_minimum_required(VERSION 3.20)\nproject(demo)\n",
            "normal_command",
        ),
        ("build.cmake", "add_library(y a.cpp)\n", "normal_command"),
        (
            "Makefile",
            "all: app\n\napp: main.o\n\tcc -o app main.o\n",
            "rule",
        ),
        (
            "lib.rb",
            "class Foo\n  def bar\n    1\n  end\nend\n",
            "class",
        ),
    ];

    for (name, source, want_kind) in cases {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        std::fs::write(&path, source).unwrap();
        let w = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
        let kinds = collect_kinds(w.analyze());
        assert!(
            kinds.iter().any(|k| k == want_kind),
            "{name} must parse with its grammar; wanted a {want_kind:?} node, got {kinds:?}"
        );
    }
}

fn collect_kinds(node: &TreeNode) -> Vec<String> {
    let mut out = vec![node.node_type.clone()];
    for c in &node.children {
        out.extend(collect_kinds(c));
    }
    out
}

/// Extensionless script files used to fall through to GenericParser.
#[test]
fn dotfile_resolves_to_the_bash_parser() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".bashrc");
    std::fs::write(&path, "export PATH=\"$PATH:/opt/bin\"\n").unwrap();
    let w = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
    let root = w.analyze();
    let kinds: Vec<&str> = root.children.iter().map(|c| c.node_type.as_str()).collect();
    assert!(
        kinds.iter().any(|k| k.contains("command")
            || k.contains("variable")
            || k.contains("declaration")
            || k.contains("export")),
        "expected bash node kinds under .bashrc, got {kinds:?}"
    );
}
