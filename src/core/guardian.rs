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
