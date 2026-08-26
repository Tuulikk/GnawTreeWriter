//! Pattern-matching rule engine (semgrep-inspired).
//!
//! Rules are semgrep-like code patterns (`$X = $X`) with `$VARIABLE`
//! placeholders. Each rule is parsed with the same parser as its target
//! language, then matched structurally against source ASTs in Rust — no
//! tree-sitter query generation. See docs/RULES_ENGINE_SPEC.md.

use crate::parser::{ParserEngine, TreeNode};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

/// Severity of a rule finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "error" => Severity::Error,
            "info" => Severity::Info,
            _ => Severity::Warning,
        }
    }
}

/// A single lint rule.
#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub id: String,
    pub language: String,
    #[serde(default = "default_severity")]
    pub severity: Severity,
    pub message: String,
    pub pattern: String,
}

fn default_severity() -> Severity {
    Severity::Warning
}

/// YAML container for a rules file.
#[derive(Debug, Deserialize)]
pub struct RulesFile {
    pub rules: Vec<Rule>,
}

/// A rule finding at a specific location.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Finding {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    /// Captured `$X` bindings (name -> matched content).
    pub captures: HashMap<String, String>,
}

/// A compiled rule: the rule plus its parsed pattern AST and placeholder map.
pub struct CompiledRule {
    pub rule: Rule,
    /// Root nodes of the parsed pattern (usually one).
    pub pattern_roots: Vec<TreeNode>,
    /// Placeholder names by node path within the pattern tree.
    pub placeholders: HashMap<String, String>,
}

/// Load rules from YAML text.
pub fn load_rules_yaml(yaml: &str) -> Result<Vec<Rule>> {
    let file: RulesFile = serde_yaml::from_str(yaml)
        .context("failed to parse rules YAML")?;
    Ok(file.rules)
}

/// Compile a rule: parse its pattern with the rule's language parser and mark
/// `$X` placeholders. Fails loudly (never silently) on invalid patterns.
pub fn compile_rule(rule: &Rule) -> Result<CompiledRule> {
    let parser = crate::parser::get_parser_for_language(&rule.language)
        .with_context(|| format!("rule '{}': unknown language '{}'", rule.id, rule.language))?;

    // Strip `#` comment lines from the pattern before parsing (spec decision 4).
    let cleaned: String = rule
        .pattern
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    // Replace `$NAME` placeholders with unique valid identifiers so the
    // pattern parses as real code (semgrep-style metavariables). The same
    // $NAME must share one binding key, so we keep a name->replacement map
    // and reuse it for repeated occurrences.
    let mut subs: Vec<(String, String)> = Vec::new(); // (placeholder, replacement)
    let mut name_to_repl: HashMap<String, String> = HashMap::new();
    let mut counter = 0usize;
    let mut rest = cleaned.as_str();
    let mut out = String::with_capacity(cleaned.len());
    while let Some(pos) = rest.find('$') {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 1..];
        let name_len = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .count();
        let name: String = after.chars().take(name_len).collect();
        let placeholder = format!("${name}");
        let replacement = if let Some(existing) = name_to_repl.get(&name) {
            existing.clone()
        } else {
            let repl = format!("_gtw_var_{}_{}", name, counter);
            counter += 1;
            name_to_repl.insert(name.clone(), repl.clone());
            repl
        };
        subs.push((placeholder, replacement.clone()));
        out.push_str(&replacement);
        rest = &after[name_len..];
    }
    out.push_str(rest);
    let substituted = out;

    // Try parsing as-is; if the pattern is a bare expression (no trailing
    // `;` or `}`), C-like parsers produce a partial tree. Try adding a
    // semicolon for statement-like languages.
    let mut parse_attempts: Vec<String> = vec![substituted.clone()];
    let needs_semi = !substituted.trim_end().ends_with(';')
        && !substituted.trim_end().ends_with('}')
        && matches!(
            rule.language.to_lowercase().as_str(),
            "rust" | "rs" | "javascript" | "js" | "typescript" | "ts" | "go" | "java"
                | "c" | "cpp" | "csharp" | "cs" | "php" | "swift" | "kotlin" | "kt"
        );
    if needs_semi {
        parse_attempts.push(format!("{substituted};"));
    }

    // Try each parse attempt and pick the one with the most substantial
    // top-level children. A bare expression (no `;`) parses to a shallow
    // tree (just an identifier); the `;`-terminated form parses to a real
    // statement node, which is what we want to match.
    let mut best: Option<(TreeNode, bool)> = None;
    for attempt in &parse_attempts {
        if let Ok(t) = parser.parse(attempt) {
            let real = t
                .children
                .iter()
                .filter(|c| c.node_type != "identifier" && c.node_type != "comment")
                .count();
            let better = match &best {
                None => true,
                Some((bt, _)) => {
                    let bt_real = bt
                        .children
                        .iter()
                        .filter(|c| c.node_type != "identifier" && c.node_type != "comment")
                        .count();
                    real > bt_real
                }
            };
            if better {
                best = Some((t, false));
            }
        }
    }
    // If nothing substantial parsed, try the language scaffold (for partial
    // statements like Python `except:`).
    if best.as_ref().map(|(t, _)| t.children.is_empty()).unwrap_or(true) {
        if let Some(wrapped) = scaffold_pattern(&rule.language, &substituted) {
            if let Ok(t) = parser.parse(&wrapped) {
                best = Some((t, true));
            }
        }
    }
    let (pattern_tree, scaffolded) = best.ok_or_else(|| {
        anyhow::anyhow!(
            "rule '{}': pattern does not parse as {}",
            rule.id,
            rule.language
        )
    })?;

    // Collect placeholder names by path: nodes whose content is one of the
    // substituted identifiers get mapped back to the $NAME.
    let mut placeholders = HashMap::new();
    collect_placeholders(&pattern_tree, &subs, &mut placeholders);

    // Determine the pattern root nodes. If scaffolding was used, the wrapper
    // is above them; otherwise the pattern is the tree's top-level children.
    let pattern_roots: Vec<TreeNode> = if scaffolded {
        if subs.is_empty() {
            // No placeholders: find the smallest node whose content contains
            // the whole (cleaned) pattern — that's the real pattern node.
            let pat = cleaned.trim();
            find_smallest_containing(&pattern_tree, pat)
                .map(|n| vec![n.clone()])
                .unwrap_or_default()
        } else {
            let first_repl = subs
                .first()
                .map(|(_, r)| r.clone())
                .unwrap_or_default();
            extract_pattern_roots(&pattern_tree, &first_repl)
        }
    } else {
        pattern_tree.children.clone()
    };
    if pattern_roots.is_empty() {
        anyhow::bail!("rule '{}': pattern produced no matchable nodes", rule.id);
    }

    Ok(CompiledRule {
        rule: rule.clone(),
        pattern_roots,
        placeholders,
    })
}

/// Recursively find placeholder nodes (their content is a substituted
/// `_gtw_var_*` identifier) and map them back to the `$NAME`.
fn collect_placeholders(
    node: &TreeNode,
    subs: &[(String, String)],
    out: &mut HashMap<String, String>,
) {
    for (placeholder, replacement) in subs {
        if node.content == *replacement {
            out.insert(node.path.clone(), placeholder.trim_start_matches('$').to_string());
            break;
        }
    }
    for child in &node.children {
        collect_placeholders(child, subs, out);
    }
}

/// Wrap a partial pattern in a minimal valid scaffold so it parses. Used for
/// statements that only parse inside a block (e.g. Python `except:`).
fn scaffold_pattern(language: &str, pattern: &str) -> Option<String> {
    match language.to_lowercase().as_str() {
        "python" | "py" => Some(format!("try:\n    pass\n{pattern}")),
        "rust" | "rs" => Some(format!("fn __gtw_scaffold() {{\n{pattern}\n}}")),
        "javascript" | "js" | "typescript" | "ts" => {
            Some(format!("function __gtw_scaffold() {{\n{pattern}\n}}"))
        }
        "go" => Some(format!("func __gtw_scaffold() {{\n{pattern}\n}}")),
        "java" => Some(format!("class __gtw_scaffold {{\n{pattern}\n}}")),
        "c" | "cpp" => Some(format!("void __gtw_scaffold() {{\n{pattern}\n}}")),
        _ => None,
    }
}

/// Extract the pattern's real nodes from a scaffolded parse: the deepest
/// node(s) containing the first substituted placeholder. The scaffold wrapper
/// is everything above them.
fn extract_pattern_roots(scaffolded: &TreeNode, first_replacement: &str) -> Vec<TreeNode> {
    // Find the deepest node containing the first placeholder.
    fn find_deepest<'a>(
        node: &'a TreeNode,
        needle: &str,
    ) -> Option<&'a TreeNode> {
        if !node.content.contains(needle) {
            return None;
        }
        let mut best: Option<&TreeNode> = None;
        for child in &node.children {
            if let Some(found) = find_deepest(child, needle) {
                best = Some(found);
            }
        }
        Some(best.unwrap_or(node))
    }

    if let Some(deepest) = find_deepest(scaffolded, first_replacement) {
        // Return the deepest node and its siblings at the same level.
        let mut roots = Vec::new();
        collect_subtree(deepest, &mut roots);
        roots
    } else {
        vec![]
    }
}

fn collect_subtree(node: &TreeNode, out: &mut Vec<TreeNode>) {
    out.push(node.clone());
    for c in &node.children {
        collect_subtree(c, out);
    }
}

/// Find the smallest node whose content contains `needle`.
fn find_smallest_containing<'a>(node: &'a TreeNode, needle: &str) -> Option<&'a TreeNode> {
    if !node.content.contains(needle) {
        return None;
    }
    // Prefer a deeper (smaller) match.
    for child in &node.children {
        if let Some(found) = find_smallest_containing(child, needle) {
            return Some(found);
        }
    }
    Some(node)
}

/// Run a compiled rule against a source tree, returning findings.
pub fn run_rule(rule: &CompiledRule, tree: &TreeNode, file: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for root in &rule.pattern_roots {
        match_pattern_recursive(root, tree, rule, file, &mut findings);
    }
    findings
}

/// Compile a set of rules once and run them against source code text.
/// Returns findings (empty if no rules match or none apply to this language).
/// Rules that fail to compile are skipped (reported via `skipped`).
pub fn check_code(
    code: &str,
    language: &str,
    rules: &[Rule],
) -> (Vec<Finding>, usize) {
    let mut compiled: Vec<CompiledRule> = Vec::new();
    let mut skipped = 0usize;
    let mut applicable = 0usize;
    for rule in rules {
        if !language_matches(&rule.language, language) {
            continue;
        }
        applicable += 1;
        match compile_rule(rule) {
            Ok(c) => compiled.push(c),
            Err(_) => skipped += 1,
        }
    }
    if applicable == 0 || compiled.is_empty() {
        return (Vec::new(), skipped);
    }
    // Parse the code once, then run all compiled rules.
    let parser = match crate::parser::get_parser_for_language(language) {
        Ok(p) => p,
        Err(_) => return (Vec::new(), skipped),
    };
    let tree = match parser.parse(code) {
        Ok(t) => t,
        Err(_) => return (Vec::new(), skipped), // invalid code is caught elsewhere
    };
    let mut findings = Vec::new();
    for rule in &compiled {
        findings.extend(run_rule(rule, &tree, ""));
    }
    (findings, skipped)
}

/// Whether a rule's language applies to a file's language/extension.
pub fn language_matches(rule_language: &str, file_language: &str) -> bool {
    let r = rule_language.to_lowercase();
    let f = file_language.to_lowercase();
    r == f
        || (f == "rs" && r == "rust")
        || (f == "py" && r == "python")
        || (f == "js" && r == "javascript")
        || (f == "ts" && r == "typescript")
        || (f == "kt" && r == "kotlin")
        || (f == "cs" && r == "csharp")
        || (f == "sh" && r == "bash")
}

/// Load the builtin rules (once, cached).
pub fn builtin_rules() -> Vec<Rule> {
    static CACHE: std::sync::OnceLock<Vec<Rule>> = std::sync::OnceLock::new();
    CACHE
        .get_or_init(|| {
            load_rules_yaml(include_str!("../../rules/builtin.yaml"))
                .unwrap_or_default()
        })
        .clone()
}

/// Run the builtin rules against source code text (for the edit guardian).
/// Returns findings and whether any were error-severity.
pub fn check_code_with_builtin(code: &str, language: &str) -> (Vec<Finding>, usize, bool) {
    let rules = builtin_rules();
    let (findings, skipped) = check_code(code, language, &rules);
    let has_error = findings.iter().any(|f| f.severity == Severity::Error);
    (findings, skipped, has_error)
}

fn match_pattern_recursive(
    pattern: &TreeNode,
    source: &TreeNode,
    rule: &CompiledRule,
    file: &str,
    findings: &mut Vec<Finding>,
) {
    // A pattern root that is a statement wrapper (expression_statement) with
    // one meaningful child should match that child's content anywhere — e.g.
    // `$X.unwrap();` must match `x.unwrap()` inside a let_declaration too.
    // `;` and similar punctuation are filtered out as non-meaningful.
    let inner: Option<&TreeNode> = {
        let meaningful: Vec<&TreeNode> = pattern
            .children
            .iter()
            .filter(|c| !is_whitespace_node(c) && !is_punct_node(c))
            .collect();
        if meaningful.len() == 1 && is_statement_wrapper(&pattern.node_type) {
            meaningful.first().copied()
        } else {
            None
        }
    };

    let effective_pattern = inner.unwrap_or(pattern);
    if let Some(bindings) = match_node(effective_pattern, source, rule) {
        let captures = bindings;
        findings.push(Finding {
            rule_id: rule.rule.id.clone(),
            severity: rule.rule.severity,
            message: rule.rule.message.clone(),
            file: file.to_string(),
            line: source.start_line,
            column: source.start_col,
            captures,
        });
    }
    for child in &source.children {
        match_pattern_recursive(pattern, child, rule, file, findings);
    }
}

/// Node types that are pure statement wrappers around an inner expression.
fn is_statement_wrapper(t: &str) -> bool {
    matches!(t, "expression_statement" | "statement" | "expression")
}

/// Structural match of a pattern node against a source node.
/// Returns Some(bindings) on match; bindings map placeholder NAME -> content
/// (so repeated `$X` occurrences share a key and must bind identically).
fn match_node(
    pattern: &TreeNode,
    source: &TreeNode,
    rule: &CompiledRule,
) -> Option<HashMap<String, String>> {
    // Placeholder: matches any node, binds its content under the $NAME.
    if let Some(name) = rule.placeholders.get(&pattern.path) {
        let mut b = HashMap::new();
        b.insert(name.clone(), source.content.clone());
        return Some(b);
    }

    // Otherwise require same node type.
    if pattern.node_type != source.node_type {
        return None;
    }

    // Leaf: compare content (normalized).
    if pattern.children.is_empty() && source.children.is_empty() {
        if normalize(pattern.content.clone()) == normalize(source.content.clone()) {
            return Some(HashMap::new());
        }
        return None;
    }

    // Structural: match children pairwise.
    // Filter to non-whitespace/meaningful children if counts differ.
    let p_children: Vec<&TreeNode> = pattern
        .children
        .iter()
        .filter(|c| !is_whitespace_node(c))
        .collect();
    let s_children: Vec<&TreeNode> = source
        .children
        .iter()
        .filter(|c| !is_whitespace_node(c))
        .collect();

    if p_children.len() != s_children.len() {
        return None;
    }

    let mut bindings = HashMap::new();
    for (p, s) in p_children.iter().zip(s_children.iter()) {
        let sub = match_node(p, s, rule)?;
        merge_bindings(&mut bindings, sub)?;
    }
    Some(bindings)
}

/// Merge child bindings; fails if the same placeholder binds different content.
fn merge_bindings(
    acc: &mut HashMap<String, String>,
    new: HashMap<String, String>,
) -> Option<()> {
    for (k, v) in new {
        if let Some(existing) = acc.get(&k) {
            if existing != &v {
                return None; // $X = $X with different content -> no match
            }
        } else {
            acc.insert(k, v);
        }
    }
    Some(())
}

/// Normalize whitespace for leaf content comparison.
fn normalize(mut s: String) -> String {
    s = s.trim().to_string();
    // Collapse runs of whitespace to a single space.
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out
}

fn is_whitespace_node(node: &TreeNode) -> bool {
    node.node_type == "whitespace"
        || node.node_type == "comment"
        || node.content.trim().is_empty()
}

/// Punctuation-only nodes (e.g. `;`) that are not structurally meaningful.
fn is_punct_node(node: &TreeNode) -> bool {
    matches!(node.node_type.as_str(), ";" | "," | "(" | ")" | "{" | "}")
        || node.content.trim().chars().all(|c| {
            matches!(c, ';' | ',' | '(' | ')' | '{' | '}' | '[' | ']')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compile(pattern: &str, language: &str) -> CompiledRule {
        let rule = Rule {
            id: "test".into(),
            language: language.into(),
            severity: Severity::Warning,
            message: "test rule".into(),
            pattern: pattern.into(),
        };
        compile_rule(&rule).expect("rule should compile")
    }

    fn parse_source(code: &str, language: &str) -> TreeNode {
        let parser = crate::parser::get_parser_for_language(language).unwrap();
        parser.parse(code).unwrap()
    }

    #[test]
    fn test_self_assignment_rust() {
        let rule = compile("$X = $X;", "rust");
        let tree = parse_source("fn main() { a = a; b = c; }", "rust");
        let findings = run_rule(&rule, &tree, "test.rs");
        // Only `a = a` should match (identical content binding).
        assert_eq!(findings.len(), 1, "only self-assignment should match");
    }

    #[test]
    fn test_unwrap_rust() {
        let rule = compile("$X.unwrap()", "rust");
        let tree = parse_source(
            "fn f() { let a = x.unwrap(); let b = y.ok(); }",
            "rust",
        );
        let findings = run_rule(&rule, &tree, "test.rs");
        assert_eq!(findings.len(), 1, "only unwrap should match");
    }

    #[test]
    fn test_except_pass_python() {
        let rule = compile("except:\n    pass", "python");
        let tree = parse_source(
            "try:\n    x()\nexcept:\n    pass\ntry:\n    y()\nexcept Exception:\n    pass\n",
            "python",
        );
        let findings = run_rule(&rule, &tree, "test.py");
        assert_eq!(findings.len(), 1, "only bare except: pass should match");
    }
}














