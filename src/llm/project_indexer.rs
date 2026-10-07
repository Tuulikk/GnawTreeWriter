#![cfg(feature = "modernbert")]

use crate::core::file_walker::walk_source_files;
use crate::llm::{AiModel, GnawSenseBroker, NodeEmbedding, SemanticIndexManager};
use crate::parser::{get_parser, TreeNode};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ProjectIndexer {
    project_root: PathBuf,
    broker: GnawSenseBroker,
    index_manager: SemanticIndexManager,
}

impl ProjectIndexer {
    pub fn new(project_root: &Path) -> Result<Self> {
        Ok(Self {
            project_root: project_root.to_path_buf(),
            broker: GnawSenseBroker::new(project_root)?,
            index_manager: SemanticIndexManager::new(project_root),
        })
    }

    /// Crawl the project and index supported source files starting from target_path
    pub async fn index_all(&self, target_path: &Path) -> Result<usize> {
        let mut total_files = 0;
        // GPU only here (indexing), behind the 20%-VRAM gate; sense/query
        // paths below stay on CPU by design — see ai_manager::indexing_device.
        let model = self
            .broker
            .get_manager()
            .load_model(AiModel::Bge, crate::llm::ai_manager::indexing_device())?;

        // Canonicalize target_path to ensure strip_prefix works
        let target_path = if target_path.is_relative() {
            fs::canonicalize(target_path).unwrap_or(target_path.to_path_buf())
        } else {
            target_path.to_path_buf()
        };

        for path in walk_source_files(&target_path) {
            if let Ok(parser) = get_parser(&path) {
                // Try to strip prefix safely
                let file_path_str = path
                    .strip_prefix(&self.project_root)
                    .unwrap_or(&path) // Fallback to full path if prefix doesn't match
                    .to_string_lossy()
                    .to_string();

                if let Ok(content) = fs::read_to_string(path) {
                    // SMART RE-INDEXING: Check if file changed
                    let file_hash =
                        crate::core::transaction_log::calculate_content_hash(&file_path_str);
                    let index_path = self
                        .index_manager
                        .get_storage_dir()
                        .join(format!("{}.json", file_hash));

                    if index_path.exists() {
                        // File already indexed and hasn't changed (hash is part of filename)
                        total_files += 1;
                        continue;
                    }

                    if let Ok(tree) = parser.parse(&content) {
                        // Two phases: walk collects (node_path, preview, text)
                        // triples, then ONE batched embedding call per file
                        // (16 nodes per forward instead of one per node).
                        let mut pending: Vec<(String, String, String)> = Vec::new();
                        Self::collect_pending(&tree, &mut pending);
                        // The file's own //! description bridges agent
                        // queries ("avoid reparsing unchanged files") to
                        // implementation symbols (get_or_parse) — prepend it
                        // to both the embedded text and the stored preview.
                        let module_doc = Self::extract_module_doc(&content);
                        if !module_doc.is_empty() {
                            for t in pending.iter_mut() {
                                t.2 = format!("{}\n\n{}", t.2, module_doc);
                            }
                        }
                        if !pending.is_empty() {
                            let texts: Vec<&str> = pending.iter().map(|t| t.2.as_str()).collect();
                            let vectors = model.get_embeddings(&texts)?;
                            let entries = pending
                                .into_iter()
                                .zip(vectors)
                                .map(|((node_path, content_preview, _), vector)| NodeEmbedding {
                                    file_path: file_path_str.clone(),
                                    node_path,
                                    content_preview,
                                    vector,
                                })
                                .collect();
                            self.index_manager.save_index(&file_path_str, entries)?;
                            total_files += 1;
                        }
                    }
                }
            }
        }

        // Save model metadata for the ecosystem
        self.index_manager
            .save_model_info("bge-base-en-v1.5", 768)?;

        Ok(total_files)
    }

    /// Walk the tree collecting (node_path, preview, text) triples for
    /// every embeddable node/chunk — embedding happens afterwards in ONE
    /// batched call per file (see ModernBertModel::get_embeddings). No
    /// model in this phase, so the walk is trivially cheap.
    fn extract_module_doc(content: &str) -> String {
        let mut doc = String::new();
        for line in content.lines() {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("//!") {
                doc.push_str(rest.trim());
                doc.push(' ');
                if doc.chars().count() >= 200 {
                    break;
                }
            } else if !trimmed.is_empty() {
                break;
            }
        }
        doc.trim().to_string()
    }
    fn collect_pending(node: &TreeNode, acc: &mut Vec<(String, String, String)>) {
        // Index functions, classes, and important definitions
        if node.node_type.contains("definition") || node.node_type.contains("item") {
            // CHUNKING LOGIC: If node is too large, split it.
            // ModernBERT safe limit is roughly 8192 tokens. Chunking
            // thresholds must respect the 8192-token rope context; worst
            // case is ~1 token/char (CJK), so chunks stay at 4000 chars and
            // nodes over 8000 chars are chunked.
            if node.content.chars().count() > 8000 {
                let chunks = Self::chunk_text(&node.content, 4000, 500);
                for (i, chunk) in chunks.into_iter().enumerate() {
                    let preview = format!(
                        "(Chunk {}) {}",
                        i,
                        crate::llm::gnaw_sense::truncate_preview(chunk.trim(), 240)
                    );
                    acc.push((format!("{}[chunk:{}]", node.path, i), preview, chunk));
                }
            } else {
                let preview = crate::llm::gnaw_sense::truncate_preview(&node.content, 240);
                acc.push((node.path.clone(), preview, node.content.clone()));
            }
        }

        for child in &node.children {
            Self::collect_pending(child, acc);
        }
    }

    /// Split `text` into overlapping chunks of at most `size` CHARACTERS
    /// (not bytes — byte arithmetic panics on multibyte UTF-8).
    fn chunk_text(text: &str, size: usize, overlap: usize) -> Vec<String> {
        let mut chunks = Vec::new();
        if text.is_empty() {
            return chunks;
        }

        let chars: Vec<char> = text.chars().collect();
        let mut start = 0;
        while start < chars.len() {
            let end = (start + size).min(chars.len());
            chunks.push(chars[start..end].iter().collect());
            if end == chars.len() {
                break;
            }
            start += size - overlap;
        }
        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectIndexer;

    /// Regression: byte-based chunking (`text[start..end]`) panicked on
    /// multibyte UTF-8 at chunk boundaries.
    #[test]
    fn chunk_text_is_char_boundary_safe() {
        let text = format!("{}{}", "函".repeat(3000), "🦀".repeat(3000));
        let chunks = ProjectIndexer::chunk_text(&text, 4000, 500);
        assert!(!chunks.is_empty());
        for c in &chunks {
            assert!(c.chars().count() <= 4000);
        }
        // Reassembly with overlap must preserve all characters.
        let total: usize = chunks.iter().map(|c| c.chars().count()).sum();
        assert!(total >= text.chars().count());
    }

    #[test]
    fn chunk_text_edge_cases() {
        assert!(ProjectIndexer::chunk_text("", 100, 10).is_empty());
        let short = "fn main() {}";
        assert_eq!(
            ProjectIndexer::chunk_text(short, 100, 10),
            vec![short.to_string()]
        );
    }
}
