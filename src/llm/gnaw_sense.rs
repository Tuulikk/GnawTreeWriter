#[cfg(feature = "modernbert")]
use crate::llm::{
    AiManager, AiModel, DeviceType, NodeEmbedding, RelationType, RelationalIndexer, SemanticIndex,
};
#[cfg(feature = "modernbert")]
use crate::parser::TreeNode;
use anyhow::Result;
#[cfg(feature = "modernbert")]
use std::fs;
use std::path::{Path, PathBuf};

pub struct GnawSenseBroker {
    #[allow(dead_code)]
    #[cfg(feature = "modernbert")]
    ai_manager: AiManager,
    #[allow(dead_code)]
    #[cfg(not(feature = "modernbert"))]
    ai_manager: crate::llm::AiManager,
    #[allow(dead_code)]
    project_root: PathBuf,
    #[cfg(feature = "modernbert")]
    relational_indexer: RelationalIndexer,
    /// JIT cache: file_path -> (SemanticIndex, content_hash).
    /// Avoids re-embedding unchanged files on repeated Zoom queries.
    #[cfg(feature = "modernbert")]
    jit_cache: std::sync::Mutex<std::collections::HashMap<String, CachedFileIndex>>,
}

/// Truncate a string to at most `max_chars` characters (char-boundary safe),
/// appending `...` when truncated. Byte slicing panics on multibyte UTF-8
/// (e.g. `—`, emoji, CJK), so previews must always go through this helper.
#[cfg_attr(not(feature = "modernbert"), allow(dead_code))]
pub(crate) fn truncate_preview(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let cut: String = s.chars().take(max_chars).collect();
    format!("{}...", cut)
}

/// Maximum number of definition nodes embedded per file during JIT indexing.
/// Embedding is the dominant cost (CPU attention is quadratic in sequence
/// length); without a cap, large files (1000+ nodes) take minutes and blow
/// the MCP client timeout. Zoom search is for localization — the biggest
/// definitions are enough; `ai index` builds the full project index.
#[cfg(feature = "modernbert")]
const MAX_EMBED_NODES: usize = 24;

/// Maximum characters of a node's content fed to the embedding model.
/// ModernBERT's context is 8192 tokens, but CPU attention cost is quadratic
/// in sequence length: a measured 14-line file (short nodes) completes in
/// ~1 s while 4000-char nodes push single-file indexing past the MCP client
/// timeout. Truncation also avoids the hard rope-context crash on huge
/// nodes ("inconsistent last dim size in rope").
#[cfg(feature = "modernbert")]
const MAX_EMBED_CHARS: usize = 1200;

/// Maximum number of files kept in the JIT cache. Embeddings are large
/// (vector per node); an unbounded cache grows to hundreds of MB in
/// long-lived MCP sessions.
#[cfg(feature = "modernbert")]
const MAX_JIT_CACHE_FILES: usize = 32;

/// Cached file index with content hash for invalidation.
#[cfg(feature = "modernbert")]
struct CachedFileIndex {
    index: SemanticIndex,
    content_hash: String,
}

#[derive(Debug, serde::Serialize)]
pub enum SenseResponse {
    Satelite {
        matches: Vec<FileMatch>,
    },
    Zoom {
        file_path: String,
        nodes: Vec<NodeMatch>,
        impact: Option<Vec<ImpactMatch>>,
    },
}

#[derive(Debug, serde::Serialize)]
pub struct FileMatch {
    pub file_path: String,
    pub node_path: Option<String>,
    pub score: f32,
    /// Chunk preview straight from the index (ROADMAP 9.4 nice-to-have):
    /// score + preview in one response saves an extra read_node round-trip.
    pub content_preview: String,
}

#[derive(Debug, serde::Serialize)]
pub struct NodeMatch {
    pub path: String,
    pub preview: String,
    pub score: f32,
}

#[derive(Debug, serde::Serialize)]
pub struct ImpactMatch {
    pub file_path: String,
    pub node_path: String,
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
            ai_manager: crate::llm::AiManager::new(project_root)?,
            project_root: project_root.to_path_buf(),
            #[cfg(feature = "modernbert")]
            relational_indexer: RelationalIndexer::new(project_root),
            #[cfg(feature = "modernbert")]
            jit_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        })
    }

    pub fn get_manager(&self) -> &crate::llm::AiManager {
        &self.ai_manager
    }

    #[cfg(feature = "modernbert")]
    pub async fn sense(&self, query: &str, file_context: Option<&str>) -> Result<SenseResponse> {
        let model = self
            .ai_manager
            .load_model(AiModel::ModernBert, DeviceType::Cpu)?;
        let query_vector_tensor = model.get_embedding(query)?;
        let query_vector: Vec<f32> = query_vector_tensor.to_vec1()?;

        if let Some(file_path) = file_context {
            // ZOOM MODE: Search within a specific file (with JIT cache)
            let index = self.index_file_cached(file_path, model).await?;
            let results = index.search(&query_vector, 5);

            // JIT RELATIONAL ANALYSIS: Index the directory to find callers
            if let Some(parent) = Path::new(file_path).parent() {
                let mut indexer = RelationalIndexer::new(&self.project_root);
                let _ = indexer.index_directory(parent);
            }

            // Find impact of top match
            let mut impact_matches = Vec::new();
            if !results.is_empty() {
                let top_node = &results[0].0;
                // If it's a definition, look for callers
                if let Some(name) = self.extract_name_from_preview(&top_node.content_preview) {
                    let all_graphs = self.relational_indexer.load_all_graphs()?;
                    for graph in all_graphs {
                        for rel in graph.relations {
                            if rel.to_name == name && rel.relation_type == RelationType::Call {
                                impact_matches.push(ImpactMatch {
                                    file_path: graph.file_path.clone(),
                                    node_path: rel.from_path.clone(),
                                });
                            }
                        }
                    }
                }
            }

            Ok(SenseResponse::Zoom {
                file_path: file_path.to_string(),
                nodes: results
                    .into_iter()
                    .map(|(n, score)| NodeMatch {
                        path: n.node_path.clone(),
                        preview: n.content_preview.clone(),
                        score,
                    })
                    .collect(),
                impact: if impact_matches.is_empty() {
                    None
                } else {
                    Some(impact_matches)
                },
            })
        } else {
            // SATELITE MODE: Search across the entire project index
            let index_mgr = crate::llm::SemanticIndexManager::new(&self.project_root);
            let project_index = index_mgr.load_project_index()?;
            let results = project_index.search(&query_vector, 10);

            Ok(SenseResponse::Satelite {
                matches: results
                    .into_iter()
                    .map(|(entry, score)| FileMatch {
                        file_path: entry.file_path.clone(),
                        node_path: Some(entry.node_path.clone()),
                        score,
                        content_preview: entry.content_preview.clone(),
                    })
                    .collect(),
            })
        }
    }

    #[cfg(feature = "modernbert")]
    pub async fn propose_edit(
        &self,
        anchor_query: &str,
        file_path: &str,
        intent: &str,
    ) -> Result<EditProposal> {
        let model = self
            .ai_manager
            .load_model(AiModel::ModernBert, DeviceType::Cpu)?;
        let index = self.index_file(file_path, model).await?;

        let query_vector_tensor = model.get_embedding(anchor_query)?;
        let query_vector: Vec<f32> = query_vector_tensor.to_vec1()?;

        let results = index.search(&query_vector, 1);
        if results.is_empty() {
            anyhow::bail!("Could not find a semantic anchor for '{}'", anchor_query);
        }

        let (anchor_node, score) = results[0];

        // Logic to determine placement based on intent
        let proposal = match intent.to_lowercase().as_str() {
            "after" => EditProposal {
                anchor_path: anchor_node.node_path.clone(),
                suggested_op: "insert".into(),
                parent_path: self.get_parent_path(&anchor_node.node_path),
                position: self.get_next_index(&anchor_node.node_path),
                confidence: score,
            },
            "before" => EditProposal {
                anchor_path: anchor_node.node_path.clone(),
                suggested_op: "insert".into(),
                parent_path: self.get_parent_path(&anchor_node.node_path),
                position: self.get_current_index(&anchor_node.node_path),
                confidence: score,
            },
            "inside" => {
                // Insert as the last child of the anchor node
                EditProposal {
                    anchor_path: anchor_node.node_path.clone(),
                    suggested_op: "insert".into(),
                    parent_path: anchor_node.node_path.clone(),
                    position: 1, // bottom of the anchor node
                    confidence: score,
                }
            }
            "replace" => EditProposal {
                anchor_path: anchor_node.node_path.clone(),
                suggested_op: "edit".into(),
                parent_path: anchor_node.node_path.clone(),
                position: 0,
                confidence: score,
            },
            _ => anyhow::bail!(
                "Unsupported intent: {}. Supported: after, before, inside, replace",
                intent
            ),
        };

        Ok(proposal)
    }

    #[allow(dead_code)]
    #[cfg(feature = "modernbert")]
    fn get_parent_path(&self, path: &str) -> String {
        if let Some(last_dot) = path.rfind('.') {
            path[..last_dot].to_string()
        } else {
            "0".to_string()
        }
    }

    #[allow(dead_code)]
    #[cfg(feature = "modernbert")]
    fn get_next_index(&self, path: &str) -> usize {
        let last_part = if let Some(last_dot) = path.rfind('.') {
            &path[last_dot + 1..]
        } else {
            path
        };

        let idx = last_part.parse::<usize>().unwrap_or(0);
        // Position encoding: 3+ means "insert after child at index (pos-3)"
        // idx+3 = insert after child at index idx (the anchor node)
        idx + 3
    }

    /// Get the position for inserting BEFORE a node (used for "before" intent).
    /// Returns 0 (top of parent) if anchor is the first child, otherwise
    /// inserts after the previous sibling.
    #[allow(dead_code)]
    #[cfg(feature = "modernbert")]
    fn get_current_index(&self, path: &str) -> usize {
        let last_part = if let Some(last_dot) = path.rfind('.') {
            &path[last_dot + 1..]
        } else {
            path
        };
        let idx = last_part.parse::<usize>().unwrap_or(0);
        if idx == 0 {
            // Anchor is first child — insert at top of parent
            0
        } else {
            // Insert after previous sibling (before anchor)
            (idx - 1) + 3
        }
    }

    #[cfg(feature = "modernbert")]
    async fn index_file_cached(
        &self,
        file_path: &str,
        model: &crate::llm::ModernBertModel,
    ) -> Result<SemanticIndex> {
        let content = fs::read_to_string(file_path)?;
        let content_hash = crate::core::calculate_content_hash(&content);

        // Check JIT cache
        {
            let cache = self.jit_cache.lock().unwrap();
            if let Some(cached) = cache.get(file_path) {
                if cached.content_hash == content_hash {
                    return Ok(cached.index.clone());
                }
            }
        }

        // Cache miss — index from pre-loaded content
        let index = self
            .index_file_from_content(file_path, &content, model)
            .await?;

        // Store in cache, evicting the oldest entry when at capacity so
        // long-lived MCP sessions cannot accumulate unbounded memory.
        {
            let mut cache = self.jit_cache.lock().unwrap();
            if cache.len() >= MAX_JIT_CACHE_FILES && !cache.contains_key(file_path) {
                if let Some(oldest) = cache.keys().next().cloned() {
                    cache.remove(&oldest);
                }
            }
            cache.insert(
                file_path.to_string(),
                CachedFileIndex {
                    index: index.clone(),
                    content_hash,
                },
            );
        }

        Ok(index)
    }

    #[cfg(feature = "modernbert")]
    async fn index_file_from_content(
        &self,
        file_path: &str,
        content: &str,
        model: &crate::llm::ModernBertModel,
    ) -> Result<SemanticIndex> {
        let path = Path::new(file_path);
        let parser = crate::parser::get_parser(path)?;
        let tree = parser.parse(content)?;

        let mut index = SemanticIndex::default();

        let mut nodes = Vec::new();
        fn collect(n: &TreeNode, acc: &mut Vec<TreeNode>) {
            if n.node_type.contains("definition") || n.node_type.contains("item") {
                acc.push(n.clone());
            }
            for c in &n.children {
                collect(c, acc);
            }
        }
        collect(&tree, &mut nodes);

        // Embed only the largest definitions when the file is huge —
        // keeps first-call JIT latency within MCP client timeouts.
        if nodes.len() > MAX_EMBED_NODES {
            nodes.sort_by_key(|n| std::cmp::Reverse(n.content.len()));
            nodes.truncate(MAX_EMBED_NODES);
        }
        for node in nodes {
            let embed_text: String = node.content.chars().take(MAX_EMBED_CHARS).collect();
            let vector_tensor = model.get_embedding(&embed_text)?;
            let vector: Vec<f32> = vector_tensor.to_vec1()?;
            let preview = truncate_preview(&node.content, 97);
            index.entries.push(NodeEmbedding {
                file_path: file_path.to_string(),
                node_path: node.path,
                content_preview: preview,
                vector,
            });
        }

        Ok(index)
    }

    #[cfg(feature = "modernbert")]
    async fn index_file(
        &self,
        file_path: &str,
        model: &crate::llm::ModernBertModel,
    ) -> Result<SemanticIndex> {
        let content = fs::read_to_string(file_path)?;
        let path = Path::new(file_path);
        let parser = crate::parser::get_parser(path)?;
        let tree = parser.parse(&content)?;

        let mut index = SemanticIndex::default();

        // Collect important nodes (functions, classes, etc.)
        let mut nodes = Vec::new();
        fn collect(n: &TreeNode, acc: &mut Vec<TreeNode>) {
            if n.node_type.contains("definition") || n.node_type.contains("item") {
                acc.push(n.clone());
            }
            for c in &n.children {
                collect(c, acc);
            }
        }
        collect(&tree, &mut nodes);

        // Embed only the largest definitions when the file is huge —
        // keeps first-call JIT latency within MCP client timeouts.
        if nodes.len() > MAX_EMBED_NODES {
            nodes.sort_by_key(|n| std::cmp::Reverse(n.content.len()));
            nodes.truncate(MAX_EMBED_NODES);
        }
        for node in nodes {
            let embed_text: String = node.content.chars().take(MAX_EMBED_CHARS).collect();
            let vector_tensor = model.get_embedding(&embed_text)?;
            let vector: Vec<f32> = vector_tensor.to_vec1()?;
            let preview = truncate_preview(&node.content, 97);
            index.entries.push(NodeEmbedding {
                file_path: file_path.to_string(),
                node_path: node.path,
                content_preview: preview,
                vector,
            });
        }

        Ok(index)
    }

    #[cfg(feature = "modernbert")]
    fn extract_name_from_preview(&self, preview: &str) -> Option<String> {
        // Multi-language name extraction from AST node content preview.
        // Returns the first identifier that looks like a definition name.
        let patterns: &[&str] = &[
            // Rust
            "fn ",
            "struct ",
            "enum ",
            "trait ",
            "impl ",
            "type ",
            "mod ",
            // Python
            "def ",
            "class ",
            // Go
            "func ",
            // JavaScript/TypeScript
            "function ",
            "let ",
            "const ",
            "var ",
            // Java/C/C++
            "void ",
            "int ",
            "bool ",
            "string ",
            // QML
            "property ",
        ];

        for pattern in patterns {
            if let Some(pos) = preview.find(pattern) {
                let after = &preview[pos + pattern.len()..];
                // Skip modifiers: "pub ", "async ", "mut ", etc.
                let after = after.trim_start();
                let name_end = after
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(after.len());
                if name_end > 0 {
                    return Some(after[..name_end].trim().to_string());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::truncate_preview;

    /// Regression: byte-slicing previews (`&s[..97]`) panicked on multibyte
    /// UTF-8. Previews must always be char-boundary safe.
    #[test]
    fn truncate_preview_is_char_boundary_safe() {
        // Em dash just past the limit (the exact crash from Motor2 system.rs
        // had it at byte 96-99).
        let s = format!("{}—", "x".repeat(200));
        let p = truncate_preview(&s, 97);
        assert_eq!(p.chars().count(), 100); // 97 + "..."
        assert!(p.ends_with("..."));

        // CJK (3 bytes/char) and emoji (4 bytes/char).
        let cjk = "函".repeat(200);
        assert_eq!(truncate_preview(&cjk, 97).chars().count(), 100);
        let emoji = "🦀".repeat(200);
        assert_eq!(truncate_preview(&emoji, 97).chars().count(), 100);
    }

    #[test]
    fn truncate_preview_short_strings_unchanged() {
        assert_eq!(truncate_preview("", 97), "");
        assert_eq!(truncate_preview("fn main() {}", 97), "fn main() {}");
        let s = "—".repeat(97); // exactly at the limit
        assert_eq!(truncate_preview(&s, 97), s);
    }
}
