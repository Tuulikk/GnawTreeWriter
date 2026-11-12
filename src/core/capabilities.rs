//! What needs a model and what does not — one static table, no model required
//! to read it.
//!
//! The question "is this done with or without ModernBERT?" must be answerable
//! from the binary (user, 2026-11-11: *svårt för mig att sära på vad som görs
//! med och utan*). A green checkmark cannot answer it: `ai status` being green
//! says nothing about whether `sense` returns usable scores.
//!
//! Honesty rules:
//! * Only rows verified against the code are listed. An unverified feature is
//!   NOT guessed into the table — absence means "not measured", not "free".
//! * `without` says what actually happens, not "degrades gracefully".
//! * The index embedder is `bge-base-en-v1.5` (768 dim, 419 MB), **not**
//!   ModernBERT — the two live side by side and conflating them is how a
//!   capability question becomes unanswerable.

use serde::Serialize;

/// The verified rows, ordered by layer: everything above the first `Embedder`
/// row works on a machine that has never downloaded a model.
pub const CAPABILITIES: &[Capability] = &[
    Capability {
        feature: "analyze / list / read_node",
        layer: "L1",
        needs: ModelNeed::None,
        without: "full function",
    },
    Capability {
        feature: "edit_node / insert / batch",
        layer: "L1",
        needs: ModelNeed::None,
        without: "full function (syntax validated before write)",
    },
    Capability {
        feature: "validate / heal / undo / diff",
        layer: "L1",
        needs: ModelNeed::None,
        without: "full function",
    },
    Capability {
        feature: "search (literal)",
        layer: "L1",
        needs: ModelNeed::None,
        without: "full function",
    },
    Capability {
        feature: "code-index --db",
        layer: "L1+L2",
        needs: ModelNeed::None,
        without: "full function (~40 ms per changed file)",
    },
    Capability {
        feature: "code-index --coverage",
        layer: "L2",
        needs: ModelNeed::None,
        without: "full function (reports L3 as 'not built')",
    },
    Capability {
        feature: "code-index --impact/--cycles/--chain",
        layer: "L2",
        needs: ModelNeed::None,
        without: "full function over RESOLVED edges, graph_coverage reported",
    },
    Capability {
        feature: "index_entities / index_relations",
        layer: "L2",
        needs: ModelNeed::None,
        without: "full function (the graph is extracted, not inferred)",
    },
    Capability {
        feature: "doctor",
        layer: "L0",
        needs: ModelNeed::None,
        without: "full function (it REPORTS the model state)",
    },
    Capability {
        feature: "ai index",
        layer: "L3",
        needs: ModelNeed::Embedder,
        without: "unavailable — L3 stays empty, L0-L2 unaffected",
    },
    Capability {
        feature: "sense (satellite / zoom)",
        layer: "L3",
        needs: ModelNeed::Embedder,
        without: "unavailable; use search (literal) or code-index --impact",
    },
    Capability {
        feature: "ai recall-eval",
        layer: "L3",
        needs: ModelNeed::Embedder,
        without: "unavailable (it measures the embedder)",
    },
    Capability {
        feature: "explain / summarize / investigate",
        layer: "L4",
        needs: ModelNeed::Llm,
        without: "unavailable; read_node / get_skeleton give the same facts",
    },
    Capability {
        feature: "edit_ask / propose_edit",
        layer: "L4",
        needs: ModelNeed::Llm,
        without: "unavailable; edit_node applies a change you wrote yourself",
    },
    Capability {
        feature: "sense --expand (query expansion)",
        layer: "L4→L3",
        needs: ModelNeed::Llm,
        without: "skipped with a note; the plain embedding channel still runs",
    },
];

/// The first row that needs any model — everything above it is model-free.
pub fn first_model_row() -> usize {
    CAPABILITIES
        .iter()
        .position(|c| c.needs != ModelNeed::None)
        .unwrap_or(CAPABILITIES.len())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelNeed {
    /// No model, no network, no GPU. Same answer on any machine.
    None,
    /// A sentence embedder (bge-base, 768-dim) — cosine ranking only.
    Embedder,
    /// A generative model (LFM2.5) — produces text, therefore can be wrong.
    Llm,
}

impl ModelNeed {
    pub fn label(self) -> &'static str {
        match self {
            ModelNeed::None => "none",
            ModelNeed::Embedder => "bge-base 768",
            // The build gate belongs in the label: the default build has NO
            // generative path at all, and `explain` fails with "requires the
            // 'mamba' feature" (verified 2026-11-11). A table that says only
            // "lfm2.5" hides that.
            ModelNeed::Llm => "lfm2.5 +mamba",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Capability {
    /// The command / tool name as the user types it.
    pub feature: &'static str,
    /// Which index level it reads (L0 disk, L1 AST, L2 graph, L3 vectors, L4 LLM).
    pub layer: &'static str,
    pub needs: ModelNeed,
    /// What happens with no model installed. Never "degrades gracefully".
    pub without: &'static str,
}

/// One installed model on disk: what it is and what it costs.
#[derive(Debug, Clone, Serialize)]
pub struct InstalledModel {
    pub dir: String,
    pub megabytes: u64,
    pub role: &'static str,
}

/// Models actually present (project store first, then the user cache) with
/// their size on disk — the price side of "is L3 worth it on this machine?".
pub fn installed_models(project_root: &std::path::Path) -> Vec<InstalledModel> {
    let mut roots = vec![project_root.join(".gnawtreewriter_ai").join("models")];
    if let Ok(home) = std::env::var("HOME") {
        roots.push(
            std::path::PathBuf::from(home)
                .join(".gnawtreewriter_ai")
                .join("models"),
        );
    }
    let mut out: Vec<InstalledModel> = Vec::new();
    for root in roots {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for e in entries.flatten() {
            if !e.path().is_dir() {
                continue;
            }
            let name = e.file_name().to_string_lossy().to_string();
            if out.iter().any(|m| m.dir == name) {
                continue;
            }
            out.push(InstalledModel {
                megabytes: dir_size_mb(&e.path()),
                role: role_of(&name),
                dir: name,
            });
        }
    }
    out.sort_by_key(|m| std::cmp::Reverse(m.megabytes));
    out
}

/// Role of a directory in the model tree.
///
/// RULE: only directories that an `AiModel` variant maps to (`dir_name()`) may
/// claim a role. A directory is not a model. `mamba3/` sits in the tree with
/// 6.4 GB of weights — twice, as safetensors and pytorch_model.bin — and NO
/// loader: this table called it "L4 generative (largest; opt-in)" because that
/// is what the folder was named, which is the same lie as a path hash in a
/// content-hash column (2026-11-11).
fn role_of(dir: &str) -> &'static str {
    match dir {
        "bge-base" => "L3 embedder (ai index / sense)",
        "modernbert" => "no reader since 2026-11-11 — its only load was removed",
        "lfm25" | "lfm2.5" => "L4 generative (explain / summarize / investigate)",
        _ => "no loader in this build — not referenced by AiModel",
    }
}

fn dir_size_mb(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(p) = stack.pop() {
        let Ok(md) = std::fs::metadata(&p) else {
            continue;
        };
        if md.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&p) {
                for e in entries.flatten() {
                    stack.push(e.path());
                }
            }
        } else {
            total += md.len();
        }
    }
    total / (1024 * 1024)
}

/// Human table. Same rows every run (no model load, no network) so it can be
/// diffed and trusted.
pub fn render(project_root: &std::path::Path) -> String {
    let mut s = String::new();
    s.push_str("capability map — what needs a model, and what happens without one\n");
    s.push_str("(only verified rows; a feature not listed here is not measured)\n\n");
    s.push_str(&format!(
        "{:<42} {:<6} {:<15} {}\n",
        "feature", "layer", "needs", "without it"
    ));
    s.push_str(&format!("{}\n", "-".repeat(118)));
    let first = first_model_row();
    for (i, c) in CAPABILITIES.iter().enumerate() {
        if i == first {
            s.push_str(&format!("{}\n", "-".repeat(118)));
            s.push_str("  ^ everything ABOVE this line works with no model at all\n");
            s.push_str(&format!("{}\n", "-".repeat(118)));
        }
        s.push_str(&format!(
            "{:<42} {:<6} {:<15} {}\n",
            c.feature,
            c.layer,
            c.needs.label(),
            c.without
        ));
    }
    s.push('\n');
    s.push_str(
        "note: L4 rows additionally need a build with `--features mamba`; the\n\
         default build has no generative path (`explain` errors out). Disk/RAM\n\
         cost is the installed model, not the binary.\n\n",
    );
    let models = installed_models(project_root);
    if models.is_empty() {
        s.push_str("installed models: none — L0-L2 fully usable, L3/L4 unavailable\n");
    } else {
        s.push_str("installed models (the cost side of the L3/L4 choice):\n");
        for m in &models {
            s.push_str(&format!(
                "  {:<14} {:>6} MB  {}\n",
                m.dir, m.megabytes, m.role
            ));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The table's whole purpose: a reader must SEE where the model-free floor
    /// ends. If a model row ever sorts above a model-free row, the separator
    /// line lies about the boundary.
    #[test]
    fn model_rows_come_after_model_free_rows() {
        let first = first_model_row();
        assert!(first > 0, "some rows must be model-free");
        assert!(first < CAPABILITIES.len(), "some rows must need a model");
        for (i, c) in CAPABILITIES.iter().enumerate() {
            if i < first {
                assert_eq!(c.needs, ModelNeed::None, "{} needs a model", c.feature);
            } else {
                assert_ne!(c.needs, ModelNeed::None, "{} is model-free", c.feature);
            }
        }
    }

    /// `without` must SAY something. An empty or vague cell is how a capability
    /// table turns into decoration.
    #[test]
    fn every_row_states_what_happens_without_the_model() {
        for c in CAPABILITIES {
            assert!(
                c.without.trim().len() > 10,
                "{} has a vague fallback: {:?}",
                c.feature,
                c.without
            );
        }
    }

    /// The embedder label must name the model the index actually runs
    /// (`index_model()` is Bge). Saying "ModernBERT" here would describe a
    /// different model in a different directory.
    #[test]
    fn embedder_label_names_bge_not_modernbert() {
        let label = ModelNeed::Embedder.label();
        assert!(label.contains("bge-base"), "{label}");
        assert!(!label.to_lowercase().contains("modernbert"), "{label}");
    }

    /// The rule `role_of` exists for: a directory name is not a model. Only the
    /// three dirs an `AiModel` maps to may claim a role; anything else must say
    /// "no loader". `mamba3/` held 6.4 GB of unloadable weights while this table
    /// described it as "L4 generative".
    #[test]
    fn only_known_model_dirs_claim_a_role() {
        for known in ["bge-base", "modernbert", "lfm25", "lfm2.5"] {
            assert!(
                !role_of(known).contains("no loader"),
                "{known} is a real model and must have a role"
            );
        }
        for unknown in ["mamba3", "whatever", ""] {
            assert_eq!(
                role_of(unknown),
                "no loader in this build — not referenced by AiModel",
                "{unknown} is not an AiModel dir and must not claim a role"
            );
        }
    }

    #[test]
    fn render_marks_the_model_free_floor() {
        let tmp = tempfile::tempdir().unwrap();
        let text = render(tmp.path());
        assert!(text.contains("works with no model at all"));
        assert!(text.contains("analyze / list / read_node"));
        // The model section is present either way; which one depends on the
        // machine's cache, so assert the header, not the emptiness (a test that
        // reads $HOME is not hermetic — lesson #6).
        assert!(text.contains("installed models"));
        // The floor must be ABOVE the first model row in the rendered text.
        let floor = text.find("works with no model at all").unwrap();
        let first_model = text.find("bge-base 768").unwrap();
        assert!(floor < first_model, "the floor marker must precede L3");
    }
}
