//! Third measurement: what does ONE backup file actually contain, and what is
//! the AST tree worth on disk versus the source text it already carries?
//!
//! Read-only.

use std::path::Path;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        ".gnawtreewriter_backups/cli.rs_backup_20261111_084515_713.json".to_string()
    });
    let p = Path::new(&path);
    let raw = std::fs::read_to_string(p)?;
    println!("file                  : {path}");
    println!("bytes on disk         : {}", raw.len());

    let v: serde_json::Value = serde_json::from_str(&raw)?;
    println!(
        "\ntop-level keys        : {:?}",
        v.as_object().map(|o| o.keys().collect::<Vec<_>>())
    );

    if let Some(src) = v.get("source_code").and_then(|s| s.as_str()) {
        println!(
            "source_code           : {} bytes, {} lines",
            src.len(),
            src.lines().count()
        );
    } else {
        println!("source_code           : ABSENT at top level");
    }

    // Serialize the tree alone to measure what the AST costs.
    if let Some(tree) = v.get("tree") {
        let tree_json = serde_json::to_string(tree)?;
        println!("tree (AST) as JSON    : {} bytes", tree_json.len());

        // Count nodes and how much text the nodes carry in total.
        fn walk(n: &serde_json::Value, count: &mut usize, content_bytes: &mut usize, depth: usize) {
            *count += 1;
            if let Some(c) = n.get("content").and_then(|c| c.as_str()) {
                *content_bytes += c.len();
            }
            if depth < 200 {
                if let Some(children) = n.get("children").and_then(|c| c.as_array()) {
                    for ch in children {
                        walk(ch, count, content_bytes, depth + 1);
                    }
                }
            }
        }
        let mut count = 0usize;
        let mut content_bytes = 0usize;
        walk(tree, &mut count, &mut content_bytes, 0);
        println!("AST node count        : {count}");
        println!(
            "sum of node 'content' : {content_bytes} bytes  ({:.1}x the source)",
            content_bytes as f64 / raw.len() as f64
        );
        println!(
            "content / source_code : {:.1}x",
            content_bytes as f64
                / v.get("source_code")
                    .and_then(|s| s.as_str())
                    .map(|s| s.len())
                    .unwrap_or(1) as f64
        );
    }

    // What would just the source cost? (No compressor in Cargo.toml — the
    // uncompressed size is already the number that decides the design; a
    // general-purpose compressor only improves it by ~4x.)
    if let Some(src) = v.get("source_code").and_then(|s| s.as_str()) {
        println!(
            "\nsource only (no AST)  : {} bytes — {:.1}x smaller than this file",
            src.len(),
            raw.len() as f64 / src.len() as f64
        );
    }
    Ok(())
}
