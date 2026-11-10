//! code_index coverage (Phase 1 of CODE_INDEX_RELATION_HISTORY_PROPOSAL,
//! 2026-11-09): measure the CURRENT graph stored in `code_index.db` —
//! resolution rate per relation type, D7 source coverage (matched vs
//! explicit synthetic vs unmatched identity classes) and unresolved
//! target classes.
//!
//! READ-ONLY surface: opens the existing store, never creates one
//! (decision #8). This is the store-half of the coverage harness; the
//! fixture-half (extraction recall/precision vs hand-marked facit)
//! measures the EXTRACTOR and lives with the integration tests.
//!
//! Honesty rules (proposal §Täckning före hård grind): `resolution_rate`
//! is resolved/extracted — never claimed as recall; every unmatched
//! source lands in a NAMED class, never a silent drop (D7); macro and
//! qualified-path unresolved targets get their own classes so increment C
//! (method/macro extraction) has a before/after yardstick.

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

/// Resolution quality of ONE relation type. `resolution_rate` is
/// resolved/extracted — a rate over EXTRACTED relations, never recall
/// (method calls etc. that are not extracted at all are not in the
/// denominator — proposal §Nuläge).
#[derive(Serialize)]
pub struct TypeCoverage {
    pub relation_type: String,
    pub extracted: i64,
    pub resolved: i64,
    pub resolution_rate: f64,
}

/// One D7 source-identity class across all relation types. `matched`
/// means `from_id` exists as an entity id. The other classes are
/// explicit and named — unmatched-by-design (synthetic callsites) or
/// unmatched-by-identity-gap (function/impl/file scope, other), which
/// are exactly the classes the extractor repair must shrink.
#[derive(Serialize)]
pub struct SourceClass {
    pub class: String,
    pub count: i64,
}

/// Where relations point: resolved, or one of the named unresolved
/// target classes (`macro!` suffix, qualified path, plain name).
#[derive(Serialize)]
pub struct TargetClass {
    pub class: String,
    pub count: i64,
}

#[derive(Serialize)]
pub struct SymbolSummary {
    pub total: i64,
    pub external: i64,
}

/// Phase 2 prose layer (optional enrichment): the index owns only the
/// FACTS about prose claims — freshness counts + the regenerate
/// recommendation. The prose TEXT lives in prose_annotations; producers
/// (model / agents / background tools) read the queue and rewrite.
#[derive(Serialize)]
pub struct ProseSummary {
    pub fresh: i64,
    pub stale: i64,
    pub invalid: i64,
    /// stale + invalid > 0 → producers should regenerate (a
    /// recommendation surfaced in coverage/doctor — never a gate).
    pub regenerate_recommended: bool,
}

#[derive(Serialize)]
pub struct CoverageReport {
    pub files: i64,
    pub relations_total: i64,
    pub types: Vec<TypeCoverage>,
    pub source_coverage: Vec<SourceClass>,
    pub target_classes: Vec<TargetClass>,
    pub symbols: SymbolSummary,
    /// D9 read-time signals: staleness queue non-empty at measure time.
    pub index_stale: bool,
    /// D9: files indexed but NO completed revision in history — the store
    /// predates the revision chain (or history was lost); every impact /
    /// delta claim must carry this, never silently green.
    pub history_gap: bool,
    pub prose: ProseSummary,
    pub extractor_version: String,
    pub generation_key: Option<String>,
    pub last_refresh_rev: Option<String>,
}

/// Measure graph quality from the store. Deterministic: same DB bytes →
/// same report (the scan-tools determinism principle).
pub fn measure(conn: &Connection) -> Result<CoverageReport> {
    let mut types = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT relation_type, COUNT(*), COALESCE(SUM(to_id IS NOT NULL), 0)
             FROM relations
             GROUP BY relation_type
             ORDER BY COUNT(*) DESC, relation_type",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?;
        for row in rows {
            let (relation_type, extracted, resolved) = row?;
            let rate = if extracted > 0 {
                resolved as f64 / extracted as f64
            } else {
                0.0
            };
            types.push(TypeCoverage {
                relation_type,
                extracted,
                resolved,
                resolution_rate: (rate * 1000.0).round() / 1000.0,
            });
        }
    }

    // D7 source coverage. LEFT JOIN against entity ids — `entities.id`
    // is the PRIMARY KEY, so one relation row maps to at most one class
    // and the GROUP BY totals stay equal to relations_total. `matched`
    // is checked first; the identity classes follow the gtw id format
    // (`gtw:{file}:callsite:{line}` etc., index_relations.rs).
    let mut source_coverage = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT CASE
                WHEN e.id IS NOT NULL THEN 'matched'
                WHEN r.from_id LIKE '%:callsite:%' THEN 'synthetic_callsite'
                WHEN r.from_id LIKE '%:function:%' THEN 'function_scope_unmatched'
                WHEN r.from_id LIKE '%:impl:%' THEN 'impl_scope_unmatched'
                WHEN r.from_id LIKE '%:file:%' THEN 'file_scope_unmatched'
                ELSE 'other_unmatched'
             END AS class,
             COUNT(*)
             FROM relations r
             LEFT JOIN entities e ON e.id = r.from_id
             GROUP BY class
             ORDER BY COUNT(*) DESC, class",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        for row in rows {
            let (class, count) = row?;
            source_coverage.push(SourceClass { class, count });
        }
    }

    // Target classes: macro targets carry a `!` suffix (increment C
    // convention), qualified paths hint at module paths the resolver
    // does not walk yet; the rest are plain names.
    let mut target_classes = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT CASE
                WHEN to_id IS NOT NULL THEN 'resolved'
                WHEN to_name LIKE '%!' THEN 'unresolved_macro'
                WHEN to_name LIKE '%::%' THEN 'unresolved_qualified_path'
                ELSE 'unresolved_plain_name'
             END AS class,
             COUNT(*)
             FROM relations
             GROUP BY class
             ORDER BY COUNT(*) DESC, class",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        for row in rows {
            let (class, count) = row?;
            target_classes.push(TargetClass { class, count });
        }
    }

    let symbols_total: i64 = conn.query_row("SELECT COUNT(*) FROM symbols", [], |r| r.get(0))?;
    let symbols_external: i64 =
        conn.query_row("SELECT COALESCE(SUM(external), 0) FROM symbols", [], |r| {
            r.get(0)
        })?;

    let files: i64 = conn.query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?;

    let meta = |key: &str| -> Result<Option<String>> {
        let mut stmt = conn.prepare("SELECT value FROM meta WHERE key = ?1")?;
        let mut rows = stmt.query_map([key], |r| r.get::<_, String>(0))?;
        Ok(rows.next().transpose()?)
    };

    // D9 read-time signals (phase 2 lock 5): staleness queue non-empty =
    // index_stale; files present but no completed revision = history_gap
    // (the store predates the revision chain, or history was lost).
    let index_stale: bool = conn
        .query_row("SELECT COUNT(*) FROM staleness_queue", [], |r| {
            r.get::<_, i64>(0)
        })
        .map(|c| c > 0)
        .unwrap_or(false);
    let completed_revisions: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM index_revisions WHERE status = 'completed'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0);
    let history_gap = files > 0 && completed_revisions == 0;
    let prose: ProseSummary = {
        let mut stmt =
            conn.prepare("SELECT status, COUNT(*) FROM prose_annotations GROUP BY status")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        let mut fresh = 0i64;
        let mut stale = 0i64;
        let mut invalid = 0i64;
        for row in rows.flatten() {
            match row.0.as_str() {
                "fresh" => fresh = row.1,
                "stale" => stale = row.1,
                "invalid" => invalid = row.1,
                _ => {}
            }
        }
        ProseSummary {
            fresh,
            stale,
            invalid,
            regenerate_recommended: stale + invalid > 0,
        }
    };
    let relations_total: i64 = relations_total(conn)?;
    Ok(CoverageReport {
        files,
        relations_total,
        types,
        source_coverage,
        target_classes,
        symbols: SymbolSummary {
            total: symbols_total,
            external: symbols_external,
        },
        index_stale,
        history_gap,
        prose,
        extractor_version: meta("extractor_version")?.unwrap_or_default(),
        generation_key: meta("generation_key")?,
        last_refresh_rev: meta("last_refresh_rev")?,
    })
}

fn relations_total(conn: &Connection) -> anyhow::Result<i64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM relations", [], |r| r.get(0))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hermetic in-memory store with the REAL schema — one DDL truth
    /// with the file-backed path (lesson #6: never operate on repo-root
    /// state in tests).
    fn mem_store() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        crate::core::code_index_db::migrate_connection(&conn).unwrap();
        conn
    }

    fn seed(conn: &Connection) {
        conn.execute_batch(
            r#"
            INSERT INTO files(path, hash, lines, warning, parsed_at)
              VALUES ('src/a.rs', 'h1', 10, NULL, 1), ('src/b.rs', 'h2', 5, NULL, 1);
            INSERT INTO entities(id, file, kind, name, line, node_path) VALUES
              ('gtw:src/a.rs:function:alpha', 'src/a.rs', 'function', 'alpha', 1, '1'),
              ('gtw:src/a.rs:function:Mod::beta', 'src/a.rs', 'function', 'Mod::beta', 5, '2');
            INSERT INTO relations(from_id, to_name, to_id, relation_type, line, file) VALUES
              -- matched source, resolved target
              ('gtw:src/a.rs:function:alpha', 'alpha', 'gtw:src/a.rs:function:alpha', 'calls', 2, 'src/a.rs'),
              -- identity gap: relation scope uses the bare name, entity id is qualified
              ('gtw:src/a.rs:function:gamma', 'Mod::beta', 'gtw:src/a.rs:function:Mod::beta', 'calls', 6, 'src/a.rs'),
              -- synthetic callsite (unmatched by design)
              ('gtw:src/a.rs:callsite:12', 'cfg!', NULL, 'calls', 12, 'src/a.rs'),
              -- file-scope import
              ('gtw:src/b.rs:file:b', 'std::collections::HashMap', NULL, 'imports', 1, 'src/b.rs');
            INSERT INTO symbols(name, file, external, occurrences)
              VALUES ('cfg!', 'src/a.rs', 1, 1);
            INSERT INTO meta(key, value) VALUES
              ('extractor_version', '1'), ('generation_key', 'gk'), ('last_refresh_rev', 'r9');
            "#,
        )
        .unwrap();
    }

    #[test]
    fn classifies_sources_and_targets_per_d7() {
        let conn = mem_store();
        seed(&conn);
        let report = measure(&conn).unwrap();

        assert_eq!(report.relations_total, 4);
        assert_eq!(report.files, 2);

        let calls = report
            .types
            .iter()
            .find(|t| t.relation_type == "calls")
            .expect("calls row");
        assert_eq!((calls.extracted, calls.resolved), (3, 2));
        assert!((calls.resolution_rate - 0.667).abs() < 1e-6);

        let imports = report
            .types
            .iter()
            .find(|t| t.relation_type == "imports")
            .expect("imports row");
        assert_eq!((imports.extracted, imports.resolved), (1, 0));

        let class = |name: &str| {
            report
                .source_coverage
                .iter()
                .find(|c| c.class == name)
                .map(|c| c.count)
                .unwrap_or(0)
        };
        assert_eq!(class("matched"), 1);
        assert_eq!(class("function_scope_unmatched"), 1);
        assert_eq!(class("synthetic_callsite"), 1);
        assert_eq!(class("file_scope_unmatched"), 1);
        assert_eq!(class("other_unmatched"), 0);
        // No silent drops: the named classes account for every relation.
        let classified: i64 = report.source_coverage.iter().map(|c| c.count).sum();
        assert_eq!(classified, report.relations_total);

        let tclass = |name: &str| {
            report
                .target_classes
                .iter()
                .find(|c| c.class == name)
                .map(|c| c.count)
                .unwrap_or(0)
        };
        assert_eq!(tclass("resolved"), 2);
        assert_eq!(tclass("unresolved_macro"), 1);
        assert_eq!(tclass("unresolved_qualified_path"), 1);
        assert_eq!(tclass("unresolved_plain_name"), 0);

        assert_eq!(report.symbols.total, 1);
        assert_eq!(report.symbols.external, 1);
        assert_eq!(report.extractor_version, "1");
        assert_eq!(report.last_refresh_rev.as_deref(), Some("r9"));
        // D9 pins: seeded store has files but NO completed revisions —
        // history_gap must be true; the queue is empty → not stale.
        assert!(report.history_gap, "files without revision history = gap");
        assert!(!report.index_stale);
    }

    #[test]
    fn empty_store_is_zeros_not_error() {
        let conn = mem_store();
        let report = measure(&conn).unwrap();
        assert_eq!(report.relations_total, 0);
        assert!(report.types.is_empty());
        assert!(report.source_coverage.is_empty());
        assert_eq!(report.files, 0);
        assert_eq!(report.symbols.total, 0);
        // D9: no files → no gap, no staleness.
        assert!(!report.history_gap);
        assert!(!report.index_stale);
    }

    #[test]
    fn share_of_classes_matches_total_on_real_shapes() {
        // Aggregation invariant: every relation row lands in exactly one
        // target class as well (same no-silent-drop rule as D7 sources).
        let conn = mem_store();
        seed(&conn);
        let report = measure(&conn).unwrap();
        let classified: i64 = report.target_classes.iter().map(|c| c.count).sum();
        assert_eq!(classified, report.relations_total);
    }
}
