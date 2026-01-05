use anyhow::{Context, Result};
#[cfg(feature = "modernbert")]
use candle_core::{DType, Device, Tensor};
#[cfg(feature = "modernbert")]
use candle_nn::{self, VarBuilder};
#[cfg(feature = "modernbert")]
use candle_transformers::models::modernbert::{Config, ModernBert};
#[cfg(feature = "modernbert")]
use hf_hub::{Repo, RepoType};
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(feature = "modernbert")]
use tokenizers::Tokenizer;

/// Supported AI models for local execution
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AiModel {
    ModernBert,
}

/// Execution device for AI models
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum DeviceType {
    Cpu,
    Cuda,
    Metal,
}

impl From<&str> for DeviceType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cuda" => DeviceType::Cuda,
            "metal" => DeviceType::Metal,
            _ => DeviceType::Cpu,
        }
    }
}

#[cfg(feature = "modernbert")]
pub struct ModernBertModel {
    pub model: ModernBert,
    pub tokenizer: Tokenizer,
    pub device: Device,
}

#[cfg(feature = "modernbert")]
impl ModernBertModel {
    pub fn get_embedding(&self, text: &str) -> Result<Tensor> {
        let tokens = self
            .tokenizer
            .encode(text, true)
            .map_err(anyhow::Error::msg)?;
        let token_ids = tokens.get_ids();
        let input_ids = Tensor::new(token_ids, &self.device)?.unsqueeze(0)?;

        // ModernBERT forward pass
        let mask = input_ids.ones_like()?;
        let embeddings = self.model.forward(&input_ids, &mask)?;

        // Mean pooling to get a single vector for the text
        // embeddings shape: [batch, seq_len, hidden_size]
        let mean_embedding = embeddings.mean(1)?; // Mean over seq_len

        Ok(mean_embedding.squeeze(0)?) // Remove batch dim
    }

    pub fn fill_mask(&self, text: &str, top_k: usize) -> Result<Vec<(String, f32)>> {
        let tokens = self
            .tokenizer
            .encode(text, true)
            .map_err(anyhow::Error::msg)?;
        let token_ids = tokens.get_ids();

        // Find mask token ID
        let mask_token = "[MASK]";
        let mask_id = self
            .tokenizer
            .token_to_id(mask_token)
            .context("Mask token not found in tokenizer")?;

        let mask_indices: Vec<usize> = token_ids
            .iter()
            .enumerate()
            .filter(|(_, &id)| id == mask_id)
            .map(|(i, _)| i)
            .collect();

        if mask_indices.is_empty() {
            anyhow::bail!("No [MASK] token found in input text");
        }

        let input_ids = Tensor::new(token_ids, &self.device)?.unsqueeze(0)?;
        let mask = input_ids.ones_like()?;
        let logits = self.model.forward(&input_ids, &mask)?;

        let mut results = Vec::new();
        for &mask_idx in &mask_indices {
            let mask_logits = logits.get(0)?.get(mask_idx)?;
            let probs = candle_nn::ops::softmax(&mask_logits, 0)?;
            let probs_v: Vec<f32> = probs.to_vec1()?;

            let mut indexed_probs: Vec<(usize, f32)> = probs_v.into_iter().enumerate().collect();
            indexed_probs
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            for i in 0..top_k.min(indexed_probs.len()) {
                let (id, prob) = indexed_probs[i];
                let token = self
                    .tokenizer
                    .id_to_token(id as u32)
                    .unwrap_or_else(|| format!("[ID:{}]", id));
                results.push((token, prob));
            }
        }

        Ok(results)
    }
}

/// Manager for local AI models and inference
pub struct AiManager {
    model_cache_dir: PathBuf,
}

impl AiManager {
    #[cfg(feature = "modernbert")]
    fn get_candle_device(&self, device_type: &DeviceType) -> Result<Device> {
        match device_type {
            DeviceType::Cpu => Ok(Device::Cpu),
            DeviceType::Cuda => {
                Ok(Device::new_cuda(0).context("CUDA device not found or not supported")?)
            }
            DeviceType::Metal => {
                Ok(Device::new_metal(0).context("Metal device not found or not supported")?)
            }
        }
    }

    #[cfg(feature = "modernbert")]
    pub fn load_model(
        &self,
        model_type: AiModel,
        device_type: DeviceType,
    ) -> Result<ModernBertModel> {
        let model_dir = self.get_model_path(&model_type);
        let config_path = model_dir.join("config.json");
        let weights_path = model_dir.join("model.safetensors");
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !config_path.exists() || !weights_path.exists() || !tokenizer_path.exists() {
            anyhow::bail!("Model files missing. Run 'gnawtreewriter ai setup' first.");
        }

        let device = self.get_candle_device(&device_type)?;

        let config_str = fs::read_to_string(config_path)?;
        let config: Config = serde_json::from_str(&config_str)?;
        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(anyhow::Error::msg)?;

        let vb =
            unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &device)? };

        let model = ModernBert::load(vb, &config)?;

        Ok(ModernBertModel {
            model,
            tokenizer,
            device,
        })
    }

    pub fn new(project_root: &Path) -> Result<Self> {
        let model_cache_dir = project_root.join(".gnawtreewriter_ai").join("models");
        if !model_cache_dir.exists() {
            fs::create_dir_all(&model_cache_dir)
                .context("Failed to create AI model cache directory")?;
        }

        Ok(Self { model_cache_dir })
    }

    /// Setup and download a model
    pub async fn setup(&self, model: AiModel, device: DeviceType, force: bool) -> Result<()> {
        println!("🤖 Initializing setup for {:?} on {:?}...", model, device);

        #[cfg(not(feature = "modernbert"))]
        {
            anyhow::bail!(
                "ModernBERT feature is not enabled. Recompile with --features modernbert"
            );
        }

        #[cfg(feature = "modernbert")]
        {
            let model_id = match model {
                AiModel::ModernBert => "answerdotai/ModernBERT-base",
            };

            let api = hf_hub::api::sync::ApiBuilder::new()
                .with_progress(true)
                .build()
                .context("Failed to initialize Hugging Face API")?;
            let repo = api.repo(Repo::new(model_id.to_string(), RepoType::Model));

            println!(
                "📥 Downloading model files from Hugging Face Hub: {}...",
                model_id
            );

            let files = [
                "config.json",
                "model.safetensors",
                "tokenizer.json",
                "tokenizer_config.json",
            ];
            let model_dir = self.get_model_path(&model);

            if !model_dir.exists() {
                fs::create_dir_all(&model_dir).context("Failed to create model directory")?;
            }

            for file in files {
                let dest_path = model_dir.join(file);
                if dest_path.exists() && !force {
                    println!("✅ {} already exists.", file);
                    continue;
                }

                println!("⏳ Downloading {}...", file);
                match repo.get(file) {
                    Ok(downloaded_path) => {
                        fs::copy(&downloaded_path, &dest_path).with_context(|| {
                            format!("Failed to copy {} to {}", file, dest_path.display())
                        })?;
                    }
                    Err(e) => {
                        if file == "tokenizer_config.json" {
                            println!("⚠️ Optional file {} not found, skipping.", file);
                            continue;
                        }
                        return Err(e).context(format!("Failed to download {}", file));
                    }
                }
            }

            println!("🔍 Verifying model integrity...");
            // Basic verification: check if required files exist and are non-empty
            for file in &["config.json", "model.safetensors", "tokenizer.json"] {
                let path = model_dir.join(file);
                if !path.exists() {
                    anyhow::bail!("Required model file {} is missing", file);
                }
                let metadata = fs::metadata(&path)?;
                if metadata.len() == 0 {
                    anyhow::bail!("Downloaded file {} is empty", file);
                }
            }

            println!("✨ Model setup successfully for {:?}", device);
        }

        Ok(())
    }

    /// Get status of installed models
    pub fn get_status(&self) -> Result<AiStatus> {
        let modern_bert_installed = self
            .get_model_path(&AiModel::ModernBert)
            .join("config.json")
            .exists();

        // Detect available hardware
        let available_devices = vec![DeviceType::Cpu];

        // Mock detection logic
        #[cfg(feature = "cuda")]
        available_devices.push(DeviceType::Cuda);

        #[cfg(feature = "metal")]
        available_devices.push(DeviceType::Metal);

        Ok(AiStatus {
            modern_bert_installed,
            cache_dir: self.model_cache_dir.clone(),
            available_devices,
        })
    }

    /// Suggest refactorings based on code semantics
    pub async fn suggest_refactor(
        &self,
        file_path: &str,
        node_path: Option<&str>,
    ) -> Result<Vec<RefactorSuggestion>> {
        #[cfg(not(feature = "modernbert"))]
        {
            anyhow::bail!(
                "ModernBERT feature is not enabled. Recompile with --features modernbert"
            );
        }

        #[cfg(feature = "modernbert")]
        {
            if !self
                .get_model_path(&AiModel::ModernBert)
                .join("config.json")
                .exists()
            {
                anyhow::bail!("ModernBERT is not installed. Run 'gnawtreewriter ai setup' first.");
            }

            println!(
                "🧠 Analyzing {} for refactoring opportunities...",
                file_path
            );

            let model = self.load_model(AiModel::ModernBert, DeviceType::Cpu)?;

            // Read and parse the file
            let content = fs::read_to_string(file_path)
                .with_context(|| format!("Failed to read file: {}", file_path))?;

            let path = Path::new(file_path);
            let parser = crate::parser::get_parser(path)?;
            let tree = parser.parse(&content)?;

            // Flatten tree to get nodes
            let mut nodes = Vec::new();
            fn collect_nodes(
                node: &crate::parser::TreeNode,
                acc: &mut Vec<crate::parser::TreeNode>,
            ) {
                acc.push(node.clone());
                for child in &node.children {
                    collect_nodes(child, acc);
                }
            }
            collect_nodes(&tree, &mut nodes);

            let mut suggestions = Vec::new();

            // Heuristic: Find complex nodes using ModernBERT embeddings
            for node in nodes {
                if let Some(target) = node_path {
                    if node.path != target {
                        continue;
                    }
                }

                // Skip very small nodes (noise) and very large nodes (OOM)
                if node.content.len() < 50 || node.content.len() > 10000 || node.children.is_empty()
                {
                    continue;
                }

                let emb = model.get_embedding(&node.content)?;
                let norm = emb.sqr()?.sum_all()?.sqrt()?.to_scalar::<f32>()?;

                // Simple heuristic: high embedding norm often correlates with high information density/complexity
                if norm > 15.0 {
                    suggestions.push(RefactorSuggestion {
                        title: "Simplify Complex Node".to_string(),
                        description: format!(
                            "This {} node has high semantic complexity. Consider breaking it down.",
                            node.node_type
                        ),
                        target_node: node.path.clone(),
                        confidence: (norm / 20.0).min(0.95),
                    });
                }
            }

            if suggestions.is_empty() && node_path.is_some() {
                suggestions.push(RefactorSuggestion {
                    title: "General Cleanup".to_string(),
                    description: "No specific issues found, but a general cleanup is always good."
                        .to_string(),
                    target_node: node_path.unwrap().to_string(),
                    confidence: 0.5,
                });
            }

            Ok(suggestions)
        }
    }

    /// Get context-aware code completion suggestions
    pub async fn complete_code(
        &self,
        file_path: &str,
        node_path: &str,
    ) -> Result<Vec<CompletionSuggestion>> {
        #[cfg(not(feature = "modernbert"))]
        {
            anyhow::bail!(
                "ModernBERT feature is not enabled. Recompile with --features modernbert"
            );
        }

        #[cfg(feature = "modernbert")]
        {
            println!(
                "🧠 Generating code completion for {} at {}...",
                file_path, node_path
            );

            let model = self.load_model(AiModel::ModernBert, DeviceType::Cpu)?;

            // Read file content
            let content = fs::read_to_string(file_path)
                .with_context(|| format!("Failed to read file: {}", file_path))?;

            // For ModernBERT (encoder), we use fill-mask with AST context.
            let path = Path::new(file_path);
            let parser = crate::parser::get_parser(path)?;
            let tree = parser.parse(&content)?;

            // Find the target node to mask
            fn find_node<'a>(
                node: &'a crate::parser::TreeNode,
                path: &str,
            ) -> Option<&'a crate::parser::TreeNode> {
                if node.path == path {
                    return Some(node);
                }
                for child in &node.children {
                    if let Some(found) = find_node(child, path) {
                        return Some(found);
                    }
                }
                None
            }
            let target_node = find_node(&tree, node_path);

            let masked_content = if let Some(node) = target_node {
                let lines: Vec<&str> = content.lines().collect();
                let mut new_lines: Vec<String> = Vec::new();

                // Add context before the node
                for (i, line) in lines.iter().enumerate() {
                    let line_num = i + 1;
                    if line_num < node.start_line {
                        new_lines.push(line.to_string());
                    } else if line_num == node.start_line {
                        // Replace node content with [MASK]
                        let indentation: String =
                            line.chars().take_while(|c| c.is_whitespace()).collect();
                        new_lines.push(format!("{}[MASK]", indentation));
                    } else if line_num > node.end_line {
                        new_lines.push(line.to_string());
                    }
                }
                new_lines.join("\n")
            } else {
                // Fallback if node not found
                let mut fallback = content.clone();
                fallback.push_str(" [MASK]");
                fallback
            };

            let mask_results = model.fill_mask(&masked_content, 5)?;

            let mut suggestions = Vec::new();
            for (token, score) in mask_results {
                // Clean up BPE tokens (Ġ is space, Ċ is newline in some tokenizers)
                let clean_token = token.replace("Ġ", " ").replace("Ċ", "\n");

                suggestions.push(CompletionSuggestion {
                    text: clean_token,
                    description: format!("Confidence: {:.1}%", score * 100.0),
                    confidence: score,
                });
            }

            Ok(suggestions)
        }
    }

    /// Suggest a coordinated batch of edits based on a high-level intent
    pub async fn suggest_batch_edits(
        &self,
        file_path: &str,
        intent: &str,
    ) -> Result<Vec<RefactorSuggestion>> {
        #[cfg(not(feature = "modernbert"))]
        {
            anyhow::bail!(
                "ModernBERT feature is not enabled. Recompile with --features modernbert"
            );
        }

        #[cfg(feature = "modernbert")]
        {
            if !self
                .get_model_path(&AiModel::ModernBert)
                .join("config.json")
                .exists()
            {
                anyhow::bail!("ModernBERT is not installed. Run 'gnawtreewriter ai setup' first.");
            }

            println!(
                "🧠 Analyzing {} for batch intent: '{}'...",
                file_path, intent
            );

            // Read and parse the file
            let content = fs::read_to_string(file_path)
                .with_context(|| format!("Failed to read file: {}", file_path))?;

            let path = Path::new(file_path);
            let parser = crate::parser::get_parser(path)?;
            let tree = parser.parse(&content)?;

            // Flatten tree
            let mut nodes = Vec::new();
            fn collect_nodes(
                node: &crate::parser::TreeNode,
                acc: &mut Vec<crate::parser::TreeNode>,
            ) {
                acc.push(node.clone());
                for child in &node.children {
                    collect_nodes(child, acc);
                }
            }
            collect_nodes(&tree, &mut nodes);

            // Use semantic search logic to find nodes matching the intent
            let search_results = self
                .semantic_search(intent, &nodes, DeviceType::Cpu)
                .await?;

            let mut suggestions = Vec::new();
            for res in search_results {
                if res.score > 0.3 {
                    // Threshold for relevance
                    suggestions.push(RefactorSuggestion {
                        title: format!("Apply Intent: {}", intent),
                        description: format!(
                            "Modify this node to match intent: '{}' (Similarity: {:.2})",
                            intent, res.score
                        ),
                        target_node: res.path,
                        confidence: res.score,
                    });
                }
            }

            if suggestions.is_empty() {
                println!("⚠️ No nodes found that strongly match the intent.");
            }

            Ok(suggestions)
        }
    }

    /// Perform semantic search across a set of nodes
    pub async fn semantic_search(
        &self,
        query: &str,
        nodes: &[crate::parser::TreeNode],
        device: DeviceType,
    ) -> Result<Vec<SearchResult>> {
        #[cfg(not(feature = "modernbert"))]
        {
            anyhow::bail!(
                "ModernBERT feature is not enabled. Recompile with --features modernbert"
            );
        }

        #[cfg(feature = "modernbert")]
        {
            println!(
                "🧠 Running semantic search on {:?} for: '{}'",
                device, query
            );

            let model = self.load_model(AiModel::ModernBert, device)?;
            let query_emb = model.get_embedding(query)?;

            // Normalize query embedding
            let query_norm = query_emb.sqr()?.sum_all()?.sqrt()?;
            let query_emb = query_emb.broadcast_div(&query_norm)?;

            let mut results = Vec::new();

            for node in nodes {
                if node.content.trim().is_empty() {
                    continue;
                }

                // Skip very small nodes (noise) and very large nodes (OOM)
                if node.content.len() < 20 || node.content.len() > 10000 {
                    continue;
                }

                let node_emb = model.get_embedding(&node.content)?;
                // Normalize node embedding
                let node_norm = node_emb.sqr()?.sum_all()?.sqrt()?;
                let node_emb = node_emb.broadcast_div(&node_norm)?;

                // Cosine similarity (dot product of normalized vectors)
                let similarity = (query_emb.clone() * node_emb)?
                    .sum_all()?
                    .to_scalar::<f32>()?;

                results.push(SearchResult {
                    path: node.path.clone(),
                    score: similarity,
                    content_preview: node.content.lines().next().unwrap_or("").to_string(),
                });
            }

            // Sort by score descending
            results.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let top_results: Vec<_> = results.into_iter().take(5).collect();
            if top_results.is_empty() {
                println!("⚠️ No relevant nodes found for the given query.");
            } else {
                println!("✅ Found {} relevant nodes.", top_results.len());
            }
            Ok(top_results)
        }
    }

    fn get_model_path(&self, model: &AiModel) -> PathBuf {
        match model {
            AiModel::ModernBert => self.model_cache_dir.join("modernbert"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AiStatus {
    pub modern_bert_installed: bool,
    pub cache_dir: PathBuf,
    pub available_devices: Vec<DeviceType>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub path: String,
    pub score: f32,
    pub content_preview: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RefactorSuggestion {
    pub title: String,
    pub description: String,
    pub target_node: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CompletionSuggestion {
    pub text: String,
    pub description: String,
    pub confidence: f32,
}
