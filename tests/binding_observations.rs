use gnawtreewriter::core::binding_observations::{
    analyze_bindings_from_tree, BindingKind, FindingKind, Language, OperationKind, Requirement,
};
use gnawtreewriter::parser::get_parser;

fn analyze(
    source: &str,
    ext: &str,
    language: Language,
) -> gnawtreewriter::core::binding_observations::BindingAnalysis {
    let parser = get_parser(std::path::Path::new(&format!("fixture.{ext}"))).unwrap();
    let tree = parser.parse(source).unwrap();
    analyze_bindings_from_tree(&tree, &format!("fixture.{ext}"), language)
}

#[test]
fn rust_shadowing_borrow_and_indirect_write() {
    let source = "fn f() {\n let mut x = 1;\n x += 1;\n { let x = &mut x; *x = 3; }\n let y = 1; y = 2;\n}\n";
    let a = analyze(source, "rs", Language::Rust);
    let outer = a
        .bindings
        .iter()
        .find(|b| b.name == "x" && b.anchor.start_line == 2)
        .unwrap();
    let inner = a
        .bindings
        .iter()
        .find(|b| b.name == "x" && b.anchor.start_line == 4)
        .unwrap();
    assert_ne!(outer.id, inner.id);
    let compound = a
        .operations
        .iter()
        .find(|o| o.kind == OperationKind::CompoundAssign)
        .unwrap();
    assert_eq!(compound.binding_id.as_deref(), Some(outer.id.as_str()));
    let borrow = a
        .operations
        .iter()
        .find(|o| o.kind == OperationKind::MutableBorrow)
        .unwrap();
    assert_eq!(borrow.binding_id.as_deref(), Some(outer.id.as_str()));
    let indirect = a
        .operations
        .iter()
        .find(|o| o.kind == OperationKind::IndirectWrite)
        .unwrap();
    assert_eq!(indirect.binding_id.as_deref(), Some(inner.id.as_str()));
    assert_eq!(indirect.binding_mutability, Requirement::Unknown);
    assert!(a
        .findings
        .iter()
        .any(|f| f.kind == FindingKind::NeedsMutableBinding
            && a.bindings
                .iter()
                .any(|b| b.id == f.binding_id && b.name == "y")));
    assert!(!a.operations.iter().any(|o| o.verified_effect));
}

#[test]
fn javascript_const_object_and_destructuring() {
    let source = "function f() {\n const obj = {n: 1};\n obj.n++;\n const x = 1; x = 2;\n let {n} = obj; n += 1;\n let stable = 3;\n}\n";
    let a = analyze(source, "js", Language::JavaScript);
    let object = a.bindings.iter().find(|b| b.name == "obj").unwrap();
    let member = a
        .operations
        .iter()
        .find(|o| o.kind == OperationKind::MemberWrite)
        .unwrap();
    assert_eq!(member.binding_id.as_deref(), Some(object.id.as_str()));
    assert_eq!(member.binding_mutability, Requirement::NotRequired);
    assert!(!a.findings.iter().any(|f| f.binding_id == object.id));
    assert!(a
        .findings
        .iter()
        .any(|f| f.kind == FindingKind::NeedsMutableBinding
            && a.bindings
                .iter()
                .any(|b| b.id == f.binding_id && b.name == "x" && b.kind == BindingKind::Const)));
    assert!(a
        .findings
        .iter()
        .any(|f| f.kind == FindingKind::CouldBeImmutable
            && a.bindings
                .iter()
                .any(|b| b.id == f.binding_id && b.name == "stable")));
    assert!(a.bindings.iter().any(|b| b.name == "n"));
    assert!(a
        .operations
        .iter()
        .any(|o| o.kind == OperationKind::CompoundAssign
            && o.target == "n"
            && o.binding_id.is_some()));
}

#[test]
fn unresolved_identifier_stays_unresolved() {
    let a = analyze("fn f() { missing = 1; }", "rs", Language::Rust);
    let op = a
        .operations
        .iter()
        .find(|o| o.kind == OperationKind::Reassign)
        .unwrap();
    assert!(op.binding_id.is_none());
    assert!(a.findings.is_empty());
}
