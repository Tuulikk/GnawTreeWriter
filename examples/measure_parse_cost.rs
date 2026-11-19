//! Fourth measurement: is the stored AST worth keeping, or is it derivable?
//!
//! Parses the same file the backup covers and times it. If the tree can be
//! regenerated in milliseconds from `source_code`, then storing 14 MB of AST
//! per backup is paying disk for a cache.

use std::path::Path;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let target = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "src/cli.rs".to_string());
    let path = Path::new(&target);
    let source = std::fs::read_to_string(path)?;
    println!("file    : {target}");
    println!("source  : {} bytes", source.len());

    let parser = gnawtreewriter::parser::get_parser(path)?;

    // Warm the parser once (grammar init), then measure real parses.
    let _ = parser.parse(&source)?;

    let mut best = f64::MAX;
    let mut total = 0.0;
    const N: usize = 10;
    let mut nodes = 0usize;
    for _ in 0..N {
        let t = Instant::now();
        let tree = parser.parse(&source)?;
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        total += ms;
        if ms < best {
            best = ms;
        }
        nodes = count(&tree);
    }
    println!(
        "\nparse time: best {best:.2} ms, avg {:.2} ms over {N} runs",
        total / N as f64
    );
    println!("nodes     : {nodes}");
    println!("\ncost of NOT storing the AST: {:.0} ms per restore", best);
    println!("cost of storing it        : ~14 MB per backup version");
    Ok(())
}

fn count(n: &gnawtreewriter::TreeNode) -> usize {
    1 + n.children.iter().map(count).sum::<usize>()
}
