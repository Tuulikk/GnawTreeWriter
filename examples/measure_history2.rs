//! Second measurement: is ``file_versions`` actually storing distinct versions,
//! or the same row repeated across revisions? Read-only.

use rusqlite::{Connection, OpenFlags};
use std::collections::{HashMap, HashSet};

fn main() -> anyhow::Result<()> {
    let store = std::env::args()
        .nth(1)
        .unwrap_or_else(|| ".gnawtreewriter_ai/code_index.db".to_string());
    let conn = Connection::open_with_flags(&store, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    // Does before_hash ever get filled? The delta chain depends on it.
    let (total, with_before, with_after): (i64, i64, i64) = conn.query_row(
        "SELECT COUNT(*),
                COALESCE(SUM(before_hash IS NOT NULL), 0),
                COALESCE(SUM(after_hash IS NOT NULL), 0)
         FROM file_versions",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    println!("file_versions rows      : {total}");
    println!("  with before_hash      : {with_before}");
    println!("  with after_hash       : {with_after}");

    // Per path: how many rows vs how many DISTINCT after_hash values.
    println!("\nper path: rows vs distinct after_hash (top 12 by rows)");
    let mut stmt = conn.prepare(
        "SELECT path, COUNT(*), COUNT(DISTINCT after_hash)
         FROM file_versions
         GROUP BY path
         ORDER BY COUNT(*) DESC
         LIMIT 12",
    )?;
    let mut rows = stmt.query([])?;
    println!("  {:>6} {:>8} {:>8}  path", "rows", "distinct", "ratio");
    while let Some(r) = rows.next()? {
        let path: String = r.get(0)?;
        let n: i64 = r.get(1)?;
        let d: i64 = r.get(2)?;
        let ratio = if d > 0 { n as f64 / d as f64 } else { 0.0 };
        println!("  {n:6} {d:8} {ratio:8.1}x  {path}");
    }

    // Global distinct after_hash and total rows.
    let distinct_after: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT after_hash) FROM file_versions WHERE after_hash IS NOT NULL",
        [],
        |r| r.get(0),
    )?;
    println!("\nrows / distinct after_hash : {total} / {distinct_after}");

    // Revision spread: are the same hashes re-recorded every revision?
    let revs_per_hash: Option<(i64, i64)> = conn
        .query_row(
            "SELECT MAX(c), MIN(c) FROM (
                SELECT after_hash, COUNT(*) AS c
                FROM file_versions WHERE after_hash IS NOT NULL
                GROUP BY after_hash
             )",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    if let Some((max, min)) = revs_per_hash {
        println!("same after_hash recorded between {min} and {max} time(s)");
    }

    // Per-file version counts keyed by REAL path (the backup dir key is the
    // basename, which merges core/mod.rs with anything else named mod.rs).
    let mut by_path: HashMap<String, HashSet<String>> = HashMap::new();
    {
        let mut stmt = conn
            .prepare("SELECT path, after_hash FROM file_versions WHERE after_hash IS NOT NULL")?;
        let mut rows = stmt.query([])?;
        while let Some(r) = rows.next()? {
            let p: String = r.get(0)?;
            let h: String = r.get(1)?;
            by_path.entry(p).or_default().insert(h);
        }
    }
    let mut v: Vec<(String, usize)> = by_path.into_iter().map(|(p, s)| (p, s.len())).collect();
    v.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    println!("\ntrue distinct versions per real path (top 10):");
    for (p, n) in v.iter().take(10) {
        println!("  {n:5}  {p}");
    }
    let total_distinct: usize = v.iter().map(|(_, n)| n).sum();
    println!("sum of distinct versions across paths: {total_distinct}");

    Ok(())
}
