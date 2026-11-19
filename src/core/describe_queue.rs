//! `describe_queue`: which files are worth describing FIRST (spec §5 Nivå 3).
//!
//! Level 1 of the prose layer can describe any file, but coverage starts at
//! zero and a full pass costs ~10–90 s per file on CPU. Without an order,
//! "describe more files" is not an actionable instruction — so this ranks the
//! work instead of guessing it.
//!
//! **Determinism is the contract** (SCAN_TOOLS §2 principle 3): same store +
//! same inputs = same order. No model participates in ranking, so a large and
//! a small model see the same facts — they only differ in what they SAY about a
//! file, never in which file matters.
//!
//! Signals, all already in the store (no new tables, consolidation rule):
//!
//! | Signal | Meaning | Why it earns a slot |
//! |---|---|---|
//! | `relations_out` | edges leaving the file | describes how much it DOES |
//! | `relations_in` | edges arriving from other files | describes how much depends on it |
//! | `entities` | definitions in the file | the surface a reader must understand |
//! | `has_claim` / `claim_stale` | prose lifecycle | stale = regeneration is due now |
//!
//! Ranking is a **sum of normalised ranks**, not a weighted score: weights
//! would be invented numbers wearing a measurement's clothes (M7). Every
//! component is reported per file, so a reader can disagree with a position on
//! its inputs rather than on an opaque total.
//!
//! Test/production separation carries over from island_scan: test files are
//! ranked last by a named flag, never silently, because "describe the tests"
//! is almost never the intent of a coverage pass.

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;

/// Ordering is LEXICOGRAPHIC over declared priorities, not a weighted sum.
///
/// A sum of normalised ranks is symmetric: "many depend on me" and "I call many"
/// cancel out, so a hub and a caller tie (measured 2026-11-20, the test that
/// caught it). For a description pass the dependency DIRECTION matters, so the
/// priority is declared instead of implied:
///
///   1. regeneration is due (stale claim)  — work already started
///   2. incoming edges                     — how much depends on this file
///   3. outgoing edges                     — how much this file does
///   4. entities                           — how much surface it exposes
///   5. production before test             — named flag, never silent
///
/// Lexicographic priority is not a weight: nothing is multiplied, nothing is
/// traded off implicitly, and each level is visible in the row.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct OrderKey {
    not_stale: i64,
    is_test: i64,
    in_rank: i64,
    out_rank: i64,
    ent_rank: i64,
    path: String,
}

const QUEUE_CAP: usize = 40;

/// Default population scope: the shipped crate, not examples/tests.
pub const DEFAULT_PREFIX: &str = "src/";

/// Why a file landed where it did — every component is visible.
#[derive(Debug, Clone, Serialize, Default)]
pub struct QueueRow {
    pub path: String,
    pub lines: i64,
    /// Edges whose ORIGIN is this file.
    pub relations_out: i64,
    /// Edges from OTHER files whose resolved target lives here.
    pub relations_in: i64,
    pub entities: i64,
    /// A fresh claim already exists (nothing to do).
    pub has_fresh_claim: bool,
    /// A claim exists but its anchor moved — regeneration is due now.
    pub claim_stale: bool,
    /// Test file: ranked last, with the reason stated.
    pub is_test: bool,
    /// Lexicographic order key, rendered as a comparable integer for sorting
    /// and for the `rank` field consumers already read. Lower = describe sooner.
    pub rank: i64,
    /// Human-readable ordering evidence, e.g. "in:3/202 out:1/202".
    pub why: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueReport {
    /// Files considered (the population the ranks are normalised over).
    pub population: i64,
    /// Files already carrying a fresh claim — excluded from the queue.
    pub covered: i64,
    pub queue: Vec<QueueRow>,
    /// Queue length after the cap.
    pub returned: usize,
    /// Honest cap accounting: matched is counted BEFORE the cap.
    pub matched: i64,
    pub truncated: bool,
    pub scope: String,
    pub note: String,
}

/// Parse `#[cfg(test)]`-adjacent signals from the path alone. A path is a
/// weak signal, so it only ever demotes; the AST verdict would need the source
/// walk island_scan already does, and a wrong promotion is worse than a
/// demotion.
fn path_looks_like_test(path: &str) -> bool {
    let p = path.to_ascii_lowercase();
    p.starts_with("tests/")
        || p.contains("/tests/")
        || p.ends_with("_test.rs")
        || p.ends_with("_tests.rs")
        || p.starts_with("test_")
        || p.contains("/test_")
}

/// Rank candidates for description. Pure over the store; never writes.
pub fn describe_queue(
    conn: &Connection,
    prefix: Option<&str>,
    limit: Option<usize>,
) -> Result<QueueReport> {
    let scope = prefix.unwrap_or(DEFAULT_PREFIX).to_string();
    let like = format!("{scope}%");

    // Population + per-file input counts in ONE pass each. `files` is the
    // population; a file with no entities is still real and still describable.
    let mut files: Vec<(String, i64)> = Vec::new();
    {
        let mut stmt =
            conn.prepare("SELECT path, lines FROM files WHERE path LIKE ?1 ORDER BY path")?;
        let rows = stmt.query_map([&like], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?;
        for row in rows.flatten() {
            files.push(row);
        }
    }
    if files.is_empty() {
        anyhow::bail!(
            "describe_queue: 0 files under prefix {scope:?} — wrong prefix, or the store \
             was built from a different tree (run code_index mode=refresh)"
        );
    }

    let mut out_edges: HashMap<String, i64> = HashMap::new();
    {
        let mut stmt =
            conn.prepare("SELECT file, COUNT(*) FROM relations WHERE file LIKE ?1 GROUP BY file")?;
        let rows = stmt.query_map([&like], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?;
        for (f, n) in rows.flatten() {
            out_edges.insert(f, n);
        }
    }

    // In-edges: a relation points at `to_id`, an entity id; the entity's file
    // is where the dependency lands. Counted only for edges whose ORIGIN is a
    // different file (a self-reference is not a dependency).
    let mut in_edges: HashMap<String, i64> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT e.file, COUNT(*) FROM relations r
             JOIN entities e ON e.id = r.to_id
             WHERE r.file <> e.file AND e.file LIKE ?1
             GROUP BY e.file",
        )?;
        let rows = stmt.query_map([&like], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?;
        for (f, n) in rows.flatten() {
            in_edges.insert(f, n);
        }
    }

    let mut entity_counts: HashMap<String, i64> = HashMap::new();
    {
        let mut stmt =
            conn.prepare("SELECT file, COUNT(*) FROM entities WHERE file LIKE ?1 GROUP BY file")?;
        let rows = stmt.query_map([&like], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?;
        for (f, n) in rows.flatten() {
            entity_counts.insert(f, n);
        }
    }

    // Claim state per anchor_key.
    let mut fresh: HashMap<String, bool> = HashMap::new();
    let mut stale: HashMap<String, bool> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT anchor_key, status FROM prose_annotations \
             WHERE anchor_kind = 'file' AND status IN ('fresh','stale')",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for (k, st) in rows.flatten() {
            if st == "fresh" {
                fresh.insert(k, true);
            } else {
                stale.insert(k, true);
            }
        }
    }

    // Normalised ranks: 0 (most) .. n (least). Ties keep path order, so the
    // whole ordering is deterministic.
    //
    // A value ABSENT from the list means "this file has none of this signal",
    // which must rank WORST (len), not best: `unwrap_or(0)` would silently
    // promote every zero-signal file to the front (caught by
    // `in_edges_outrank_out_edges`, 2026-11-20).
    let rank_of = |value: i64, sorted_desc: &[i64]| -> i64 {
        sorted_desc
            .iter()
            .position(|&v| v <= value)
            .unwrap_or(sorted_desc.len()) as i64
    };

    let mut sorted_out: Vec<i64> = out_edges.values().copied().collect();
    sorted_out.sort_unstable_by(|a, b| b.cmp(a));
    let mut sorted_in: Vec<i64> = in_edges.values().copied().collect();
    sorted_in.sort_unstable_by(|a, b| b.cmp(a));
    let mut sorted_ent: Vec<i64> = entity_counts.values().copied().collect();
    sorted_ent.sort_unstable_by(|a, b| b.cmp(a));

    let population = files.len() as i64;
    let mut covered = 0i64;
    let mut rows: Vec<QueueRow> = Vec::new();
    let mut keys: Vec<(OrderKey, usize)> = Vec::new();

    for (path, lines) in &files {
        let has_fresh = fresh.get(path).copied().unwrap_or(false);
        let is_stale = stale.get(path).copied().unwrap_or(false);
        if has_fresh {
            covered += 1;
            continue; // nothing to do: it already carries a current claim
        }
        let ro = out_edges.get(path).copied().unwrap_or(0);
        let ri = in_edges.get(path).copied().unwrap_or(0);
        let en = entity_counts.get(path).copied().unwrap_or(0);
        let is_test = path_looks_like_test(path);

        let key = OrderKey {
            // Ascending sort: the value that should come FIRST is the smaller
            // one. Work already started (stale) comes first, so it is 0.
            not_stale: if is_stale { 0 } else { 1 },
            is_test: if is_test { 1 } else { 0 },
            in_rank: rank_of(ri, &sorted_in),
            out_rank: rank_of(ro, &sorted_out),
            ent_rank: rank_of(en, &sorted_ent),
            path: path.clone(),
        };

        rows.push(QueueRow {
            path: path.clone(),
            lines: *lines,
            relations_out: ro,
            relations_in: ri,
            entities: en,
            has_fresh_claim: false,
            claim_stale: is_stale,
            is_test,
            rank: 0, // filled after sorting
            why: format!(
                "in:{ri} out:{ro} entities:{en}{}{}",
                if is_stale { " STALE" } else { "" },
                if is_test { " test" } else { "" }
            ),
        });
        keys.push((key, rows.len() - 1));
    }

    keys.sort();
    let ordered: Vec<QueueRow> = keys
        .into_iter()
        .enumerate()
        .map(|(pos, (_, idx))| {
            let mut row = rows[idx].clone();
            row.rank = pos as i64;
            row
        })
        .collect();
    let mut rows = ordered;
    let matched = rows.len() as i64;
    let cap = limit.unwrap_or(QUEUE_CAP).min(matched as usize);
    let truncated = matched as usize > cap;
    rows.truncate(cap);

    let note = format!(
        "Ordering is lexicographic over declared priorities, not a weighted score: \
         (1) a stale claim sorts first because regeneration is due now, (2) incoming edges — \
         how much depends on the file, (3) outgoing edges, (4) entities, (5) production before \
         test. Each level is visible in `why`, so a position can be disputed on its inputs. \
         No model participates, so a large and a small model see the same order. Returns at \
         most {QUEUE_CAP} rows by default (matched={matched}, returned={cap}). Optional \
         enrichment: a file with no claim is reported, never blocking."
    );

    Ok(QueueReport {
        population,
        covered,
        returned: rows.len(),
        matched,
        truncated,
        scope,
        note,
        queue: rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::core::code_index_db::migrate_connection(&conn).unwrap();
        conn
    }

    fn file(conn: &Connection, path: &str, lines: i64) {
        conn.execute(
            "INSERT INTO files (path, hash, lines, warning, parsed_at) VALUES (?1,'h',?2,NULL,0)",
            rusqlite::params![path, lines],
        )
        .unwrap();
    }

    fn entity(conn: &Connection, id: &str, file: &str) {
        conn.execute(
            "INSERT INTO entities (id, file, kind, name, line) VALUES (?1,?2,'fn','n',1)",
            rusqlite::params![id, file],
        )
        .unwrap();
    }

    fn relation(conn: &Connection, from: &str, to_id: Option<&str>, file: &str) {
        conn.execute(
            "INSERT INTO relations (from_id, to_name, to_id, relation_type, line, file) \
             VALUES (?1,'t',?2,'calls',1,?3)",
            rusqlite::params![from, to_id, file],
        )
        .unwrap();
    }

    /// A file nothing points at and that points nowhere still appears — the
    /// population must never silently shrink to "interesting" files.
    #[test]
    fn every_file_is_ranked_including_unconnected_ones() {
        let conn = store();
        file(&conn, "src/a.rs", 10);
        file(&conn, "src/lonely.rs", 3);
        let q = describe_queue(&conn, None, None).unwrap();
        assert_eq!(q.population, 2);
        assert_eq!(q.matched, 2, "the lonely file must be in the queue");
    }

    /// The dependency direction that matters: a file many others depend on is
    /// ranked above a file that merely calls out.
    #[test]
    fn in_edges_outrank_out_edges() {
        let conn = store();
        file(&conn, "src/hub.rs", 50);
        file(&conn, "src/caller1.rs", 10);
        file(&conn, "src/caller2.rs", 10);
        entity(&conn, "hub::thing", "src/hub.rs");
        entity(&conn, "c1::go", "src/caller1.rs");
        entity(&conn, "c2::go", "src/caller2.rs");
        // Two files depend on hub; hub depends on nothing.
        relation(&conn, "c1::go", Some("hub::thing"), "src/caller1.rs");
        relation(&conn, "c2::go", Some("hub::thing"), "src/caller2.rs");
        let q = describe_queue(&conn, None, None).unwrap();
        assert_eq!(
            q.queue[0].path,
            "src/hub.rs",
            "the depended-on file must rank first: {:?}",
            q.queue
                .iter()
                .map(|r| (&r.path, r.rank))
                .collect::<Vec<_>>()
        );
        assert_eq!(q.queue[0].relations_in, 2);
        assert_eq!(q.queue[0].relations_out, 0);
    }

    /// A fresh claim takes the file OUT of the queue — that is what makes the
    /// queue shrink as coverage grows.
    #[test]
    fn a_fresh_claim_removes_the_file_from_the_queue() {
        let mut conn = store();
        file(&conn, "src/a.rs", 10);
        file(&conn, "src/b.rs", 10);
        crate::core::code_index_db::upsert_prose(
            &mut conn, "file", "src/a.rs", "h", "src/a.rs", "claim", "t", None, "en", None,
        )
        .unwrap();
        let q = describe_queue(&conn, None, None).unwrap();
        assert_eq!(q.covered, 1);
        assert_eq!(q.matched, 1);
        assert_eq!(q.queue[0].path, "src/b.rs");
    }

    /// Stale sorts first: regeneration is due now, unlike a file nobody has
    /// ever described.
    #[test]
    fn a_stale_claim_sorts_before_undescribed_files() {
        let mut conn = store();
        file(&conn, "src/big_hub.rs", 500);
        file(&conn, "src/stale.rs", 1);
        entity(&conn, "hub::x", "src/big_hub.rs");
        crate::core::code_index_db::upsert_prose(
            &mut conn,
            "file",
            "src/stale.rs",
            "oldhash",
            "src/stale.rs",
            "claim",
            "t",
            None,
            "en",
            None,
        )
        .unwrap();
        conn.execute(
            "UPDATE prose_annotations SET status = 'stale' WHERE anchor_key = 'src/stale.rs'",
            [],
        )
        .unwrap();
        let q = describe_queue(&conn, None, None).unwrap();
        assert_eq!(
            q.queue[0].path, "src/stale.rs",
            "a due regeneration must outrank an undescribed file"
        );
        assert!(q.queue[0].claim_stale);
    }

    /// Test files are demoted by a NAMED flag, and say so — never silently.
    #[test]
    fn test_files_rank_last_and_say_why() {
        let conn = store();
        file(&conn, "src/tests/helper.rs", 5);
        file(&conn, "src/prod.rs", 5);
        let q = describe_queue(&conn, None, None).unwrap();
        assert_eq!(q.queue[0].path, "src/prod.rs");
        assert_eq!(q.queue[1].path, "src/tests/helper.rs");
        assert!(q.queue[1].is_test);
        assert!(
            q.queue[1].why.contains("test"),
            "the demotion must be visible in the row: {}",
            q.queue[1].why
        );
    }

    /// Empty population is a LOUD error, never an empty list that looks fine.
    #[test]
    fn an_empty_population_is_an_error_not_an_empty_queue() {
        let conn = store();
        file(&conn, "other/x.rs", 1);
        let err = describe_queue(&conn, Some("src/"), None).unwrap_err();
        assert!(
            err.to_string().contains("0 files under prefix"),
            "must name the empty population: {err}"
        );
    }

    /// The cap is honest: matched is counted before truncation.
    #[test]
    fn the_cap_reports_matched_and_returned_separately() {
        let conn = store();
        for i in 0..60 {
            file(&conn, &format!("src/f{i:02}.rs"), 1);
        }
        let q = describe_queue(&conn, None, Some(5)).unwrap();
        assert_eq!(q.matched, 60);
        assert_eq!(q.returned, 5);
        assert!(q.truncated);
    }

    /// Determinism: two runs over the same store give the same order.
    #[test]
    fn the_order_is_deterministic() {
        let conn = store();
        for i in 0..20 {
            file(&conn, &format!("src/f{i:02}.rs"), 1 + i);
        }
        let a = describe_queue(&conn, None, None).unwrap();
        let b = describe_queue(&conn, None, None).unwrap();
        let pa: Vec<&str> = a.queue.iter().map(|r| r.path.as_str()).collect();
        let pb: Vec<&str> = b.queue.iter().map(|r| r.path.as_str()).collect();
        assert_eq!(pa, pb);
    }
}
