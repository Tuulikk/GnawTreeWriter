//! Fas 1 (docs/GUARDIAN_V2_PLAN.md): structural edit deltas.
//!
//! Compares the OLD node tree against the NEW node tree and classifies
//! semantic changes that survive a clean parse: inverted operators,
//! removed conditions/guards, dropped error handling, changed signatures
//! and literals. Full-depth comparison is reserved for logically critical
//! node types (Gemini feedback 2026-10-06); everything else is pruned by
//! text equality or a bounded walk.

use crate::core::rules::Severity;
use crate::parser::TreeNode;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeltaKind {
    OperatorChange { old: String, new: String },
    ConditionRemoved,
    ErrorHandlingRemoved { what: String },
    SignatureChange,
    LiteralChange { old: String, new: String },
    StatementRemoved { count: usize },
    AssertRemoved { what: String },
    PanicPathIntroduced,
    DocRemoved,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EditDelta {
    pub kind: DeltaKind,
    pub severity: Severity,
    pub message: String,
    pub suggestion: Option<String>,
}

const MAX_DEPTH: usize = 24;
const MAX_DELTAS: usize = 16;

/// Logically critical node types get full-depth comparison; other nodes
/// are only recursed into when their text actually differs (pruning).
fn is_critical(node_type: &str) -> bool {
    let t = node_type;
    t.contains("binary")
        || t.contains("unary")
        || t.contains("condition")
        || t.contains("return")
        || t.contains("if")
        || t.contains("while")
        || t.contains("for")
        || t.contains("match")
        || t.contains("switch")
        || t.contains("try")
}

fn is_definition(node_type: &str) -> bool {
    let t = node_type;
    t.contains("function")
        || t.contains("method")
        || t.contains("def")
        || t.contains("class")
        || t.contains("struct")
}

fn is_noise(content: &str) -> bool {
    let t = content.trim();
    t.is_empty() || t.chars().all(|c| ";,(){}[]".contains(c))
}

fn meaningful_children(node: &TreeNode) -> Vec<&TreeNode> {
    node.children
        .iter()
        .filter(|c| !is_noise(&c.content))
        .collect()
}

fn is_operator_token(node_type: &str) -> bool {
    if node_type.is_empty() {
        return false;
    }
    if node_type.chars().all(|c| "!<>=&|+-*/%^~".contains(c)) {
        return true;
    }
    matches!(node_type, "and" | "or" | "not" | "in" | "is")
}

fn is_number_token(node_type: &str, content: &str) -> bool {
    let t = node_type;
    (t.contains("integer") || t.contains("float") || t.contains("number"))
        && content.trim().parse::<f64>().is_ok()
}

/// Word-boundary occurrence count ("try" must not match "entry").
fn count_word(content: &str, word: &str) -> usize {
    if word.is_empty() || content.len() < word.len() {
        return 0;
    }
    let bytes = content.as_bytes();
    let mut count = 0;
    for i in 0..=content.len() - word.len() {
        if &content[i..i + word.len()] == word {
            let before_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
            let after = i + word.len();
            let after_ok = after >= bytes.len() || !bytes[after].is_ascii_alphanumeric();
            if before_ok && after_ok {
                count += 1;
            }
        }
    }
    count
}

/// Multiset difference: entries in `old` not covered by `new`.
fn multiset_removed(old: &[String], new: &[String]) -> Vec<String> {
    let mut remaining: Vec<&String> = new.iter().collect();
    old.iter()
        .filter(|o| match remaining.iter().position(|n| *n == *o) {
            Some(p) => {
                remaining.remove(p);
                false
            }
            None => true,
        })
        .cloned()
        .collect()
}

/// Short header (up to the first `{`, joined to one line) for signature
/// comparison on definition nodes.
fn header(content: &str) -> String {
    let head = content.split('{').next().unwrap_or(content);
    let mut one_line = head
        .split('\n')
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ");
    one_line.truncate(120);
    one_line
}

/// Unwrap unit wrappers (source_file / translation_unit / program) when
/// the parsed snippet has exactly one meaningful child.
pub fn unwrap_root(tree: &TreeNode) -> &TreeNode {
    let mut cur = tree;
    for _ in 0..3 {
        let kids = meaningful_children(cur);
        if kids.len() == 1 {
            cur = kids[0];
        } else {
            break;
        }
    }
    cur
}

/// Compare the old and new node trees and return classified semantic deltas.
pub fn diff_node_trees(old: &TreeNode, new: &TreeNode) -> Vec<EditDelta> {
    let mut deltas = Vec::new();
    diff_walk(old, new, 0, &mut deltas);
    deltas
}

fn diff_walk(old: &TreeNode, new: &TreeNode, depth: usize, deltas: &mut Vec<EditDelta>) {
    if deltas.len() >= MAX_DELTAS || depth > MAX_DEPTH {
        return;
    }
    if old.content.trim() == new.content.trim() {
        return; // prune: identical subtree
    }

    // Shape mismatch: deep structural pairing is unreliable, fall back to
    // token-level checks on the two contents.
    if old.node_type != new.node_type {
        diff_tokens_fallback(old, new, deltas);
        return;
    }

    let t = old.node_type.as_str();

    if is_critical(t) {
        diff_operators(old, new, deltas);
        diff_logic_operators(old, new, deltas);
        diff_numbers(old, new, deltas);
        diff_error_handling(old, new, deltas);
    }

    if is_definition(t) {
        let (oh, nh) = (header(&old.content), header(&new.content));
        if oh != nh {
            deltas.push(EditDelta {
                kind: DeltaKind::SignatureChange,
                severity: Severity::Warning,
                message: format!(
                    "Definition header changed: '{}' -> '{}' — verify parameters, return type and visibility",
                    oh, nh
                ),
                suggestion: Some(
                    "If the header change was unintentional, restore the original signature"
                        .into(),
                ),
            });
        }
    }

    let old_kids = meaningful_children(old);
    let new_kids = meaningful_children(new);
    let common = old_kids.len().min(new_kids.len());

    for gone in old_kids.iter().skip(common) {
        if is_noise(&gone.content) {
            continue;
        }
        classify_removed(gone, t, deltas);
        if deltas.len() >= MAX_DELTAS {
            return;
        }
    }

    for (o, n) in old_kids.iter().zip(new_kids.iter()) {
        diff_walk(o, n, depth + 1, deltas);
        if deltas.len() >= MAX_DELTAS {
            return;
        }
    }
}

fn diff_operators(old: &TreeNode, new: &TreeNode, deltas: &mut Vec<EditDelta>) {
    let ops = |n: &TreeNode| -> Vec<String> {
        n.children
            .iter()
            .filter(|c| is_operator_token(&c.node_type))
            .map(|c| c.content.trim().to_string())
            .collect()
    };
    let (removed, added) = (ops(old), ops(new));
    let removed_ops = multiset_removed(&removed, &added);
    let added_ops = multiset_removed(&added, &removed);
    for (r, a) in removed_ops.iter().zip(added_ops.iter()) {
        deltas.push(EditDelta {
            kind: DeltaKind::OperatorChange {
                old: r.clone(),
                new: a.clone(),
            },
            severity: Severity::Warning,
            message: format!(
                "Operator changed: '{}' -> '{}' — an inverted or altered comparison/logic silently changes behavior",
                r, a
            ),
            suggestion: Some(
                "If the operator change was unintentional, restore the original operator".into(),
            ),
        });
    }
    for r in removed_ops.iter().skip(added_ops.len()) {
        deltas.push(EditDelta {
            kind: DeltaKind::OperatorChange {
                old: r.clone(),
                new: String::new(),
            },
            severity: Severity::Warning,
            message: format!("Operator '{}' removed", r),
            suggestion: None,
        });
    }
}

/// Content-level logic-operator comparison for critical nodes: catches a
/// removed condition operand even when the tree restructured and index
/// pairing can no longer align the subtrees.
fn diff_logic_operators(old: &TreeNode, new: &TreeNode, deltas: &mut Vec<EditDelta>) {
    const LOGIC: [&str; 4] = ["&&", "||", "and", "or"];
    for tok in LOGIC {
        let (a, b) = (count_word(&old.content, tok), count_word(&new.content, tok));
        if b < a {
            deltas.push(EditDelta {
                kind: DeltaKind::ConditionRemoved,
                severity: Severity::Error,
                message: format!(
                    "Logical operator '{}' appears {} fewer time(s) — a condition operand was likely removed",
                    tok,
                    a - b
                ),
                suggestion: Some("If this removed a guard (e.g. bounds check), restore it".into()),
            });
        }
    }
}

fn diff_numbers(old: &TreeNode, new: &TreeNode, deltas: &mut Vec<EditDelta>) {
    let nums = |n: &TreeNode| -> Vec<String> {
        n.children
            .iter()
            .filter(|c| is_number_token(&c.node_type, &c.content))
            .map(|c| c.content.trim().to_string())
            .collect()
    };
    let removed = multiset_removed(&nums(old), &nums(new));
    let added = multiset_removed(&nums(new), &nums(old));
    for (r, a) in removed.iter().zip(added.iter()) {
        deltas.push(EditDelta {
            kind: DeltaKind::LiteralChange {
                old: r.clone(),
                new: a.clone(),
            },
            severity: Severity::Info,
            message: format!("Numeric literal changed: '{}' -> '{}'", r, a),
            suggestion: None,
        });
    }
}

fn diff_error_handling(old: &TreeNode, new: &TreeNode, deltas: &mut Vec<EditDelta>) {
    const CALL_TOKENS: [&str; 2] = ["unwrap(", "expect("];
    const WORD_TOKENS: [&str; 3] = ["try", "catch", "except"];
    for tok in CALL_TOKENS {
        let (a, b) = (count_word(&old.content, tok), count_word(&new.content, tok));
        if b < a {
            deltas.push(EditDelta {
                kind: DeltaKind::ErrorHandlingRemoved {
                    what: tok.to_string(),
                },
                severity: Severity::Warning,
                message: format!(
                    "Error handling call '{}' appears {} fewer time(s) — error paths may have been dropped",
                    tok,
                    a - b
                ),
                suggestion: Some("Verify the error case is still handled after this edit".into()),
            });
        }
    }
    for tok in WORD_TOKENS {
        let (a, b) = (count_word(&old.content, tok), count_word(&new.content, tok));
        if b < a {
            deltas.push(EditDelta {
                kind: DeltaKind::ErrorHandlingRemoved {
                    what: tok.to_string(),
                },
                severity: Severity::Warning,
                message: format!(
                    "Error handling construct '{}' appears {} fewer time(s) — error paths may have been dropped",
                    tok,
                    a - b
                ),
                suggestion: Some("Verify the error case is still handled after this edit".into()),
            });
        }
    }
}

fn diff_tokens_fallback(old: &TreeNode, new: &TreeNode, deltas: &mut Vec<EditDelta>) {
    let (a, b) = (
        count_word(&old.content, "if"),
        count_word(&new.content, "if"),
    );
    if b < a {
        deltas.push(EditDelta {
            kind: DeltaKind::ConditionRemoved,
            severity: Severity::Error,
            message: format!(
                "{} condition statement(s) removed — a guard or bounds check may be gone",
                a - b
            ),
            suggestion: Some("If this removed a guard (e.g. bounds check), restore it".into()),
        });
    }
    diff_error_handling(old, new, deltas);
}

/// Fas 2 (docs/GUARDIAN_V2_PLAN.md): invariant contract extracted from the
/// OLD node, checked against the NEW content independent of tree alignment
/// — catches agent regenerations that silently drop guards even after a
/// full rewrite changed the tree shape.
#[derive(Debug, Clone, Default)]
pub struct Contract {
    pub error_handling: usize,
    pub guard_lines: Vec<String>,
    pub asserts: usize,
    pub unwraps: usize,
    pub doc_lines: usize,
}

/// Extract the invariant contract from node content (text-level,
/// language-agnostic).
pub fn extract_contract(node: &TreeNode) -> Contract {
    let content = node.content.as_str();
    let err_tokens = ["unwrap(", "expect(", "try", "catch", "except"];
    let assert_tokens = ["assert!(", "assert_eq!(", "assert_ne!(", "debug_assert!("];
    Contract {
        error_handling: err_tokens.iter().map(|t| count_word(content, t)).sum(),
        guard_lines: guard_lines(content),
        asserts: assert_tokens
            .iter()
            .map(|t| content.matches(t).count())
            .sum(),
        unwraps: count_word(content, "unwrap("),
        doc_lines: content
            .lines()
            .filter(|l| l.trim_start().starts_with("///"))
            .count(),
    }
}

/// Condition lines that look like guards (comparison-based ifs), quoted
/// whole so a regeneration that drops them can be named.
fn guard_lines(content: &str) -> Vec<String> {
    content
        .lines()
        .map(str::trim_start)
        .filter(|l| {
            (l.starts_with("if ") || l.starts_with("else if ") || l.starts_with("elif "))
                && (l.contains("<")
                    || l.contains(">")
                    || l.contains("==")
                    || l.contains("!=")
                    || l.contains(".len()")
                    || l.contains("is_empty"))
        })
        .map(str::to_string)
        .collect()
}

/// Check the old contract against the new content; fires only on
/// full-loss (complementary to the per-token Fas 1 deltas).
pub fn check_contract(old: &Contract, new_content: &str) -> Vec<EditDelta> {
    let mut deltas = Vec::new();
    let err_tokens = ["unwrap(", "expect(", "try", "catch", "except"];
    let assert_tokens = ["assert!(", "assert_eq!(", "assert_ne!(", "debug_assert!("];

    let new_err: usize = err_tokens.iter().map(|t| count_word(new_content, t)).sum();
    if old.error_handling > 0 && new_err == 0 {
        deltas.push(EditDelta {
            kind: DeltaKind::ErrorHandlingRemoved {
                what: "all".into(),
            },
            severity: Severity::Error,
            message: "Invariant contract: ALL error handling (unwrap/expect/try/catch) present before the edit is gone".into(),
            suggestion: Some("Restore at least the primary error path".into()),
        });
    }

    if !old.guard_lines.is_empty() {
        let new_guards = guard_lines(new_content);
        let removed: Vec<&String> = old
            .guard_lines
            .iter()
            .filter(|g| !new_guards.iter().any(|n| n == *g))
            .collect();
        if !removed.is_empty() && new_guards.len() < old.guard_lines.len() {
            let quote: String = removed[0].chars().take(80).collect();
            deltas.push(EditDelta {
                kind: DeltaKind::ConditionRemoved,
                severity: Severity::Error,
                message: format!(
                    "Invariant contract: guard/bounds check removed: [{}]",
                    quote
                ),
                suggestion: Some("If this removed a guard (e.g. bounds check), restore it".into()),
            });
        }
    }

    let new_asserts: usize = assert_tokens
        .iter()
        .map(|t| new_content.matches(t).count())
        .sum();
    if new_asserts < old.asserts {
        deltas.push(EditDelta {
            kind: DeltaKind::AssertRemoved {
                what: format!("{}", old.asserts - new_asserts),
            },
            severity: Severity::Warning,
            message: format!(
                "Invariant contract: {} assert(s) removed",
                old.asserts - new_asserts
            ),
            suggestion: Some("Verify the assertion is no longer needed".into()),
        });
    }

    if old.unwraps == 0 {
        let new_unwraps = count_word(new_content, "unwrap(") + count_word(new_content, "expect(");
        if new_unwraps > 0 {
            deltas.push(EditDelta {
                kind: DeltaKind::PanicPathIntroduced,
                severity: Severity::Warning,
                message: "Invariant contract: unwrap/expect introduced — the edited code was panic-free before".into(),
                suggestion: Some("Prefer returning a Result/error over panicking".into()),
            });
        }
    }

    let new_docs = new_content
        .lines()
        .filter(|l| l.trim_start().starts_with("///"))
        .count();
    if old.doc_lines > 0 && new_docs == 0 {
        deltas.push(EditDelta {
            kind: DeltaKind::DocRemoved,
            severity: Severity::Warning,
            message: format!(
                "Invariant contract: {} doc comment line(s) stripped",
                old.doc_lines
            ),
            suggestion: Some("Keep documentation unless the edit intentionally removes it".into()),
        });
    }

    deltas
}

fn classify_removed(gone: &TreeNode, parent_type: &str, deltas: &mut Vec<EditDelta>) {
    let gt = gone.node_type.as_str();
    let guard_like = gt.contains("if")
        || gt.contains("condition")
        || gt.contains("while")
        || gt.contains("for")
        || ((gt.contains("binary") || gt.contains("unary")) && is_critical(parent_type));
    if guard_like {
        deltas.push(EditDelta {
            kind: DeltaKind::ConditionRemoved,
            severity: Severity::Error,
            message: format!(
                "A condition operand/statement ({}) was removed — a guard or bounds check may be gone",
                gt
            ),
            suggestion: Some("If this removed a guard (e.g. bounds check), restore it".into()),
        });
    } else if gt.contains("catch")
        || gt.contains("try")
        || gt.contains("except")
        || gt.contains("match")
    {
        deltas.push(EditDelta {
            kind: DeltaKind::ErrorHandlingRemoved {
                what: gt.to_string(),
            },
            severity: Severity::Warning,
            message: format!("Error handling construct ({}) removed", gt),
            suggestion: Some("Verify the error case is still handled after this edit".into()),
        });
    } else {
        deltas.push(EditDelta {
            kind: DeltaKind::StatementRemoved { count: 1 },
            severity: Severity::Info,
            message: format!("Statement node removed ({})", gt),
            suggestion: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::rust::RustParser;
    use crate::parser::ParserEngine;

    fn parse(code: &str) -> TreeNode {
        RustParser::new().parse(code).unwrap()
    }

    #[test]
    fn operator_inversion_is_detected() {
        let old = parse("fn f(x: i32) -> bool {\n    x == 1\n}\n");
        let new = parse("fn f(x: i32) -> bool {\n    x != 1\n}\n");
        let deltas = diff_node_trees(&old, &new);
        assert!(
            deltas.iter().any(|d| matches!(
                d.kind,
                DeltaKind::OperatorChange { ref old, ref new }
                    if old == "==" && new == "!="
            )),
            "operator inversion not detected: {:?}",
            deltas
        );
    }

    #[test]
    fn identical_content_yields_no_deltas() {
        let code = "fn f(x: i32) -> bool {\n    x == 1\n}\n";
        let a = parse(code);
        let b = parse(code);
        assert!(diff_node_trees(&a, &b).is_empty());
    }

    #[test]
    fn contract_detects_total_invariant_loss() {
        let old_node = parse(
            "/// Adds one.\nfn f(x: i32) -> i32 {\n    let v = do_stuff(x).unwrap();\n    assert!(v > 0);\n    v + 1\n}\n",
        );
        let contract = extract_contract(&old_node);
        assert!(contract.error_handling > 0);
        assert!(contract.asserts > 0);
        assert!(contract.doc_lines > 0);

        let new_content = "fn f(x: i32) -> i32 {\n    x + 1\n}\n";
        let deltas = check_contract(&contract, new_content);
        assert!(
            deltas
                .iter()
                .any(|d| matches!(d.kind, DeltaKind::ErrorHandlingRemoved { .. })),
            "error-handling loss not detected: {:?}",
            deltas
        );
        assert!(
            deltas
                .iter()
                .any(|d| matches!(d.kind, DeltaKind::AssertRemoved { .. })),
            "assert loss not detected: {:?}",
            deltas
        );
        assert!(
            deltas
                .iter()
                .any(|d| matches!(d.kind, DeltaKind::DocRemoved)),
            "doc loss not detected: {:?}",
            deltas
        );
    }

    #[test]
    fn contract_flags_panic_path_introduction() {
        let old_node = parse("fn f(x: i32) -> i32 {\n    x + 1\n}\n");
        let contract = extract_contract(&old_node);
        assert_eq!(contract.unwraps, 0);
        let deltas = check_contract(
            &contract,
            "fn f(s: &str) -> i32 {\n    s.parse::<i32>().unwrap()\n}\n",
        );
        assert!(
            deltas
                .iter()
                .any(|d| matches!(d.kind, DeltaKind::PanicPathIntroduced)),
            "panic path introduction not detected: {:?}",
            deltas
        );
    }

    #[test]
    fn removed_condition_operand_is_error_severity() {
        let old = parse(
            "fn f(a: usize, b: usize) -> usize {\n    if a < b && a > 0 { a } else { b }\n}\n",
        );
        let new = parse("fn f(a: usize, b: usize) -> usize {\n    if a != b { a } else { b }\n}\n");
        let deltas = diff_node_trees(&old, &new);
        assert!(
            deltas
                .iter()
                .any(|d| matches!(d.kind, DeltaKind::ConditionRemoved)),
            "condition removal not detected: {:?}",
            deltas
        );
        assert!(
            deltas.iter().any(|d| matches!(
                d.kind,
                DeltaKind::OperatorChange { ref old, ref new } if old == "&&" && new == "!="
            )),
            "operator change not detected: {:?}",
            deltas
        );
    }
}
