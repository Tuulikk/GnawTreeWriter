//! Attribution: voluntary, declared traceability — Git-like, no crypto.
//!
//! Model (owner, 2026-11-12):
//! * a CONTRIBUTOR is an identity with one or more ROLES (initiator, author,
//!   co-designer, reviewer, operator, responsible) — one identity is not
//!   duplicated per role;
//! * a git identity is a DEFAULT for who is accountable, never a claim about who
//!   executed: the tool can be run by anyone, and GTW is a tool, not an actor;
//! * `mechanism` (cli/mcp/batch/plugin) is the one thing GTW observes for
//!   certain — the caller cannot lie about which code path ran;
//! * `change_id` groups the transactions of ONE logical change across files;
//!   `task_id` sits above it (a task may hold several changes);
//! * model/agent/client are different things: a model is optional metadata, the
//!   MCP client is a handshake identity and need not be the agent platform;
//! * unknown stays unknown. Nothing here is inferred or repaired.
//!
//! Storage: `Transaction.metadata` keys, NOT a new struct field, so 983 old
//! journal rows keep deserialising and no library API breaks. The key names are
//! the contract; a typed field can be promoted later with `serde(default)`
//! without touching the accessors in this module.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// Process-wide declared attribution (set from CLI globals or the MCP
/// handshake). Write paths read it and FORCE `mechanism` to their own value, so
/// the one field GTW can observe is never taken from the caller's word.
///
/// A RwLock rather than a OnceLock because the MCP server learns its CLIENT
/// only at `initialize`, after the process has started.
static PROCESS_ATTRIBUTION: RwLock<Option<Attribution>> = RwLock::new(None);

pub fn set_process_attribution(a: Attribution) {
    if let Ok(mut slot) = PROCESS_ATTRIBUTION.write() {
        *slot = Some(a);
    }
}

/// Enrich the process context (e.g. the MCP client from the handshake) without
/// discarding what is already declared.
pub fn update_process_attribution(f: impl FnOnce(&mut Attribution)) {
    if let Ok(mut slot) = PROCESS_ATTRIBUTION.write() {
        let mut current = slot.take().unwrap_or_default();
        f(&mut current);
        *slot = Some(current);
    }
}

pub fn process_attribution() -> Attribution {
    PROCESS_ATTRIBUTION
        .read()
        .ok()
        .and_then(|s| s.clone())
        .unwrap_or_default()
}

/// The attribution a given code path must record. `mechanism` comes from HERE,
/// never from the caller.
pub fn for_path(mechanism: &str) -> Attribution {
    let mut a = process_attribution();
    a.mechanism = Some(mechanism.to_string());
    a
}

pub const CONTRIBUTORS: &str = "contributors";
pub const MECHANISM: &str = "mechanism";
pub const CLIENT: &str = "client";
pub const CLIENT_VERSION: &str = "client_version";
pub const CHANGE_ID: &str = "change_id";
pub const TASK_ID: &str = "task_id";
pub const MODEL: &str = "model";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContributorKind {
    Human,
    Agent,
    Model,
    Organization,
    /// Declared but unclassified. The honest default.
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Asked for the change.
    Initiator,
    /// Wrote it.
    Author,
    /// Took part in the design.
    CoDesigner,
    Reviewed,
    /// Ran the tool.
    Operator,
    /// Accountable for it.
    Responsible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Contributor {
    /// GitHub-ish handle, name or agent id. Absent when only a role is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default)]
    pub kind: ContributorKind,
    /// At least one. A contributor with several roles appears ONCE.
    #[serde(default)]
    pub roles: Vec<Role>,
    /// Optional: the model that assisted this contributor. Never implies
    /// authorship — a model can be attached to a reviewer too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Attribution {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributors: Vec<Contributor>,
    /// cli | mcp | batch | plugin. OBSERVED by the code path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mechanism: Option<String>,
    /// MCP handshake identity (self-reported by the client).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// Free-standing model note when it is not tied to one contributor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl Contributor {
    pub fn new(id: &str, kind: ContributorKind, roles: &[Role]) -> Self {
        Self {
            id: Some(id.to_string()),
            kind,
            roles: roles.to_vec(),
            model: None,
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = Some(model.to_string());
        self
    }

    pub fn has_role(&self, role: Role) -> bool {
        self.roles.contains(&role)
    }
}

impl Attribution {
    pub fn is_empty(&self) -> bool {
        self.contributors.is_empty()
            && self.mechanism.is_none()
            && self.client.is_none()
            && self.change_id.is_none()
            && self.task_id.is_none()
            && self.model.is_none()
    }

    /// Read the attribution out of a transaction's metadata.
    ///
    /// Unknown stays unknown: a missing key is `None`, never a default string.
    /// A `contributors` value that does not parse is DROPPED with a warning —
    /// inventing contributors would be worse than losing them.
    pub fn from_metadata(meta: &HashMap<String, String>) -> Self {
        let contributors = match meta.get(CONTRIBUTORS) {
            Some(raw) => match serde_json::from_str::<Vec<Contributor>>(raw) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "⚠️  attribution: {CONTRIBUTORS} did not parse ({e}) — treating as unknown"
                    );
                    Vec::new()
                }
            },
            None => Vec::new(),
        };
        Self {
            contributors,
            mechanism: meta.get(MECHANISM).cloned(),
            client: meta.get(CLIENT).cloned(),
            client_version: meta.get(CLIENT_VERSION).cloned(),
            change_id: meta.get(CHANGE_ID).cloned(),
            task_id: meta.get(TASK_ID).cloned(),
            model: meta.get(MODEL).cloned(),
        }
    }

    /// Write the attribution into metadata. Empty fields are NOT written, so an
    /// absent key keeps meaning "unknown" instead of an empty string.
    pub fn apply_to(&self, meta: &mut HashMap<String, String>) {
        if !self.contributors.is_empty() {
            if let Ok(json) = serde_json::to_string(&self.contributors) {
                meta.insert(CONTRIBUTORS.to_string(), json);
            }
        }
        for (key, value) in [
            (MECHANISM, &self.mechanism),
            (CLIENT, &self.client),
            (CLIENT_VERSION, &self.client_version),
            (CHANGE_ID, &self.change_id),
            (TASK_ID, &self.task_id),
            (MODEL, &self.model),
        ] {
            if let Some(v) = value {
                if !v.is_empty() {
                    meta.insert(key.to_string(), v.clone());
                }
            }
        }
    }

    /// Merge a caller context in: contributors are combined WITHOUT duplicating
    /// an identity that already holds the same roles (one identity, several
    /// roles), and `other` wins for the single-valued fields it states.
    pub fn merged_with(&self, other: &Attribution) -> Attribution {
        let mut out = self.clone();
        for c in &other.contributors {
            match out
                .contributors
                .iter_mut()
                .find(|e| e.id == c.id && e.kind == c.kind)
            {
                Some(existing) => {
                    for r in &c.roles {
                        if !existing.roles.contains(r) {
                            existing.roles.push(*r);
                        }
                    }
                    if existing.model.is_none() {
                        existing.model = c.model.clone();
                    }
                }
                None => out.contributors.push(c.clone()),
            }
        }
        out.mechanism = other.mechanism.clone().or(out.mechanism);
        out.client = other.client.clone().or(out.client);
        out.client_version = other.client_version.clone().or(out.client_version);
        out.change_id = other.change_id.clone().or(out.change_id);
        out.task_id = other.task_id.clone().or(out.task_id);
        out.model = other.model.clone().or(out.model);
        out
    }

    /// One terse line for ALF / onion / logs — parseable, no LLM needed.
    /// `tuulikk[human:initiator,responsible] + cline[agent:author] via mcp change=c1`
    pub fn terse(&self) -> String {
        let names: Vec<String> = self
            .contributors
            .iter()
            .map(|c| {
                let roles: Vec<&str> = c.roles.iter().map(Role::as_str).collect();
                let model = c
                    .model
                    .as_deref()
                    .map(|m| format!("{{{m}}}"))
                    .unwrap_or_default();
                format!(
                    "{}[{}:{}]{}",
                    c.id.as_deref().unwrap_or("?"),
                    // Readable, not Debug: this string is a user-facing surface
                    // (head --trace, ALF, logs), so `unknown` not `Unknown`.
                    match c.kind {
                        ContributorKind::Human => "human",
                        ContributorKind::Agent => "agent",
                        ContributorKind::Model => "model",
                        ContributorKind::Organization => "org",
                        ContributorKind::Unknown => "unknown",
                    },
                    roles.join(","),
                    model
                )
            })
            .collect();
        let mut parts: Vec<String> = Vec::new();
        if !names.is_empty() {
            parts.push(names.join(" + "));
        }
        if let Some(m) = &self.mechanism {
            parts.push(format!("via {m}"));
        }
        if let Some(c) = &self.change_id {
            parts.push(format!("change={c}"));
        }
        if let Some(t) = &self.task_id {
            parts.push(format!("task={t}"));
        }
        parts.join(" ")
    }
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Initiator => "initiator",
            Role::Author => "author",
            Role::CoDesigner => "co_designer",
            Role::Reviewed => "reviewer",
            Role::Operator => "operator",
            Role::Responsible => "responsible",
        }
    }

    /// Unknown role strings stay unknown (None) — never coerced into `Author`.
    pub fn parse(s: &str) -> Option<Role> {
        match s.trim().to_ascii_lowercase().as_str() {
            "initiator" => Some(Role::Initiator),
            "author" => Some(Role::Author),
            "co_designer" | "codesigner" | "co-designer" => Some(Role::CoDesigner),
            "reviewer" | "review" => Some(Role::Reviewed),
            "operator" => Some(Role::Operator),
            "responsible" => Some(Role::Responsible),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_identity_can_hold_several_roles_without_duplication() {
        let a = Attribution {
            contributors: vec![Contributor::new(
                "tuulikk",
                ContributorKind::Human,
                &[Role::Initiator],
            )],
            ..Default::default()
        };
        let b = Attribution {
            contributors: vec![Contributor::new(
                "tuulikk",
                ContributorKind::Human,
                &[Role::Responsible, Role::Initiator],
            )],
            ..Default::default()
        };
        let merged = a.merged_with(&b);
        assert_eq!(merged.contributors.len(), 1, "one identity, one entry");
        assert!(merged.contributors[0].has_role(Role::Initiator));
        assert!(merged.contributors[0].has_role(Role::Responsible));
    }

    #[test]
    fn metadata_roundtrip_keeps_unknown_unknown() {
        let a = Attribution {
            contributors: vec![
                Contributor::new("cline", ContributorKind::Agent, &[Role::Author])
                    .with_model("gpt-5"),
            ],
            mechanism: Some("mcp".to_string()),
            change_id: Some("c1".to_string()),
            ..Default::default()
        };
        let mut meta = HashMap::new();
        a.apply_to(&mut meta);
        // Absent fields are absent, not empty strings.
        assert!(!meta.contains_key(TASK_ID));
        assert!(!meta.contains_key(CLIENT));

        let back = Attribution::from_metadata(&meta);
        assert_eq!(back, a);

        // A row with no attribution at all is EMPTY, never guessed.
        let empty = Attribution::from_metadata(&HashMap::new());
        assert!(empty.is_empty());
        assert!(empty.terse().is_empty());
    }

    #[test]
    fn unparseable_contributors_are_dropped_not_invented() {
        let mut meta = HashMap::new();
        meta.insert(CONTRIBUTORS.to_string(), "{not json".to_string());
        meta.insert(MECHANISM.to_string(), "cli".to_string());
        let a = Attribution::from_metadata(&meta);
        assert!(
            a.contributors.is_empty(),
            "garbage in must not become people"
        );
        assert_eq!(a.mechanism.as_deref(), Some("cli"), "other keys survive");

        assert_eq!(Role::parse("reviewer"), Some(Role::Reviewed));
        assert_eq!(Role::parse("nonsense"), None, "unknown role stays unknown");
    }
}
