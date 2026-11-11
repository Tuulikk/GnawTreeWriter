//! PARKED 2026-11-11: the core is written and reviewed; the CLI/MCP wiring is
//! NOT applied.
//!
//! **The saved patch (`docs/wip/experiment-cli-wiring.patch`) is BROKEN — do not
//! trust it.** Proved 2026-11-11: its third hunk anchors on
//! `GTW_CODE_INDEX_SOURCE=git_hook …`, which lives INSIDE a raw string literal
//! in `src/cli.rs`, so applying it inserts 161 lines of Rust into that string
//! and the file stops parsing (`unexpected closing delimiter`, cli.rs:6544).
//! Add the enum variant, the dispatch arm and the handler as three separate
//! AST-verified steps instead.
//!
//! The module itself now compiles as part of the crate (registered in
//! `core/mod.rs`) and its structural defect is fixed: `git()` had lost its tail
//! (`Ok(...)` + closing brace), which nested `ensure_disposable`, `start`,
//! `latest_dir`, `load` and `append` inside it. Braces balanced, so `heal` said
//! "parses cleanly" — only the compiler could see the mis-nesting.

//! Destructive debugging: isolate a fault by gating code out, with a journal.
//!
//! The owner's own method, made safe: destroy deliberately, observe, restore —
//! and never lose the thread. GTW's job is what a human does badly at scale:
//! proving the workspace is disposable, proving the restore is byte-identical,
//! and recording what was tried (`docs/DESTRUCTIVE_DEBUGGING_ASSESSMENT.md`).
//!
//! Division of labour (decided 2026-11-11):
//!   - STRUCTURE  GTW measures it: node present, file parses, node counts.
//!   - BEHAVIOUR  the agent observes it and reports (`record --outcome`).
//!   - APPEARANCE nobody. Free text only; GTW never grades looks.
//!
//! Safety rules, in order of severity:
//!   1. the workspace must be a LINKED git worktree (`--allow-primary` overrides)
//!   2. its tree must be clean — never start on top of someone's work
//!   3. restore = `git checkout -- .` + hash comparison, never "the model
//!      remembers what it changed"
//!   4. every disable is syntax-validated before it is written

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// How the next candidate set is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    /// One node at a time — the careful mode.
    Single,
    /// Halve the candidate set each round — mechanical, needs stable outcomes.
    Bisect,
}

impl Strategy {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "single" => Ok(Strategy::Single),
            "bisect" => Ok(Strategy::Bisect),
            other => Err(anyhow!(
                "unknown strategy '{other}' — use 'single' or 'bisect'"
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub max_runs: usize,
    pub max_seconds: u64,
}

impl Default for Budget {
    fn default() -> Self {
        Budget {
            max_runs: 12,
            max_seconds: 900,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub id: String,
    pub workspace: String,
    pub symptom: String,
    pub strategy: Strategy,
    pub created_at: String,
    /// HEAD of the worktree when the experiment started — the restore anchor.
    pub head_sha: String,
    /// True when the target is the repository's PRIMARY worktree (allowed, but
    /// stated, because that is where the user's own work lives).
    pub primary_worktree: bool,
    pub budget: Budget,
    pub candidates: Vec<String>,
}

/// What GTW measured itself — always `verified`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Structure {
    pub file: String,
    pub file_hash: String,
    pub parse_ok: bool,
    pub nodes: usize,
    pub node_present: bool,
    /// Which form was addressed, so a later agent can re-verify at the same
    /// granularity.
    pub form: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    Disabled {
        at: String,
        nodes: Vec<String>,
        before: Structure,
        after: Structure,
        verified: bool,
    },
    Recorded {
        at: String,
        outcome: String,
        evidence: String,
        exit: Option<i32>,
        reported_by: String,
        /// GTW's own verdict when the claim and the structure disagree.
        contradiction: Option<String>,
    },
    Restored {
        at: String,
        /// Byte-identical restore proved by hashing the file before/after.
        verified: bool,
        file_hash: String,
    },
}

#[derive(Debug, Clone)]
pub struct Journal {
    pub dir: PathBuf,
    pub header: Header,
    pub events: Vec<Event>,
}

pub fn now_utc() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub(crate) fn git(workspace: &Path, args: &[&str]) -> Result<String> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(workspace)
        .output()
        .with_context(|| format!("git {} failed to start", args.join(" ")))?;
    if !out.status.success() {
        return Err(anyhow!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Refuse to run anywhere the user's work could be lost.
///
/// Returns whether the target is the PRIMARY worktree (allowed only with an
/// explicit override).
pub fn ensure_disposable(workspace: &Path) -> Result<bool> {
    if !workspace.is_dir() {
        return Err(anyhow!("workspace does not exist: {}", workspace.display()));
    }
    let toplevel = git(workspace, &["rev-parse", "--show-toplevel"])
        .map_err(|_| anyhow!("{} is not a git repository", workspace.display()))?;
    let top = std::fs::canonicalize(&toplevel).unwrap_or_else(|_| PathBuf::from(&toplevel));
    let ws = std::fs::canonicalize(workspace).unwrap_or_else(|_| workspace.to_path_buf());
    if top != ws {
        return Err(anyhow!(
            "{} is inside the repo {} — point --workspace at the worktree root",
            ws.display(),
            top.display()
        ));
    }
    let dirty = git(workspace, &["status", "--porcelain"])?;
    if !dirty.is_empty() {
        return Err(anyhow!(
            "workspace has uncommitted changes — an experiment must start from a clean tree:\n{}",
            dirty
        ));
    }
    // A linked worktree has ".git" as a FILE (a gitdir pointer); the primary
    // worktree has it as a directory.
    Ok(workspace.join(".git").is_dir())
}

pub fn start(
    workspace: &Path,
    symptom: &str,
    strategy: Strategy,
    candidates: Vec<String>,
    allow_primary: bool,
    budget: Budget,
) -> Result<Header> {
    let primary = ensure_disposable(workspace)?;
    if primary && !allow_primary {
        return Err(anyhow!(
            "{} is the repository's PRIMARY worktree — that is where your own work lives.\n\
             Create a disposable one first:  git worktree add /tmp/gtw-exp-1\n\
             (or pass --allow-primary if you really mean it)",
            workspace.display()
        ));
    }
    let head_sha = git(workspace, &["rev-parse", "--short", "HEAD"])?;
    let header = Header {
        id: format!("exp-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S")),
        workspace: workspace.display().to_string(),
        symptom: symptom.to_string(),
        strategy,
        created_at: now_utc(),
        head_sha,
        primary_worktree: primary,
        budget,
        candidates,
    };
    let dir = journal_root(workspace).join(&header.id);
    std::fs::create_dir_all(&dir)?;
    let line = serde_json::to_string(&header)?;
    std::fs::write(dir.join("journal.jsonl"), format!("{line}\n"))?;
    Ok(header)
}

fn latest_dir(workspace: &Path) -> Result<PathBuf> {
    let root = journal_root(workspace);
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(&root)
        .map_err(|_| anyhow!("no experiments recorded in {}", workspace.display()))?
        .flatten()
    {
        let path = entry.path();
        if !path.is_dir() || !path.join("journal.jsonl").exists() {
            continue;
        }
        let mtime = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        if best.as_ref().map(|(t, _)| mtime > *t).unwrap_or(true) {
            best = Some((mtime, path));
        }
    }
    best.map(|(_, p)| p)
        .ok_or_else(|| anyhow!("no experiments recorded in {}", workspace.display()))
}

pub fn load(workspace: &Path) -> Result<Journal> {
    let dir = latest_dir(workspace)?;
    let text = std::fs::read_to_string(dir.join("journal.jsonl"))?;
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header: Header = serde_json::from_str(
        lines
            .next()
            .ok_or_else(|| anyhow!("journal is empty: {}", dir.display()))?,
    )?;
    let mut events = Vec::new();
    for line in lines {
        events.push(serde_json::from_str(line).with_context(|| {
            format!(
                "unreadable event in {}: {line}",
                dir.join("journal.jsonl").display()
            )
        })?);
    }
    Ok(Journal {
        dir,
        header,
        events,
    })
}

pub fn append(journal: &Journal, event: &Event) -> Result<()> {
    use std::io::Write;
    let path = journal.dir.join("journal.jsonl");
    let mut f = std::fs::OpenOptions::new().append(true).open(&path)?;
    writeln!(f, "{}", serde_json::to_string(event)?)?;
    Ok(())
}

pub(crate) fn file_hash(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(crate::core::transaction_log::calculate_content_hash(
        &String::from_utf8_lossy(&bytes),
    ))
}

pub fn journal_root(workspace: &Path) -> PathBuf {
    workspace.join(".gnawtreewriter_ai").join("experiments")
}

/// Measure the structure of `node_path` in `file` — GTW's own (verified) half of
/// the observation. Never fails on a broken file: a file that no longer parses
/// is a *finding* (`parse_ok: false`), not an error.
pub fn structure_of(workspace: &Path, file: &str, node_path: &str) -> Result<Structure> {
    let path = workspace.join(file);
    let hash = file_hash(&path).unwrap_or_else(|_| "missing".to_string());
    match crate::core::GnawTreeWriter::new(&path.display().to_string()) {
        Ok(writer) => Ok(Structure {
            file: file.to_string(),
            file_hash: hash,
            parse_ok: true,
            nodes: writer.node_count(),
            node_present: writer.node_exists(node_path),
            form: "node_path".to_string(),
        }),
        Err(_) => Ok(Structure {
            file: file.to_string(),
            file_hash: hash,
            parse_ok: false,
            nodes: 0,
            node_present: false,
            form: "node_path".to_string(),
        }),
    }
}

/// Gate a Rust item out with `#[cfg(any())]`.
///
/// Syntax-valid by construction (the attribute is a legal item prefix), so the
/// rest of the file still parses and a later `restore` can prove itself. v1 is
/// deliberately Rust-only: other languages need their own neutral form, and a
/// guess would silently produce a *different* experiment than the one recorded.
pub fn disable(journal: &Journal, file: &str, nodes: &[String]) -> Result<Event> {
    if !file.to_ascii_lowercase().ends_with(".rs") {
        return Err(anyhow!(
            "v1 gates Rust items only (`#[cfg(any())]`); '{file}' is not a .rs file"
        ));
    }
    let first = nodes
        .first()
        .ok_or_else(|| anyhow!("no nodes given — pass at least one node path"))?;
    let ws = PathBuf::from(&journal.header.workspace);
    let path = ws.join(file);
    let before = structure_of(&ws, file, first)?;

    let mut writer = crate::core::GnawTreeWriter::new(&path.display().to_string())?;
    for node in nodes {
        let text = writer
            .show_node(node)
            .map_err(|e| anyhow!("node {node} is not addressable: {e}"))?;
        if text.trim_start().starts_with("#[cfg(any())]") {
            return Err(anyhow!("node {node} is already gated"));
        }
        let gated = format!("#[cfg(any())]\n{text}");
        writer
            .edit(
                crate::core::EditOperation::Edit {
                    node_path: node.clone(),
                    content: gated,
                },
                false,
            )
            .map_err(|e| {
                anyhow!("gating {node} was rejected (syntax is checked before write): {e}")
            })?;
    }

    let after = structure_of(&ws, file, first)?;
    // Gating keeps the item in the AST (tree-sitter still parses it), so the
    // proof that the disable took is the file changing.
    let verified = after.file_hash != before.file_hash;
    let event = Event::Disabled {
        at: now_utc(),
        nodes: nodes.to_vec(),
        before,
        after,
        verified,
    };
    append(journal, &event)?;
    Ok(event)
}

/// Record what the AGENT observed. GTW cannot check behaviour; it can check
/// whether the claim is compatible with what the structure says.
pub fn record(
    journal: &Journal,
    outcome: &str,
    evidence: &str,
    exit: Option<i32>,
) -> Result<Event> {
    let known = ["gone", "same", "changed", "other"];
    if !known.contains(&outcome) {
        return Err(anyhow!(
            "unknown outcome '{outcome}' — use one of: {}",
            known.join(", ")
        ));
    }
    let contradiction = match (outcome, last_disabled(journal)) {
        ("gone", Some((before, after))) if before.file_hash == after.file_hash => {
            Some("inconclusive_not_disabled".to_string())
        }
        (_, Some((_, after))) if !after.parse_ok => Some("changed_more_than_reported".to_string()),
        _ => None,
    };
    let event = Event::Recorded {
        at: now_utc(),
        outcome: outcome.to_string(),
        evidence: evidence.to_string(),
        exit,
        reported_by: "agent".to_string(),
        contradiction,
    };
    append(journal, &event)?;
    Ok(event)
}

/// Undo every gate: `git checkout -- .`, then PROVE it with the hash from the
/// first disable. Never relies on the model remembering anything.
pub fn restore(journal: &Journal) -> Result<Event> {
    let ws = PathBuf::from(&journal.header.workspace);
    let want = journal.events.iter().find_map(|e| match e {
        Event::Disabled { before, .. } => Some((before.file.clone(), before.file_hash.clone())),
        _ => None,
    });
    git(&ws, &["checkout", "--", "."])?;
    let (file, hash) = want.unwrap_or_default();
    let now_hash = if file.is_empty() {
        String::new()
    } else {
        file_hash(&ws.join(&file))?
    };
    let verified = !file.is_empty() && now_hash == hash;
    let event = Event::Restored {
        at: now_utc(),
        verified,
        file_hash: now_hash,
    };
    append(journal, &event)?;
    Ok(event)
}

/// Which candidates are still in play, given the outcomes so far.
///
/// `gone` means the culprit is inside the gated set; `same` means outside;
/// `changed` is treated as `gone` (the symptom responded) so the caller can
/// refine rather than restart.
pub fn narrow(candidates: &[String], events: &[Event]) -> Vec<String> {
    let mut active: Vec<String> = candidates.to_vec();
    let mut last: Option<Vec<String>> = None;
    for event in events {
        match event {
            Event::Disabled { nodes, .. } => last = Some(nodes.clone()),
            Event::Recorded { outcome, .. } => {
                if let Some(gated) = last.take() {
                    match outcome.as_str() {
                        "gone" | "changed" => {
                            let inside: Vec<String> = gated
                                .iter()
                                .filter(|n| active.contains(n))
                                .cloned()
                                .collect();
                            active = if inside.is_empty() { gated } else { inside };
                        }
                        "same" => active.retain(|n| !gated.contains(n)),
                        _ => {}
                    }
                }
            }
            Event::Restored { .. } => {}
        }
    }
    active
}

/// The next set to gate: one node (`single`) or the first half (`bisect`).
pub fn next_batch(journal: &Journal) -> (Vec<String>, Vec<String>) {
    let active = narrow(&journal.header.candidates, &journal.events);
    let take = match journal.header.strategy {
        Strategy::Single => 1,
        Strategy::Bisect => active.len().div_ceil(2).max(1),
    };
    (active.iter().take(take).cloned().collect(), active)
}

fn last_disabled(journal: &Journal) -> Option<(&Structure, &Structure)> {
    journal.events.iter().rev().find_map(|e| match e {
        Event::Disabled { before, after, .. } => Some((before, after)),
        _ => None,
    })
}

/// The handover report: what GTW verified, what the agent reported, and what
/// remains uncertain. The two kinds of fact are never mixed.
pub fn report(journal: &Journal) -> serde_json::Value {
    let mut experiments = Vec::new();
    let mut runs = 0usize;
    for event in &journal.events {
        if let Event::Disabled {
            nodes,
            before,
            after,
            verified,
            ..
        } = event
        {
            runs += 1;
            experiments.push(serde_json::json!({
                "n": runs,
                "gated": nodes,
                "structure": {
                    "verified": verified,
                    "file": before.file,
                    "form": before.form,
                    "file_changed": before.file_hash != after.file_hash,
                    "parse_ok": after.parse_ok,
                    "nodes_before": before.nodes,
                    "nodes_after": after.nodes,
                },
            }));
        }
    }
    let behavioural: Vec<serde_json::Value> = journal
        .events
        .iter()
        .filter_map(|e| match e {
            Event::Recorded {
                outcome,
                evidence,
                exit,
                contradiction,
                ..
            } => Some(serde_json::json!({
                "outcome": outcome,
                "reported_by": "agent",
                "evidence": evidence,
                "exit": exit,
                "contradiction": contradiction,
            })),
            _ => None,
        })
        .collect();
    let active = narrow(&journal.header.candidates, &journal.events);
    let restore_verified = journal.events.iter().rev().find_map(|e| match e {
        Event::Restored { verified, .. } => Some(*verified),
        _ => None,
    });
    serde_json::json!({
        "experiment": journal.header.id,
        "workspace": journal.header.workspace,
        "symptom": journal.header.symptom,
        "strategy": journal.header.strategy,
        "head_sha": journal.header.head_sha,
        "runs": runs,
        "max_runs": journal.header.budget.max_runs,
        "structure": experiments,
        "behaviour": behavioural,
        "minimal_scope": if active.len() == 1 { serde_json::json!(active) } else { serde_json::Value::Null },
        "remaining_candidates": active,
        "restore_verified": restore_verified,
    })
}
