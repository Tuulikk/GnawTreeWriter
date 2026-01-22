use anyhow::Result;
use std::path::{Path, PathBuf};
use crate::llm::{AiManager, AiModel, DeviceType, SemanticIndex};
use crate::parser::TreeNode;
use std::fs;

pub struct GnawSenseBroker {
    ai_manager: AiManager,
    project_root: PathBuf,
}

#[derive(Debug, serde::Serialize)]
pub enum SenseResponse {
    Satelite {
        matches: Vec<FileMatch>,
    },
    Zoom {
        file_path: String,
        nodes: Vec<NodeMatch>,
    },
}

#[derive(Debug, serde::Serialize)]
pub struct FileMatch {
    pub file_path: String,
    pub score: f32,
}

#[derive(Debug, serde::Serialize)]
pub struct NodeMatch {
    pub path: String,
    pub preview: String,
    pub score: f32,
}

#[derive(Debug, serde::Serialize)]
pub struct EditProposal {
    pub anchor_path: String,
    pub suggested_op: String,
    pub parent_path: String,
    pub position: usize,
    pub confidence: f32,
}

impl GnawSenseBroker {
    pub fn new(project_root: &Path) -> Result<Self> {
        Ok(Self {
            ai_manager: AiManager::new(project_root)?,
            project_root: project_root.to_path_buf(),
        })
    }

    #[cfg(feature = "modernbert")]
    pub async fn sense(&self, query: &str, file_context: Option<&str>) -> Result<SenseResponse> {
        let model = self.ai_manager.load_model(AiModel::ModernBert, DeviceType::Cpu)?;
        let query_vector_tensor = model.get_embedding(query)?;
        let query_vector: Vec<f32> = query_vector_tensor.to_vec1()?;

        if let Some(file_path) = file_context {
            // ZOOM MODE: Search within a specific file
            let index = self.index_file(file_path, &model).await?;
            let results = index.search(&query_vector, 5);
            
            Ok(SenseResponse::Zoom {
                file_path: file_path.to_string(),
                nodes: results.into_iter().map(|(n, score)| NodeMatch {
                    path: n.path.clone(),
                    preview: n.content_preview.clone(),
                    score,
                }).collect(),
            })
        } else {
            // SATELITE MODE: Search across files
            // For now, let's pretend we have a list of important files to check
            // In a real implementation, we would use a pre-built project index
            Ok(SenseResponse::Satelite {
                matches: vec![
                    FileMatch { file_path: "src/main.rs".into(), score: 0.8 },
                    FileMatch { file_path: "src/core/mod.rs".into(), score: 0.6 },
                ]
            })
        }
    }

    #[cfg(feature = "modernbert")]
    pub async fn propose_edit(&self, anchor_query: &str, file_path: &str, intent: &str) -> Result<EditProposal> {
        let model = self.ai_manager.load_model(AiModel::ModernBert, DeviceType::Cpu)?;
        let index = self.index_file(file_path, &model).await?;
        
        let query_vector_tensor = model.get_embedding(anchor_query)?;
        let query_vector: Vec<f32> = query_vector_tensor.to_vec1()?;
        
        let results = index.search(&query_vector, 1);
        if results.is_empty() {
            anyhow::bail!("Could not find a semantic anchor for '{}'", anchor_query);
        }
        
        let (anchor_node, score) = results[0];
        
        // Logic to determine placement based on intent
        // (Simplified for first version)
        let proposal = match intent.to_lowercase().as_str() {
            "after" => {
                // To insert after, we need to find the parent and the index of the anchor
                EditProposal {
                    anchor_path: anchor_node.path.clone(),
                    suggested_op: "insert".into(),
                    parent_path: self.get_parent_path(&anchor_node.path),
                    position: self.get_next_index(&anchor_node.path),
                    confidence: score,
                }
            }
            _ => anyhow::bail!("Unsupported intent: {}", intent),
        };
        
        Ok(proposal)
    }

    fn get_parent_path(&self, path: &str) -> String {
        if let Some(last_dot) = path.rfind('.') {
            path[..last_dot].to_string()
        } else {
            // If there's no dot, it's a top-level node. 
            // In GnawTreeWriter, top-level nodes are children of the root "" or "0"
            "0".to_string() 
        }
    }

    fn get_next_index(&self, path: &str) -> usize {
        let last_part = if let Some(last_dot) = path.rfind('.') {
            &path[last_dot + 1..]
        } else {
            path
        };
        
        last_part.parse::<usize>().unwrap_or(0) + 1
    }

    #[cfg(feature = "modernbert")]
    async fn index_file(&self, file_path: &str, model: &crate::llm::ModernBertModel) -> Result<SemanticIndex> {
        let content = fs::read_to_string(file_path)?;
        let path = Path::new(file_path);
        let parser = crate::parser::get_parser(path)?;
        let tree = parser.parse(&content)?;

        let mut index = SemanticIndex::new(file_path);
        
        // Collect important nodes (functions, classes, etc.)
        let mut nodes = Vec::new();
        fn collect(n: &TreeNode, acc: &mut Vec<TreeNode>) {
            // Only index "meaningful" nodes to save time/space
            if n.node_type.contains("definition") || n.node_type.contains("item") {
                acc.push(n.clone());
            }
            for c in &n.children { collect(c, acc); }
        }
        collect(&tree, &mut nodes);

        for node in nodes {
            let vector_tensor = model.get_embedding(&node.content)?;
            let vector: Vec<f32> = vector_tensor.to_vec1()?;
            let preview = if node.content.len() > 100 {
                format!("{}...", &node.content[..97])
            } else {
                node.content.clone()
            };
            index.add_node(node.path, preview, vector);
        }

        Ok(index)
    }
}
