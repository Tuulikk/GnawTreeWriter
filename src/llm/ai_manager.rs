use anyhow::{Context, Result};
#[cfg(feature = "modernbert")]
use candle_core::{DType, Device, Tensor};
#[cfg(feature = "modernbert")]
use candle_nn::{self, VarBuilder};
#[cfg(feature = "modernbert")]
use candle_transformers::models::modernbert::{Config, ModernBert};
#[cfg(feature = "modernbert")]
use hf_hub::{Repo, RepoType};
use std::collections::HashMap;
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

        // Mean pooling
        let mean_embedding = embeddings.mean(1)?; 
        Ok(mean_embedding.squeeze(0)?)
    }

    pub fn fill_mask(&self, text: &str, top_k: usize) -> Result<Vec<(String, f32)>> {
        let tokens = self
            .tokenizer
            .encode(text, true)
            .map_err(anyhow::Error::msg)?;
        let token_ids = tokens.get_ids();

        let mask_token = "[MASK]";
        let mask_id = self
            .tokenizer
            .token_to_id(mask_token)
            .context("Mask token not found")?;

        let mask_indices: Vec<usize> = token_ids.iter().enumerate()
            .filter(|(_, &id)| id == mask_id).map(|(i, _)| i).collect();

        if mask_indices.is_empty() { anyhow::bail!("No [MASK] found"); }

        let input_ids = Tensor::new(token_ids, &self.device)?.unsqueeze(0)?;
        let mask = input_ids.ones_like()?;
        let logits = self.model.forward(&input_ids, &mask)?;

        let mut results = Vec::new();
        for &mask_idx in &mask_indices {
            let mask_logits = logits.get(0)?.get(mask_idx)?;
            let probs = candle_nn::ops::softmax(&mask_logits, 0)?;
            let probs_v: Vec<f32> = probs.to_vec1()?;

            let mut indexed_probs: Vec<(usize, f32)> = probs_v.into_iter().enumerate().collect();
            indexed_probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            for i in 0..top_k.min(indexed_probs.len()) {
                let (id, prob) = indexed_probs[i];
                let token = self.tokenizer.id_to_token(id as u32).unwrap_or_default();
                results.push((token, prob));
            }
        }
        Ok(results)
    }
}

pub struct AiManager {
    model_cache_dir: PathBuf,
}

impl AiManager {
    #[cfg(feature = "modernbert")]
    fn get_candledevice(&self, device_type: &DeviceType) -> Result<Device> {
        match device_type {
            DeviceType::Cpu => Ok(Device::Cpu),
            DeviceType::Cuda => Ok(Device::new_cuda(0).context("CUDA error")?),
            DeviceType::Metal => Ok(Device::new_metal(0).context("Metal error")?),
        }
    }

    #[cfg(feature = "modernbert")]
    pub fn load_model(&self, model_type: AiModel, device_type: DeviceType) -> Result<ModernBertModel> {
        let model_dir = self.get_model_path(&model_type);
        let config_path = model_dir.join("config.json");
        let weights_path = model_dir.join("model.safetensors");
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !config_path.exists() { anyhow::bail!("Run 'ai setup' first"); }

        let device = self.get_candledevice(&device_type)?;
        let config: Config = serde_json::from_str(&fs::read_to_string(config_path)?)?;
        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(anyhow::Error::msg)?;
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &device)? };
        let model = ModernBert::load(vb, &config)?;

        Ok(ModernBertModel { model, tokenizer, device })
    }

    pub fn new(project_root: &Path) -> Result<Self> {
        let model_cache_dir = project_root.join(".gnawtreewriter_ai").join("models");
        if !model_cache_dir.exists() { fs::create_dir_all(&model_cache_dir)?; }
        Ok(Self { model_cache_dir })
    }

    #[cfg(feature = "modernbert")]
    pub async fn generate_semantic_report(&self, file_path: &str) -> Result<SemanticReport> {
        let model = self.load_model(AiModel::ModernBert, DeviceType::Cpu)?;
        let content = fs::read_to_string(file_path)?;
        let path = Path::new(file_path);
        let parser = crate::parser::get_parser(path)?;
        let tree = parser.parse(&content)?;

        let mut nodes = Vec::new();
        fn collect(node: &crate::parser::TreeNode, acc: &mut Vec<crate::parser::TreeNode>) {
            acc.push(node.clone());
            for child in &node.children { collect(child, acc); }
        }
        collect(&tree, &mut nodes);

        let mut findings = Vec::new();
        let mut node_embeddings = Vec::new();

        for node in &nodes {
            if node.content.len() < 30 || node.content.len() > 5000 { continue; }
            
            // Brace density check (Dumhets-detektorn)
            let brace_count = node.content.chars().filter(|&c| c == '{' || c == '}').count();
            let density = brace_count as f32 / node.content.len() as f32;
            if density > 0.15 && node.content.len() > 100 {
                findings.push(QualityFinding {
                    path: node.path.clone(),
                    severity: "Warning".into(),
                    message: format!("High brace density ({:.1}%). Potential data-binding mess or f-string error.", density * 100.0),
                });
            }

            // Collect embeddings for duplication check
            if node.node_type.contains("definition") || node.node_type.contains("item") {
                if let Ok(emb) = model.get_embedding(&node.content) {
                    node_embeddings.push((node.path.clone(), emb));
                }
            }
        }

        // Semantic Duplication Check
        for i in 0..node_embeddings.len() {
            for j in i + 1..node_embeddings.len() {
                let (p1, e1) = &node_embeddings[i];
                let (p2, e2) = &node_embeddings[j];
                
                // Simplified similarity (dot product of supposedly normalized embeddings)
                let similarity = (e1.clone() * e2.clone())?.sum_all()?.to_scalar::<f32>()?;
                if similarity > 30.0 { // ModernBERT-base hidden size is 768, 30.0 is a heuristic threshold for raw mean-pool
                    findings.push(QualityFinding {
                        path: format!("{} & {}", p1, p2),
                        severity: "Info".into(),
                        message: "High semantic similarity detected. Potential code duplication.".into(),
                    });
                }
            }
        }

        Ok(SemanticReport {
            file_path: file_path.to_string(),
            findings,
            summary: format!("Analyzed {} nodes using ModernBERT.", nodes.len()),
        })
    }

    #[allow(unused_variables, unreachable_code)]
    pub async fn setup(&self, model: AiModel, device: DeviceType, force: bool) -> Result<()> {
        println!("🤖 Initializing setup for {:?} on {:?}...", model, device);
        #[cfg(feature = "modernbert")]
        {
            let model_id = "answerdotai/ModernBERT-base";
            let api = hf_hub::api::sync::ApiBuilder::new().with_progress(true).build()?;
            let repo = api.repo(Repo::new(model_id.to_string(), RepoType::Model));
            let files = ["config.json", "model.safetensors", "tokenizer.json"];
            let model_dir = self.get_model_path(&model);
            if !model_dir.exists() { fs::create_dir_all(&model_dir)?; }
            for file in files {
                let dest = model_dir.join(file);
                if !dest.exists() || force {
                    let downloaded = repo.get(file)?;
                    fs::copy(&downloaded, &dest)?;
                }
            }
            println!("✨ Setup complete.");
        }
        Ok(())
    }

    pub fn get_status(&self) -> Result<AiStatus> {
        let modern_bert_installed = self.get_model_path(&AiModel::ModernBert).join("config.json").exists();
        let mut available_devices = vec![DeviceType::Cpu];
        #[cfg(feature = "cuda")] available_devices.push(DeviceType::Cuda);
        #[cfg(feature = "metal")] available_devices.push(DeviceType::Metal);
        Ok(AiStatus { modern_bert_installed, cache_dir: self.model_cache_dir.clone(), available_devices })
    }

    fn get_model_path(&self, model: &AiModel) -> PathBuf {
        match model { AiModel::ModernBert => self.model_cache_dir.join("modernbert") }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SemanticReport {
    pub file_path: String,
    pub summary: String,
    pub findings: Vec<QualityFinding>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct QualityFinding {
    pub path: String,
    pub severity: String,
    pub message: String,
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