//! island_scan (T4 §3.2, SCAN_TOOLS proposal): pub surfaces that nothing
//! outside their own file refers to. Population comes from the code-index
//! store (`items`, pub declarations), coupling evidence from `relations`
//! (edges whose origin lives in a DIFFERENT file). Findings are ISLAND
//! CANDIDATES, never verdicts - a `pub` item in a library crate is
//! deliberately public API for consumers the store cannot see, so verify
//! against the intended surface before acting (SCAN_TOOLS §2 principle 4).
//!
//! Test/production separation is a hard rule (SCAN_TOOLS §2 principle 5):
//! references from test files and from `#[cfg(test)]` modules are NOT
//! coupling proof - the motivating extensions case had green unit tests
//! calling the hooks directly while production dispatched nothing. The
//! `#[cfg(test)]` verdict is exact, not heuristic: module entities carry
//! their line, so the attribute is read from the source at that anchor.
//!
//! Determinism: same store bytes and same source give the same report.

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

const FINDINGS_CAP: usize = 25;

/// Default population scope: the shipped crate, not examples/tests.
pub const DEFAULT_PREFIX: &str = "src/";

/// One pub surface with no external, production coupling.
#[derive(Debug, Clone, Serialize)]
pub struct IslandFinding {
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: usize,
    pub signature: String,
    /// GTW dot-path captured at extraction - chains into read_node.
    pub node_path: String,
    /// Methods inside this item (impl/class containers): the priority signal.
    pub method_surface: usize,
    pub severity: &'static str,
    pub why: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IslandScanReport {
    /// Pub surfaces considered after test-scope exclusions (the denominator).
    pub population: usize,
    /// Surfaces with coupling proof (a named graph edge or a verbatim mention
    /// in another production file of the scope).
    pub with_external_refs: usize,
    pub findings_total: usize,
    pub findings: Vec<IslandFinding>,
    pub excluded_test_files: usize,
    pub excluded_test_scope: usize,
    pub note: String,
}

/// A test file by path: integration tests, benches, or *_test(s).rs.
fn is_test_file(path: &str) -> bool {
    if path.starts_with("tests/")
        || path.contains("/tests/")
        || path.starts_with("benches/")
        || path.contains("/benches/")
    {
        return true;
    }
    let stem = path.rsplit('/').next().unwrap_or(path);
    let stem = stem.strip_suffix(".rs").unwrap_or(stem);
    stem.ends_with("_test") || stem.ends_with("_tests") || stem == "tests"
}

/// A pub declaration: the token after "pub" is a space or an opening paren.
/// `public` (Java/C#) must NOT pass - "pub" is a prefix of it.
fn is_pub_decl(sig: &str) -> bool {
    match sig.strip_prefix("pub") {
        Some(rest) => rest.is_empty() || rest.starts_with(' ') || rest.starts_with('('),
        None => false,
    }
}

/// Descendant-or-self test in the GTW dot-path space ("46" contains "46.2.1").
fn is_within(node_path: &str, ancestor: &str) -> bool {
    node_path == ancestor
        || (node_path.len() > ancestor.len()
            && node_path.starts_with(ancestor)
            && node_path.as_bytes()[ancestor.len()] == b'.')
}

/// Test-scope module dot-paths per file, read from the SOURCE at the store's
/// line anchor. Exact by construction: tree-sitter puts `#[cfg(test)]` on a
/// sibling attribute node, so the store's signature never contains it.
fn test_module_paths(root: &Path, conn: &Connection) -> Result<HashMap<String, Vec<String>>> {
    let mut stmt =
        conn.prepare("SELECT file, line, node_path FROM entities WHERE kind = 'module'")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    let mut file_lines: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let (file, line, node_path) = row?;
        let lines = file_lines.entry(file.clone()).or_insert_with(|| {
            std::fs::read_to_string(root.join(&file))
                .map(|t| t.lines().map(str::to_string).collect())
                .unwrap_or_default()
        });
        let start_line = (line as usize).saturating_sub(1);
        let from = start_line.saturating_sub(4);
        let to = start_line.min(lines.len());
        if from >= to {
            continue;
        }
        let has_cfg_test = lines[from..to]
            .iter()
            .any(|l| l.replace(' ', "").contains("#[cfg(test)]"));
        if has_cfg_test {
            out.entry(file).or_default().push(node_path);
        }
    }
    Ok(out)
}

/// One relation edge, as much as the scan needs.
struct Edge {
    from_id: String,
    to_name: String,
    to_id: Option<String>,
    relation_type: String,
    file: String,
}

/// One index item row (a declaration in the store).
struct ItemRow {
    id: i64,
    file: String,
    kind: String,
    name: String,
    signature: String,
    line: usize,
    node_path: String,
    parent_id: Option<i64>,
}

/// A pub surface that survived the test-scope filters.
struct Candidate {
    file: String,
    kind: String,
    name: String,
    signature: String,
    line: usize,
    node_path: String,
    method_surface: usize,
}

/// Strip Rust line comments from a source line, leaving string literals alone
/// (an import path never contains `//` inside a string, and a comment can).
fn strip_line_comment(line: &str) -> &str {
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    }
}

/// Does `name` appear as a word in the given source TEXT, other than in a doc
/// comment? SCAN_TOOLS §3.2 is explicit that the INDEX supplies the population
/// and the GREP supplies the evidence, and the store alone cannot: a `use`
/// inside a function body is not extracted at all, and multi-line imports are
/// stored truncated. Word boundaries matter - `Contributor` must not be
/// satisfied by `ContributorKind`.
fn mentioned_as_word(lines: &[String], name: &str) -> bool {
    let bytes = name.as_bytes();
    for line in lines {
        let code = strip_line_comment(line);
        if code.trim_start().starts_with("///") || code.trim_start().starts_with("//!") {
            continue;
        }
        let hay = code.as_bytes();
        if hay.len() < bytes.len() {
            continue;
        }
        let mut from = 0usize;
        while let Some(rel) = find_bytes(&hay[from..], bytes) {
            let at = from + rel;
            let before_ok = at == 0 || !is_word_byte(hay[at - 1]);
            let after = at + bytes.len();
            let after_ok = after >= hay.len() || !is_word_byte(hay[after]);
            if before_ok && after_ok {
                return true;
            }
            from = at + 1;
        }
    }
    false
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn find_bytes(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Coupling = an edge from a DIFFERENT file, in production scope.
fn is_production_origin(
    edge: &Edge,
    node_paths: &HashMap<String, String>,
    test_modules: &HashMap<String, Vec<String>>,
) -> bool {
    // Synthetic callsite ids are not entity rows (index_relations doc): they
    // carry no node_path, so test scope cannot be proven - count them as
    // production and say so in the note (never silently claim test coupling).
    let Some(node_path) = node_paths.get(&edge.from_id) else {
        return true;
    };
    match test_modules.get(&edge.file) {
        Some(paths) => !paths.iter().any(|p| is_within(node_path, p)),
        None => true,
    }
}

pub fn island_scan(
    conn: &Connection,
    root: &Path,
    prefix: Option<&str>,
) -> Result<IslandScanReport> {
    let scope = prefix.unwrap_or(DEFAULT_PREFIX);
    let test_modules = test_module_paths(root, conn)?;

    // Source lines per file, loaded once. The store gives the POPULATION and
    // the test-scope anchors; the coupling evidence is read from the source,
    // because the store cannot carry it faithfully: a `use` inside a function
    // body is never extracted, and a multi-line `use crate::core::{` is stored
    // truncated (both measured 2026-11-19 — they made 70 % of this crate
    // look like islands). SCAN_TOOLS §3.2: index = population, source = proof.
    let mut sources: HashMap<String, Vec<String>> = HashMap::new();
    let mut loaded_files: Vec<String> = Vec::new();
    for r in conn
        .prepare("SELECT DISTINCT file FROM items WHERE file LIKE ?1 || '%'")?
        .query_map([scope], |r| r.get::<_, String>(0))?
    {
        loaded_files.push(r?);
    }
    loaded_files.sort();
    for file in &loaded_files {
        let lines: Vec<String> = std::fs::read_to_string(root.join(file))
            .map(|t| t.lines().map(str::to_string).collect())
            .unwrap_or_default();
        sources.insert(file.clone(), lines);
    }

    // Entity identity map: id -> Scope, for the (file, bare name) join and
    // the test-scope containment check.
    let mut entity_scope: HashMap<String, Scope> = HashMap::new();
    {
        let mut stmt = conn.prepare("SELECT id, file, node_path, name FROM entities")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?;
        for row in rows {
            let (id, file, node_path, name) = row?;
            entity_scope.insert(
                id,
                Scope {
                    file,
                    node_path,
                    name,
                },
            );
        }
    }

    // Graph edges: they prove coupling when they carry a NAME and an origin in
    // another production file. Imports are excluded here on purpose — the
    // source scan below covers them (and covers the ones the store drops).
    let mut edges: Vec<Edge> = Vec::new();
    {
        let mut stmt =
            conn.prepare("SELECT from_id, to_name, to_id, relation_type, file FROM relations")?;
        let rows = stmt.query_map([], |r| {
            Ok(Edge {
                from_id: r.get(0)?,
                to_name: r.get(1)?,
                to_id: r.get(2)?,
                relation_type: r.get(3)?,
                file: r.get(4)?,
            })
        })?;
        for row in rows {
            edges.push(row?);
        }
    }
    let mut by_id: HashMap<String, Vec<usize>> = HashMap::new();
    let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, e) in edges.iter().enumerate() {
        if let Some(id) = &e.to_id {
            by_id.entry(id.clone()).or_default().push(idx);
        }
        if e.relation_type != "imports" {
            by_name.entry(e.to_name.clone()).or_default().push(idx);
        }
    }

    // The scope's items, loaded once: the population AND the method surface
    // come from the same rows. Method surface cannot be a single parent-count:
    // methods hang off the impl item, which is a SIBLING of the struct (a
    // struct candidate's own children are its fields, not its methods).
    let mut rows: Vec<ItemRow> = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT id, file, kind, name, signature, line, node_path, parent_id
             FROM items
             WHERE file LIKE ?1 || '%'
             ORDER BY file, line",
        )?;
        let mapped = stmt.query_map([scope], |r| {
            Ok(ItemRow {
                id: r.get(0)?,
                file: r.get(1)?,
                kind: r.get(2)?,
                name: r.get(3)?,
                signature: r.get(4)?,
                line: r.get::<_, i64>(5)? as usize,
                node_path: r.get(6)?,
                parent_id: r.get(7)?,
            })
        })?;
        for row in mapped {
            rows.push(row?);
        }
    }

    let mut child_count: HashMap<i64, usize> = HashMap::new();
    for r in &rows {
        if let Some(p) = r.parent_id {
            *child_count.entry(p).or_insert(0) += 1;
        }
    }
    let mut impl_methods: HashMap<(String, String), usize> = HashMap::new();
    for r in &rows {
        if r.kind == "impl" {
            let n = child_count.get(&r.id).copied().unwrap_or(0);
            *impl_methods
                .entry((r.file.clone(), r.name.clone()))
                .or_insert(0) += n;
        }
    }

    let mut population = 0usize;
    let mut excluded_test_files = 0usize;
    let mut excluded_test_scope = 0usize;
    let mut candidates: Vec<Candidate> = Vec::new();
    for r in &rows {
        // Top-level surfaces only: a method row is covered by its impl's
        // surface, otherwise every method becomes its own "island".
        if r.parent_id.is_some() || !is_pub_decl(&r.signature) || r.name.is_empty() {
            continue;
        }
        if is_test_file(&r.file) {
            excluded_test_files += 1;
            continue;
        }
        if test_modules
            .get(&r.file)
            .is_some_and(|paths| paths.iter().any(|p| is_within(&r.node_path, p)))
        {
            excluded_test_scope += 1;
            continue;
        }
        population += 1;
        let own_children = child_count.get(&r.id).copied().unwrap_or(0);
        let from_impls = impl_methods
            .get(&(r.file.clone(), r.name.clone()))
            .copied()
            .unwrap_or(0);
        candidates.push(Candidate {
            file: r.file.clone(),
            kind: r.kind.clone(),
            name: r.name.clone(),
            signature: r.signature.clone(),
            line: r.line,
            node_path: r.node_path.clone(),
            method_surface: own_children + from_impls,
        });
    }

    // Entity ids per (file, bare name): a call edge names the bare method while
    // the entity row is qualified (Type::name), so the pair is the join key.
    let mut id_lookup: HashMap<(String, String), Vec<String>> = HashMap::new();
    for (id, s) in &entity_scope {
        id_lookup
            .entry((s.file.clone(), s.name.clone()))
            .or_default()
            .push(id.clone());
    }
    let node_paths: HashMap<String, String> = entity_scope
        .iter()
        .map(|(id, s)| (id.clone(), s.node_path.clone()))
        .collect();

    let mut findings: Vec<IslandFinding> = Vec::new();
    let mut findings_total = 0usize;
    let mut with_external_refs = 0usize;
    for cand in &candidates {
        // 1. Graph proof: a NAMED edge from another production file.
        let candidate_ids = id_lookup
            .get(&(cand.file.clone(), cand.name.clone()))
            .cloned()
            .unwrap_or_default();
        let mut seen: Vec<usize> = Vec::new();
        for id in &candidate_ids {
            if let Some(idxs) = by_id.get(id) {
                seen.extend(idxs.iter().copied());
            }
        }
        if let Some(idxs) = by_name.get(&cand.name) {
            seen.extend(idxs.iter().copied());
        }
        seen.sort_unstable();
        seen.dedup();
        let named = seen
            .into_iter()
            .map(|i| &edges[i])
            .filter(|e| e.file != cand.file)
            .filter(|e| !is_test_file(&e.file))
            .filter(|e| is_production_origin(e, &node_paths, &test_modules))
            .count();
        if named > 0 {
            with_external_refs += 1;
            continue;
        }

        // 2. Source proof: the name appears as a WORD in another production
        //    file of the scope (import, call, type position — all count).
        let in_source = sources.iter().any(|(file, lines)| {
            file != &cand.file && !is_test_file(file) && mentioned_as_word(lines, &cand.name)
        });
        if in_source {
            with_external_refs += 1;
            continue;
        }

        findings_total += 1;
        let severity = if cand.method_surface >= 2
            || matches!(
                cand.kind.as_str(),
                "struct" | "trait" | "enum" | "interface" | "function"
            ) {
            "HIGH"
        } else {
            "MEDIUM"
        };
        if findings.len() < FINDINGS_CAP {
            findings.push(IslandFinding {
                name: cand.name.clone(),
                kind: cand.kind.clone(),
                file: cand.file.clone(),
                line: cand.line,
                signature: cand.signature.clone(),
                node_path: cand.node_path.clone(),
                method_surface: cand.method_surface,
                severity,
                why: format!(
                    "pub {} '{}' has no named graph edge and no verbatim mention in any other production file of the scope ({} method(s) inside) - built but never wired. A lib crate's pub surface MAY be deliberate external API for consumers outside this scope; check the intended surface before acting",
                    cand.kind, cand.name, cand.method_surface
                ),
            });
        }
    }

    // Rank: severity first, then the biggest method surface (a Manager with 8
    // methods without a consumer weighs more than a helper), then name+file
    // for determinism. String-sorting "HIGH"/"MEDIUM" would put MEDIUM first.
    findings.sort_by(|a, b| {
        severity_rank(a.severity)
            .cmp(&severity_rank(b.severity))
            .then_with(|| b.method_surface.cmp(&a.method_surface))
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.file.cmp(&b.file))
    });

    let note = format!(
        "ISLAND CANDIDATES, not verdicts. Coupling is proven two ways: a NAMED graph edge (calls/uses/implements) from another production file, or a VERBATIM WORD mention of the name in another production file of the scope (this subsumes imports, and catches the ones the store drops: a `use` inside a fn body is never extracted and multi-line imports are stored truncated — measured 2026-11-19). Excluded as coupling: {excluded_test_files} test-file item(s) and {excluded_test_scope} #[cfg(test)] item(s). The index sees structure, never bodies — 'called but result discarded' is invisible here, and a lib crate's pub items are deliberate API for consumers outside this scope. Scope '{scope}': {population} pub surface(s), {with_external_refs} coupled, {findings_total} island candidates ({shown} shown, cap {cap}).",
        shown = findings.len(),
        cap = FINDINGS_CAP
    );

    Ok(IslandScanReport {
        population,
        with_external_refs,
        findings_total,
        findings,
        excluded_test_files,
        excluded_test_scope,
        note,
    })
}

/// Entity row identity: file + node_path for test-scope containment, and the
/// bare name for the (file, name) join against call edges. The name is read
/// from the entities row, never parsed out of the id (ids carry the "::"
/// qualifier and the "#N" disambiguation suffix).
struct Scope {
    file: String,
    node_path: String,
    name: String,
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "HIGH" => 0,
        _ => 1,
    }
}

#[cfg(test)]
mod island_scan_tests {
    use super::*;
    use crate::core::code_index::CodeIndexOptions;
    use crate::core::code_index_db::{migrate_connection, refresh};

    fn store(root: &Path) -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrate_connection(&conn).unwrap();
        let opts = CodeIndexOptions::default();
        refresh(&mut conn, root, &opts).unwrap();
        conn
    }

    /// The motivating class (SCAN_TOOLS 3.2, extensions case): a pub item that
    /// nothing outside its own file mentions is an island; the moment ANOTHER
    /// file mentions it, it is not. Measured both ways in one tree.
    #[test]
    fn finds_a_pub_item_nothing_else_mentions_and_spares_used_ones() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("src/orphan.rs"),
            "pub struct NeverWired { pub x: u8 }

pub fn also_never_wired() {}
",
        )
        .unwrap();
        std::fs::write(root.join("src/user.rs"), "pub fn wired() {}\n").unwrap();
        // The caller must live in a DIFFERENT file: the spec says coupling is
        // proven by references OUTSIDE the defining file. A same-file caller is
        // a file-local helper, which IS an island candidate (measured: this
        // fixture once put the caller in user.rs and asserted the wrong side).
        std::fs::write(root.join("src/other.rs"), "pub fn caller() { wired(); }\n").unwrap();

        let conn = store(root);
        let report = island_scan(&conn, root, Some("src/")).unwrap();

        let names: Vec<&str> = report.findings.iter().map(|f| f.name.as_str()).collect();
        assert!(
            names.contains(&"NeverWired"),
            "a struct nothing references must be an island candidate: {report:?}"
        );
        assert!(
            names.contains(&"also_never_wired"),
            "a fn nothing calls must be an island candidate: {report:?}"
        );
        assert!(
            !names.contains(&"wired"),
            "a fn called from another file is NOT an island: {report:?}"
        );
        assert!(
            report.population >= 3,
            "denominator must count the pub surface: {report:?}"
        );
    }

    /// Test/production separation (SCAN_TOOLS 2 principle 5): references from a
    /// TEST FILE do not prove production coupling. This is the extensions case
    /// exactly - green tests calling the hook directly, nothing in production.
    #[test]
    fn a_test_file_reference_does_not_make_an_island_wired() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("tests")).unwrap();
        std::fs::write(
            root.join("src/hook.rs"),
            "pub fn run_before_tool_call() {}
",
        )
        .unwrap();
        std::fs::write(
            root.join("tests/hook_test.rs"),
            "fn u() { run_before_tool_call(); }
",
        )
        .unwrap();

        let conn = store(root);
        let report = island_scan(&conn, root, Some("src/")).unwrap();
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.name == "run_before_tool_call"),
            "a hook mentioned ONLY by a test file is still an island: {report:?}"
        );
    }

    /// Word boundaries: Contributor must not count as wired just because
    /// ContributorKind appears elsewhere (measured false positive 2026-11-19).
    #[test]
    fn a_longer_identifier_does_not_satisfy_a_short_name() {
        assert!(!mentioned_as_word(
            &["let k = ContributorKind::Unknown;".to_string()],
            "Contributor"
        ));
        assert!(mentioned_as_word(
            &["use x::Contributor;".to_string()],
            "Contributor"
        ));
        assert!(
            mentioned_as_word(&["fn f(c: Contributor) {}".to_string()], "Contributor"),
            "a type position counts as a mention"
        );
        assert!(
            !mentioned_as_word(
                &["/// see Contributor for details".to_string()],
                "Contributor"
            ),
            "a doc comment is prose, not coupling evidence"
        );
    }

    /// M14: the DEFAULT surface must read problems. A wrong prefix finds no pub
    /// items and reports population 0 - it must never look like health.
    #[test]
    fn an_empty_population_is_reported_as_zero_not_as_clean() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("src/lib.rs"),
            "pub fn real() {}
",
        )
        .unwrap();

        let conn = store(root);
        let report = island_scan(&conn, root, Some("nope/")).unwrap();
        assert_eq!(
            report.population, 0,
            "an empty population must be visible as 0, never a vacuous clean run: {report:?}"
        );
    }
}
