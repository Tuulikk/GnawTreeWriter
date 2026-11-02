//! Regression (2026-10-30, GTW_MCP_ISSUE_LOG — upptäckt i Jobseeker-sessionen):
//! edit_node på ett JS statement_block INUTI en .then(function(){...})-kedja
//! korrumperade filen TYST. Rot-orsak: parsers utan kolumner (js/ts/go/java/
//! bash/css/... 20 st) satte start_col/end_col = 0 och föll i hel-raders-
//! fallbacken i edit_node_at_path — ett block som BÖRJAR mid-line (`.then(
//! function () {`) och SLUTAR mid-line (`})` på egen rad) fick hela sin första
//! och sista rad utbytta → promise-kedjan bruten, `SyntaxError: Unexpected
//! token '.'`. Samma bugg-klass: rust.rs satte BYTE-kolumner medan splatten
//! räknar TECKEN (å/ä/ö före noden på samma rad divergerar).
//!
//! Fix: alla tree-sitter-parsers använder nu char_col_pair() (tecken-baserat,
//! UTF-8-säkert). Kontrakt under test: edit_node på ett mid-line-block
//! bevarar rad-kontexten före/efter blocket, kedjan runt om och multibyte.
//!
//! Repro-mönstret är exakt det som slogs sönder i pkg/mods/jobs.js
//! (Jobseeker): `.then(...).then(function () { ... }).catch(...)`.
use gnawtreewriter::core::EditOperation;
use gnawtreewriter::parser::TreeNode;

const JS_FILE: &str = r#"// Åäö + emoji i prefixet — kolumnräkningen ska vara tecken-baserad 🌲
(function () {
    'use strict';
    function outer() {
        var box = { value: '' };
        fetch('/api/x', { method: 'PUT', body: JSON.stringify(box) })
            .then(function (r) { if (!r.ok) throw new Error('HTTP ' + r.status); return r.json(); })
            .then(function () {
                box.value = 'sparat';
                notify('ok');
            })
            .catch(function (e2) { notify('err', e2.message); });
    }
})();
"#;

/// Djup-walk: returnera path till den DJUPASTE nod vars content innehåller
/// `needle` OCH börjar med `starts_with` (ankrar nod-typen — annars landar
/// man på ett djupare uttryck inuti blocket). Numeriska top-level-loopar
/// räcker inte — målet ligger flera nivåer ner i IIFE → funktionsuttryck →
/// promise-kedja → funktionsuttryck → block.
fn find_path_containing(node: &TreeNode, needle: &str, starts_with: &str) -> Option<String> {
    let matches =
        node.content.contains(needle) && node.content.trim_start().starts_with(starts_with);
    if matches {
        for child in &node.children {
            if let Some(deeper) = find_path_containing(child, needle, starts_with) {
                return Some(deeper);
            }
        }
        return Some(node.path.clone());
    }
    for child in &node.children {
        if let Some(found) = find_path_containing(child, needle, starts_with) {
            return Some(found);
        }
    }
    None
}

#[test]
fn edit_node_on_midline_js_block_preserves_promise_chain() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("repro.js");
    std::fs::write(&path, JS_FILE).unwrap();

    let mut w = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
    let tree = w.analyze();
    let target = find_path_containing(tree, "notify('ok')", "{")
        .expect("then-blocket måste resolve:a till en nod-path");

    // Nodens exakta text (statement_block, INKLUSIVE klamrar — read_nodes kontrakt).
    let new_block = "{
                box.value = 'sparat';
                notify('ok');
                guard(); // ny rad — bevis på att blocket skrevs
            }";

    w.edit(
        EditOperation::Edit {
            node_path: target,
            content: new_block.to_string(),
        },
        false,
    )
    .expect("mid-line block-edit ska lyckas, inte korrumpera tyst");

    let out = std::fs::read_to_string(&path).unwrap();
    assert!(
        out.contains(".then(function () {"),
        "then-radens prefix måste överleva (hel-raders-fallbacken åt den förr): {out}"
    );
    assert!(
        out.contains("})\n            .catch(function (e2)"),
        "kedjan (}} + .catch) måste överleva intakt: {out}"
    );
    assert!(
        out.contains("guard();"),
        "nytt innehåll måste vara skrivet: {out}"
    );
    assert!(
        out.contains("notify('ok');"),
        "befintligt innehåll i blocket måste överleva: {out}"
    );
    assert!(
        out.contains("🌲"),
        "multibyte i prefixraden måste överleva (tecken-kolumner): {out}"
    );
    assert!(
        out.contains("Åäö"),
        "å/ä/ö i kommentar måste överleva: {out}"
    );
}

/// Samma kontrakt för RUST: byte-kolumner + tecken-splatte divergerade vid
/// å/ä/ö före noden på samma rad. Nu char_col_pair överallt.
#[test]
fn edit_node_on_rust_line_with_multibyte_prefix_is_exact() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("repro.rs");
    // fn-bodyns sista statement börjar MID-LINE? Nej — men prefixet på dess
    // rad innehåller multibyte, vilket förr throwsde byte/char-splatten.
    std::fs::write(
        &path,
        "// Åäö-kommentar 🌲\nfn calc() {\n    let s = \"åäö\"; // ✓ behåll\n    let n = 1; // ändra denna rad\n    n\n}\n",
    )
    .unwrap();

    let mut w = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
    let tree = w.analyze();
    let target = find_path_containing(tree, "let n = 1;", "let")
        .expect("let_statement måste resolve:a till en nod-path");

    w.edit(
        EditOperation::Edit {
            node_path: target,
            content: "let n = 42; // ändrad".to_string(),
        },
        false,
    )
    .expect("rust-edit med multibyte-prefix ska lyckas");

    let out = std::fs::read_to_string(&path).unwrap();
    assert!(
        out.contains("let n = 42; // ändrad"),
        "ny rad saknas: {out}"
    );
    assert!(
        out.contains("let s = \"åäö\"; // ✓ behåll"),
        "grannraden med multibyte måste överleva orörd: {out}"
    );
    assert!(out.contains("🌲"), "emoji-kommentar måste överleva: {out}");
}
