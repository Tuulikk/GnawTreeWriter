use crate::parser::TreeNode;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use std::fs;

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
}

pub struct GuardianEngine;

impl GuardianEngine {
    pub fn new() -> Self {
        Self
    }

    /// Analyze the difference between the current node and the proposed new content
    pub fn audit_edit(&self, old_node: &TreeNode, new_content: &str) -> IntegrityReport {
        let mut messages = Vec::new();
        let mut score = 1.0;

        // 1. Volume Check (Quantitative)
        let old_len = old_node.content.len();
        let new_len = new_content.len();

        if new_len < old_len / 2 && old_len > 100 {
            score -= 0.3;
            messages.push(format!("Significant volume reduction: {}% of code removed.", 
                (1.0 - (new_len as f32 / old_len as f32)) * 100.0));
        }

        // 2. Structural Check (Qualitative - Simplified for now)
        // Count logical keywords as a proxy for complexity
        let old_complexity = self.estimate_complexity(&old_node.content);
        let new_complexity = self.estimate_complexity(new_content);

        if new_complexity < old_complexity && old_complexity > 2 {
            score -= 0.4;
            messages.push(format!("Structural complexity drop: {} logical markers lost.", 
                old_complexity - new_complexity));
        }

        // 3. Comment Preservation
        if old_node.content.contains("//") || old_node.content.contains("/*") {
            if !new_content.contains("//") && !new_content.contains("/*") {
                score -= 0.2;
                messages.push("Documentation/Comments appear to have been stripped.".into());
            }
        }

        let level = if score <= 0.3 {
            IntegrityLevel::Critical
        } else if score <= 0.6 {
            IntegrityLevel::Warning
        } else if score <= 0.9 {
            IntegrityLevel::Notice
        } else {
            IntegrityLevel::Safe
        };

        IntegrityReport { level, score, messages }
    }

    fn estimate_complexity(&self, content: &str) -> usize {
        let keywords = ["if ", "else", "for ", "while", "match ", "switch", "try", "catch", "unwrap", "expect"];
        keywords.iter().filter(|&&k| content.contains(k)).count()
    }
}
