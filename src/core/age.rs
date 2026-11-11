//! Tree rings: how old is this part of the structure?
//!
//! The owner's analogy (2026-11-11): a tree's rings show what has been there a
//! long time. A code base has the same signal — a node that has survived years
//! and dozens of commits is a different thing to delete than one written
//! yesterday, *even when it is the right place to edit*.
//!
//! Method: `git blame --line-porcelain` over the node's line range (TreeNode
//! already carries `start_line`/`end_line`), cached per file content hash.
//!
//! ## What these numbers mean — and do not mean
//!
//! `git blame` reports the last commit that touched each line. Refactors reset
//! it: a moved or renamed function looks new; `--follow` tracks file renames,
//! not line identity. So the honest reading is:
//!
//!   - `oldest_days`  = how long the OLDEST line here has sat unchanged
//!   - `touched_days` = how long since the NEWEST line changed
//!   - both describe *this text in this file*, never "how long the idea existed"
//!
//! A shallow clone has no history: every field is `None` with
//! `history: "shallow …"` — a zero here would be a lie, and a lie is worse than
//! a gap.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineAge {
    pub line: usize,
    pub commit: String,
    pub author_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAge {
    pub lines: Vec<LineAge>,
    /// `None` when the repository had history to blame.
    pub history: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAge {
    pub file: String,
    pub first_line: usize,
    pub last_line: usize,
    /// Days since the oldest line in range last changed (the "tree ring").
    pub oldest_days: Option<i64>,
    /// Days since the newest line in range changed (freshness).
    pub touched_days: Option<i64>,
    /// Distinct commits that own lines inside the range.
    pub commits: Option<usize>,
    /// Share of lines still owned by the oldest commit in the range (0..=1).
    pub survival: Option<f64>,
    /// Why the numbers are missing, when they are.
    pub history: Option<String>,
}

/// Parse `git blame --line-porcelain` output.
///
/// Pure on purpose: the format is stable and the parsing is where bugs live, so
/// it is tested without a repository.
pub fn parse_porcelain(text: &str) -> Vec<LineAge> {
    let mut out: Vec<LineAge> = Vec::new();
    for line in text.lines() {
        if line.starts_with('\t') {
            continue; // the content line closes a block
        }
        let parts: Vec<&str> = line.split(' ').collect();
        let looks_like_header = parts.len() >= 3
            && parts[0].len() == 40
            && parts[0].chars().all(|c| c.is_ascii_hexdigit());
        if looks_like_header {
            if let Ok(n) = parts[2].parse::<usize>() {
                out.push(LineAge {
                    line: n,
                    commit: parts[0].to_string(),
                    author_time: 0,
                });
            }
        } else if let Some(v) = line.strip_prefix("author-time ") {
            if let Some(last) = out.last_mut() {
                last.author_time = v.trim().parse::<i64>().unwrap_or(0);
            }
        }
    }
    out
}

pub fn cache_dir(root: &Path) -> PathBuf {
    root.join(".gnawtreewriter_ai").join("age")
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}


/// Blame one file, cached by content hash. A changed file re-blames; an
/// unchanged one costs a file read.
pub fn file_age(root: &Path, file: &str) -> Result<FileAge> {
    let path = root.join(file);
    if !path.is_file() {
        return Err(anyhow!("no such file: {}", path.display()));
    }
    let shallow = std::process::Command::new("git")
        .args(["rev-parse", "--is-shallow-repository"])
        .current_dir(root)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    if shallow.as_deref() == Some("true") {
        return Ok(FileAge {
            lines: Vec::new(),
            history: Some("shallow clone — no history to blame".to_string()),
        });
    }

    let hash = crate::core::code_index_db::content_hash(&path).unwrap_or_else(|| "nohash".into());
    let cache = cache_dir(root).join(format!("{hash}.json"));
    if let Ok(text) = std::fs::read_to_string(&cache) {
        if let Ok(age) = serde_json::from_str::<FileAge>(&text) {
            return Ok(age);
        }
    }

    let out = std::process::Command::new("git")
        .args(["blame", "--line-porcelain", "--", file])
        .current_dir(root)
        .output()
        .context("git blame failed to start")?;
    if !out.status.success() {
        return Ok(FileAge {
            lines: Vec::new(),
            history: Some(format!(
                "git blame failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )),
        });
    }
    let age = FileAge {
        lines: parse_porcelain(&String::from_utf8_lossy(&out.stdout)),
        history: None,
    };
    if std::fs::create_dir_all(cache_dir(root)).is_ok() {
        let _ = std::fs::write(&cache, serde_json::to_string(&age).unwrap_or_default());
    }
    Ok(age)
}

/// Age of a structural range — the tree ring for one node.
pub fn node_age(root: &Path, file: &str, first_line: usize, last_line: usize) -> Result<NodeAge> {
    let age = file_age(root, file)?;
    let gap = |reason: String| NodeAge {
        file: file.to_string(),
        first_line,
        last_line,
        oldest_days: None,
        touched_days: None,
        commits: None,
        survival: None,
        history: Some(reason),
    };
    if let Some(reason) = &age.history {
        return Ok(gap(reason.clone()));
    }
    let in_range: Vec<&LineAge> = age
        .lines
        .iter()
        .filter(|l| l.line >= first_line && l.line <= last_line && l.author_time > 0)
        .collect();
    if in_range.is_empty() {
        return Ok(gap("no blame lines in range".to_string()));
    }
    let oldest = in_range.iter().map(|l| l.author_time).min().unwrap_or(0);
    let newest = in_range.iter().map(|l| l.author_time).max().unwrap_or(0);
    let mut per_commit: HashMap<&str, usize> = HashMap::new();
    for l in &in_range {
        *per_commit.entry(l.commit.as_str()).or_insert(0) += 1;
    }
    let oldest_share =
        in_range.iter().filter(|l| l.author_time == oldest).count() as f64 / in_range.len() as f64;
    let now = now_secs();
    Ok(NodeAge {
        file: file.to_string(),
        first_line,
        last_line,
        oldest_days: Some((now - oldest).max(0) / 86_400),
        touched_days: Some((now - newest).max(0) / 86_400),
        commits: Some(per_commit.len()),
        survival: Some((oldest_share * 100.0).round() / 100.0),
        history: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn porcelain_parsing_pairs_lines_with_author_time() {
        let sample = "\
aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa 1 1 2
author A
author-time 1600000000
\tfn first() {
bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb 2 2
author B
author-time 1700000000
\t}
";
        let parsed = parse_porcelain(sample);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].line, 1);
        assert_eq!(parsed[0].author_time, 1_600_000_000);
        assert_eq!(parsed[1].line, 2);
        assert_eq!(parsed[1].author_time, 1_700_000_000);
        assert_ne!(parsed[0].commit, parsed[1].commit);
    }

    #[test]
    fn a_missing_file_is_an_error_not_an_empty_ring() {
        let dir = tempfile::tempdir().unwrap();
        let err = file_age(dir.path(), "nope.rs").expect_err("missing file must be loud");
        assert!(err.to_string().contains("no such file"), "{err}");
    }
}
