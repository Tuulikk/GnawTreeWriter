#![cfg(feature = "modernbert")]

use crate::core::file_walker::walk_source_files;
use crate::llm::{AiModel, DeviceType, GnawSenseBroker, NodeEmbedding, SemanticIndexManager};
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
        let model = self
            .broker
            .get_manager()
            .load_model(AiModel::ModernBert, DeviceType::Cpu)?;

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
                        let mut entries = Vec::new();
                        self.collect_embeddings(&tree, &file_path_str, model, &mut entries)?;

                        if !entries.is_empty() {
                            self.index_manager.save_index(&file_path_str, entries)?;
                            total_files += 1;
                        }
                    }
                }
            }
        }

        // Save model metadata for the ecosystem
        self.index_manager
            .save_model_info("ModernBERT-base-v1", 768)?;

        Ok(total_files)
    }

    fn collect_embeddings(
        &self,
        node: &TreeNode,
        file_path: &str,
        model: &crate::llm::ModernBertModel,
        acc: &mut Vec<NodeEmbedding>,
    ) -> Result<()> {
        // Index functions, classes, and important definitions
        if node.node_type.contains("definition") || node.node_type.contains("item") {
            // CHUNKING LOGIC: If node is too large, split it
            // ModernBERT safe limit is roughly 8192 tokens.
            // Chunking thresholds must respect ModernBERT's 8192-token rope
            // context. Worst case is ~1 token/char (CJK), so chunks are kept
            // at 4000 chars; nodes over 8000 chars are chunked. (The old
            // 15000-char threshold could blow the context and crash the
            // model, and chunking itself was byte-based — panic on multibyte
            // UTF-8.)
            if node.content.chars().count() > 8000 {
                let chunks = Self::chunk_text(&node.content, 4000, 500);
                for (i, chunk) in chunks.into_iter().enumerate() {
                    let vector_tensor = model.get_embedding(&chunk)?;
                    let vector: Vec<f32> = vector_tensor.to_vec1()?;

                    acc.push(NodeEmbedding {
                        file_path: file_path.to_string(),
                        node_path: format!("{}[chunk:{}]", node.path, i),
                        content_preview: format!(
                            "(Chunk {}) {}",
                            i,
                            crate::llm::gnaw_sense::truncate_preview(chunk.trim(), 97)
                        ),
                        vector,
                    });
                }
            } else {
                let vector_tensor = model.get_embedding(&node.content)?;
                let vector: Vec<f32> = vector_tensor.to_vec1()?;

                let preview = crate::llm::gnaw_sense::truncate_preview(&node.content, 97);

                acc.push(NodeEmbedding {
                    file_path: file_path.to_string(),
                    node_path: node.path.clone(),
                    content_preview: preview,
                    vector,
                });
            }
        }

        for child in &node.children {
            self.collect_embeddings(child, file_path, model, acc)?;
        }

        Ok(())
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
        assert_eq!(ProjectIndexer::chunk_text(short, 100, 10), vec![short.to_string()]);
    }
}
