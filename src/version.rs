//! Version & build info: which code is running, and is it stale?
//!
//! `build.rs` embeds the git revision, build time (epoch seconds) and a
//! dirty-worktree flag at compile time. At runtime we compare against the
//! *current* source: if git HEAD moved, or any source file is newer than the
//! build, there are changes not yet in the running binary.
//!
//! Why this exists (the pain it kills): GTW's MCP binary is process-bound —
//! every opencode session loads it at host start and keeps that build until
//! the next restart. Without stale reporting, an agent silently works against
//! old tooling (seen 2026-10-09: gnawdriver's stale guard caught its own case;
//! GTW had no such guard). `doctor` surfaces `build` so one call answers both
//! "is GTW healthy?" and "is GTW current?".

use std::time::UNIX_EPOCH;

/// Build info embedded at compile time (via build.rs).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BuildInfo {
    /// Short git revision (HEAD at build time).
    pub git_sha: &'static str,
    /// Build time as UTC string.
    pub built_at: String,
    /// Did the worktree have uncommitted changes at build time?
    pub dirty: bool,
    /// Is the source newer than the build? (runtime check)
    pub stale: bool,
}

/// Current build info, with a live staleness check against the repo.
pub fn current() -> BuildInfo {
    let git_sha: &'static str = match option_env!("GTW_GIT_SHA") {
        Some(s) => s,
        None => "unknown",
    };
    let built_epoch: u64 = option_env!("GTW_BUILT_AT")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let dirty = option_env!("GTW_GIT_DIRTY").map(|s| s == "true").unwrap_or(false);

    BuildInfo {
        git_sha,
        built_at: format_epoch_utc(built_epoch),
        dirty,
        stale: is_stale(git_sha, built_epoch),
    }
}

/// True when the running binary does not match the current source:
/// the repo HEAD moved since the build, or a `src/` file is newer than
/// the build. Both comparisons are advisory — read errors count as
/// "cannot prove stale" (false), never as healthy lies about freshness.
fn is_stale(built_sha: &str, built_epoch: u64) -> bool {
    if built_sha == "unknown" || built_epoch == 0 {
        return true;
    }
    let head_moved = run_git(&["rev-parse", "--short", "HEAD"])
        .map(|rev| rev != built_sha)
        .unwrap_or(false);
    let src_newer = src_newer_than(built_epoch).unwrap_or(false);
    head_moved || src_newer
}

/// Newest mtime across `src/` and `build.rs`, relative to `built_epoch`.
/// `None` = cannot read the tree (advisory check gives up silently).
fn src_newer_than(built_epoch: u64) -> Option<bool> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let mut newest = built_epoch;
    let mut found_any = false;
    let src_dir = std::path::Path::new(&manifest).join("src");
    walk(&src_dir, &mut newest, &mut found_any);
    if let Ok(md) = std::fs::metadata(std::path::Path::new(&manifest).join("build.rs")) {
        if let Ok(m) = md.modified() {
            found_any = true;
            if let Ok(s) = m.duration_since(UNIX_EPOCH) {
                newest = newest.max(s.as_secs());
            }
        }
    }
    if !found_any {
        return None;
    }
    Some(newest > built_epoch)
}

fn walk(dir: &std::path::Path, newest: &mut u64, found_any: &mut bool) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            walk(&entry.path(), newest, found_any);
        } else if let Ok(m) = entry.metadata().and_then(|md| md.modified()) {
            *found_any = true;
            if let Ok(s) = m.duration_since(UNIX_EPOCH) {
                *newest = (*newest).max(s.as_secs());
            }
        }
    }
}

fn run_git(args: &[&str]) -> Option<String> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(manifest)
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Epoch seconds → `YYYY-MM-DD HH:MM UTC` (same format as gnawdriver).
/// Uses chrono — a hand-rolled civil-calendar algorithm was tried here and
/// was off by three days (caught by the unit test before it shipped); the
/// proven library is the whole point of the dependency.
pub fn format_epoch_utc(secs: u64) -> String {
    use chrono::TimeZone;
    match chrono::Utc.timestamp_opt(secs as i64, 0).single() {
        Some(dt) => dt.format("%Y-%m-%d %H:%M UTC").to_string(),
        None => format!("epoch {secs}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_formatting_matches_known_instants() {
        assert_eq!(format_epoch_utc(0), "1970-01-01 00:00 UTC");
        // 2026-10-09 15:26:10 UTC (the test-env deploy of rev 97aaa77).
        assert_eq!(format_epoch_utc(1_791_559_570), "2026-10-09 15:26 UTC");
    }

    #[test]
    fn unknown_sha_is_stale_by_construction() {
        // A binary without embedded build info cannot prove freshness —
        // the honest reading is stale, never a silent "healthy" lie.
        assert!(is_stale("unknown", 0));
        assert!(is_stale("unknown", 1_791_559_570));
    }

    #[test]
    fn fresh_binary_with_matching_head_and_old_src_is_not_stale() {
        // We cannot control the real repo state in a unit test, but the
        // semantic contract is: head unchanged + no src newer than build
        // => stale == false. The real repo at test time IS the built HEAD
        // with no newer src right after a commit-build cycle, so assert
        // current().stale reflects exactly that contract on this machine.
        let info = current();
        if info.git_sha != "unknown" {
            // Right after a commit-build the binary is fresh; if someone
            // edited src/ since the last build this assert legitimately
            // fails and tells them to rebuild.
            assert!(
                !info.stale,
                "binary reports stale (sha {}, built {}) — rebuild via scripts/commit-build.sh",
                info.git_sha, info.built_at
            );
        }
    }
}
