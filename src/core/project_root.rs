//! Project-root sanity: is this directory a project, or something too broad?
//!
//! Field findings, 2026-11-10, all from one wrong root:
//!   - an MCP daemon started with `cwd=$HOME` answered `sense` with hits from
//!     `~/AndroidSDK`, reported the caller's own store as absent, and degraded
//!     the semantic channel;
//!   - a daemon sat in `.../Gnaw-Software`, a folder holding six repos, and
//!     would have walked all of them;
//!   - `doctor`'s staleness check quietly reported "fresh" because its git call
//!     ran outside the repo.
//!
//! Lives in `core` (not in the MCP module) so the CLI and the MCP tools apply
//! the SAME rule: one predicate, one place. The CLI could index `$HOME` while
//! the MCP tools refused — a drift this module exists to prevent.

use std::path::{Path, PathBuf};

/// Files/directories that make a directory a project root.
pub const PROJECT_MARKERS: &[&str] = &[
    ".git",
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
    "gnawtreewriter.yaml",
];

/// The current user's home directory, used only to detect an unusable root.
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

fn normalize(p: &Path) -> PathBuf {
    if let Ok(c) = std::fs::canonicalize(p) {
        return c;
    }
    // Relative paths that do not exist yet resolve against the cwd.
    if p.is_relative() {
        if let Ok(cwd) = std::env::current_dir() {
            if let Ok(c) = std::fs::canonicalize(cwd.join(p)) {
                return c;
            }
        }
    }
    p.to_path_buf()
}

/// A directory that is not a project itself but whose subdirectories are mostly
/// projects is a workspace container (`~/dev`, `.../Gnaw-Software`).
///
/// "Mostly projects" is required, not just "two look like projects": `/tmp` and
/// other junk drawers contain a couple of build dirs and must stay usable (the
/// first version of this rule refused `/tmp` — caught by a test).
pub fn looks_like_workspace_container(root: &Path) -> bool {
    if PROJECT_MARKERS.iter().any(|m| root.join(m).exists()) {
        return false; // a project (or a monorepo root) — its own business
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return false;
    };
    let mut dirs = 0;
    let mut projects = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue; // .git, .cache, … say nothing about the layout
        }
        dirs += 1;
        if PROJECT_MARKERS.iter().any(|m| path.join(m).exists()) {
            projects += 1;
        }
    }
    projects >= 2 && projects * 2 >= dirs
}

/// Is `root` too broad to be treated as ONE project?
///
/// `home` is passed in (rather than read here) so the rule is testable without
/// touching the environment.
pub fn root_is_too_broad(root: &Path, home: Option<&Path>) -> bool {
    let root = normalize(root);
    if root.parent().is_none() {
        return true; // "/" — everything is under it
    }
    if let Some(h) = home {
        if normalize(h) == root {
            return true; // the home directory
        }
    }
    looks_like_workspace_container(&root)
}

/// Why `root` must not be used, or `Ok(())` when it is a usable project root.
///
/// Callers add their own advice ("pass a root", "start the client here") on top
/// of this: what is wrong is the same everywhere, what to do is tool-specific.
pub fn guard(root: &Path) -> Result<(), String> {
    if root_is_too_broad(root, home_dir().as_deref()) {
        Err(format!(
            "refusing to use {} as a project root — it is the home directory, the filesystem \
             root, or a folder that holds several projects, so any store, index or lexical walk \
             under it covers unrelated trees (a $HOME-rooted server once answered a sense query \
             with hits from ~/AndroidSDK)",
            root.display()
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_and_filesystem_root_are_too_broad() {
        let home = Path::new("/home/nonexistent-user");
        assert!(root_is_too_broad(home, Some(home)));
        assert!(root_is_too_broad(Path::new("/"), None));
    }

    #[test]
    fn a_project_directory_is_not_too_broad() {
        let home = Path::new("/home/nonexistent-user");
        // A junk drawer (like /tmp): several ordinary dirs plus ONE project.
        let dir = tempfile::tempdir().unwrap();
        for name in ["scratch", "logs", "builds", "one-project"] {
            std::fs::create_dir_all(dir.path().join(name)).unwrap();
        }
        std::fs::write(dir.path().join("one-project").join("Cargo.toml"), "").unwrap();
        assert!(
            !root_is_too_broad(dir.path(), Some(home)),
            "a junk drawer holding one project must stay usable"
        );
        assert!(!root_is_too_broad(
            &dir.path().join("one-project"),
            Some(home)
        ));
        assert!(!root_is_too_broad(
            Path::new("/home/nonexistent-user/proj"),
            Some(home)
        ));
    }

    #[test]
    fn a_workspace_holding_several_projects_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path().join("dev");
        std::fs::create_dir_all(ws.join("alpha")).unwrap();
        std::fs::create_dir_all(ws.join("beta")).unwrap();
        std::fs::write(ws.join("alpha").join("Cargo.toml"), "").unwrap();
        std::fs::write(ws.join("beta").join("package.json"), "{}").unwrap();
        assert!(
            root_is_too_broad(&ws, None),
            "a folder holding two projects is a workspace, not a project"
        );

        // A project of its own stays usable even when it contains others
        // (monorepo): the marker on the ROOT is what decides.
        std::fs::write(ws.join(".git"), "").unwrap();
        assert!(
            !root_is_too_broad(&ws, None),
            "a repo root is its own project, whatever it contains"
        );

        // One child project is not a workspace.
        let single = dir.path().join("single");
        std::fs::create_dir_all(single.join("only")).unwrap();
        std::fs::write(single.join("only").join("Cargo.toml"), "").unwrap();
        assert!(!root_is_too_broad(&single, None));
    }

    #[test]
    fn guard_explains_itself() {
        let msg = guard(Path::new("/")).expect_err("the filesystem root must be refused");
        assert!(msg.contains("refusing to use"), "{msg}");
        assert!(
            msg.contains("unrelated trees"),
            "the reason must be stated: {msg}"
        );
    }
}
