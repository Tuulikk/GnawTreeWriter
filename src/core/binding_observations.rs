//! File-local binding and operation evidence. No type or control-flow claims.
use crate::core::index_relations::SourceAnchor;
use crate::parser::TreeNode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    JavaScript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    Immutable,
    Mutable,
    MutableReference,
    Const,
    Let,
    Parameter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// File-local identity. Valid only for the source version that produced this tree.
    pub id: String,
    pub name: String,
    pub kind: BindingKind,
    pub anchor: SourceAnchor,
    pub scope_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Read,
    Reassign,
    CompoundAssign,
    Update,
    MemberWrite,
    MutableBorrow,
    IndirectWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    Required,
    NotRequired,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PossibleEffect {
    BindingValue,
    ObjectOrReferent,
    None,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationObservation {
    pub kind: OperationKind,
    pub anchor: SourceAnchor,
    pub target: String,
    /// None means unresolved, never a guessed binding.
    pub binding_id: Option<String>,
    pub possible_effect: PossibleEffect,
    pub binding_mutability: Requirement,
    /// Syntactic evidence is never proof that an effect occurred at runtime.
    pub verified_effect: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingKind {
    NeedsMutableBinding,
    CouldBeImmutable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingFinding {
    pub kind: FindingKind,
    pub binding_id: String,
    /// A finding is a source-level indication, not a compiler verdict.
    pub evidence_paths: Vec<String>,
    pub coverage: &'static str,
}

#[derive(Debug, Clone)]
pub struct BindingAnalysis {
    pub file: String,
    pub language: Language,
    pub bindings: Vec<Binding>,
    pub operations: Vec<OperationObservation>,
    pub findings: Vec<BindingFinding>,
    pub limitations: Vec<&'static str>,
}

fn anchor(file: &str, n: &TreeNode) -> SourceAnchor {
    SourceAnchor {
        file: file.into(),
        node_path: n.path.clone(),
        start_line: n.start_line,
        start_col: n.start_col,
        end_line: n.end_line,
        end_col: n.end_col,
    }
}

fn scope_node(n: &TreeNode) -> bool {
    matches!(
        n.node_type.as_str(),
        "block"
            | "statement_block"
            | "function_item"
            | "function_declaration"
            | "function"
            | "arrow_function"
            | "method_definition"
    )
}

fn pattern_identifiers<'a>(n: &'a TreeNode, out: &mut Vec<&'a TreeNode>) {
    if matches!(
        n.node_type.as_str(),
        "identifier" | "shorthand_property_identifier_pattern"
    ) {
        out.push(n);
    } else if matches!(
        n.node_type.as_str(),
        "tuple_pattern"
            | "object_pattern"
            | "array_pattern"
            | "pair_pattern"
            | "assignment_pattern"
            | "mut_pattern"
    ) {
        for c in &n.children {
            pattern_identifiers(c, out);
        }
    }
}

struct Walker<'a> {
    result: BindingAnalysis,
    scopes: Vec<Vec<usize>>,
    tree: &'a TreeNode,
}

impl Walker<'_> {
    fn declare(&mut self, n: &TreeNode, kind: BindingKind) {
        let id = format!("{}#{}", self.result.file, n.path);
        let mut parent = n.path.as_str();
        let scope_path = loop {
            let Some((path, _)) = parent.rsplit_once('.') else {
                break String::new();
            };
            parent = path;
            if self.tree.find_path(path).is_some_and(scope_node) {
                break path.to_string();
            }
        };
        let i = self.result.bindings.len();
        self.result.bindings.push(Binding {
            id,
            name: n.content.clone(),
            kind,
            anchor: anchor(&self.result.file, n),
            scope_path,
        });
        self.scopes.last_mut().unwrap().push(i);
    }

    fn resolve(&self, name: &str) -> Option<String> {
        self.scopes
            .iter()
            .rev()
            .flat_map(|s| s.iter().rev())
            .find(|i| self.result.bindings[**i].name == name)
            .map(|i| self.result.bindings[*i].id.clone())
    }

    fn emit(
        &mut self,
        kind: OperationKind,
        operation: &TreeNode,
        target: &TreeNode,
        effect: PossibleEffect,
        requirement: Requirement,
    ) {
        self.result.operations.push(OperationObservation {
            kind,
            anchor: anchor(&self.result.file, operation),
            target: target.content.clone(),
            binding_id: self.resolve(&target.content),
            possible_effect: effect,
            binding_mutability: requirement,
            verified_effect: false,
        });
    }

    fn visit(&mut self, n: &TreeNode) {
        let push = scope_node(n);
        if push {
            self.scopes.push(Vec::new());
        }
        match n.node_type.as_str() {
            "let_declaration" if self.result.language == Language::Rust => {
                let eq = n.children.iter().position(|c| c.node_type == "=");
                if let Some(i) = eq {
                    for c in &n.children[i + 1..] {
                        self.visit(c);
                    }
                }
                let mut ids = Vec::new();
                for c in &n.children[..eq.unwrap_or(n.children.len())] {
                    if matches!(
                        c.node_type.as_str(),
                        "identifier" | "tuple_pattern" | "mut_pattern"
                    ) {
                        pattern_identifiers(c, &mut ids);
                    }
                }
                let mutable = n
                    .children
                    .iter()
                    .any(|c| c.node_type == "mutable_specifier");
                let is_ref = n
                    .children
                    .iter()
                    .any(|c| c.node_type == "reference_type" && c.content.starts_with("&mut"));
                for id in ids {
                    self.declare(
                        id,
                        if is_ref {
                            BindingKind::MutableReference
                        } else if mutable {
                            BindingKind::Mutable
                        } else {
                            BindingKind::Immutable
                        },
                    );
                }
            }
            "lexical_declaration" | "variable_declaration"
                if self.result.language == Language::JavaScript =>
            {
                let kind = if n.children.iter().any(|c| c.node_type == "const") {
                    BindingKind::Const
                } else {
                    BindingKind::Let
                };
                for d in n
                    .children
                    .iter()
                    .filter(|c| c.node_type == "variable_declarator")
                {
                    let eq = d.children.iter().position(|c| c.node_type == "=");
                    if let Some(i) = eq {
                        for c in &d.children[i + 1..] {
                            self.visit(c);
                        }
                    }
                    if let Some(pattern) = d.children.first() {
                        let mut ids = Vec::new();
                        pattern_identifiers(pattern, &mut ids);
                        for id in ids {
                            self.declare(id, kind);
                        }
                    }
                }
            }
            "parameter" if self.result.language == Language::Rust => {
                let mutable = n
                    .children
                    .iter()
                    .any(|c| c.node_type == "mutable_specifier");
                if let Some(id) = n.children.iter().find(|c| c.node_type == "identifier") {
                    self.declare(
                        id,
                        if mutable {
                            BindingKind::Mutable
                        } else {
                            BindingKind::Parameter
                        },
                    );
                }
            }
            "required_parameter" | "optional_parameter"
                if self.result.language == Language::JavaScript =>
            {
                if let Some(p) = n.children.first() {
                    let mut ids = Vec::new();
                    pattern_identifiers(p, &mut ids);
                    for id in ids {
                        self.declare(id, BindingKind::Parameter);
                    }
                }
            }
            "assignment_expression"
            | "compound_assignment_expr"
            | "augmented_assignment_expression"
            | "update_expression" => {
                if let Some(lhs) = n.children.iter().find(|c| {
                    matches!(
                        c.node_type.as_str(),
                        "identifier"
                            | "member_expression"
                            | "field_expression"
                            | "subscript_expression"
                            | "index_expression"
                            | "unary_expression"
                    )
                }) {
                    let direct = lhs.node_type == "identifier";
                    let target = if direct {
                        Some(lhs)
                    } else {
                        lhs.children.iter().find(|c| c.node_type == "identifier")
                    };
                    if let Some(target) = target {
                        let kind = if !direct {
                            if lhs.node_type == "unary_expression" {
                                OperationKind::IndirectWrite
                            } else {
                                OperationKind::MemberWrite
                            }
                        } else if n.node_type == "update_expression" {
                            OperationKind::Update
                        } else if n.node_type != "assignment_expression" {
                            OperationKind::CompoundAssign
                        } else {
                            OperationKind::Reassign
                        };
                        let requirement = if direct {
                            Requirement::Required
                        } else if self.result.language == Language::JavaScript {
                            Requirement::NotRequired
                        } else {
                            Requirement::Unknown
                        };
                        self.emit(
                            kind,
                            n,
                            target,
                            if direct {
                                PossibleEffect::BindingValue
                            } else {
                                PossibleEffect::ObjectOrReferent
                            },
                            requirement,
                        );
                        if kind != OperationKind::Reassign {
                            self.emit(
                                OperationKind::Read,
                                lhs,
                                target,
                                PossibleEffect::None,
                                Requirement::NotRequired,
                            );
                        }
                    }
                }
                // RHS only: do not misclassify the write target as a read.
                for c in n.children.iter().skip(1) {
                    self.visit(c);
                }
            }
            "reference_expression"
                if self.result.language == Language::Rust
                    && n.children
                        .iter()
                        .any(|c| c.node_type == "mutable_specifier") =>
            {
                if let Some(id) = n.children.iter().find(|c| c.node_type == "identifier") {
                    self.emit(
                        OperationKind::MutableBorrow,
                        n,
                        id,
                        PossibleEffect::None,
                        Requirement::Required,
                    );
                } else {
                    for c in &n.children {
                        self.visit(c);
                    }
                }
            }
            "identifier" => {
                self.emit(
                    OperationKind::Read,
                    n,
                    n,
                    PossibleEffect::None,
                    Requirement::NotRequired,
                );
            }
            _ => {
                for c in &n.children {
                    self.visit(c);
                }
            }
        }
        if push {
            self.scopes.pop();
        }
    }
}

fn derive_findings(result: &mut BindingAnalysis) {
    for b in &result.bindings {
        let requiring: Vec<_> = result
            .operations
            .iter()
            .filter(|o| {
                o.binding_id.as_deref() == Some(&b.id)
                    && o.binding_mutability == Requirement::Required
            })
            .collect();
        if !requiring.is_empty() {
            let immutable = matches!(b.kind, BindingKind::Immutable | BindingKind::Const)
                || (result.language == Language::Rust && b.kind == BindingKind::Parameter);
            if immutable {
                result.findings.push(BindingFinding {
                    kind: FindingKind::NeedsMutableBinding,
                    binding_id: b.id.clone(),
                    evidence_paths: requiring
                        .iter()
                        .map(|o| o.anchor.node_path.clone())
                        .collect(),
                    coverage: "file-local lexical binding; compiler/type checking required",
                });
            }
        } else if matches!(b.kind, BindingKind::Mutable | BindingKind::Let) {
            result.findings.push(BindingFinding {
                kind: FindingKind::CouldBeImmutable,
                binding_id: b.id.clone(),
                evidence_paths: vec![b.anchor.node_path.clone()],
                coverage: "observed file-local syntax only; dynamic effects and macros excluded",
            });
        }
    }
}

/// Analyze one parsed file. Callers retain the tree and content hash to validate anchors.
pub fn analyze_bindings_from_tree(
    tree: &TreeNode,
    file: &str,
    language: Language,
) -> BindingAnalysis {
    let mut walker = Walker {
        result: BindingAnalysis { file: file.into(), language, bindings: Vec::new(),
            operations: Vec::new(), findings: Vec::new(),
            limitations: vec!["No type, macro expansion, alias, control-flow or runtime verification",
                "Node paths require the same source hash; JavaScript hoisting and eval are not modeled"] },
        scopes: vec![Vec::new()], tree,
    };
    walker.visit(tree);
    derive_findings(&mut walker.result);
    walker.result
}
