use crate::core::rules::Severity;
use crate::parser::TreeNode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegrityLevel {
    Safe,
    Notice,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub level: IntegrityLevel,
    pub score: f32, // 0.0 (total destruction) to 1.0 (perfectly safe)
    pub messages: Vec<String>,
    /// Fas 4 (docs/GUARDIAN_V2_PLAN.md): the structural deltas behind this
    /// report (empty for audit_edit without a known language). Skipped in
    /// serde — EditDelta has no Deserialize and this is runtime info.
    #[serde(skip)]
    pub deltas: Vec<crate::core::edit_delta::EditDelta>,
}

pub struct GuardianEngine;

impl Default for GuardianEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GuardianEngine {
    pub fn new() -> Self {
        Self
    }

    /// Analyze the difference between the current node and the proposed new content
    pub fn audit_edit(&self, old_node: &TreeNode, new_content: &str) -> IntegrityReport {
        self.audit_edit_with_language(old_node, new_content, "")
    }

    /// Fas 1 (docs/GUARDIAN_V2_PLAN.md): same audit, plus structural edit
    /// deltas when the language parser is known. Lone operator changes stay
    /// warnings; when the same edit also removed a condition or error
    /// handling, operator changes are upgraded to error (context-aware
    /// severity, Gemini feedback 2026-10-06).
    pub fn audit_edit_with_language(
        &self,
        old_node: &TreeNode,
        new_content: &str,
        language: &str,
    ) -> IntegrityReport {
        let mut messages = Vec::new();
        let mut score = 1.0f32;

        // 1. Volume Check (Quantitative)
        let old_len = old_node.content.len();
        let new_len = new_content.len();

        if new_len < old_len / 2 && old_len > 100 {
            score -= 0.3;
            messages.push(format!(
                "Significant volume reduction: {}% of code removed.",
                (1.0 - (new_len as f32 / old_len as f32)) * 100.0
            ));
        }

        // 2. Structural Check (Qualitative - Simplified for now)
        // Count logical keywords as a proxy for complexity
        let old_complexity = self.estimate_complexity(&old_node.content);
        let new_complexity = self.estimate_complexity(new_content);

        if new_complexity < old_complexity && old_complexity > 2 {
            score -= 0.4;
            messages.push(format!(
                "Structural complexity drop: {} logical markers lost.",
                old_complexity - new_complexity
            ));
        }

        // 3. Comment Preservation
        if (old_node.content.contains("//") || old_node.content.contains("/*"))
            && !new_content.contains("//")
            && !new_content.contains("/*")
        {
            score -= 0.2;
            messages.push("Documentation/Comments appear to have been stripped.".into());
        }

        // 4. Structural deltas (Fas 1): parse the new node content and
        // compare trees. Penalties: error -0.35, warning -0.15, info -0.05.
        let mut report_deltas: Vec<crate::core::edit_delta::EditDelta> = Vec::new();
        if !language.is_empty() {
            if let Ok(parser) = crate::parser::get_parser_for_language(language) {
                if let Ok(new_tree) = parser.parse(new_content) {
                    let new_root = crate::core::edit_delta::unwrap_root(&new_tree);
                    let mut deltas = crate::core::edit_delta::diff_node_trees(old_node, new_root);

                    // Fas 2: invariant contract — fires even when the edit
                    // restructured the tree beyond index-pairing.
                    let contract = crate::core::edit_delta::extract_contract(old_node);
                    deltas.extend(crate::core::edit_delta::check_contract(
                        &contract,
                        new_content,
                    ));

                    let context_broken = deltas.iter().any(|d| {
                        matches!(
                            d.kind,
                            crate::core::edit_delta::DeltaKind::ConditionRemoved
                                | crate::core::edit_delta::DeltaKind::ErrorHandlingRemoved { .. }
                        )
                    });
                    report_deltas = deltas;
                    for d in &report_deltas {
                        let effective = if context_broken
                            && matches!(
                                d.kind,
                                crate::core::edit_delta::DeltaKind::OperatorChange { .. }
                            ) {
                            Severity::Error
                        } else {
                            d.severity
                        };
                        score -= match effective {
                            Severity::Error => 0.35,
                            Severity::Warning => 0.15,
                            Severity::Info => 0.05,
                        };
                        messages.push(d.message.clone());
                    }
                }
            }
        }

        let score = score.clamp(0.0, 1.0);

        let level = if score <= 0.3 {
            IntegrityLevel::Critical
        } else if score <= 0.6 {
            IntegrityLevel::Warning
        } else if score <= 0.9 {
            IntegrityLevel::Notice
        } else {
            IntegrityLevel::Safe
        };

        IntegrityReport {
            level,
            score,
            messages,
            deltas: report_deltas,
        }
    }

    fn estimate_complexity(&self, content: &str) -> usize {
        let keywords = [
            "if ", "else", "for ", "while", "match ", "switch", "try", "catch", "unwrap", "expect",
        ];
        keywords.iter().filter(|&&k| content.contains(k)).count()
    }
}

#[cfg(test)]
mod guardian_alignment_tests {
    use super::{GuardianEngine, IntegrityLevel, IntegrityReport};
    use crate::core::GnawTreeWriter;

    /// 65-statement body-block with one if-condition (mirrors the Motor2
    /// relay shape, issue log (IV) 2026-10-31). Parses the src, returns
    /// the guardian audit of a new content built from the block's own
    /// text via `transform`.
    fn audit_block(transform: impl Fn(&str) -> String) -> IntegrityReport {
        let mut src = String::new();
        src.push_str("pub fn podman_build(slots: &[usize]) -> String {\n");
        for i in 1..=65 {
            src.push_str(&format!("    let s{} = render(slots, {});\n", i, i));
            if i == 30 {
                src.push_str("    if s30.len() > 0 {\n");
                src.push_str("        assert!(s30.contains('3'));\n");
                src.push_str("    }\n");
            }
        }
        src.push_str("    format!(\"done: {}\", slots.len())\n}\n");
        src.push_str("fn render(slots: &[usize], n: usize) -> String {\n");
        src.push_str("    format!(\"slot {}\", n + slots.len())\n}\n");

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.rs");
        std::fs::write(&path, &src).unwrap();
        let writer = GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
        let tree = writer.analyze();
        let f = tree
            .children
            .iter()
            .find(|c| c.node_type == "function_item" && c.content.contains("podman_build"))
            .expect("podman_build node must parse");
        let block = f
            .children
            .iter()
            .find(|c| c.node_type == "block" && c.content.contains("let s1"))
            .expect("body block node must parse");

        let new_content = transform(&block.content);
        GuardianEngine::new().audit_edit_with_language(block, &new_content, "rust")
    }

    /// Fix contract (IV/A): ADDING a line at the TOP of a big body-block
    /// used to shift positional pairing so the token fallback reported
    /// the addition as removed statements -> GUARDIAN BLOCK on valid
    /// edits. A top addition must classify as addition, not removal.
    #[test]
    fn body_block_top_addition_is_not_removal() {
        let report = audit_block(|c| c.replacen("{\n", "{\n    // added marker line\n", 1));
        let critical = matches!(report.level, IntegrityLevel::Critical);
        let claims_removal = report.messages.iter().any(|m| m.contains("removed"));
        assert!(
            !(critical && claims_removal),
            "top addition must not be classified as removal. level: {:?}, messages: {:?}, deltas: {:?}",
            report.level,
            report.messages,
            report.deltas
        );
    }

    /// The same shape it had before the fix: appending at the END passed.
    /// Regression guard that the pairing fix does not break that either.
    #[test]
    fn body_block_end_addition_still_passes() {
        let report = audit_block(|c| {
            let cut = c.rfind('}').unwrap();
            let mut s = String::new();
            s.push_str(&c[..cut]);
            s.push_str("    // trailing marker line\n");
            s.push_str(&c[cut..]);
            s
        });
        let blocked = matches!(report.level, IntegrityLevel::Critical);
        assert!(
            !blocked,
            "end addition must stay clean. level: {:?}, messages: {:?}",
            report.level, report.messages
        );
    }

    /// Core-level bypass contract: `force` (CLI --force / MCP edit_node
    /// force:true, both route to edit(force)) applies a top-adding edit
    /// the guardian suspected — after the pairing fix such edits pass
    /// WITHOUT force; force stays the deliberate last resort.
    #[test]
    fn force_path_applies_top_addition() {
        let mut src = String::new();
        src.push_str("pub fn podman_build(slots: &[usize]) -> String {\n");
        for i in 1..=65 {
            src.push_str(&format!("    let s{} = render(slots, {});\n", i, i));
        }
        src.push_str("    format!(\"done: {}\", slots.len())\n}\n");
        src.push_str("fn render(slots: &[usize], n: usize) -> String {\n");
        src.push_str("    format!(\"slot {}\", n + slots.len())\n}\n");

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.rs");
        std::fs::write(&path, &src).unwrap();
        let mut writer = GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
        let tree = writer.analyze();
        let f = tree
            .children
            .iter()
            .find(|c| c.node_type == "function_item" && c.content.contains("podman_build"))
            .expect("podman_build node");
        let block = f
            .children
            .iter()
            .find(|c| c.node_type == "block" && c.content.contains("let s1"))
            .expect("body block node");
        let block_path = block.path.clone();
        let new_content =
            block
                .content
                .replacen("{\n", "{\n    // top marker added deliberately\n", 1);

        writer
            .edit(
                crate::core::EditOperation::Edit {
                    node_path: block_path,
                    content: new_content,
                },
                true,
            )
            .expect("force path must apply the edit");

        let written = std::fs::read_to_string(&path).unwrap();
        assert!(
            written.contains("// top marker added deliberately") && written.contains("let s65"),
            "force edit must land intact; file: {}",
            written
        );
    }

    /// Negative control — the fix must NOT neuter genuine removal
    /// detection: deleting a condition statement must still surface a
    /// ConditionRemoved delta.
    #[test]
    fn genuine_condition_removal_is_still_detected() {
        let report = audit_block(|c| {
            let lines: Vec<&str> = c.lines().collect();
            let start = lines
                .iter()
                .position(|l| l.contains("if s30.len() > 0"))
                .expect("condition line must exist");
            let mut s = String::new();
            for (i, l) in lines.iter().enumerate() {
                // surgical drop: the if-line, its inner statement, its brace
                if i >= start && i <= start + 2 {
                    continue;
                }
                s.push_str(l);
                s.push('\n');
            }
            s
        });
        assert!(
            report
                .deltas
                .iter()
                .any(|d| matches!(d.kind, crate::core::edit_delta::DeltaKind::ConditionRemoved)),
            "genuine condition removal must still be detected; deltas: {:?}",
            report.deltas
        );
    }
}
