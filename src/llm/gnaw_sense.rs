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

/// Search-outcome log (feedback loop / roadmap AUTO-koppling): failed
/// satellite searches are appended so `prior_failures` can be surfaced
/// to agents and the data can drive future scoring work.
pub const SEARCH_LOG_FILE: &str = ".gnawtreewriter_search_log.jsonl";

fn normalize_search_query(q: &str) -> String {
    q.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Failures recorded for this exact query BEFORE now (0 = first time).
pub fn search_log_prior_failures(project_root: &std::path::Path, query: &str) -> u64 {
    let norm = normalize_search_query(query);
    std::fs::read_to_string(project_root.join(SEARCH_LOG_FILE))
        .ok()
        .map(|text| {
            text.lines()
                .filter(|line| {
                    serde_json::from_str::<serde_json::Value>(line)
                        .ok()
                        .and_then(|v| {
                            v.get("query_norm")
                                .and_then(|n| n.as_str())
                                .map(str::to_string)
                        })
                        .as_deref()
                        == Some(norm.as_str())
                })
                .count() as u64
        })
        .unwrap_or(0)
}

/// Append one failure entry; rotate above 1 MB (keep the newest 500).
pub fn search_log_record_failure(
    project_root: &std::path::Path,
    query: &str,
    reason: &str,
    result_count: usize,
    top_cosine: Option<f32>,
) -> std::io::Result<()> {
    use std::io::Write;
    let path = project_root.join(SEARCH_LOG_FILE);
    let entry = serde_json::json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "query": query,
        "query_norm": normalize_search_query(query),
        "reason": reason,
        "result_count": result_count,
        "top_cosine": top_cosine,
    });
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    writeln!(file, "{}", entry)?;
    drop(file);
    if std::fs::metadata(&path)
        .map(|m| m.len() > 1_048_576)
        .unwrap_or(false)
    {
        let text = std::fs::read_to_string(&path)?;
        let mut kept: Vec<&str> = text.lines().rev().take(500).collect();
        kept.reverse();
        std::fs::write(&path, format!("{}\n", kept.join("\n")))?;
    }
    Ok(())
}

/// Quality signals attached to satellite answers (feedback loop): what
/// was suspect about this search and how often the same query failed
/// before — so agents can adjust instead of trusting a flat result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SatelliteQuality {
    /// Failures for this exact query recorded BEFORE this call.
    pub prior_failures: u64,
    /// None = looked fine; else "empty" | "low_cosine" | "no_lex_overlap".
    pub suspect_reason: Option<String>,
    /// Present only when expand=true was requested.
    pub expansion: Option<ExpansionInfo>,
}

/// Query-expansion outcome (expand=true; LFM2.5 terms, mamba builds).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExpansionInfo {
    pub applied: bool,
    pub terms: Vec<String>,
    pub note: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub enum SenseResponse {
    Satelite {
        matches: Vec<FileMatch>,
        quality: SatelliteQuality,
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
    /// PARENT node path — chunked index entries (`8[chunk:2]`) are mapped
    /// back so read_node always resolves (chain must not break).
    pub node_path: Option<String>,
    /// Reranked display score (cosine + bounded priors) — can exceed 1.0.
    pub score: f32,
    /// Raw cosine from the embedding model, verbatim (0.0..=1.0).
    pub cosine: Option<f32>,
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
        self.sense_with(query, file_context, false).await
    }

    /// `expand = true` (satellite only): expand the query with LFM2.5
    /// terms (mamba builds), embed the expanded text as a SECOND channel,
    /// fuse both searches by max cosine, and give the reranker the
    /// expanded text for its lexical side. One extra embedding (+ one LLM
    /// call when mamba is present) — opt-in per call.
    #[cfg(feature = "modernbert")]
    pub async fn sense_with(
        &self,
        query: &str,
        file_context: Option<&str>,
        expand: bool,
    ) -> Result<SenseResponse> {
        let model = self.ai_manager.load_model(AiModel::Bge, DeviceType::Cpu)?;
        let query_vector_tensor = model.get_query_embedding(query)?;
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

            // Query expansion (opt-in): a second semantic channel fused by
            // max cosine; the expanded text also feeds the reranker's
            // lexical side. Skips gracefully on non-mamba builds.
            // (cfg-split bindings: mut lives inside each branch, so neither
            // build sees a needlessly-mut variable.)
            #[cfg(feature = "mamba")]
            let (lex_query, expanded, mut expansion_info) = {
                let mut lex_query = query.to_string();
                let mut expanded = false;
                let mut expansion_info: Option<ExpansionInfo> = None;
                if expand {
                    let outcome = crate::llm::AiManager::new(&self.project_root)
                        .and_then(|mgr| crate::llm::pipeline::expand_query_terms(&mgr, query));
                    match outcome {
                        Ok(terms) if !terms.is_empty() => {
                            lex_query = format!("{} {}", query, terms.join(" "));
                            expanded = true;
                            expansion_info = Some(ExpansionInfo {
                                applied: true,
                                terms,
                                note: None,
                            });
                        }
                        Ok(_) => {
                            expansion_info = Some(ExpansionInfo {
                                applied: false,
                                terms: vec![],
                                note: Some("expansion model returned no terms".to_string()),
                            });
                        }
                        Err(e) => {
                            expansion_info = Some(ExpansionInfo {
                                applied: false,
                                terms: vec![],
                                note: Some(format!("expansion failed: {}", e)),
                            });
                        }
                    }
                }
                (lex_query, expanded, expansion_info)
            };
            #[cfg(not(feature = "mamba"))]
            let (lex_query, expanded, mut expansion_info) = {
                let expansion_info = if expand {
                    Some(ExpansionInfo {
                        applied: false,
                        terms: vec![],
                        note: Some(
                            "expansion requires the 'mamba' feature — rebuild with --features mamba"
                                .to_string(),
                        ),
                    })
                } else {
                    None
                };
                (query.to_string(), false, expansion_info)
            };

            // Wide net: implementations are long/diffuse and rank low in
            // raw cosine, so a narrow window would never even show them to
            // the reranker. Lower floor + large window, then rerank to 10.
            let base_hits = project_index.search_with_threshold(&query_vector, 2000, 0.1);
            let hits = if expanded {
                match model.get_embedding(&lex_query) {
                    Ok(exp_tensor) => {
                        let exp_vector: Vec<f32> = exp_tensor.to_vec1()?;
                        let exp_hits = project_index.search_with_threshold(&exp_vector, 2000, 0.1);
                        crate::llm::fuse_by_max(base_hits, exp_hits)
                    }
                    Err(e) => {
                        if let Some(info) = expansion_info.as_mut() {
                            info.applied = false;
                            info.note = Some(format!("expanded embedding failed: {}", e));
                        }
                        base_hits
                    }
                }
            } else {
                base_hits
            };
            // Graph channel: call-graph proximity feeds the reranker
            // (silent degrade — no graphs on disk means two-channel).
            let graph_ok = self
                .relational_indexer
                .load_all_graphs()
                .unwrap_or_default();
            let results =
                crate::llm::rerank_satellite_with_graphs(&lex_query, hits, 10, Some(&graph_ok));

            // Feedback loop (roadmap AUTO-koppling): flag suspicious
            // outcomes and remember failures so the agent sees
            // prior_failures instead of trusting a flat field blindly.
            let (suspect_reason, top_cosine) = if results.is_empty() {
                (Some("empty".to_string()), None)
            } else {
                let top = &results[0];
                let hay = format!(
                    "{} {}",
                    top.entry.content_preview.to_lowercase(),
                    top.entry.file_path.to_lowercase()
                );
                let lex_hit = lex_query
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|w| w.len() >= 3)
                    .any(|w| hay.contains(w));
                if top.cosine < 0.5 {
                    (Some("low_cosine".to_string()), Some(top.cosine))
                } else if !lex_hit {
                    (Some("no_lex_overlap".to_string()), Some(top.cosine))
                } else {
                    (None, Some(top.cosine))
                }
            };
            let prior_failures = search_log_prior_failures(&self.project_root, query);
            if let Some(reason) = &suspect_reason {
                let _ = search_log_record_failure(
                    &self.project_root,
                    query,
                    reason,
                    results.len(),
                    top_cosine,
                );
            }
            let quality = SatelliteQuality {
                prior_failures,
                suspect_reason,
                expansion: expansion_info,
            };

            Ok(SenseResponse::Satelite {
                matches: results
                    .into_iter()
                    .map(|hit| FileMatch {
                        file_path: hit.entry.file_path.clone(),
                        node_path: Some(
                            crate::llm::parent_node_path(&hit.entry.node_path).to_string(),
                        ),
                        score: hit.adjusted,
                        cosine: Some(hit.cosine),
                        content_preview: hit.entry.content_preview.clone(),
                    })
                    .collect(),
                quality,
            })
        }
    }

    /// Honest stub for --no-default-features builds (Motor2 bug report
    /// 2026-10-05): the real sense_with body needs ModernBERT. Callers
    /// get a clean runtime error instead of a compile failure — external
    /// path-dependents may call this directly.
    #[cfg(not(feature = "modernbert"))]
    pub async fn sense_with(
        &self,
        query: &str,
        file_context: Option<&str>,
        expand: bool,
    ) -> Result<SenseResponse> {
        let _ = (query, file_context, expand);
        anyhow::bail!(
            "sense_with requires the 'modernbert' feature — rebuild with --features modernbert (README: Full power)"
        )
    }

    #[cfg(feature = "modernbert")]
    pub async fn propose_edit(
        &self,
        anchor_query: &str,
        file_path: &str,
        intent: &str,
    ) -> Result<EditProposal> {
        let model = self.ai_manager.load_model(AiModel::Bge, DeviceType::Cpu)?;
        let index = self.index_file(file_path, model).await?;

        let query_vector_tensor = model.get_query_embedding(anchor_query)?;
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
        // One forward per 16 nodes instead of one per node (batched embedding).
        let texts: Vec<String> = nodes
            .iter()
            .map(|n| n.content.chars().take(MAX_EMBED_CHARS).collect())
            .collect();
        let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
        let vectors = model.get_embeddings(&refs)?;
        for (node, vector) in nodes.into_iter().zip(vectors) {
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
        // One forward per 16 nodes instead of one per node (batched embedding).
        let texts: Vec<String> = nodes
            .iter()
            .map(|n| n.content.chars().take(MAX_EMBED_CHARS).collect())
            .collect();
        let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
        let vectors = model.get_embeddings(&refs)?;
        for (node, vector) in nodes.into_iter().zip(vectors) {
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

    /// Feedback loop: failures persist, prior_failures counts per exact
    /// query (normalized), successes are never logged by the helpers.
    #[test]
    fn search_log_records_and_counts_prior_failures() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        assert_eq!(
            super::search_log_prior_failures(root, "how does undo work"),
            0,
            "no log yet"
        );

        super::search_log_record_failure(root, "how does undo work", "empty", 0, None)
            .expect("record");
        // Normalization: case + whitespace must match the same query.
        assert_eq!(
            super::search_log_prior_failures(root, "  HOW does   undo work "),
            1
        );
        // Different query unaffected.
        assert_eq!(super::search_log_prior_failures(root, "other query"), 0);

        super::search_log_record_failure(
            root,
            "how does undo work",
            "no_lex_overlap",
            10,
            Some(0.87),
        )
        .expect("record");
        assert_eq!(
            super::search_log_prior_failures(root, "how does undo work"),
            2,
            "second failure counted"
        );

        // The log file holds parseable JSONL with the expected shape.
        let text = std::fs::read_to_string(root.join(super::SEARCH_LOG_FILE)).unwrap();
        assert_eq!(text.lines().count(), 2);
        let first: serde_json::Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        assert_eq!(first["reason"], "empty");
        assert_eq!(first["result_count"], 0);
        assert!(first["ts"].is_string());
    }

    #[test]
    fn search_log_rotates_large_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let path = root.join(super::SEARCH_LOG_FILE);
        // Fake a > 1 MB log (padded so 600 lines cross the threshold).
        let line = format!(
            "{}{}",
            r#"{"ts":"2026-10-05T00:00:00Z","query":"q","query_norm":"q","reason":"empty","result_count":0,"top_cosine":null,"pad":"#,
            "x".repeat(2000)
        );
        std::fs::write(&path, format!("{}\n", vec![line; 600].join("\n"))).unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() > 1_048_576);

        super::search_log_record_failure(root, "new query", "empty", 0, None)
            .expect("record+rotate");
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            text.lines().count(),
            500,
            "rotation keeps the newest 500 lines (the just-appended entry included)"
        );
        assert!(
            text.lines().last().unwrap().contains("new query"),
            "newest entry survives rotation"
        );
    }
}
