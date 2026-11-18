//! claim_scan (T4 §3.1, SCAN_TOOLS proposal): prose claims against store
//! facts. Extracts backticked identifiers and parenthesized enumerations
//! (>= 2 commas) from doc comments, then joins them against the store's
//! truth sets: entity names, item names, relation types, symbol names and
//! file paths. Findings are LIE-CANDIDATES, never verdicts (SCAN_TOOLS
//! §8.4): a backtick can name a parameter, a command or a config key -
//! verify with grep before judging. Deterministic: same store bytes and
//! same sources give the same report.

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const FINDINGS_CAP: usize = 25;

/// One unverified claim, anchored to file:line (a finding without an anchor
/// is a guess, SCAN_TOOLS §2 principle 6).
#[derive(Debug, Clone, Serialize)]
pub struct ClaimFinding {
    pub identifier: String,
    pub file: String,
    pub line: usize,
    /// HIGH: qualified path (contains ::); MEDIUM: everything else. Rank,
    /// don't bury: the consumer reviews, the scan suspects.
    pub severity: &'static str,
    pub claim_kind: &'static str,
    pub why: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaimScanReport {
    pub sources_scanned: usize,
    pub claims_total: usize,
    pub verified: usize,
    pub findings: Vec<ClaimFinding>,
    pub findings_total: usize,
    pub note: String,
}

fn is_identifier_shaped(s: &str) -> bool {
    if s.len() < 3 {
        return false;
    }
    let first = s.chars().next().unwrap();
    (first.is_ascii_alphabetic() || first == '_')
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
}

/// Backticked spans + parenthesized enumerations (only when the paren
/// contains at least 2 commas - an explicit enumeration, not prose noise
/// like "(review 2026-11-02)").
fn extract_claims(line: &str) -> Vec<(String, &'static str)> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find(BT) {
        let after = &rest[start + 1..];
        match after.find(BT) {
            Some(end) => {
                let candidate = &after[..end];
                if is_identifier_shaped(candidate) {
                    let severity = if candidate.contains("::") {
                        "HIGH"
                    } else {
                        "MEDIUM"
                    };
                    out.push((candidate.to_string(), severity));
                }
                rest = &after[end + 1..];
            }
            None => break,
        }
    }
    // Parenthesized enumerations: (a, b, c) with >= 2 commas.
    let mut search = line;
    while let Some(open) = search.find('(') {
        match search[open..].find(')') {
            Some(close_rel) => {
                let close = open + close_rel;
                let inner = &search[open + 1..close];
                if inner.matches(',').count() >= 2 {
                    for token in inner.split([',', '/']) {
                        let token = token.trim();
                        if is_identifier_shaped(token) {
                            out.push((token.to_string(), "MEDIUM"));
                        }
                    }
                }
                search = &search[close + 1..];
            }
            None => break,
        }
    }
    out
}

/// The ASCII backtick, named so the extraction code stays readable.
const BT: char = '`';

fn load_truth_sets(conn: &Connection) -> Result<HashSet<String>> {
    let mut truth: HashSet<String> = HashSet::new();
    let queries = [
        "SELECT DISTINCT name FROM entities",
        "SELECT DISTINCT name FROM items",
        "SELECT DISTINCT relation_type FROM relations",
        "SELECT DISTINCT name FROM symbols",
    ];
    for sql in queries {
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        for row in rows {
            truth.insert(row?);
        }
    }
    // File paths verify by full path AND by stem (docs.rs / docs).
    let mut stmt = conn.prepare("SELECT DISTINCT path FROM files")?;
    let paths: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    for path in paths {
        truth.insert(path.clone());
        if let Some(stem) = path.rsplit_once('.').map(|(base, _)| base.to_string()) {
            truth.insert(stem);
        }
    }
    Ok(truth)
}

fn verified(identifier: &str, truth: &HashSet<String>) -> bool {
    if truth.contains(identifier) {
        return true;
    }
    // Qualified paths verify by their LAST segment (Foo::new verifies
    // against an entity named `new`).
    identifier
        .rsplit("::")
        .next()
        .map(|tail| truth.contains(tail))
        .unwrap_or(false)
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "HIGH" => 0,
        _ => 1,
    }
}

pub fn claim_scan(conn: &Connection, root: &Path, files: &[String]) -> Result<ClaimScanReport> {
    let truth = load_truth_sets(conn)?;
    let resolved: Vec<String> = if files.is_empty() {
        ["src/lib.rs", "src/main.rs", "lib.rs", "main.rs"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    } else {
        files.to_vec()
    };
    let mut sources_scanned = 0usize;
    let mut claims_total = 0usize;
    let mut verified_count = 0usize;
    let mut findings: Vec<ClaimFinding> = Vec::new();
    let mut findings_total = 0usize;
    for rel in &resolved {
        let path = root.join(rel);
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        sources_scanned += 1;
        for (idx, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            let is_doc = trimmed.starts_with("///") || trimmed.starts_with("//!");
            if !is_doc {
                continue;
            }
            for (identifier, severity) in extract_claims(line) {
                claims_total += 1;
                if verified(&identifier, &truth) {
                    verified_count += 1;
                    continue;
                }
                findings_total += 1;
                let finding = ClaimFinding {
                    identifier: identifier.clone(),
                    file: rel.clone(),
                    line: idx + 1,
                    severity,
                    claim_kind: "claim without fact",
                    why: format!(
                        "doc comment names '{identifier}' but no entity, item, relation type, symbol or file in the store matches it - a doc lie-candidate, a parameter name, or prose noise (grep to disambiguate)"
                    ),
                };
                if findings.len() < FINDINGS_CAP {
                    findings.push(finding);
                }
            }
        }
    }
    findings.sort_by(|a, b| {
        severity_rank(a.severity)
            .cmp(&severity_rank(b.severity))
            .then_with(|| a.identifier.cmp(&b.identifier))
    });
    let note = format!(
        "Lie-CANDIDATES, not verdicts: a backtick or enumeration token can name a parameter, command or config key the store does not track ({} of {} findings shown, cap {}). Verify with grep before judging. Truth sets: entities, items, relation types, symbols, file paths.",
        findings.len(),
        findings_total,
        FINDINGS_CAP
    );
    Ok(ClaimScanReport {
        sources_scanned,
        claims_total,
        verified: verified_count,
        findings,
        findings_total,
        note,
    })
}

/// Default sources: the crate-root doc files where the header claims live
/// (the motivating case was a lib.rs doc-header lie).
pub fn default_source_files(root: &Path) -> Vec<PathBuf> {
    ["src/lib.rs", "src/main.rs", "lib.rs", "main.rs"]
        .iter()
        .map(|s| root.join(s))
        .filter(|p| p.is_file())
        .collect()
}

#[cfg(test)]
mod claim_scan_tests {
    use super::*;
    use crate::core::code_index::CodeIndexOptions;
    use crate::core::code_index_db::{migrate_connection, refresh};

    /// Hermetic end to end: a doc-header lie (backtick + enumeration) is
    /// found with its anchor, a verified claim is not, and the enumeration
    /// line with fewer than 2 commas is skipped as prose noise.
    #[test]
    fn finds_doc_lies_and_verifies_real_names() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("hub.rs"), "pub fn hot_target() {}\n").unwrap();
        std::fs::write(root.join("user.rs"), "fn caller() { hot_target(); }\n").unwrap();
        std::fs::write(
            root.join("docs.rs"),
            "//! Real names: `hot_target` and (calls, uses) work.\n\
             //! Lies: `missing_fn` and (calls, defines, contains) do not.\n\
             //! Prose noise: (review 2026-11-02) skipped.\n",
        )
        .unwrap();

        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrate_connection(&conn).unwrap();
        let opts = CodeIndexOptions::default();
        refresh(&mut conn, root, &opts).unwrap();

        let report = claim_scan(&conn, root, &["docs.rs".to_string()]).unwrap();
        assert_eq!(report.sources_scanned, 1);
        let ids: Vec<&str> = report
            .findings
            .iter()
            .map(|f| f.identifier.as_str())
            .collect();
        assert!(
            ids.contains(&"missing_fn"),
            "the backticked lie must be found: {ids:?}"
        );
        assert!(
            ids.contains(&"defines") && ids.contains(&"contains"),
            "enumeration lies must be found (N8-class): {ids:?}"
        );
        assert!(
            !ids.contains(&"hot_target"),
            "a verified name must NOT be a finding: {ids:?}"
        );
        assert!(
            !ids.contains(&"calls") && !ids.contains(&"uses"),
            "real relation types verify: {ids:?}"
        );
        assert!(
            !ids.contains(&"review"),
            "a paren with < 2 commas is prose noise, skipped: {ids:?}"
        );
        // Rank order: severity_rank non-decreasing (HIGH before MEDIUM).
        let ranks: Vec<u8> = report
            .findings
            .iter()
            .map(|f| severity_rank(f.severity))
            .collect();
        assert!(
            ranks.windows(2).all(|w| w[0] <= w[1]),
            "findings must be rank-sorted: {ranks:?}"
        );
        assert!(report.findings[0].file.contains("docs.rs"));
    }
}
