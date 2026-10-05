use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEmbedding {
    pub file_path: String,
    pub node_path: String,
    pub content_preview: String,
    pub vector: Vec<f32>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SemanticIndex {
    pub entries: Vec<NodeEmbedding>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub model_name: String,
    pub dimension: usize,
    pub created_at: DateTime<Utc>,
}

pub struct SemanticIndexManager {
    storage_dir: PathBuf,
}

impl SemanticIndexManager {
    pub fn new(project_root: &Path) -> Self {
        let storage_dir = project_root.join(".gnawtreewriter_ai").join("index");
        if !storage_dir.exists() {
            let _ = fs::create_dir_all(&storage_dir);
        }
        Self { storage_dir }
    }

    pub fn get_storage_dir(&self) -> &Path {
        &self.storage_dir
    }

    pub fn save_model_info(&self, model_name: &str, dimension: usize) -> Result<()> {
        let info = ModelInfo {
            model_name: model_name.to_string(),
            dimension,
            created_at: Utc::now(),
        };
        let save_path = self.storage_dir.join("model_info.json");
        let data = serde_json::to_string_pretty(&info)?;
        fs::write(save_path, data)?;
        Ok(())
    }

    pub fn get_model_info(&self) -> Result<Option<ModelInfo>> {
        let load_path = self.storage_dir.join("model_info.json");
        if load_path.exists() {
            let data = fs::read_to_string(load_path)?;
            let info = serde_json::from_str(&data)?;
            Ok(Some(info))
        } else {
            Ok(None)
        }
    }

    pub fn save_index(&self, file_path: &str, entries: Vec<NodeEmbedding>) -> Result<()> {
        let file_hash = crate::core::transaction_log::calculate_content_hash(file_path);
        let save_path = self.storage_dir.join(format!("{}.json", file_hash));
        let data = serde_json::to_string_pretty(&entries)?;
        fs::write(save_path, data)?;
        Ok(())
    }

    pub fn load_project_index(&self) -> Result<SemanticIndex> {
        let mut index = SemanticIndex::default();
        if !self.storage_dir.exists() {
            return Ok(index);
        }

        for entry in fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let data = fs::read_to_string(path)?;
                if let Ok(mut entries) = serde_json::from_str::<Vec<NodeEmbedding>>(&data) {
                    index.entries.append(&mut entries);
                }
            }
        }
        Ok(index)
    }
}

impl SemanticIndex {
    /// Search for entries most similar to query_vector.
    /// Results with cosine similarity below 0.2 are filtered out.
    pub fn search(&self, query_vector: &[f32], limit: usize) -> Vec<(&NodeEmbedding, f32)> {
        self.search_with_threshold(query_vector, limit, 0.2)
    }

    /// Search with explicit minimum score threshold.
    pub fn search_with_threshold(
        &self,
        query_vector: &[f32],
        limit: usize,
        min_score: f32,
    ) -> Vec<(&NodeEmbedding, f32)> {
        let mut results: Vec<(&NodeEmbedding, f32)> = self
            .entries
            .iter()
            .map(|entry| {
                let score = cosine_similarity(query_vector, &entry.vector);
                (entry, score)
            })
            .filter(|(_, score)| *score >= min_score)
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

/// Bounded rerank adjustment for satellite hits (see rerank_satellite).
/// Declaration heads demoted, implementation heads nudged, query-term
/// overlap rewarded — pure cosine alone ranks `pub mod x;` above `impl`s
/// for name-heavy queries: short decls are lexically tight (their names
/// ARE the query terms) while real implementations are long and diffuse.
/// Declarations are pointers; implementations are answers.
const DECL_PENALTY: f32 = 0.35;
const IMPL_BONUS: f32 = 0.06;
const LEX_WEIGHT: f32 = 0.18;

#[derive(PartialEq, Clone, Copy, Debug)]
enum PreviewKind {
    Decl,
    ImplLike,
    Other,
}

fn classify_preview(preview: &str) -> PreviewKind {
    let first = preview
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    // Chunked entries prefix their head ("(Chunk 0) impl ...") — classify
    // on the real code after the marker, or everything lands in Other.
    let head = match first.find(')') {
        Some(pos) if first.starts_with("(Chunk") => first[pos + 1..].trim(),
        _ => first,
    };
    const DECLS: &[&str] = &[
        "pub mod",
        "mod ",
        "use ",
        "pub use",
        "pub(crate) use",
        "extern crate",
        "import ",
        "export ",
        "from ",
    ];
    const IMPLS: &[&str] = &[
        "fn ",
        "pub fn",
        "async fn",
        "pub async fn",
        "pub(crate) fn",
        "impl ",
        "impl<",
        "pub impl",
        "struct ",
        "pub struct",
        "enum ",
        "pub enum",
        "trait ",
        "pub trait",
        "macro_rules",
        "type ",
        "pub type",
        "const ",
        "pub const",
        "static ",
        "pub static",
        "def ",
        "class ",
        "function ",
    ];
    if head.starts_with("#[") || head.starts_with("#![") {
        // Attribute/test-attrs are metadata, not the thing being asked about.
        PreviewKind::Decl
    } else if DECLS.iter().any(|p| head.starts_with(p)) {
        PreviewKind::Decl
    } else if IMPLS.iter().any(|p| head.starts_with(p)) {
        PreviewKind::ImplLike
    } else {
        PreviewKind::Other
    }
}

/// Rerank satellite hits: adjusted = cosine + lexical bonus - decl penalty.
/// Fetch a WIDER raw window than you serve (e.g. search(.., 30) then
/// rerank(.., top=10)) so demoted decls release slots to implementations.
pub fn rerank_satellite<'a>(
    query: &str,
    hits: Vec<(&'a NodeEmbedding, f32)>,
    top: usize,
) -> Vec<(&'a NodeEmbedding, f32)> {
    let q_tokens: Vec<String> = query
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3)
        .map(str::to_string)
        .collect();

    // Inventory-intent queries ("which modules exist …") ANSWER with
    // declarations — soft-penalize decls and stop boosting impls there.
    let q_lower = query.to_lowercase();
    let inventory_intent = [
        "module", "modules", "import", "imports", "file", "files", "symbol", "symbols", "export",
        "exports", "class", "classes",
    ]
    .iter()
    .any(|w| q_lower.contains(w));
    let impl_bonus = if inventory_intent { 0.0 } else { IMPL_BONUS };
    // In inventory mode declarations ARE the answer: nudge them like items.
    let decl_bonus = if inventory_intent { IMPL_BONUS } else { 0.0 };

    let mut scored: Vec<(&NodeEmbedding, f32)> = hits
        .into_iter()
        .map(|(entry, cosine)| {
            // Lexical reward scans preview AND identifiers — a query for
            // "undo" should find undo_redo.rs even when the preview head
            // is a generic item signature.
            let haystack = format!(
                "{} {} {}",
                entry.content_preview.to_lowercase(),
                entry.file_path.to_lowercase(),
                entry.node_path.to_lowercase()
            );
            let matched = q_tokens
                .iter()
                .filter(|t| haystack.contains(t.as_str()))
                .count();
            let lex = if q_tokens.is_empty() {
                0.0
            } else {
                matched as f32 / q_tokens.len() as f32
            };
            let kind = classify_preview(&entry.content_preview);
            let adjust = match kind {
                // Normal mode: scale the penalty by (1 - lex) — a decl whose
                // NAME answers the query keeps some standing; irrelevant
                // decls die. Inventory mode: decls get the item bonus.
                PreviewKind::Decl => {
                    if inventory_intent {
                        decl_bonus
                    } else {
                        -DECL_PENALTY * (1.0 - lex)
                    }
                }
                PreviewKind::ImplLike => impl_bonus,
                PreviewKind::Other => 0.0,
            } + LEX_WEIGHT * lex;
            (entry, cosine + adjust)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(top);
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, preview: &str) -> NodeEmbedding {
        NodeEmbedding {
            file_path: "src/x.rs".to_string(),
            node_path: path.to_string(),
            content_preview: preview.to_string(),
            vector: vec![0.0],
        }
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![0.0, 1.0];
        assert!(cosine_similarity(&a, &c).abs() < 1e-6);
    }

    #[test]
    fn rerank_demotes_module_decls_below_impls() {
        // Raw cosine ties them — declarations must lose for a name-heavy query.
        let e1 = entry("1", "pub mod undo_redo;");
        let e2 = entry("2", "impl UndoRedoManager {");
        let e3 = entry("3", "use crate::llm::Source;");
        let hits = vec![(&e1, 0.90), (&e2, 0.89), (&e3, 0.88)];
        let out = rerank_satellite("how is undo implemented", hits, 10);
        assert_eq!(out[0].0.node_path, "2", "impl must outrank mod decl");
        assert_eq!(out[1].0.node_path, "1");
        assert_eq!(out[2].0.node_path, "3");
    }

    #[test]
    fn rerank_rewards_query_terms() {
        let e1 = entry("1", "fn unrelated_thing() {}");
        let e2 = entry("2", "fn undo_transaction() {}");
        let hits = vec![(&e1, 0.90), (&e2, 0.88)];
        let out = rerank_satellite("undo the transaction", hits, 10);
        assert_eq!(out[0].0.node_path, "2", "lexical overlap must win");
    }

    #[test]
    fn rerank_respects_top_limit() {
        let e1 = entry("1", "fn a() {}");
        let e2 = entry("2", "fn b() {}");
        let e3 = entry("3", "fn c() {}");
        let hits = vec![(&e1, 0.5), (&e2, 0.4), (&e3, 0.3)];
        let out = rerank_satellite("anything", hits, 2);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn rerank_empty_query_keeps_cosine_order() {
        let e1 = entry("1", "fn a() {}");
        let e2 = entry("2", "pub mod m;");
        let hits = vec![(&e1, 0.5), (&e2, 0.4)];
        let out = rerank_satellite("", hits, 10);
        assert_eq!(out[0].0.node_path, "1");
    }

    #[test]
    fn rerank_inventory_query_keeps_decls_competitive() {
        // "Which modules …" is answered by declarations: soft penalty must
        // not let an unrelated impl/struct crowd out the mod entries.
        let e1 = entry("1", "pub struct BlueprintEngine {");
        let e2 = entry("2", "pub mod transaction_log;");
        let hits = vec![(&e1, 0.80), (&e2, 0.80)];
        let out = rerank_satellite("which modules exist in this project", hits, 10);
        assert_eq!(
            out[0].0.node_path, "2",
            "decl must win an inventory query at equal cosine"
        );
    }

    #[test]
    fn classify_kinds() {
        assert_eq!(classify_preview("pub mod undo_redo;"), PreviewKind::Decl);
        assert_eq!(classify_preview("  use std::io;"), PreviewKind::Decl);
        assert_eq!(
            classify_preview("impl UndoRedoManager {"),
            PreviewKind::ImplLike
        );
        assert_eq!(
            classify_preview("pub async fn load() {}"),
            PreviewKind::ImplLike
        );
        assert_eq!(classify_preview("let x = 1;"), PreviewKind::Other);
        // Chunk markers must not hide the real head.
        assert_eq!(
            classify_preview("(Chunk 0) impl UndoRedoManager {"),
            PreviewKind::ImplLike
        );
        assert_eq!(
            classify_preview("(Chunk 3) pub mod whatever;"),
            PreviewKind::Decl
        );
    }
}
