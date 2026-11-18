use gnawtreewriter::core::binding_observations::{
    analyze_bindings_from_tree, BindingKind, FindingKind, Language, OperationKind, Requirement,
};
use gnawtreewriter::parser::get_parser;

#[test]
fn typescript_shadowing_and_parameter_reassignment() {
    let source = "function f(p: number) { p = 2; let x = 1; { const x = 3; x = 4; } x = 5; }";
    let tree = get_parser(std::path::Path::new("probe.ts"))
        .unwrap()
        .parse(source)
        .unwrap();
    let a = analyze_bindings_from_tree(&tree, "probe.ts", Language::JavaScript);
    let xs: Vec<_> = a.bindings.iter().filter(|b| b.name == "x").collect();
    assert_eq!(xs.len(), 2);
    assert_ne!(xs[0].scope_path, xs[1].scope_path);
    let x_writes: Vec<_> = a
        .operations
        .iter()
        .filter(|o| o.target == "x" && o.kind == OperationKind::Reassign)
        .collect();
    assert_eq!(x_writes.len(), 2);
    assert_ne!(x_writes[0].binding_id, x_writes[1].binding_id);
    assert!(a
        .findings
        .iter()
        .any(|f| f.kind == FindingKind::NeedsMutableBinding && f.binding_id == xs[1].id));
    assert!(!a.findings.iter().any(|f| a
        .bindings
        .iter()
        .any(|b| b.name == "p" && b.id == f.binding_id)));
}

#[test]
fn rust_mutable_reference_is_not_mutable_binding() {
    let source = "fn f(mut n: i32) { let r = &mut n; *r = 2; }";
    let tree = get_parser(std::path::Path::new("probe.rs"))
        .unwrap()
        .parse(source)
        .unwrap();
    let a = analyze_bindings_from_tree(&tree, "probe.rs", Language::Rust);
    let r = a.bindings.iter().find(|b| b.name == "r").unwrap();
    assert_eq!(r.kind, BindingKind::Immutable);
    let write = a
        .operations
        .iter()
        .find(|o| o.kind == OperationKind::IndirectWrite)
        .unwrap();
    assert_eq!(write.binding_id.as_deref(), Some(r.id.as_str()));
    assert_eq!(write.binding_mutability, Requirement::Unknown);
    assert!(!a.findings.iter().any(|f| f.binding_id == r.id));
}
