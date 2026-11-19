//! One-off measurement: how large would a content-addressed history be?
//!
//! Read-only. Never writes the store. Run:
//!   cargo run --example measure_history -- .gnawtreewriter_ai/code_index.db
//!
//! Answers the question that decides the storage model:
//!   - how many DISTINCT file contents exist across the recorded history
//!     (content addressing dedupes across revisions AND across files)
//!   - how much those distinct blobs would weigh
//!   - how that compares with what the AST-backup directory stores today

use rusqlite::{Connection, OpenFlags};
use std::collections::{HashMap, HashSet};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let store = std::env::args()
        .nth(1)
        .unwrap_or_else(|| ".gnawtreewriter_ai/code_index.db".to_string());
    let root = Path::new(".");
    let conn = Connection::open_with_flags(&store, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    let revisions: i64 =
        conn.query_row("SELECT COUNT(*) FROM index_revisions", [], |r| r.get(0))?;
    let completed: i64 = conn.query_row(
        "SELECT COUNT(*) FROM index_revisions WHERE status = 'completed'",
        [],
        |r| r.get(0),
    )?;
    let fv_rows: i64 = conn.query_row("SELECT COUNT(*) FROM file_versions", [], |r| r.get(0))?;

    println!("=== Store: {store}");
    println!("revisions          : {revisions} ({completed} completed)");
    println!("file_versions rows : {fv_rows}");

    // Distinct contents across the WHOLE history: a blob is one content hash.
    // Both columns count — before_hash is a version nobody's after_hash may name.
    let mut all_hashes: HashSet<String> = HashSet::new();
    let mut per_path: HashMap<String, HashSet<String>> = HashMap::new();
    {
        let mut stmt = conn.prepare("SELECT path, before_hash, after_hash FROM file_versions")?;
        let mut rows = stmt.query([])?;
        while let Some(r) = rows.next()? {
            let path: String = r.get(0)?;
            for i in 1..=2 {
                let h: Option<String> = r.get(i)?;
                if let Some(h) = h {
                    all_hashes.insert(h.clone());
                    per_path.entry(path.clone()).or_default().insert(h);
                }
            }
        }
    }

    println!("\n=== Content addressing (the ceiling of dedup)");
    println!(
        "distinct content hashes (all history): {}",
        all_hashes.len()
    );
    println!("distinct tracked paths               : {}", per_path.len());
    if !per_path.is_empty() {
        let mut sizes: Vec<(String, usize)> =
            per_path.iter().map(|(p, s)| (p.clone(), s.len())).collect();
        sizes.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        let total: usize = sizes.iter().map(|(_, n)| n).sum();
        println!(
            "avg versions per path                : {:.1}",
            total as f64 / sizes.len() as f64
        );
        println!("\ntop paths by distinct versions:");
        for (p, n) in sizes.iter().take(10) {
            println!("  {n:5}  {p}");
        }
    }

    // Current worktree sizes, to bound what one blob costs today.
    let mut current_bytes = 0u64;
    let mut file_count = 0usize;
    for path in per_path.keys() {
        if let Ok(meta) = std::fs::metadata(root.join(path)) {
            current_bytes += meta.len();
            file_count += 1;
        }
    }
    let avg_file = if file_count > 0 {
        current_bytes / file_count as u64
    } else {
        0
    };
    println!("\n=== Size model (using CURRENT worktree sizes as the blob proxy)");
    println!("tracked files present on disk        : {file_count}");
    println!("current total source bytes           : {current_bytes} B");
    println!("average file size                    : {avg_file} B");
    println!(
        "if every distinct hash were a full file blob: {} B ({:.2} GB)",
        all_hashes.len() as u64 * avg_file,
        (all_hashes.len() as u64 * avg_file) as f64 / 1e9
    );
    println!(
        "  ... zstd~0.25 of that                     : {:.2} GB",
        (all_hashes.len() as u64 * avg_file) as f64 * 0.25 / 1e9
    );

    // What the AST backup directory actually costs today.
    let bdir = root.join(".gnawtreewriter_backups");
    let mut backups = 0usize;
    let mut backup_bytes = 0u64;
    let mut per_file: HashMap<String, (usize, HashSet<u64>)> = HashMap::new();
    if let Ok(entries) = std::fs::read_dir(&bdir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().is_none_or(|x| x != "json") {
                continue;
            }
            let Ok(meta) = e.metadata() else { continue };
            backups += 1;
            backup_bytes += meta.len();
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            // <orig>_backup_<YYYYMMDD>_<HHMMSS>_<mmm>.json
            let orig = name
                .rsplit_once("_backup_")
                .map(|(a, _)| a.to_string())
                .unwrap_or(name);
            let entry = per_file.entry(orig).or_insert((0, HashSet::new()));
            entry.0 += 1;
            entry.1.insert(meta.len());
        }
    }
    println!("\n=== AST backup directory (what we have today)");
    println!("backup files                         : {backups}");
    println!(
        "total bytes                          : {backup_bytes} B ({:.2} GB)",
        backup_bytes as f64 / 1e9
    );
    if backups > 0 {
        println!(
            "average per backup                   : {} B",
            backup_bytes / backups as u64
        );
    }
    let mut worst: Vec<(String, usize, usize)> = per_file
        .into_iter()
        .map(|(k, (n, sizes))| (k, n, sizes.len()))
        .collect();
    worst.sort_by_key(|(_, n, _)| std::cmp::Reverse(*n));
    println!("\ntop files by backup count (count, distinct sizes):");
    for (f, n, u) in worst.iter().take(10) {
        println!("  {n:5} backups, {u:5} distinct sizes  {f}");
    }

    Ok(())
}
