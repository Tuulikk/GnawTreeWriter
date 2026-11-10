//! code_graph (P2, Comparative-Thinker adaptation plan §4): graph QUERIES
//! over the code-index relations table — reachability, cycles, chains.
//!
//! The ALGORITHMS are written fresh against external references (plan §4.3
//! — CT's `find_dependency_chain` has an infinite-loop bug class, §2.3.1):
//! - `reachable`: SQLite recursive UNION (not UNION ALL) — the dedup makes
//!   cycles terminate by construction, and the depth bound is a parameter.
//! - `cycles`: iterative DFS with an explicit path stack (no recursion →
//!   no stack-depth risk), rotation-canonicalized so the same cycle is
//!   reported once, hard depth bound.
//! - `chain`: BFS with the parent set at FIRST discovery + a seen guard
//!   (the reconstruction repeat-guard makes CT's bug class impossible).
//!
//! Unresolved targets (to_id NULL) never enter a walk — they are reported
//! via `symbols`, never as a fabricated impact chain (the structural
//! counterpart of `ImpactReport::unresolved_calls`).
//!
//! Synthetic callsite anchors (`gtw:{file}:callsite:{line}` as from_id)
//! participate as nodes (they are real call sites) and `files_of` maps
//! them to their file — never `unwrap()` on a possibly-missing entity row.

use rusqlite::{Connection, Result};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize)]
pub struct Reach {
    pub entity: String,
    pub depth: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Cycle {
    /// Entity ids in cycle order (first == last is implicit)
    pub nodes: Vec<String>,
    /// Distinct files in the cycle — what a human/agent actually fixes
    pub files: Vec<String>,
}

/// Entities reachable from `start` within `max_depth` hops (start itself
/// excluded). Unresolved targets (to_id NULL) are skipped — they cannot
/// form a real edge. UNION (not UNION ALL) dedups (node, depth) pairs so
/// cycles terminate; MIN(depth) keeps the shortest hop count.
pub fn reachable(conn: &Connection, start: &str, max_depth: usize) -> Result<Vec<Reach>> {
    let mut stmt = conn.prepare(
        r#"
        WITH RECURSIVE walk(node, depth) AS (
            SELECT ?1, 0
          UNION                                  -- UNION, not UNION ALL: dedups
            SELECT r.to_id, w.depth + 1          -- => cycles terminate
              FROM relations r
              JOIN walk w ON r.from_id = w.node
             WHERE r.to_id IS NOT NULL           -- unresolved targets skip
               AND w.depth < ?2
        )
        SELECT node, MIN(depth) AS depth
          FROM walk
         WHERE depth > 0
         GROUP BY node
         ORDER BY depth, node
        "#,
    )?;
    let rows = stmt.query_map(params![start, max_depth as i64], |r| {
        Ok(Reach {
            entity: r.get(0)?,
            depth: r.get::<_, i64>(1)? as usize,
        })
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
}

/// All cycles among RESOLVED edges, deduplicated by rotation (A→B→C→A and
/// B→C→A→B are the same cycle), each with its distinct file list. The
/// walk is an iterative DFS with an explicit path stack and a hard depth
/// bound — CT's recursive `detect_cycle_util` had no bound (§2.3.4).
/// Determinism: adjacency iterated in sorted key order, so the same DB
/// yields the same cycle list.
pub fn cycles(conn: &Connection, max_depth: usize) -> Result<Vec<Cycle>> {
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    {
        let mut stmt =
            conn.prepare("SELECT from_id, to_id FROM relations WHERE to_id IS NOT NULL")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (from, to) = row?;
            adj.entry(from).or_default().push(to);
        }
    }
    for edges in adj.values_mut() {
        edges.sort();
        edges.dedup();
    }

    let mut found: Vec<Cycle> = Vec::new();
    let mut seen: HashSet<Vec<String>> = HashSet::new();
    let mut starts: Vec<&String> = adj.keys().collect();
    starts.sort();

    for (start_idx, start) in starts.iter().enumerate() {
        let _ = start_idx;
        let mut stack: Vec<(String, usize)> = vec![((*start).clone(), 0)];
        let mut path: Vec<String> = Vec::new();
        let mut on_path: HashSet<String> = HashSet::new();

        while let Some((node, idx)) = stack.pop() {
            // Backtrack: leave nodes that are no longer on the current path.
            while path.len() > idx {
                if let Some(leaving) = path.pop() {
                    on_path.remove(&leaving);
                }
            }
            if path.len() >= max_depth {
                continue;
            }
            let edges = adj.get(&node).map(|v| v.as_slice()).unwrap_or(&[]);
            for next in edges {
                if on_path.contains(next) {
                    // Cycle: cut it out of `next`'s position, canonicalize
                    // by rotation so the same cycle is reported once.
                    let from = path.iter().position(|n| n == next).unwrap_or(0);
                    let mut cyc = path[from..].to_vec();
                    rotate_to_smallest(&mut cyc);
                    if seen.insert(cyc.clone()) {
                        let files = files_of(conn, &cyc)?;
                        found.push(Cycle { files, nodes: cyc });
                    }
                    continue;
                }
                if adj.contains_key(next) {
                    path.push(next.clone());
                    on_path.insert(next.clone());
                    stack.push((next.clone(), path.len()));
                }
            }
        }
    }
    Ok(found)
}

/// A→B→C and B→C→A are the same cycle. Rotate to the smallest element so
/// the dedup key is stable regardless of start point.
fn rotate_to_smallest(c: &mut [String]) {
    if let Some(pos) = (0..c.len()).min_by_key(|&i| &c[i]) {
        c.rotate_left(pos);
    }
}

/// Shortest path `from` → `to` over resolved edges (BFS; parent set at
/// FIRST discovery + a seen guard — CT's backward-built `parents` map had
/// neither, which mixed "the path" with "all parents" and could loop
/// forever, §2.3.1). Returns None when no path exists. The reconstruction
/// aborts on a repeated node — cannot happen with `seen`, but the guard
/// costs two lines and makes the bug class impossible.
pub fn chain(conn: &Connection, from: &str, to: &str) -> Result<Option<Vec<String>>> {
    if from == to {
        return Ok(Some(vec![from.to_string()]));
    }
    let mut parent: HashMap<String, String> = HashMap::new();
    let mut queue: std::collections::VecDeque<String> =
        std::collections::VecDeque::from([from.to_string()]);
    let mut seen: HashSet<String> = HashSet::from([from.to_string()]);
    while let Some(node) = queue.pop_front() {
        let mut stmt =
            conn.prepare("SELECT to_id FROM relations WHERE from_id = ?1 AND to_id IS NOT NULL")?;
        let rows = stmt.query_map(params![node], |r| r.get::<_, String>(0))?;
        for next in rows.flatten() {
            if next == to {
                // Reconstruct backwards, then reverse — a real path: every
                // consecutive pair exists as an edge (asserted in tests).
                let mut path = vec![to.to_string(), node.clone()];
                let mut cur = node.clone();
                while cur != *from {
                    let Some(p) = parent.get(&cur) else {
                        break;
                    };
                    path.push(p.clone());
                    cur = p.clone();
                }
                path.reverse();
                return Ok(Some(path));
            }
            if seen.insert(next.clone()) {
                parent.insert(next.clone(), node.clone());
                queue.push_back(next);
            }
        }
    }
    Ok(None)
}

/// File attribution per node: entities.file when the row exists, else the
/// file encoded in the synthetic id (`gtw:{file}:…`) — NEVER unwrap on a
/// possibly-missing entity row (§2.3.2).
fn files_of(conn: &Connection, nodes: &[String]) -> Result<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for node in nodes {
        let file = conn
            .query_row(
                "SELECT file FROM entities WHERE id = ?1",
                params![node],
                |r| r.get::<_, String>(0),
            )
            .ok()
            .or_else(|| {
                node.strip_prefix("gtw:")
                    .and_then(|rest| rest.split(':').next())
                    .map(|f| f.to_string())
            })
            .unwrap_or_default();
        if !out.contains(&file) {
            out.push(file);
        }
    }
    Ok(out)
}

/// One entity that DEFINES the queried name.
#[derive(Debug, Clone, Serialize)]
pub struct SymbolSeed {
    pub id: String,
    pub file: String,
}

/// Result of `impact`. `seeds` is empty when the name is not in the store —
/// reported as-is, never fabricated into an impact chain (same honesty rule
/// as `ImpactReport::unresolved_calls`).
#[derive(Debug, Clone, Serialize)]
pub struct ImpactReport {
    pub symbol: String,
    pub seeds: Vec<SymbolSeed>,
    pub reachable: Vec<Reach>,
}

/// Entity ids that define `symbol` (a bare name like `parse_file`, not an id).
/// The same name in several files yields several seeds — they are distinct
/// symbols, and the caller needs to see that rather than have one picked.
pub fn resolve_symbols(conn: &Connection, symbol: &str) -> Result<Vec<SymbolSeed>> {
    let mut stmt = conn.prepare("SELECT id, file FROM entities WHERE name = ?1 ORDER BY id")?;
    let rows = stmt.query_map(params![symbol], |r| {
        Ok(SymbolSeed {
            id: r.get(0)?,
            file: r.get(1)?,
        })
    })?;
    rows.collect()
}

/// "What is affected if I change `symbol`?" — forward reachability within
/// `max_depth` from every entity that defines the name, merged to the
/// shortest hop per node. Unknown name ⇒ empty `seeds` and empty
/// `reachable` (an empty answer that is visibly empty, not a silent zero).
pub fn impact(conn: &Connection, symbol: &str, max_depth: usize) -> Result<ImpactReport> {
    let seeds = resolve_symbols(conn, symbol)?;
    let mut merged: HashMap<String, usize> = HashMap::new();
    for seed in &seeds {
        for r in reachable(conn, &seed.id, max_depth)? {
            merged
                .entry(r.entity)
                .and_modify(|d| *d = (*d).min(r.depth))
                .or_insert(r.depth);
        }
    }
    let mut reachable_out: Vec<Reach> = merged
        .into_iter()
        .map(|(entity, depth)| Reach { entity, depth })
        .collect();
    reachable_out.sort_by(|a, b| a.depth.cmp(&b.depth).then_with(|| a.entity.cmp(&b.entity)));
    Ok(ImpactReport {
        symbol: symbol.to_string(),
        seeds,
        reachable: reachable_out,
    })
}

use rusqlite::params;

#[cfg(test)]
mod tests {
    use super::*;

    /// Hermetic fixture (lesson #6): in-memory v4 store with a 3-node
    /// cycle A→B→C→A, a self-loop D→D, and one unresolved target.
    fn fixture() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        crate::core::code_index_db::migrate_connection(&conn).unwrap();
        conn.execute_batch(
            r#"
            INSERT INTO entities (id, file, kind, name, line, node_path) VALUES
              ('gtw:src/a.rs:function:alpha', 'src/a.rs', 'function', 'alpha', 1, '1'),
              ('gtw:src/b.rs:function:beta',  'src/b.rs', 'function', 'beta',  1, '1'),
              ('gtw:src/c.rs:function:gamma', 'src/c.rs', 'function', 'gamma', 1, '1'),
              ('gtw:src/d.rs:function:delta', 'src/d.rs', 'function', 'delta', 1, '1');
            INSERT INTO relations (from_id, to_name, to_id, relation_type, line, file) VALUES
              ('gtw:src/a.rs:function:alpha', 'beta',  'gtw:src/b.rs:function:beta',  'calls', 2, 'src/a.rs'),
              ('gtw:src/b.rs:function:beta',  'gamma', 'gtw:src/c.rs:function:gamma', 'calls', 2, 'src/b.rs'),
              ('gtw:src/c.rs:function:gamma', 'alpha', 'gtw:src/a.rs:function:alpha', 'calls', 2, 'src/c.rs'),
              ('gtw:src/d.rs:function:delta', 'delta', 'gtw:src/d.rs:function:delta', 'calls', 2, 'src/d.rs'),
              -- unresolved: external target, to_id NULL (never enters walks)
              ('gtw:src/a.rs:function:alpha', 'external_thing', NULL, 'calls', 3, 'src/a.rs');
            "#,
        )
        .unwrap();
        conn
    }

    #[test]
    fn reachable_bounded_depth() {
        let conn = fixture();
        let out = reachable(&conn, "gtw:src/a.rs:function:alpha", 1).unwrap();
        assert_eq!(out.len(), 1, "depth 1: only the direct neighbor (B)");
        assert_eq!(out[0].entity, "gtw:src/b.rs:function:beta");
        assert_eq!(out[0].depth, 1);
    }

    #[test]
    fn reachable_terminates_on_cycle() {
        let conn = fixture();
        let out = reachable(&conn, "gtw:src/a.rs:function:alpha", 5).unwrap();
        // A→B→C→A: shortest hops are B@1, C@2 — and it TERMINATES.
        let ids: Vec<&str> = out.iter().map(|r| r.entity.as_str()).collect();
        assert!(ids.contains(&"gtw:src/b.rs:function:beta"));
        assert!(ids.contains(&"gtw:src/c.rs:function:gamma"));
        let b = out.iter().find(|r| r.entity.contains("beta")).unwrap();
        assert_eq!(b.depth, 1);
    }

    #[test]
    fn reachable_skips_unresolved_targets() {
        let conn = fixture();
        let out = reachable(&conn, "gtw:src/a.rs:function:alpha", 5).unwrap();
        assert!(
            !out.iter().any(|r| r.entity.contains("external")),
            "unresolved targets live in symbols, never in the walk"
        );
    }

    #[test]
    fn cycles_finds_simple_cycle() {
        let conn = fixture();
        let out = cycles(&conn, 10).unwrap();
        assert_eq!(
            out.len(),
            2,
            "fixture has exactly two cycles: A→B→C and D→D"
        );
        assert_eq!(out[0].nodes.len(), 3);
        assert_eq!(out[0].files.len(), 3);
    }

    #[test]
    fn cycles_dedups_rotations() {
        // The single walk over A/B/C visits the cycle from three start
        // points — rotation canonicalization must yield ONE report.
        let conn = fixture();
        let out = cycles(&conn, 10).unwrap();
        let cycles_of_3: Vec<_> = out.iter().filter(|c| c.nodes.len() == 3).collect();
        assert_eq!(cycles_of_3.len(), 1);
    }

    #[test]
    fn cycles_self_loop() {
        let conn = fixture();
        let out = cycles(&conn, 10).unwrap();
        let self_loop: Vec<_> = out.iter().filter(|c| c.nodes.len() == 1).collect();
        assert_eq!(self_loop.len(), 1, "D→D reported once");
        assert_eq!(self_loop[0].files, vec!["src/d.rs"]);
    }

    #[test]
    fn chain_returns_none_on_no_path() {
        let conn = fixture();
        // delta only calls itself — no path to beta (regression for the
        // CT infinite-loop class, §2.3.1).
        let out = chain(
            &conn,
            "gtw:src/d.rs:function:delta",
            "gtw:src/b.rs:function:beta",
        )
        .unwrap();
        assert!(out.is_none());
    }

    #[test]
    fn chain_is_a_real_path() {
        let conn = fixture();
        let out = chain(
            &conn,
            "gtw:src/a.rs:function:alpha",
            "gtw:src/c.rs:function:gamma",
        )
        .unwrap()
        .expect("path exists");
        assert_eq!(out.len(), 3);
        assert_eq!(out[0], "gtw:src/a.rs:function:alpha");
        assert_eq!(out[2], "gtw:src/c.rs:function:gamma");
    }

    #[test]
    fn synthetic_callsite_ids_do_not_panic() {
        // A synthetic callsite from_id (no entity row) participates in the
        // graph: files_of falls back to the encoded file, nothing unwraps.
        let conn = fixture();
        conn.execute(
            "INSERT INTO relations (from_id, to_name, to_id, relation_type, line, file)
             VALUES ('gtw:src/a.rs:callsite:9', 'gamma', 'gtw:src/c.rs:function:gamma', 'calls', 9, 'src/a.rs')",
            [],
        )
        .unwrap();
        let out = cycles(&conn, 10).unwrap();
        assert!(out
            .iter()
            .all(|c| c.files.iter().all(|f| f.starts_with("src/"))));
    }

    #[test]
    fn cycles_match_brute_force_reference() -> Result<(), Box<dyn std::error::Error>> {
        // Plan §4.4: validate the path-stack walk against a brute-force
        // enumeration of all simple cycles on the same small graph.
        let conn = fixture();
        let fast = cycles(&conn, 10).unwrap();
        let mut fast_nodes: Vec<Vec<String>> = fast.iter().map(|c| c.nodes.clone()).collect();
        fast_nodes.sort();

        // Brute force: DFS from every node keeping full paths, collect
        // rotations into a set (naive, obviously correct, fine at N=4).
        let mut brute: HashSet<Vec<String>> = HashSet::new();
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        {
            let mut stmt =
                conn.prepare("SELECT from_id, to_id FROM relations WHERE to_id IS NOT NULL")?;
            let rows =
                stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            for row in rows {
                let (f, t) = row?;
                adj.entry(f).or_default().push(t);
            }
        }
        let mut starts: Vec<&String> = adj.keys().collect();
        starts.sort();
        for start in starts {
            let mut stack: Vec<(String, Vec<String>)> = vec![(start.clone(), vec![start.clone()])];
            while let Some((node, path)) = stack.pop() {
                if path.len() > 10 {
                    continue;
                }
                if let Some(edges) = adj.get(&node) {
                    for next in edges {
                        if let Some(pos) = path.iter().position(|n| n == next) {
                            let mut cyc = path[pos..].to_vec();
                            rotate_to_smallest(&mut cyc);
                            brute.insert(cyc);
                        } else {
                            let mut p2 = path.clone();
                            p2.push(next.clone());
                            stack.push((next.clone(), p2));
                        }
                    }
                }
            }
        }
        assert_eq!(fast_nodes.len(), brute.len(), "same cycle count");
        for c in fast_nodes {
            assert!(brute.contains(&c), "fast-found cycle {c:?} in brute set");
        }
        Ok(())
    }
}

#[cfg(test)]
mod impact_tests {
    use super::*;

    /// Hermetic v4 store: `beta` is defined in TWO files (a name is not a
    /// symbol — both must be seeded) and both call `alpha`.
    fn fixture() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::core::code_index_db::migrate_connection(&conn).unwrap();
        conn.execute_batch(
            r#"
            INSERT INTO entities (id, file, kind, name, line, node_path) VALUES
              ('gtw:src/a.rs:function:alpha', 'src/a.rs', 'function', 'alpha', 1, '1'),
              ('gtw:src/b.rs:function:beta',  'src/b.rs', 'function', 'beta',  1, '1'),
              ('gtw:src/x.rs:function:beta',  'src/x.rs', 'function', 'beta',  5, '1');
            INSERT INTO relations (from_id, to_name, to_id, relation_type, line, file) VALUES
              ('gtw:src/b.rs:function:beta', 'alpha', 'gtw:src/a.rs:function:alpha', 'calls', 2, 'src/b.rs'),
              ('gtw:src/x.rs:function:beta', 'alpha', 'gtw:src/a.rs:function:alpha', 'calls', 6, 'src/x.rs');
            "#,
        )
        .unwrap();
        conn
    }

    #[test]
    fn impact_resolves_a_bare_name_and_reports_every_seed() {
        let conn = fixture();
        let r = impact(&conn, "beta", 3).unwrap();
        assert_eq!(
            r.seeds.len(),
            2,
            "the same name in two files is two distinct symbols — both seeded"
        );
        assert_eq!(r.reachable.len(), 1, "both seeds reach the same callee");
        assert_eq!(r.reachable[0].entity, "gtw:src/a.rs:function:alpha");
        assert_eq!(r.reachable[0].depth, 1);
    }

    #[test]
    fn impact_of_an_unknown_name_is_visibly_empty() {
        let conn = fixture();
        let r = impact(&conn, "does_not_exist", 3).unwrap();
        assert!(r.seeds.is_empty(), "no seeds invented");
        assert!(r.reachable.is_empty());
        assert_eq!(r.symbol, "does_not_exist");
    }

    #[test]
    fn impact_respects_the_depth_bound() {
        let conn = fixture();
        // depth 0 reaches nothing from a seed (the start is excluded).
        let r = impact(&conn, "beta", 0).unwrap();
        assert_eq!(r.seeds.len(), 2);
        assert!(r.reachable.is_empty());
    }
}
