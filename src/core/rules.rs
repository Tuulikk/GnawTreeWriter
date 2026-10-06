//! Pattern-matching rule engine (semgrep-inspired).
//!
//! Rules are semgrep-like code patterns (`$X = $X`) with `$VARIABLE`
//! placeholders. Each rule is parsed with the same parser as its target
//! language, then matched structurally against source ASTs in Rust — no
//! tree-sitter query generation. See docs/RULES_ENGINE_SPEC.md.

use crate::parser::TreeNode;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

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
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct Rule {
    pub id: String,
    pub language: String,
    #[serde(default = "default_severity")]
    pub severity: Severity,
    pub message: String,
    pub pattern: String,
    /// Optional rewrite template (ast-grep parity): `$X` metavariables bind
    /// from the pattern match and expand into replacement code. A rule with
    /// a fix is APPLYABLE via `lint --fix` (never applied without the flag).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

fn default_severity() -> Severity {
    Severity::Warning
}

/// YAML container for a rules file.
#[derive(Debug, Deserialize, serde::Serialize)]
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
    /// AST node path of the matched node (for surgical multi-edit).
    pub node_path: String,
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
    let file: RulesFile = serde_yaml::from_str(yaml).context("failed to parse rules YAML")?;
    Ok(file.rules)
}

/// Serialize rules back to YAML (for saving project rules).
pub fn rules_to_yaml(rules: &[Rule]) -> Result<String> {
    let file = RulesFile {
        rules: rules.to_vec(),
    };
    serde_yaml::to_string(&file).context("failed to serialize rules")
}

/// The default project rules file path (cwd-based).
pub fn project_rules_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("gnawtreewriter.rules.yaml")
}

/// Load project rules from `gnawtreewriter.rules.yaml` if it exists.
pub fn load_project_rules() -> Result<Vec<Rule>> {
    let path = project_rules_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let yaml = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    load_rules_yaml(&yaml)
}

/// Append a rule to the project rules file (creating it if needed).
/// Returns Err if a rule with the same id already exists.
pub fn append_project_rule(rule: &Rule) -> Result<()> {
    let mut rules = load_project_rules()?;
    if rules.iter().any(|r| r.id == rule.id) {
        anyhow::bail!(
            "rule '{}' already exists in {}",
            rule.id,
            project_rules_path().display()
        );
    }
    rules.push(rule.clone());
    let yaml = rules_to_yaml(&rules)?;
    std::fs::write(project_rules_path(), yaml)
        .with_context(|| format!("failed to write {}", project_rules_path().display()))
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
            "rust"
                | "rs"
                | "javascript"
                | "js"
                | "typescript"
                | "ts"
                | "go"
                | "java"
                | "c"
                | "cpp"
                | "csharp"
                | "cs"
                | "php"
                | "swift"
                | "kotlin"
                | "kt"
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
    if best
        .as_ref()
        .map(|(t, _)| t.children.is_empty())
        .unwrap_or(true)
    {
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
            let first_repl = subs.first().map(|(_, r)| r.clone()).unwrap_or_default();
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
            out.insert(
                node.path.clone(),
                placeholder.trim_start_matches('$').to_string(),
            );
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
    fn find_deepest<'a>(node: &'a TreeNode, needle: &str) -> Option<&'a TreeNode> {
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
    // Deduplicate: the same code location can be matched as several nodes
    // (e.g. a statement and its inner children). Keep the first per location.
    let mut seen: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    findings.retain(|f| seen.insert((f.line, f.column)));
    findings
}

/// Compile a set of rules once and run them against source code text.
/// Returns findings (empty if no rules match or none apply to this language).
/// Rules that fail to compile are skipped (reported via `skipped`).
pub fn check_code(code: &str, language: &str, rules: &[Rule]) -> (Vec<Finding>, usize) {
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
            load_rules_yaml(include_str!("../../rules/builtin.yaml")).unwrap_or_default()
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

/// Options controlling a lint run over files or directories.
pub struct LintOptions<'a> {
    /// Expand directories into their supported files. Without it, a
    /// directory path is an error (same safety rule as the CLI).
    pub recursive: bool,
    /// Extra rules YAML file, loaded after builtin + project rules.
    pub rules_file: Option<&'a str>,
    /// Minimum severity to keep (e.g. "warning" hides "info" findings).
    pub severity_filter: Option<&'a str>,
    /// Only run the rule with this exact id.
    pub rule_filter: Option<&'a str>,
    /// Ad-hoc pattern search: compile this `$X` pattern in-memory and
    /// report matches (never write) — the ast-grep `run -p` equivalent.
    /// When set, stored rules are not used.
    pub ad_hoc_pattern: Option<&'a str>,
    /// Language for the ad-hoc pattern (required together with it).
    pub ad_hoc_language: Option<&'a str>,
    /// Stop after this many findings; the result is flagged truncated.
    pub max_findings: usize,
}

impl Default for LintOptions<'_> {
    fn default() -> Self {
        LintOptions {
            recursive: false,
            rules_file: None,
            severity_filter: None,
            rule_filter: None,
            ad_hoc_pattern: None,
            ad_hoc_language: None,
            max_findings: 1000,
        }
    }
}

/// Result of a lint run over a set of files.
pub struct LintResult {
    pub findings: Vec<Finding>,
    pub files_checked: usize,
    pub skipped_rules: usize,
    /// Per-rule compile warnings ("skipping rule 'x': ...").
    pub rule_warnings: Vec<String>,
    /// Per-file read/parse errors ("file: message").
    pub file_errors: Vec<String>,
    /// True when `max_findings` was reached and the run stopped early.
    pub truncated: bool,
    /// The effective rule set used for this run (builtin + project +
    /// rules_file, after filters). Carries the fix templates needed by
    /// `lint --fix` without re-reading the rules files.
    pub rules: Vec<Rule>,
}

/// Recursively collect lintable files under `dir` (same extension list the
/// CLI uses, so CLI and MCP lint the same files).
pub fn find_lintable_files(dir: &std::path::Path) -> Result<Vec<String>> {
    const SUPPORTED: &[&str] = &[
        "py", "rs", "ts", "tsx", "js", "jsx", "php", "html", "htm", "qml", "go", "toml", "json",
        "yaml", "yml", "css", "md", "markdown", "txt", "xml", "svg", "xsl", "xsd", "rss", "atom",
    ];
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                files.extend(find_lintable_files(&path)?);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if SUPPORTED.contains(&ext) {
                    if let Some(s) = path.to_str() {
                        files.push(s.to_string());
                    }
                }
            }
        }
    }
    Ok(files)
}

/// Expand a `fix:` template with the bindings from a match. `$NAME`
/// metavariables are replaced by the captured source content; a
/// metavariable with no binding is an error — the fix cannot be built and
/// must never be guessed.
pub fn expand_fix(fix: &str, captures: &HashMap<String, String>) -> Result<String> {
    let mut out = String::with_capacity(fix.len());
    let mut rest = fix;
    while let Some(pos) = rest.find('$') {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 1..];
        let name_len = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .count();
        let name: String = after.chars().take(name_len).collect();
        if name.is_empty() {
            anyhow::bail!("fix template contains a bare '$' with no metavariable name");
        }
        let bound = captures.get(&name).ok_or_else(|| {
            anyhow::anyhow!(
                "fix template references ${} which the pattern does not capture",
                name
            )
        })?;
        out.push_str(bound);
        rest = &after[name_len..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Validate that a fix template parses as the rule's language once its
/// `$NAME` metavariables are replaced by placeholder identifiers (the same
/// substitution strategy `compile_rule` uses for patterns). Rejected fixes
/// never reach the rules file.
pub fn validate_fix(language: &str, fix: &str) -> Result<()> {
    let parser = crate::parser::get_parser_for_language(language)
        .with_context(|| format!("unknown language '{}'", language))?;
    let mut rest = fix;
    let mut substituted = String::with_capacity(fix.len());
    let mut seen: HashMap<String, String> = HashMap::new();
    while let Some(pos) = rest.find('$') {
        substituted.push_str(&rest[..pos]);
        let after = &rest[pos + 1..];
        let name_len = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .count();
        let name: String = after.chars().take(name_len).collect();
        if name.is_empty() {
            anyhow::bail!("fix template contains a bare '$' with no metavariable name");
        }
        let entry = seen.len();
        let repl = seen
            .entry(name)
            .or_insert_with(|| format!("_gtw_fix_{entry}"));
        substituted.push_str(repl);
        rest = &after[name_len..];
    }
    substituted.push_str(rest);
    // Expression-shaped fixes (e.g. `if $X { $Y } else { $Z }`, or anything
    // ending in `?`) do not parse as top-level items — retry inside a
    // function scaffold, mirroring how compile_rule scaffolds statement-like
    // patterns. Both attempts must fail to reject the fix.
    if parser.parse(&substituted).is_err() {
        let scaffolded = format!("fn _gtw_fix_probe() {{ {} }}", substituted);
        parser
            .parse(&scaffolded)
            .with_context(|| "fix template does not parse as valid code")?;
    }
    Ok(())
}

/// Build the apply-plan for `lint --fix`: one atomic batch of Edit
/// operations, one per finding whose rule carries a fix. Editing replaces a
/// node's content in place, so sibling node paths stay valid and the whole
/// set applies atomically (batch semantics: validated in memory first,
/// rollback on any failure). Returns the batch plus the number of findings
/// skipped because their rule has no fix (report-only rules are never
/// touched). The batch is returned unapplied — the caller decides preview
/// vs apply, and apply only happens behind an explicit `--fix`.
pub fn fix_batch(findings: &[Finding], rules: &[Rule]) -> Result<(crate::core::Batch, usize)> {
    let fixes: HashMap<&str, &str> = rules
        .iter()
        .filter_map(|r| r.fix.as_deref().map(|f| (r.id.as_str(), f)))
        .collect();
    let mut ops: Vec<crate::core::BatchOp> = Vec::new();
    let mut skipped = 0usize;
    // Never emit two writes for the same node, even across duplicated
    // findings (dedup is line+column based; node paths can still repeat).
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for f in findings {
        let template = match fixes.get(f.rule_id.as_str()) {
            Some(t) => *t,
            None => {
                skipped += 1;
                continue;
            }
        };
        let content = expand_fix(template, &f.captures)?;
        if seen.insert((f.file.clone(), f.node_path.clone())) {
            ops.push(crate::core::BatchOp::Edit {
                file: f.file.clone(),
                path: f.node_path.clone(),
                content,
            });
        }
    }
    let batch = crate::core::Batch {
        description: Some("lint --fix auto-edits".to_string()),
        operations: ops,
        transaction_ids: std::cell::RefCell::new(Vec::new()),
    };
    Ok((batch, skipped))
}

/// Shared lint core used by both the CLI `lint` command and the MCP `lint`
/// tool — single implementation so the two never diverge. Report-only:
/// this function never writes to disk.
/// Substitute $METAVARS in a rule message with the matched bindings so a
/// finding reads e.g. "… if `code` is a String/&str" instead of the raw
/// template. Unbound names stay literal (messages must never fail — unlike
/// fix templates, which validate bindings strictly).
pub fn interpolate_message(msg: &str, captures: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(msg.len());
    let mut rest = msg;
    while let Some(pos) = rest.find('$') {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 1..];
        let name: String = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if name.is_empty() {
            out.push('$');
            rest = after;
            continue;
        }
        match captures.get(&name) {
            Some(v) => out.push_str(v),
            None => {
                out.push('$');
                out.push_str(&name);
            }
        }
        rest = &after[name.len()..];
    }
    out.push_str(rest);
    out
}

pub fn run_lint(paths: &[String], opts: &LintOptions) -> Result<LintResult> {
    // Build the rule set. An ad-hoc pattern replaces stored rules entirely:
    // callers ask "where does this pattern occur", not "what rules fire".
    let ad_hoc_rule = match (opts.ad_hoc_pattern, opts.ad_hoc_language) {
        (Some(pattern), Some(language)) => Some(Rule {
            id: "ad_hoc".to_string(),
            language: language.to_string(),
            severity: Severity::Warning,
            message: format!("Pattern match: {}", pattern.trim()),
            pattern: pattern.to_string(),
            fix: None,
        }),
        (Some(_), None) | (None, Some(_)) => {
            anyhow::bail!("ad-hoc pattern search requires both `pattern` and `language`");
        }
        _ => None,
    };

    let mut rules: Vec<Rule> = Vec::new();
    if let Some(rule) = &ad_hoc_rule {
        rules.push(rule.clone());
    } else {
        rules.extend(load_rules_yaml(include_str!("../../rules/builtin.yaml"))?);
        if let Ok(cwd) = std::env::current_dir() {
            let project_rules = cwd.join("gnawtreewriter.rules.yaml");
            if project_rules.exists() {
                if let Ok(yaml) = std::fs::read_to_string(&project_rules) {
                    rules.extend(load_rules_yaml(&yaml)?);
                }
            }
        }
        if let Some(rf) = opts.rules_file {
            let yaml = std::fs::read_to_string(rf)
                .map_err(|e| anyhow::anyhow!("failed to read rules file {}: {}", rf, e))?;
            rules.extend(load_rules_yaml(&yaml)?);
        }
    }

    // Unknown rule id must fail loudly — a silent empty result would read
    // as "the rule passed" (the exact lie finding #14 was about).
    if let (Some(rf), None) = (opts.rule_filter, opts.ad_hoc_pattern) {
        if !rules.iter().any(|r| r.id == rf) {
            anyhow::bail!(
                "unknown rule id {} — not in the {} loaded rules (builtin + project + --rules file). \
Run `gnawtreewriter rules list` to see ids, or add it with `rules add` / MCP add_rule.",
                rf,
                rules.len()
            );
        }
    }

    // Compile rules once; report (never silence) invalid ones.
    let mut compiled = Vec::new();
    let mut skipped = 0usize;
    let mut rule_warnings = Vec::new();
    for rule in &rules {
        if let Some(sf) = opts.severity_filter {
            let min = severity_rank(Severity::parse(sf));
            if severity_rank(rule.severity) < min {
                continue;
            }
        }
        if let Some(rf) = opts.rule_filter {
            if rule.id != rf {
                continue;
            }
        }
        match compile_rule(rule) {
            Ok(c) => compiled.push(c),
            Err(e) => {
                rule_warnings.push(format!("skipping rule '{}': {}", rule.id, e));
                skipped += 1;
            }
        }
    }

    // Resolve the file list (directories require `recursive`).
    let mut all_files: Vec<String> = Vec::new();
    for path in paths {
        let path_buf = std::path::PathBuf::from(path);
        if path_buf.is_dir() {
            if !opts.recursive {
                anyhow::bail!(
                    "Directory '{}' requires recursive=true for safety. \
                     Pass recursive: true, or list files explicitly.",
                    path
                );
            }
            all_files.extend(find_lintable_files(&path_buf)?);
        } else {
            all_files.push(path.clone());
        }
    }

    let mut result = LintResult {
        findings: Vec::new(),
        files_checked: 0,
        skipped_rules: skipped,
        rule_warnings,
        file_errors: Vec::new(),
        truncated: false,
        rules: rules.clone(),
    };

    for file_path in &all_files {
        result.files_checked += 1;
        let code = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                result.file_errors.push(format!("{}: {}", file_path, e));
                continue;
            }
        };
        let lang = std::path::Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let tree = match crate::parser::get_parser_for_language(&lang) {
            Ok(p) => match p.parse(&code) {
                Ok(t) => t,
                Err(e) => {
                    result.file_errors.push(format!("{}: {}", file_path, e));
                    continue;
                }
            },
            Err(e) => {
                result.file_errors.push(format!("{}: {}", file_path, e));
                continue;
            }
        };
        for rule in &compiled {
            if !language_matches(&rule.rule.language, &lang) {
                continue;
            }
            for f in run_rule(rule, &tree, file_path) {
                if result.findings.len() >= opts.max_findings {
                    result.truncated = true;
                    return Ok(result);
                }
                result.findings.push(f);
            }
        }
    }
    Ok(result)
}

/// Numeric rank for severity filtering (error > warning > info).
pub fn severity_rank(s: Severity) -> u8 {
    match s {
        Severity::Error => 3,
        Severity::Warning => 2,
        Severity::Info => 1,
    }
}

/// Format findings as compact prompt annotations, one per line:
/// `⚠️ line N [rule_id] message`. Empty string when there are no findings.
pub fn format_findings_for_prompt(findings: &[Finding]) -> String {
    if findings.is_empty() {
        return String::new();
    }
    let mut out = String::from("Known issues in this file (from lint rules):\n");
    for f in findings {
        let sev = match f.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        };
        out.push_str(&format!(
            "- line {} [{}] ({}) {}\n",
            f.line, f.rule_id, sev, f.message
        ));
    }
    out
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
            message: interpolate_message(&rule.rule.message, &captures),
            file: file.to_string(),
            line: source.start_line,
            column: source.start_col,
            node_path: source.path.clone(),
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
fn merge_bindings(acc: &mut HashMap<String, String>, new: HashMap<String, String>) -> Option<()> {
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
    node.node_type == "whitespace" || node.node_type == "comment" || node.content.trim().is_empty()
}

/// Punctuation-only nodes (e.g. `;`) that are not structurally meaningful.
fn is_punct_node(node: &TreeNode) -> bool {
    matches!(node.node_type.as_str(), ";" | "," | "(" | ")" | "{" | "}")
        || node
            .content
            .trim()
            .chars()
            .all(|c| matches!(c, ';' | ',' | '(' | ')' | '{' | '}' | '[' | ']'))
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
            fix: None,
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
        let tree = parse_source("fn f() { let a = x.unwrap(); let b = y.ok(); }", "rust");
        let findings = run_rule(&rule, &tree, "test.rs");
        assert_eq!(findings.len(), 1, "only unwrap should match");
    }

    #[test]
    fn test_no_duplicate_findings() {
        // A statement wrapper + inner child can both match the same pattern;
        // run_rule must report each code location only once.
        let rule = compile("$X.unwrap()", "rust");
        let tree = parse_source("fn f() { let a = x.unwrap(); let b = x.unwrap(); }", "rust");
        let findings = run_rule(&rule, &tree, "test.rs");
        assert_eq!(findings.len(), 2, "two separate unwrap calls expected");

        // Distinct locations but same content must both be reported.
        let lines: Vec<usize> = findings.iter().map(|f| f.line).collect();
        assert_eq!(lines, vec![1, 1]);
        let cols: Vec<usize> = findings.iter().map(|f| f.column).collect();
        assert_ne!(cols[0], cols[1], "different columns are separate findings");
    }
    #[test]
    fn test_run_lint_finds_unwrap_in_fixture() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sample.rs");
        std::fs::write(
            &file,
            "fn f() {\n    let a = x.unwrap();\n    let b = y.ok();\n}\n",
        )
        .unwrap();

        let opts = LintOptions {
            ad_hoc_pattern: Some("$X.unwrap()"),
            ad_hoc_language: Some("rust"),
            ..Default::default()
        };
        let result = run_lint(&[file.to_string_lossy().to_string()], &opts).unwrap();
        assert_eq!(result.files_checked, 1);
        assert_eq!(result.findings.len(), 1, "only the unwrap should match");
        let f = &result.findings[0];
        assert_eq!(f.rule_id, "ad_hoc");
        assert_eq!(f.line, 2, "unwrap is on line 2");
        assert!(f.file.ends_with("sample.rs"));
        assert!(result.file_errors.is_empty());
        assert!(!result.truncated);
    }

    #[test]
    fn test_run_lint_directory_requires_recursive() {
        let dir = tempfile::tempdir().unwrap();
        let opts = LintOptions::default();
        let result = run_lint(&[dir.path().to_string_lossy().to_string()], &opts);
        assert!(
            result.is_err(),
            "directory without recursive must be an error"
        );
    }

    #[test]
    fn test_run_lint_recursive_directory() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("nested");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(dir.path().join("a.rs"), "fn f() { x.unwrap(); }").unwrap();
        std::fs::write(sub.join("b.rs"), "fn g() { y.unwrap(); }").unwrap();
        std::fs::write(sub.join("c.txt"), "not linted").unwrap();

        let opts = LintOptions {
            recursive: true,
            ad_hoc_pattern: Some("$X.unwrap()"),
            ad_hoc_language: Some("rust"),
            ..Default::default()
        };
        let result = run_lint(&[dir.path().to_string_lossy().to_string()], &opts).unwrap();
        assert_eq!(result.files_checked, 3, "txt is also in the supported set");
        assert_eq!(result.findings.len(), 2, "one unwrap per file");
        let files: Vec<&str> = result.findings.iter().map(|f| f.file.as_str()).collect();
        assert!(
            files.iter().any(|f| f.ends_with("b.rs")),
            "nested file must be visited"
        );
    }

    #[test]
    fn test_run_lint_truncation() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("many.rs");
        std::fs::write(
            &file,
            "fn f() {\n    a.unwrap();\n    b.unwrap();\n    c.unwrap();\n}\n",
        )
        .unwrap();

        let opts = LintOptions {
            ad_hoc_pattern: Some("$X.unwrap()"),
            ad_hoc_language: Some("rust"),
            max_findings: 2,
            ..Default::default()
        };
        let result = run_lint(&[file.to_string_lossy().to_string()], &opts).unwrap();
        assert_eq!(result.findings.len(), 2);
        assert!(result.truncated, "reaching max_findings must set truncated");
    }

    #[test]
    fn test_run_lint_severity_filter() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("s.rs");
        std::fs::write(&file, "fn f() { x.unwrap(); }").unwrap();

        // Ad-hoc findings are Warning; filtering to "error" must drop them.
        let opts = LintOptions {
            ad_hoc_pattern: Some("$X.unwrap()"),
            ad_hoc_language: Some("rust"),
            severity_filter: Some("error"),
            ..Default::default()
        };
        let result = run_lint(&[file.to_string_lossy().to_string()], &opts).unwrap();
        assert!(
            result.findings.is_empty(),
            "warning findings must be filtered out"
        );

        let opts = LintOptions {
            ad_hoc_pattern: Some("$X.unwrap()"),
            ad_hoc_language: Some("rust"),
            severity_filter: Some("warning"),
            ..Default::default()
        };
        let result = run_lint(&[file.to_string_lossy().to_string()], &opts).unwrap();
        assert_eq!(
            result.findings.len(),
            1,
            "warning must pass a warning filter"
        );
    }

    #[test]
    fn test_run_lint_reports_unreadable_file() {
        let opts = LintOptions {
            ad_hoc_pattern: Some("$X.unwrap()"),
            ad_hoc_language: Some("rust"),
            ..Default::default()
        };
        let result = run_lint(&["/nonexistent/definitely_missing.rs".to_string()], &opts).unwrap();
        assert_eq!(result.files_checked, 1, "attempted files are counted");

        assert_eq!(result.findings.len(), 0);
        assert!(
            !result.file_errors.is_empty(),
            "missing file must be surfaced as a file error, never silenced"
        );
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

    #[test]
    fn interpolate_message_binds_metavars() {
        let mut caps = HashMap::new();
        caps.insert("X".to_string(), "code".to_string());
        assert_eq!(
            interpolate_message("slice on $X; check $X twice", &caps),
            "slice on code; check code twice"
        );
        // Unbound names stay literal — messages must never fail.
        assert_eq!(interpolate_message("keep $Z as-is", &caps), "keep $Z as-is");
        // Bare dollar survives.
        assert_eq!(interpolate_message("cost: 5$ ok", &caps), "cost: 5$ ok");
    }

    #[test]
    fn run_lint_unknown_rule_id_fails_loudly() {
        let opts = LintOptions {
            rule_filter: Some("finns_inte"),
            ..Default::default()
        };
        let err = run_lint(&["src".to_string()], &opts)
            .err()
            .expect("unknown id must not pass silently");
        let msg = format!("{}", err);
        assert!(msg.contains("unknown rule id"), "got: {}", msg);
        assert!(
            msg.contains("rules list"),
            "must point at next step: {}",
            msg
        );
    }

    #[test]
    fn run_lint_known_rule_id_zero_findings_is_ok() {
        let opts = LintOptions {
            rule_filter: Some("rust_string_byte_slice"),
            ..Default::default()
        };
        // Fresh temp file without byte slices: legitimately empty, no error.
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("clean.rs");
        std::fs::write(&f, "fn main() { let x = 1; println!(\"{}\", x); }").unwrap();
        let res = run_lint(&[f.to_string_lossy().to_string()], &opts)
            .expect("known id on clean file must succeed");
        assert_eq!(res.findings.len(), 0);
    }
}
