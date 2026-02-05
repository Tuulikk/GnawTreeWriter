use anyhow::Result;
#[cfg(feature = "modernbert")]
use candle_core::{DType, Device, Tensor};
#[cfg(feature = "modernbert")]
use candle_nn::{self, VarBuilder};
#[cfg(feature = "modernbert")]
use candle_transformers::models::modernbert::{Config, ModernBert};
#[cfg(feature = "modernbert")]
use hf_hub::{Repo, RepoType};
#[cfg(feature = "modernbert")]
use crate::core::LabelManager;
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
        let tokens = self.tokenizer.encode(text, true).map_err(anyhow::Error::msg)?;
        let input_ids = Tensor::new(tokens.get_ids(), &self.device)?.unsqueeze(0)?;
        let mask = input_ids.ones_like()?;
        let embeddings = self.model.forward(&input_ids, &mask)?;
        Ok(embeddings.mean(1)?.squeeze(0)?)
    }
}

pub struct AiManager {
    model_cache_dir: PathBuf,
    #[allow(dead_code)]
    project_root: PathBuf,
}

impl AiManager {
    pub fn new(project_root: &Path) -> Result<Self> {
        let local_cache = project_root.join(".gnawtreewriter_ai").join("models");
        
        // Try local first, then global home dir
        let model_cache_dir = if local_cache.exists() && local_cache.join("modernbert").exists() {
            local_cache
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let global_cache = PathBuf::from(home).join(".gnawtreewriter_ai").join("models");
            
            if !global_cache.exists() {
                let _ = fs::create_dir_all(&global_cache);
            }
            global_cache
        };

        Ok(Self { 
            model_cache_dir,
            project_root: project_root.to_path_buf(),
        })
    }

    #[cfg(feature = "modernbert")]
    pub fn load_model(&self, model_type: AiModel, device_type: DeviceType) -> Result<ModernBertModel> {
        let model_dir = self.get_model_path(&model_type);
        
        let config_path = model_dir.join("config.json");
        let tokenizer_path = model_dir.join("tokenizer.json");
        let weights_path = model_dir.join("model.safetensors");

        if !config_path.exists() { return Err(anyhow::anyhow!("Missing config: {:?}", config_path)); }
        if !tokenizer_path.exists() { return Err(anyhow::anyhow!("Missing tokenizer: {:?}", tokenizer_path)); }
        if !weights_path.exists() { return Err(anyhow::anyhow!("Missing weights: {:?}", weights_path)); }

        let config: Config = serde_json::from_str(&fs::read_to_string(&config_path)?)?;
        let tokenizer = Tokenizer::from_file(&tokenizer_path).map_err(anyhow::Error::msg)?;
        
        let device = match device_type {
            DeviceType::Cpu => Device::Cpu,
            DeviceType::Cuda => Device::new_cuda(0)?,
            DeviceType::Metal => Device::new_metal(0)?,
        };
        
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &device)? };
        let model = ModernBert::load(vb, &config)?;
        Ok(ModernBertModel { model, tokenizer, device })
    }

    #[cfg(feature = "modernbert")]
    pub async fn generate_semantic_report(&self, file_path: &str) -> Result<SemanticReport> {
        eprintln!("[DEBUG] Starting semantic report for: {}", file_path);
        let _model = self.load_model(AiModel::ModernBert, DeviceType::Cpu)?;
        eprintln!("[DEBUG] Model loaded successfully");
        
        let mut label_mgr = LabelManager::load(&self.project_root)?;
        eprintln!("[DEBUG] Label manager loaded from: {:?}", self.project_root);
        
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(anyhow::anyhow!("File not found: {}", file_path));
        }
        
        let content = fs::read_to_string(file_path)?;
        eprintln!("[DEBUG] File content read ({} bytes)", content.len());
        
        let parser = crate::parser::get_parser(path)?;
        eprintln!("[DEBUG] Parser obtained");
        
        let tree = parser.parse(&content)?;
        eprintln!("[DEBUG] AST parsed");

        let mut nodes = Vec::new();
        fn collect(n: &crate::parser::TreeNode, acc: &mut Vec<crate::parser::TreeNode>) {
            acc.push(n.clone());
            for c in &n.children { collect(c, acc); }
        }
        collect(&tree, &mut nodes);
        eprintln!("[DEBUG] Collected {} nodes", nodes.len());

        let mut findings = Vec::new();
        for node in &nodes {
            if node.content.len() < 30 || node.content.len() > 5000 { continue; }
            
            let braces = node.content.chars().filter(|&c| c == '{' || c == '}').count();
            let density = braces as f32 / node.content.len() as f32;
            if density > 0.15 && node.content.len() > 100 {
                let msg = format!("High brace density ({:.1}%)", density * 100.0);
                findings.push(QualityFinding {
                    path: node.path.clone(),
                    severity: "Warning".into(),
                    category: "Complexity".into(),
                    message: msg.clone(),
                });
                let _ = label_mgr.add_label(file_path, &node.content, "quality:high-brace-density");
            }
        }

        Ok(SemanticReport {
            file_path: file_path.to_string(),
            findings,
            summary: format!("Analyzed {} nodes.", nodes.len()),
        })
    }

    pub async fn setup(&self, _model: AiModel, _device: DeviceType, _force: bool) -> Result<()> {
        #[cfg(feature = "modernbert")]
        {
            let model_id = "answerdotai/ModernBERT-base";
            let api = hf_hub::api::sync::ApiBuilder::new().with_progress(true).build()?;
            let repo = api.repo(Repo::new(model_id.to_string(), RepoType::Model));
            let model_dir = self.get_model_path(&AiModel::ModernBert);
            if !model_dir.exists() { fs::create_dir_all(&model_dir)?; }
            for file in ["config.json", "model.safetensors", "tokenizer.json"] {
                let dest = model_dir.join(file);
                if !dest.exists() || _force {
                    fs::copy(&repo.get(file)?, &dest)?;
                }
            }
        }
        Ok(())
    }

    pub fn get_status(&self) -> Result<AiStatus> {
        let modern_bert_installed = self.get_model_path(&AiModel::ModernBert).join("config.json").exists();
        Ok(AiStatus { 
            modern_bert_installed, 
            cache_dir: self.model_cache_dir.clone(), 
            available_devices: vec![DeviceType::Cpu] 
        })
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
    pub category: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct AiStatus {
    pub modern_bert_installed: bool,
    pub cache_dir: PathBuf,
    pub available_devices: Vec<DeviceType>,
}