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

/// What a graph walk could SEE at the moment it ran. `traverse` only follows
/// resolved edges, so a small `reachable` list has two possible explanations —
/// nothing is affected, or the edges were never resolvable (std, fields,
/// unresolved receivers). Reporting the denominator is what separates them:
/// on this repo `calls` is ~12 % resolved, i.e. `impact` sees roughly one edge
/// in eight, and silently returning "3 files" would be a confident half-truth.
#[derive(Debug, Clone, Serialize)]
pub struct GraphCoverage {
    pub resolved_edges: i64,
    pub unresolved_edges: i64,
    /// Unresolved targets whose name the project DOES define — the genuinely
    /// recoverable part (see `code-index --coverage`).
    pub unresolved_with_local_candidate: i64,
    /// resolved / (resolved + local-candidate), 0.0..=1.0
    pub local_rate: f64,
    pub note: &'static str,
}

/// Measure what a walk can see. One pass; the name table is built inline so a
/// caller needs nothing but the connection.
pub fn graph_coverage(conn: &Connection) -> Result<GraphCoverage> {
    let (resolved, local): (i64, i64) = conn.query_row(
        "SELECT COALESCE(SUM(r.to_id IS NOT NULL), 0),
                COALESCE(SUM(r.to_id IS NULL AND nc.name IS NOT NULL), 0)
         FROM relations r
         LEFT JOIN (SELECT name, COUNT(*) FROM entities GROUP BY name) nc
                ON nc.name = r.to_name",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let total = resolved
        + conn.query_row(
            "SELECT COUNT(*) FROM relations WHERE to_id IS NULL",
            [],
            |r| r.get::<_, i64>(0),
        )?;
    let local_rate = if resolved + local > 0 {
        resolved as f64 / (resolved + local) as f64
    } else {
        0.0
    };
    Ok(GraphCoverage {
        resolved_edges: resolved,
        unresolved_edges: total - resolved,
        unresolved_with_local_candidate: local,
        local_rate: (local_rate * 1000.0).round() / 1000.0,
        note: "A walk follows RESOLVED edges only. unresolved_edges includes std/prelude calls that cannot resolve without indexing std; unresolved_with_local_candidate is the recoverable remainder (run `code-index --coverage`).",
    })
}

/// One aggregated file→file dependency.
#[derive(Debug, Clone, Serialize)]
pub struct FileEdge {
    pub from: String,
    pub to: String,
    /// How many entity edges this file pair stands for.
    pub edges: i64,
}

/// The entity graph projected onto FILES.
///
/// No new nodes are involved and none are needed: every `relations` row already
/// carries the file it occurs in, and the target's file comes from the resolved
/// entity. So "which files depend on which" is a GROUP BY, not a schema change
/// (measured 2026-11-12: 303 file→file edges across 114 files, while 1 826
/// relation rows still had an unmatched *source* — source identity is
/// irrelevant to this projection).
///
/// What DOES limit it is target resolution, so the coverage is reported next to
/// the edges rather than left to be inferred.
#[derive(Debug, Clone, Serialize)]
pub struct FileGraph {
    pub files: i64,
    pub edges: Vec<FileEdge>,
    /// Relation rows with a resolved target (same-file included).
    pub attributed_edges: i64,
    /// Relation rows whose target never resolved — invisible in this graph.
    pub unattributed_edges: i64,
    /// attributed / (attributed + unattributed)
    pub coverage: f64,
    pub note: &'static str,
}

/// Distinct file→file pairs, strongest first. Deterministic ordering.
fn file_adjacency(conn: &Connection) -> Result<HashMap<String, Vec<String>>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT r.file, e.file
         FROM relations r JOIN entities e ON e.id = r.to_id
         WHERE r.file <> e.file
         ORDER BY r.file, e.file",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let (a, b) = row?;
        adj.entry(a).or_default().push(b);
    }
    for v in adj.values_mut() {
        v.sort();
        v.dedup();
    }
    Ok(adj)
}

pub fn file_graph(conn: &Connection) -> Result<FileGraph> {
    let mut stmt = conn.prepare(
        "SELECT r.file, e.file, COUNT(*)
         FROM relations r JOIN entities e ON e.id = r.to_id
         WHERE r.file <> e.file
         GROUP BY r.file, e.file
         ORDER BY COUNT(*) DESC, r.file, e.file",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(FileEdge {
            from: r.get(0)?,
            to: r.get(1)?,
            edges: r.get(2)?,
        })
    })?;
    let mut edges = Vec::new();
    let mut files: std::collections::BTreeSet<String> = Default::default();
    for row in rows {
        let e = row?;
        files.insert(e.from.clone());
        files.insert(e.to.clone());
        edges.push(e);
    }

    let (attributed, unattributed): (i64, i64) = conn.query_row(
        "SELECT COALESCE(SUM(to_id IS NOT NULL), 0), COALESCE(SUM(to_id IS NULL), 0)
         FROM relations",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let total = attributed + unattributed;
    let coverage = if total > 0 {
        attributed as f64 / total as f64
    } else {
        0.0
    };

    Ok(FileGraph {
        files: files.len() as i64,
        edges,
        attributed_edges: attributed,
        unattributed_edges: unattributed,
        coverage: (coverage * 1000.0).round() / 1000.0,
        note: "Cross-file dependencies only, aggregated over ALL relation types. Derived from relations.file (source) and entities.file (resolved target), so an unresolved target contributes no edge. coverage = attributed / all relation rows and therefore includes std/prelude calls that can never resolve — read it next to `code-index --coverage`, where the local-candidate denominator separates 'external' from 'missed'.",
    })
}

/// Files that sit in a dependency cycle with each other: mutual reachability,
/// grouped. Reads better than 29 entity-level cycles because a project-scale
/// cycle is almost always "these N modules reference each other".
pub fn file_cycles(conn: &Connection) -> Result<Vec<Vec<String>>> {
    let adj = file_adjacency(conn)?;
    let mut nodes: std::collections::BTreeSet<String> = adj.keys().cloned().collect();
    for vs in adj.values() {
        nodes.extend(vs.iter().cloned());
    }

    // reach[n] = every file n can reach (BFS). 114 nodes × a few hundred edges,
    // so the simple loop beats the complexity of Tarjan here.
    let mut reach: HashMap<&str, std::collections::HashSet<&str>> = HashMap::new();
    for n in &nodes {
        let mut seen: std::collections::HashSet<&str> = Default::default();
        let mut stack = vec![n.as_str()];
        while let Some(cur) = stack.pop() {
            if let Some(next) = adj.get(cur) {
                for t in next {
                    if seen.insert(t.as_str()) {
                        stack.push(t.as_str());
                    }
                }
            }
        }
        reach.insert(n.as_str(), seen);
    }

    // Same component iff each reaches the other. A file that can reach itself
    // is in a cycle; group those by mutual reachability.
    let cyclic: Vec<&str> = nodes
        .iter()
        .map(|n| n.as_str())
        .filter(|n| reach.get(n).is_some_and(|r| r.contains(n)))
        .collect();

    let mut groups: Vec<std::collections::BTreeSet<&str>> = Vec::new();
    for a in &cyclic {
        let mut placed = false;
        for g in groups.iter_mut() {
            let b = *g.iter().next().unwrap();
            if reach.get(a).is_some_and(|r| r.contains(b))
                && reach.get(b).is_some_and(|r| r.contains(a))
            {
                g.insert(a);
                placed = true;
                break;
            }
        }
        if !placed {
            let mut g = std::collections::BTreeSet::new();
            g.insert(*a);
            groups.push(g);
        }
    }

    let mut out: Vec<Vec<String>> = groups
        .into_iter()
        .map(|g| g.into_iter().map(|s| s.to_string()).collect())
        .collect();
    out.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    Ok(out)
}

/// One edge LEAVING the subject: where it lands, and where it is written.
#[derive(Debug, Clone, Serialize)]
pub struct OutEdge {
    /// File the target lives in (never the subject's own file).
    pub file: String,
    pub name: String,
    /// Definition line of the target.
    pub line: i64,
    pub relation_type: String,
    /// Line in the SUBJECT where this use occurs.
    pub from_line: i64,
}

/// One edge ENTERING the subject: who reaches it.
#[derive(Debug, Clone, Serialize)]
pub struct InEdge {
    pub file: String,
    /// Resolved caller name, when the source id matched an entity. `None` means
    /// the use sits at file scope or in a function the extractor does not name —
    /// reported as unknown rather than guessed.
    pub from_symbol: Option<String>,
    /// Call-site line in the other file.
    pub line: i64,
    pub relation_type: String,
}

/// "What does this file / symbol touch, and where exactly?"
///
/// Two questions, one shape: outgoing edges that CROSS a file boundary (with the
/// target's file, symbol and line) and incoming edges (who reaches in). Same
/// rows as `--file-graph`, kept per-edge because "which files" is not actionable
/// without "where in the files" (user, 2026-11-12).
#[derive(Debug, Clone, Serialize)]
pub struct Touches {
    pub subject: String,
    /// "file" or "symbol"
    pub kind: &'static str,
    pub outgoing: Vec<OutEdge>,
    pub incoming: Vec<InEdge>,
    pub note: &'static str,
}

const TOUCHES_NOTE: &str = "Cross-file edges only. Outgoing = what the subject uses elsewhere (target file + symbol + definition line, plus the line here); incoming = who uses the subject. Unresolved targets cannot appear here, which is the same limit as --file-graph: run --coverage for that denominator.";

/// Distinct cross-file uses of one file.
pub fn touches_file(conn: &Connection, file: &str) -> Result<Touches> {
    let mut out_stmt = conn.prepare(
        "SELECT e.file, e.name, e.line, r.relation_type, r.line
         FROM relations r JOIN entities e ON e.id = r.to_id
         WHERE r.file = ?1 AND e.file <> ?1
         ORDER BY e.file, e.line, e.name",
    )?;
    let outgoing: Vec<OutEdge> = out_stmt
        .query_map(params![file], |r| {
            Ok(OutEdge {
                file: r.get(0)?,
                name: r.get(1)?,
                line: r.get(2)?,
                relation_type: r.get(3)?,
                from_line: r.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut in_stmt = conn.prepare(
        "SELECT r.file,
                (SELECT name FROM entities WHERE id = r.from_id),
                r.line, r.relation_type
         FROM relations r JOIN entities e ON e.id = r.to_id
         WHERE e.file = ?1 AND r.file <> ?1
         ORDER BY r.file, r.line",
    )?;
    let incoming: Vec<InEdge> = in_stmt
        .query_map(params![file], |r| {
            Ok(InEdge {
                file: r.get(0)?,
                from_symbol: r.get(1)?,
                line: r.get(2)?,
                relation_type: r.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Touches {
        subject: file.to_string(),
        kind: "file",
        outgoing,
        incoming,
        note: TOUCHES_NOTE,
    })
}

/// Cross-file uses of every entity that DEFINES `symbol` (methods included:
/// their ids carry `Type::name`, so a bare name can yield several seeds).
pub fn touches_symbol(conn: &Connection, symbol: &str) -> Result<Touches> {
    let seeds = resolve_symbols(conn, symbol)?;
    let mut outgoing = Vec::new();
    let mut incoming = Vec::new();
    let mut out_stmt = conn.prepare(
        "SELECT e.file, e.name, e.line, r.relation_type, r.line
         FROM relations r JOIN entities e ON e.id = r.to_id
         WHERE r.from_id = ?1 AND e.file <> r.file
         ORDER BY e.file, e.line, e.name",
    )?;
    let mut in_stmt = conn.prepare(
        "SELECT r.file,
                (SELECT name FROM entities WHERE id = r.from_id),
                r.line, r.relation_type
         FROM relations r JOIN entities e ON e.id = r.to_id
         WHERE r.to_id = ?1 AND r.file <> e.file
         ORDER BY r.file, r.line",
    )?;
    for seed in &seeds {
        outgoing.extend(
            out_stmt
                .query_map(params![&seed.id], |r| {
                    Ok(OutEdge {
                        file: r.get(0)?,
                        name: r.get(1)?,
                        line: r.get(2)?,
                        relation_type: r.get(3)?,
                        from_line: r.get(4)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?,
        );
        incoming.extend(
            in_stmt
                .query_map(params![&seed.id], |r| {
                    Ok(InEdge {
                        file: r.get(0)?,
                        from_symbol: r.get(1)?,
                        line: r.get(2)?,
                        relation_type: r.get(3)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?,
        );
    }
    outgoing.sort_by(|a, b| (&a.file, a.line, &a.name).cmp(&(&b.file, b.line, &b.name)));
    outgoing.dedup_by(|a, b| {
        a.file == b.file && a.line == b.line && a.name == b.name && a.from_line == b.from_line
    });
    incoming.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Ok(Touches {
        subject: symbol.to_string(),
        kind: "symbol",
        outgoing,
        incoming,
        note: TOUCHES_NOTE,
    })
}

/// One logged relation change (a row of `relation_changes`).
#[derive(Debug, Clone, Serialize)]
pub struct HistoryChange {
    /// `from_id|relation_type|to_name` — the aggregated key.
    pub relation_key: String,
    /// `added` | `removed` | `count_changed`
    pub change_kind: String,
    pub count_before: i64,
    pub count_after: i64,
    pub resolution_status: String,
}

/// All changes recorded under one revision.
#[derive(Debug, Clone, Serialize)]
pub struct HistoryRevision {
    pub revision_id: i64,
    pub git_rev: Option<String>,
    pub completed_at: Option<i64>,
    pub source: Option<String>,
    pub added: usize,
    pub removed: usize,
    pub count_changed: usize,
    pub changes: Vec<HistoryChange>,
}

/// "What changed in the graph, and when?" — the read side of
/// `relation_changes`, i.e. the onion skin's past layer.
///
/// Rows only exist from the revision where the writer landed (2026-11-12);
/// older revisions recorded that a refresh happened but not what it changed, so
/// an empty history is reported as "starts here", never as "nothing ever
/// changed".
#[derive(Debug, Clone, Serialize)]
pub struct HistoryReport {
    /// Substring filter applied to the relation key, when given.
    pub filter: Option<String>,
    pub revisions: Vec<HistoryRevision>,
    pub most_recent_revision: Option<i64>,
    /// True when the store has revisions but no delta rows at all: the writer is
    /// newer than the data, which is a fact about the past, not about the code.
    pub history_starts_now: bool,
    pub note: &'static str,
}

pub fn history(conn: &Connection, filter: Option<&str>, last: usize) -> Result<HistoryReport> {
    let filter_str = filter.unwrap_or("");
    let limit = last.max(1) as i64;
    let mut stmt = conn.prepare(
        "SELECT rc.revision_id, r.git_rev, r.completed_at, r.source,
                rc.relation_key, rc.change_kind, rc.count_before, rc.count_after,
                rc.resolution_status
         FROM relation_changes rc
         JOIN index_revisions r ON r.id = rc.revision_id
         WHERE (?1 = '' OR rc.relation_key LIKE '%' || ?1 || '%')
           AND rc.revision_id IN (
                 SELECT DISTINCT revision_id FROM relation_changes
                 WHERE (?1 = '' OR relation_key LIKE '%' || ?1 || '%')
                 ORDER BY revision_id DESC LIMIT ?2)
         ORDER BY rc.revision_id DESC, rc.change_kind, rc.relation_key",
    )?;
    let rows = stmt.query_map(params![filter_str, limit], |r| {
        Ok((
            HistoryRevision {
                revision_id: r.get(0)?,
                git_rev: r.get(1)?,
                completed_at: r.get(2)?,
                source: r.get(3)?,
                added: 0,
                removed: 0,
                count_changed: 0,
                changes: Vec::new(),
            },
            HistoryChange {
                relation_key: r.get(4)?,
                change_kind: r.get(5)?,
                count_before: r.get(6)?,
                count_after: r.get(7)?,
                resolution_status: r.get(8)?,
            },
        ))
    })?;

    let mut revisions: Vec<HistoryRevision> = Vec::new();
    for row in rows {
        let (meta, change) = row?;
        if revisions.last().map(|r| r.revision_id) != Some(meta.revision_id) {
            revisions.push(meta);
        }
        let rev = revisions.last_mut().unwrap();
        match change.change_kind.as_str() {
            "added" => rev.added += 1,
            "removed" => rev.removed += 1,
            "count_changed" => rev.count_changed += 1,
            _ => {}
        }
        rev.changes.push(change);
    }

    let total_rows: i64 = conn
        .query_row(
            "SELECT COALESCE(COUNT(*), 0) FROM relation_changes",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let revisions_in_store: i64 = conn
        .query_row(
            "SELECT COALESCE(COUNT(*), 0) FROM index_revisions",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let most_recent_revision: Option<i64> = conn
        .query_row("SELECT MAX(revision_id) FROM relation_changes", [], |r| {
            r.get(0)
        })
        .unwrap_or(None);

    Ok(HistoryReport {
        filter: filter.map(str::to_string),
        revisions,
        most_recent_revision,
        history_starts_now: total_rows == 0 && revisions_in_store > 0,
        note: "Rows come from relation_changes, written since 2026-11-12: earlier revisions recorded THAT a refresh happened, not WHAT changed. Only files that were (re)parsed after the writer landed have rows here — an untouched file has NO history even though its edges exist, so read `--touches` for the current state and this for the movement. change_kind count_changed means the edge still exists but its occurrence count moved (D8). A filter is a substring match on `from_id|relation_type|to_name`.",
    })
}

/// Result of `impact`. `seeds` is empty when the name is not in the store —
/// reported as-is, never fabricated into an impact chain (same honesty rule
/// as `ImpactReport::unresolved_calls`).
#[derive(Debug, Clone, Serialize)]
pub struct ImpactReport {
    pub symbol: String,
    pub seeds: Vec<SymbolSeed>,
    pub reachable: Vec<Reach>,
    /// Which edges the walk could have followed. Never omit this: an empty
    /// `reachable` plus a 12 % rate is a coverage statement, not a verdict.
    pub graph_coverage: GraphCoverage,
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
        graph_coverage: graph_coverage(conn)?,
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

    // ── File-level projection ────────────────────────────────────────────
    //
    // Three files: a.rs -> b.rs (two edges), b.rs -> a.rs (one, so a<->b is a
    // cycle), b.rs -> c.rs (one, one-way), and one edge whose target never
    // resolved (it must contribute NO file edge, and must show up in the
    // coverage denominator instead of vanishing).
    fn file_store() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::core::code_index_db::migrate_connection(&conn).unwrap();
        conn.execute_batch(
            r#"
            INSERT INTO entities(id, file, kind, name, line, node_path) VALUES
              ('gtw:a.rs:function:f', 'a.rs', 'function', 'f', 1, '1'),
              ('gtw:b.rs:function:g', 'b.rs', 'function', 'g', 1, '1'),
              ('gtw:c.rs:function:h', 'c.rs', 'function', 'h', 1, '1');
            INSERT INTO relations(from_id, to_name, to_id, relation_type, line, file) VALUES
              ('gtw:a.rs:function:f', 'g', 'gtw:b.rs:function:g', 'calls', 2, 'a.rs'),
              ('gtw:a.rs:function:f', 'g', 'gtw:b.rs:function:g', 'calls', 3, 'a.rs'),
              ('gtw:b.rs:function:g', 'f', 'gtw:a.rs:function:f', 'calls', 2, 'b.rs'),
              ('gtw:b.rs:function:g', 'h', 'gtw:c.rs:function:h', 'calls', 3, 'b.rs'),
              ('gtw:c.rs:function:h', 'unwrap', NULL, 'calls', 2, 'c.rs');
            "#,
        )
        .unwrap();
        conn
    }

    #[test]
    fn file_graph_projects_edges_and_states_its_coverage() {
        let conn = file_store();
        let g = file_graph(&conn).unwrap();
        assert_eq!(g.files, 3, "three files take part");
        let pair = |a: &str, b: &str| {
            g.edges
                .iter()
                .find(|e| e.from == a && e.to == b)
                .map(|e| e.edges)
        };
        assert_eq!(pair("a.rs", "b.rs"), Some(2), "weight is the edge count");
        assert_eq!(pair("b.rs", "a.rs"), Some(1));
        assert_eq!(pair("b.rs", "c.rs"), Some(1));
        // The unresolved target contributes no file edge, but it IS counted:
        // hiding it would make the graph look complete.
        assert_eq!(g.attributed_edges, 4);
        assert_eq!(g.unattributed_edges, 1);
        assert!((g.coverage - 0.8).abs() < 1e-6, "coverage {}", g.coverage);
    }

    #[test]
    fn touches_names_the_target_file_and_line() {
        let conn = file_store();
        // a.rs:f calls b.rs:g (twice) — outgoing from a file, with the target's
        // definition line and the line in the subject.
        let a = touches_file(&conn, "a.rs").unwrap();
        assert_eq!(a.kind, "file");
        assert!(
            a.outgoing
                .iter()
                .any(|e| e.file == "b.rs" && e.name == "g" && e.line == 1),
            "a.rs must list b.rs:g at its definition line: {:?}",
            a.outgoing
        );
        assert_eq!(a.incoming.len(), 1, "b.rs:g calls back into a.rs:f");
        assert_eq!(a.incoming[0].file, "b.rs");
        assert_eq!(a.incoming[0].from_symbol.as_deref(), Some("g"));

        // Symbol mode: g's cross-file use of h, and who uses g.
        let g = touches_symbol(&conn, "g").unwrap();
        assert_eq!(g.kind, "symbol");
        assert!(
            g.outgoing
                .iter()
                .any(|e| e.file == "c.rs" && e.name == "h" && e.from_line == 3),
            "g uses c.rs:h at line 3: {:?}",
            g.outgoing
        );
        assert!(
            g.incoming.iter().any(|e| e.file == "a.rs"),
            "a.rs:f uses g: {:?}",
            g.incoming
        );

        // A name that is not in the store yields empty lists, never an error
        // and never a fabricated edge.
        let none = touches_symbol(&conn, "does_not_exist").unwrap();
        assert!(none.outgoing.is_empty() && none.incoming.is_empty());
    }

    #[test]
    fn history_groups_by_revision_and_filters_the_key() {
        let conn = file_store();
        // Two revisions with relation_changes rows, plus one unrelated key.
        conn.execute_batch(
            r#"
            INSERT INTO index_revisions (id, parent_id, git_rev, dirty, source,
                identity_scheme_version, generation_key, started_at, completed_at, status)
            VALUES (1, NULL, 'aaa111', 0, 'test', 1, 'gk', 100, 100, 'completed'),
                   (2, 1, 'bbb222', 1, 'test', 1, 'gk', 200, 200, 'completed');
            INSERT INTO relation_changes
              (revision_id, relation_key, change_kind, count_before, count_after,
               before_json, after_json, resolution_status)
            VALUES
              (1, 'gtw:a.rs:function:f|calls|g', 'added', 0, 1, NULL, '{"count":1,"resolved":1}', 'resolved'),
              (1, 'gtw:b.rs:function:g|calls|h', 'added', 0, 1, NULL, '{"count":1,"resolved":1}', 'resolved'),
              (2, 'gtw:a.rs:function:f|calls|g', 'count_changed', 1, 2, '{"count":1,"resolved":1}', '{"count":2,"resolved":1}', 'resolved'),
              (2, 'gtw:c.rs:function:h|calls|unwrap', 'removed', 1, 0, '{"count":1,"resolved":0}', NULL, 'unresolved');
            "#,
        )
        .unwrap();

        let all = history(&conn, None, 5).unwrap();
        assert_eq!(all.revisions.len(), 2, "two revisions have changes");
        // Newest first, so the reader sees "what just happened" first.
        assert_eq!(all.revisions[0].revision_id, 2);
        assert_eq!(
            (
                all.revisions[0].added,
                all.revisions[0].removed,
                all.revisions[0].count_changed
            ),
            (0, 1, 1),
            "kind counters per revision: {:?}",
            all.revisions[0]
        );
        assert_eq!(all.revisions[1].added, 2);
        assert_eq!(all.revisions[0].git_rev.as_deref(), Some("bbb222"));
        assert!(!all.history_starts_now, "rows exist, so history is live");

        // Filter on a symbol: only the a.rs -> g keys survive.
        let filtered = history(&conn, Some("function:f"), 5).unwrap();
        assert_eq!(filtered.filter.as_deref(), Some("function:f"));
        let keys: Vec<&str> = filtered
            .revisions
            .iter()
            .flat_map(|r| r.changes.iter().map(|c| c.relation_key.as_str()))
            .collect();
        assert!(!keys.is_empty(), "the filter must match the a.rs keys");
        assert!(
            keys.iter().all(|k| k.contains("function:f")),
            "no unfiltered key may leak in: {keys:?}"
        );

        // `last` bounds the number of revisions, newest first.
        let one = history(&conn, None, 1).unwrap();
        assert_eq!(one.revisions.len(), 1);
        assert_eq!(one.revisions[0].revision_id, 2);

        // A store with revisions but no deltas is "history starts now", not
        // "nothing ever changed" — the distinction the writer's age creates.
        let empty_conn = Connection::open_in_memory().unwrap();
        crate::core::code_index_db::migrate_connection(&empty_conn).unwrap();
        empty_conn
            .execute(
                "INSERT INTO index_revisions (id, parent_id, git_rev, dirty, source,
                    identity_scheme_version, generation_key, started_at, completed_at, status)
                 VALUES (9, NULL, 'old', 0, 'pre-writer', 1, 'gk', 1, 1, 'completed')",
                [],
            )
            .unwrap();
        let stale = history(&empty_conn, None, 5).unwrap();
        assert!(stale.history_starts_now, "revisions without deltas");
        assert!(stale.revisions.is_empty());
    }

    #[test]
    fn file_cycles_group_mutually_reachable_files() {
        let conn = file_store();
        let groups = file_cycles(&conn).unwrap();
        assert_eq!(groups.len(), 1, "one cycle group: {groups:?}");
        assert_eq!(groups[0], vec!["a.rs".to_string(), "b.rs".to_string()]);
        // c.rs is reachable from b.rs but cannot get back: not a cycle.
        assert!(!groups[0].iter().any(|f| f == "c.rs"));
    }
}
