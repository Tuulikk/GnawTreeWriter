//! Build script: embed git revision, build time and dirty-worktree flag into
//! the binary. Consumed by `src/version.rs` to report which code is running —
//! and to detect when the source is newer than the build (stale binary).
//!
//! §33 trap (seen in gnawdriver 3× and GTW 2026-10-09): building BEFORE the
//! commit bakes a stale rev into the binary. Watching .git/HEAD and .git/refs
//! directly makes cargo re-run this script when HEAD moves, so a post-commit
//! build always embeds the right sha.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    let git_sha = run("git", &["rev-parse", "--short", "HEAD"])
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=GTW_GIT_SHA={git_sha}");

    let dirty = run("git", &["status", "--porcelain"])
        .map(|out| !out.trim().is_empty())
        .unwrap_or(false);
    println!("cargo:rustc-env=GTW_GIT_DIRTY={dirty}");

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    println!("cargo:rustc-env=GTW_BUILT_AT={now}");
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}
