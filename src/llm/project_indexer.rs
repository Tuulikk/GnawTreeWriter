use anyhow::Result;
use std::path::{Path, PathBuf};
use crate::llm::{GnawSenseBroker, SemanticIndexManager, NodeEmbedding, AiModel, DeviceType};
use crate::parser::{get_parser, TreeNode};
use walkdir::WalkDir;
use std::fs;

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
        let model = self.broker.get_manager().load_model(AiModel::ModernBert, DeviceType::Cpu)?;
        
        // Canonicalize target_path to ensure strip_prefix works
        let target_path = if target_path.is_relative() {
            fs::canonicalize(target_path).unwrap_or(target_path.to_path_buf())
        } else {
            target_path.to_path_buf()
        };

        for entry in WalkDir::new(&target_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file()) 
        {
            let path = entry.path();
            
            // Skip hidden directories (like .git, .gnawtreewriter_ai)
            if path.components().any(|c| c.as_os_str().to_str().map(|s| s.starts_with('.')).unwrap_or(false)) {
                continue;
            }

            if let Ok(parser) = get_parser(path) {
                // Try to strip prefix safely
                let file_path_str = path.strip_prefix(&self.project_root)
                    .unwrap_or(path) // Fallback to full path if prefix doesn't match
                    .to_string_lossy()
                    .to_string();

                if let Ok(content) = fs::read_to_string(path) {
                    // SMART RE-INDEXING: Check if file changed
                    let file_hash = crate::core::transaction_log::calculate_content_hash(&file_path_str);
                    let index_path = self.index_manager.get_storage_dir().join(format!("{}.json", file_hash));
                    
                    if index_path.exists() {
                        // File already indexed and hasn't changed (hash is part of filename)
                        total_files += 1;
                        continue;
                    }

                    if let Ok(tree) = parser.parse(&content) {
                        let mut entries = Vec::new();
                        self.collect_embeddings(&tree, &file_path_str, &model, &mut entries)?;
                        
                        if !entries.is_empty() {
                            self.index_manager.save_index(&file_path_str, entries)?;
                            total_files += 1;
                        }
                    }
                }
            }
        }

        // Save model metadata for the ecosystem
        self.index_manager.save_model_info("ModernBERT-base-v1", 768)?;

        Ok(total_files)
    }

    fn collect_embeddings(
        &self, 
        node: &TreeNode, 
        file_path: &str, 
        model: &crate::llm::ModernBertModel, 
        acc: &mut Vec<NodeEmbedding>
    ) -> Result<()> {
        // Index functions, classes, and important definitions
        if node.node_type.contains("definition") || node.node_type.contains("item") {
            // CHUNKING LOGIC: If node is too large, split it
            // ModernBERT safe limit is roughly 8192 tokens. 
            // 15,000 chars is a safe heuristic for ~4000-5000 tokens.
            if node.content.len() > 15000 {
                let chunks = self.chunk_text(&node.content, 10000, 1000);
                for (i, chunk) in chunks.into_iter().enumerate() {
                    let vector_tensor = model.get_embedding(&chunk)?;
                    let vector: Vec<f32> = vector_tensor.to_vec1()?;
                    
                    acc.push(NodeEmbedding {
                        file_path: file_path.to_string(),
                        node_path: format!("{}[chunk:{}]", node.path, i),
                        content_preview: format!("(Chunk {}) {}", i, &chunk[..chunk.len().min(100)]),
                        vector,
                    });
                }
            } else {
                let vector_tensor = model.get_embedding(&node.content)?;
                let vector: Vec<f32> = vector_tensor.to_vec1()?;
                
                let preview = if node.content.len() > 100 {
                    format!("{}...", &node.content[..97])
                } else {
                    node.content.clone()
                };

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

    fn chunk_text(&self, text: &str, size: usize, overlap: usize) -> Vec<String> {
        let mut chunks = Vec::new();
        if text.is_empty() { return chunks; }
        
        let mut start = 0;
        while start < text.len() {
            let end = (start + size).min(text.len());
            chunks.push(text[start..end].to_string());
            if end == text.len() { break; }
            start += size - overlap;
        }
        chunks
    }
}
