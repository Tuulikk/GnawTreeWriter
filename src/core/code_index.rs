//! code-index — project structure index generator.
//!
//! WHY: behavior docs (UPPSLAG-class) explain HOW a system works; nothing
//! explains WHERE code sits. Hand-maintained indexes rot silently — a
//! generated one carries its own freshness proof in the header (git rev +
//! GTW version), so staleness is visible at a glance instead of being
//! discovered when the index lies (§50-class lesson from consuming projects).
//!
//! WHAT: tree of source files + per-file item skeletons (fns, structs,
//! impls+methods, classes, traits, enums...) extracted from the tree-sitter
//! tree — language-agnostic kind matching with a keyword fallback, so new
//! grammars degrade gracefully instead of yielding empty sections.
//!
//! HYBRID DESIGN: the generated part is mechanical; hand-written prose enters
//! via `--intro <file>` blocks keyed by directory:
//!   `<!-- intro: crates/motor2-server --> ... <!-- /intro -->`
//! The tool splices intros under matching section headings — ONE command
//! produces the final file (scaffolding principle: the tool does the lifting).

use crate::parser::{get_parser, TreeNode};
use anyhow::Context;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ── Item extraction ──────────────────────────────────────────────────────────

/// Index categories, matched against raw tree-sitter kinds. Keyword fallback
/// covers grammar variants (rust `function_item`, ts `function_declaration`,
/// py `function_definition`, ...) without per-language tables.
fn item_category(kind: &str) -> Option<&'static str> {
    let k = kind.to_lowercase();
    // Exact-first table for the common kinds (cheap, readable):
    match k.as_str() {
        "function_item"
        | "function_declaration"
        | "function_definition"
        | "method_definition"
        | "method_declaration" => Some("fn"),
        "struct_item" | "struct_specifier" | "struct_declaration" => Some("struct"),
        "enum_item" | "enum_specifier" | "enum_declaration" => Some("enum"),
        "impl_item" => Some("impl"),
        "trait_item" | "trait_declaration" => Some("trait"),
        "class_declaration" | "class_definition" | "class_specifier" => Some("class"),
        "interface_declaration" => Some("interface"),
        "type_item" | "type_alias_declaration" | "type_declaration" => Some("type"),
        "const_item" | "const_declaration" => Some("const"),
        "static_item" => Some("static"),
        _ => {
            // Keyword fallback for grammar variants ("function_declaration",
            // "class_definition", ...). Requires an underscore: bare kinds are
            // tree-sitter KEYWORD TOKENS ("impl", "fn", "struct") — matching
            // those floods the index with noise.
            if !k.contains('_') {
                return None;
            }
            if k.contains("interface") {
                Some("interface")
            } else if k.contains("struct") {
                Some("struct")
            } else if k.contains("enum") {
                Some("enum")
            } else if k.contains("trait") {
                Some("trait")
            } else if k.contains("impl") {
                Some("impl")
            } else if k.contains("class") {
                Some("class")
            } else if k.contains("method") || k.contains("function") {
                Some("fn")
            } else if k.contains("macro") && k.contains("defin") {
                Some("macro")
            } else {
                None
            }
        }
    }
}

/// Kinds that GROUP items: indexed themselves (their header line) and recursed
/// ONE level for methods/functions inside (impl blocks, classes).
fn is_container(kind: &str) -> bool {
    matches!(
        item_category(kind),
        Some("impl") | Some("class") | Some("interface")
    )
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexItem {
    pub category: String,
    /// Identifier when extractable — the consumer's revalidation key
    /// (node_path is only valid at the indexed content hash).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub sig: String,
    pub line: usize,
    /// GTW dot-path captured at extraction — valid for the indexed content
    /// only; re-validate via read_node (name match) before editing.
    pub node_path: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<IndexItem>,
}

/// Sanity cap for STORED signatures: presentation truncation (opts.max_sig)
/// happens at render, so both render paths share one truncation — parity by
/// construction (review decision #1).
pub(crate) const STORED_SIG_CAP: usize = 512;

/// Presentation-time truncation, shared by every render path (markdown and
/// JSON): char-safe clipping with an explicit truncation marker.
fn truncate_sig(sig: &str, max_len: usize) -> String {
    if sig.chars().count() > max_len {
        sig.chars().take(max_len).collect::<String>() + "…"
    } else {
        sig.to_string()
    }
}

/// First meaningful line of an item's content = its signature. Attributes and
/// doc comments are sibling nodes, so content starts at the item itself.
fn signature(node: &TreeNode) -> String {
    let first = node
        .content
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty())
        .unwrap_or("");
    let mut sig = first.trim_end_matches(['{', ':', ';']).trim().to_string();
    // Collapse internal whitespace (multi-token signatures stay one line).
    sig = sig.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_sig(&sig, STORED_SIG_CAP)
}

/// Extract indexable items from a parsed file tree. `mod` items are listed but
/// NOT recursed — that keeps `mod tests` floods out of the index naturally.
pub fn extract_items(root: &TreeNode) -> Vec<IndexItem> {
    let mut items = Vec::new();
    for child in &root.children {
        // Skip pure noise nodes (comments, attributes, use-trees).
        if child.node_type.contains("comment")
            || child.node_type == "attribute_item"
            || child.node_type == "use_declaration"
        {
            continue;
        }
        if let Some(cat) = item_category(&child.node_type) {
            let mut item = IndexItem {
                category: cat.to_string(),
                name: child.get_name().unwrap_or_default(),
                sig: signature(child),
                line: child.start_line,
                node_path: child.path.clone(),
                methods: Vec::new(),
            };
            if is_container(&child.node_type) {
                // Methods sit behind a wrapper ("declaration_list", class
                // body, ...) — scan up to TWO levels inside the container,
                // fn-category items only.
                fn collect(nodes: &[TreeNode], depth: usize, out: &mut Vec<IndexItem>) {
                    for inner in nodes {
                        if let Some(inner_cat) = item_category(&inner.node_type) {
                            if matches!(inner_cat, "fn") {
                                out.push(IndexItem {
                                    category: inner_cat.to_string(),
                                    name: inner.get_name().unwrap_or_default(),
                                    sig: signature(inner),
                                    line: inner.start_line,
                                    node_path: inner.path.clone(),
                                    methods: Vec::new(),
                                });
                                continue;
                            }
                        }
                        if depth > 0 {
                            collect(&inner.children, depth - 1, out);
                        }
                    }
                }
                collect(&child.children, 1, &mut item.methods);
            }
            items.push(item);
        }
    }
    items
}

// ── Intro splicing ───────────────────────────────────────────────────────────

/// Parse `<!-- intro: KEY --> body <!-- /intro -->` blocks from an intro file.
/// Unknown keys are reported as a warning list so stale intros SURFACE
/// (a silently dropped intro is the §10 pattern: work done, no effect).
pub fn parse_intros(text: &str) -> (BTreeMap<String, String>, Vec<String>) {
    let mut map = BTreeMap::new();
    let mut unknown = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("<!-- intro:") {
        let after = &rest[start + "<!-- intro:".len()..];
        let Some(key_end) = after.find("-->") else {
            break;
        };
        let key = after[..key_end].trim().to_string();
        let body_start = after[key_end + 3..]
            .strip_prefix('\n')
            .unwrap_or(&after[key_end + 3..]);
        let Some(close) = body_start.find("<!-- /intro -->") else {
            break;
        };
        map.insert(key, body_start[..close].trim().to_string());
        rest = &body_start[close + "<!-- /intro -->".len()..];
    }
    // Report keys present in text but never closed/paired — a typo should
    // shout, not vanish.
    for line in text.lines().filter(|l| l.contains("<!-- intro:")) {
        let key = line
            .split("<!-- intro:")
            .nth(1)
            .and_then(|s| s.split("-->").next())
            .unwrap_or("")
            .trim();
        if !map.contains_key(key) {
            unknown.push(key.to_string());
        }
    }
    (map, unknown)
}

// ── Generation ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CodeIndexOptions {
    pub extensions: Vec<String>,
    pub max_sig: usize,
    pub intro_file: Option<PathBuf>,
    pub json: bool,
}

impl Default for CodeIndexOptions {
    fn default() -> Self {
        Self {
            extensions: [
                "rs", "py", "ts", "tsx", "js", "jsx", "go", "java", "c", "cpp", "h", "hpp", "php",
                "zig", "css",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            max_sig: 110,
            intro_file: None,
            json: false,
        }
    }
}

pub(crate) fn git_facts(root: &Path) -> (String, bool) {
    let rev = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "no-git".to_string());
    let dirty = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .ok()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    (rev, dirty)
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct FileIndex {
    pub(crate) path: String,
    pub(crate) lines: usize,
    pub(crate) items: Vec<IndexItem>,
}

/// Canonical stored/displayed path: POSIX separators, relative to the index
/// root — the shared identity contract for the direct path, the DB rows and
/// both render paths (review decision #11).
pub(crate) fn canonical_rel_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Render the final document (JSON or markdown). Shared by the direct path
/// and the DB projection — byte parity modulo stamp lines is a tested
/// invariant (review decision #5). `notices` become trailing warning lines
/// (DB recovery, root normalization); the direct path passes none.
pub(crate) fn render_doc(
    indexed: &[FileIndex],
    warned_files: &[String],
    rev: &str,
    dirty: bool,
    opts: &CodeIndexOptions,
    root: &Path,
    notices: &[String],
) -> anyhow::Result<String> {
    if opts.json {
        let files: Vec<FileIndex> = indexed
            .iter()
            .map(|f| FileIndex {
                path: f.path.clone(),
                lines: f.lines,
                items: f
                    .items
                    .iter()
                    .map(|i| render_item(i, opts.max_sig))
                    .collect(),
            })
            .collect();
        let doc = serde_json::json!({
            "generator": format!("gnawtreewriter {}", env!("CARGO_PKG_VERSION")),
            "git_rev": rev,
            "git_dirty": dirty,
            "generated_at_utc": chrono::Utc::now().to_rfc3339(),
            "files": files,
        });
        return Ok(serde_json::to_string_pretty(&doc)?);
    }

    // ── Markdown assembly ──
    let (intros, unknown_intros) = match &opts.intro_file {
        Some(f) => {
            let text = std::fs::read_to_string(f)
                .with_context(|| format!("reading intro file {}", f.display()))?;
            parse_intros(&text)
        }
        None => (BTreeMap::new(), Vec::new()),
    };

    let mut out = String::new();
    let root_name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| root.to_string_lossy().to_string());
    out.push_str(&format!("# CODE-INDEX — {root_name}\n\n"));
    out.push_str(&format!(
        "> Genererad {} UTC · gnawtreewriter {} · git `{}`{}\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M"),
        env!("CARGO_PKG_VERSION"),
        rev,
        if dirty { " +DIRTY" } else { "" }
    ));
    out.push_str(
        "> Återskapa: `gnawtreewriter code-index <root> -o code-index.md --intro <introfil>`\n",
    );
    out.push_str("> Inaktuellt om `git`-rev ≠ HEAD — kör om kommandot, redigera ALDRIG det genererade innehållet för hand.\n> Merge-konflikt i code-index.md: tag valfri sida och kör om verktyget — hand-merga aldrig (P3).\n\n");

    // Tree section — directories that contain included files.
    out.push_str("## Träd\n\n```\n");
    let mut by_dir: BTreeMap<String, Vec<&FileIndex>> = BTreeMap::new();
    for f in indexed {
        let dir = match f.path.rfind('/') {
            Some(i) => f.path[..i].to_string(),
            None => ".".to_string(),
        };
        by_dir.entry(dir).or_default().push(f);
    }
    for (dir, fs) in &by_dir {
        out.push_str(&format!("{dir}/\n"));
        for f in fs {
            let name = f.path.rsplit('/').next().unwrap_or(&f.path);
            out.push_str(&format!("  {name} ({} rader)\n", f.lines));
        }
    }
    out.push_str("```\n\n");

    // Sections per directory, intros spliced. The intro FILE is the only
    // intro source (review decision #2) — no DB copy exists to diverge.
    for (dir, fs) in &by_dir {
        out.push_str(&format!("## {dir}/\n\n"));
        if let Some(intro) = intros.get(dir) {
            out.push_str(intro);
            out.push_str("\n\n");
        }
        for f in fs {
            let name = f.path.rsplit('/').next().unwrap_or(&f.path);
            out.push_str(&format!("### {name} ({} rader)\n", f.lines));
            if f.items.is_empty() {
                out.push_str("_(inga toppnivå-items)_\n");
            }
            for item in &f.items {
                out.push_str(&format!(
                    "- `{}` :{}\n",
                    truncate_sig(&item.sig, opts.max_sig),
                    item.line
                ));
                for m in &item.methods {
                    out.push_str(&format!(
                        "  - `{}` :{}\n",
                        truncate_sig(&m.sig, opts.max_sig),
                        m.line
                    ));
                }
            }
            out.push('\n');
        }
    }

    if !unknown_intros.is_empty() {
        out.push_str(&format!(
            "> ⚠ intros utan sektion (stale/typo — ingen effekt): {}\n",
            unknown_intros.join(", ")
        ));
    }
    if !warned_files.is_empty() {
        out.push_str(&format!(
            "> ⚠ {} fil(er) med syntaxvarning (partiell AST, items kan saknas):\n",
            warned_files.len()
        ));
        for w in warned_files {
            out.push_str(&format!(">   - {w}\n"));
        }
    }
    for n in notices {
        out.push_str(&format!("> ⚠ {n}\n"));
    }
    Ok(out)
}

/// One item as rendered in JSON: presentation truncation applied, matching
/// the markdown path (pre-Phase-1 --json behavior preserved).
fn render_item(i: &IndexItem, max_sig: usize) -> IndexItem {
    IndexItem {
        category: i.category.clone(),
        name: i.name.clone(),
        sig: truncate_sig(&i.sig, max_sig),
        line: i.line,
        node_path: i.node_path.clone(),
        methods: i.methods.iter().map(|m| render_item(m, max_sig)).collect(),
    }
}

/// Generate the code index for `root`. Fails ONLY if no source files were
/// found at all — an index that silently renders empty is worse than an error.
pub fn generate_index(root: &Path, opts: &CodeIndexOptions) -> anyhow::Result<String> {
    let exts: Vec<&str> = opts.extensions.iter().map(|s| s.as_str()).collect();
    let mut files = crate::core::file_walker::walk_source_files_filtered(root, &exts);
    if files.is_empty() {
        anyhow::bail!(
            "no source files found under {} (extensions: {}) — wrong root?",
            root.display(),
            exts.join(",")
        );
    }
    files.sort();

    let mut indexed: Vec<FileIndex> = Vec::new();
    let mut warned_files: Vec<String> = Vec::new();
    for path in &files {
        let source =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let parser = get_parser(path)?;
        // Lenient parse: a localized syntax error must not kill the whole
        // file's index (READ path — partial tree + warning beats nothing).
        let (res, warning) = parser.parse_lenient(&source);
        let tree = res.with_context(|| format!("parsing {}", path.display()))?;
        if let Some(w) = warning {
            warned_files.push(format!(
                "{} ({} @ line {})",
                canonical_rel_path(root, path),
                w.message,
                w.line
            ));
        }
        let lines = source.lines().count();
        let items = extract_items(&tree);
        indexed.push(FileIndex {
            path: canonical_rel_path(root, path),
            lines,
            items,
        });
    }
    // Deterministic order across paths: both render paths (direct + DB)
    // must agree byte-for-byte; string sort is the shared contract.
    indexed.sort_by(|a, b| a.path.cmp(&b.path));

    let (rev, dirty) = git_facts(root);
    render_doc(&indexed, &warned_files, &rev, dirty, opts, root, &[])
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const RUST_SAMPLE: &str = r#"//! Module doc.
use std::fmt;

/// A config thing.
#[derive(Clone)]
pub struct Motor { pub port: u16 }

pub enum Mode { Fast, Slow }

impl Motor {
    pub fn new(port: u16) -> Self { Self { port } }
    fn hidden(&self) -> u16 { self.port }
}

mod tests {
    fn this_should_not_appear() {}
}

pub async fn run_tick(a: u8) -> u8 { a }
"#;

    fn parse_sample(code: &str) -> TreeNode {
        let parser = crate::parser::get_parser(Path::new("x.rs")).unwrap();
        parser.parse_lenient(code).0.unwrap()
    }

    #[test]
    fn extracts_top_level_items_and_impl_methods() {
        let tree = parse_sample(RUST_SAMPLE);
        let items = extract_items(&tree);
        let cats: Vec<&str> = items.iter().map(|i| i.category.as_str()).collect();
        assert!(cats.contains(&"struct"), "struct saknas: {cats:?}");
        assert!(cats.contains(&"enum"), "enum saknas: {cats:?}");
        assert!(cats.contains(&"impl"), "impl saknas: {cats:?}");
        assert!(cats.contains(&"fn"), "topp-fn saknas: {cats:?}");
        // mod tests must NOT yield its inner fn — no item named after it recursed.
        assert!(
            !items.iter().any(|i| {
                i.sig.contains("this_should_not_appear") || i.name == "this_should_not_appear"
            }),
            "tests-mod läckte in i indexet"
        );
        // Names and dot-paths are captured (the consumer revalidation key).
        let st = items.iter().find(|i| i.category == "struct").unwrap();
        assert_eq!(st.name, "Motor");
        assert!(!st.node_path.is_empty(), "node_path saknas");
        // impl carries its two methods.
        let imp = items.iter().find(|i| i.category == "impl").unwrap();
        assert_eq!(imp.methods.len(), 2, "impl-metoder: {:?}", imp.methods);
        assert!(imp.methods.iter().any(|m| m.sig.contains("hidden")));
        assert!(imp.methods.iter().all(|m| !m.node_path.is_empty()));
    }

    #[test]
    fn signatures_truncate_at_limit() {
        let tree = parse_sample(RUST_SAMPLE);
        let items = extract_items(&tree);
        // run_tick lives at line 19 of the sample — stored signatures are
        // NOT presentation-truncated; verify truncation at the render step.
        let f = items
            .iter()
            .find(|i| i.category == "fn" && i.line == 19)
            .unwrap();
        assert!(f.sig.contains("run_tick"));
        assert!(
            !f.sig.ends_with('…'),
            "lagring ska inte trunkera: {}",
            f.sig
        );
        let t = truncate_sig(&f.sig, 20);
        assert!(t.chars().count() <= 21, "truncering: {t}");
        assert!(t.ends_with('…'), "trunkeringsmarkering: {t}");
        assert_eq!(truncate_sig("short", 20), "short");
    }

    #[test]
    fn intro_blocks_parse_and_flag_unknown() {
        let text = "<!-- intro: crates/a -->\nA-text\n<!-- /intro -->\nmellan\n<!-- intro: crates/b -->\nB\n<!-- /intro -->";
        let (map, unknown) = parse_intros(text);
        assert_eq!(map.get("crates/a").map(String::as_str), Some("A-text"));
        assert_eq!(map.get("crates/b").map(String::as_str), Some("B"));
        assert!(unknown.is_empty());
        let (map2, unknown2) = parse_intros("<!-- intro: stale -->\nX"); // saknar /intro
        assert!(map2.is_empty());
        assert_eq!(
            unknown2,
            vec!["stale".to_string()],
            "okopplat intro ska skrika"
        );
    }

    #[test]
    fn keyword_fallback_matches_grammar_variants() {
        assert_eq!(item_category("function_declaration"), Some("fn"));
        assert_eq!(item_category("class_definition"), Some("class"));
        assert_eq!(item_category("some_new_thing_function_node"), Some("fn"));
        // Bare kinds are tree-sitter KEYWORD TOKENS — must never match.
        assert_eq!(item_category("impl"), None);
        assert_eq!(item_category("fn"), None);
        assert_eq!(item_category("struct"), None);
        assert_eq!(item_category("identifier"), None);
    }
}
