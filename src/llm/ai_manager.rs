use anyhow::Result;
#[cfg(feature = "modernbert")]
use candle_core::{DType, Device, Tensor};
#[cfg(feature = "modernbert")]
use candle_nn::{self, VarBuilder};
#[cfg(feature = "modernbert")]
use candle_transformers::models::modernbert::{Config, ModernBert};
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
    #[cfg(feature = "mamba")]
    Lfm25,
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

/// LFM2.5 language model: candle quantized GGUF weights + tokenizer.
/// `model` is behind a Mutex because `ModelWeights::forward` takes `&mut self`
/// (KV/conv state is mutated during generation).
#[cfg(feature = "mamba")]
pub struct Lfm25Model {
    pub model: std::sync::Mutex<candle_transformers::models::quantized_lfm2::ModelWeights>,
    pub tokenizer: tokenizers::Tokenizer,
    pub device: candle_core::Device,
}

/// Result of a generation call: the text plus whether the token budget was
/// exhausted (truncated) so callers can surface it instead of silently
/// returning a cut-off answer.
#[cfg(feature = "mamba")]
pub struct Generation {
    pub text: String,
    pub truncated: bool,
    pub tokens: usize,
}

/// How thorough a pipeline task should be. Controls chunk size / output
/// budgets, which trade time against quality. `Auto` picks per-command
/// defaults; users can override with `--resolution`.
#[cfg(feature = "mamba")]
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Resolution {
    /// Smallest chunks, lowest output budgets — fastest, least detail.
    Fast,
    /// Default balance of speed and quality.
    Balanced,
    /// Larger chunks, bigger output budgets — slowest, most detail.
    Thorough,
    /// Use the per-command default.
    Auto,
}

#[cfg(feature = "mamba")]
impl Resolution {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "fast" => Resolution::Fast,
            "thorough" | "deep" => Resolution::Thorough,
            "auto" => Resolution::Auto,
            _ => Resolution::Balanced,
        }
    }
}

/// Measured per-token timing for the machine this runs on. Populated by a
/// short self-calibration (see `calibrate_timing`) and persisted to the
/// AI config dir so time estimates reflect the user's hardware instead of a
/// fixed default. The defaults are from a reference benchmark on one CPU.
#[cfg(feature = "mamba")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimingProfile {
    /// Seconds per prefill token (linear term; there is also a fixed call
    /// overhead captured in `call_overhead_s`).
    pub prefill_s_per_token: f64,
    /// Seconds per decoded token (single-token steps).
    pub decode_s_per_token: f64,
    /// Fixed per-call overhead (model load, tensor alloc, sampling).
    pub call_overhead_s: f64,
    /// When this profile was measured (unix seconds).
    pub measured_at: u64,
}

#[cfg(feature = "mamba")]
impl Default for TimingProfile {
    fn default() -> Self {
        // Reference benchmark: LFM2.5-1.2B Q4_K_M on a desktop CPU.
        Self {
            prefill_s_per_token: 0.065,
            decode_s_per_token: 0.015,
            call_overhead_s: 3.0,
            measured_at: 0,
        }
    }
}

#[cfg(feature = "mamba")]
impl TimingProfile {
    /// Estimate wall-clock seconds for a call: overhead + prefill + decode.
    pub fn estimate_seconds(&self, input_tokens: usize, output_tokens: usize) -> f64 {
        self.call_overhead_s
            + input_tokens as f64 * self.prefill_s_per_token
            + output_tokens as f64 * self.decode_s_per_token
    }
}

/// Token accounting for a pipeline run: what the task was expected to cost,
/// what it actually cost, and whether any step hit its budget. Every command
/// should surface this so agents (and humans) always know the real cost.
/// Time is estimated from a measured per-token model (see `estimate_seconds`)
/// rather than a hard wall-clock timeout — the timeout is the user's choice.
#[cfg(feature = "mamba")]
#[derive(Debug, Clone, serde::Serialize)]
pub struct TokenBudget {
    /// Estimated input tokens for the whole task (sum of step prompts).
    pub expected_input: usize,
    /// Estimated output budget allowed for the whole task.
    pub expected_output: usize,
    /// Actual input tokens used (sum of encoded prompts).
    pub actual_input: usize,
    /// Actual output tokens generated.
    pub actual_output: usize,
    /// Number of generation calls.
    pub calls: usize,
    /// True if any call hit its token cap.
    pub truncated: bool,
    /// Estimated wall-clock seconds before the task (from token counts).
    pub expected_seconds: f64,
    /// Measured wall-clock seconds the task actually took.
    pub actual_seconds: f64,
}

#[cfg(feature = "mamba")]
impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            expected_input: 0,
            expected_output: 0,
            actual_input: 0,
            actual_output: 0,
            calls: 0,
            truncated: false,
            expected_seconds: 0.0,
            actual_seconds: 0.0,
        }
    }
}

#[cfg(feature = "mamba")]
impl TokenBudget {
    /// Record one generation call: add its actual usage.
    pub fn record(&mut self, gen: &Generation) {
        self.calls += 1;
        self.actual_output += gen.tokens;
        self.truncated |= gen.truncated;
    }

    /// Estimate wall-clock seconds using the machine's measured timing
    /// profile. A cost *estimate*, never a hard limit — the caller decides
    /// whether the expected time is acceptable.
    pub fn estimate_seconds(&mut self, profile: &TimingProfile) {
        self.expected_seconds =
            profile.estimate_seconds(self.expected_input, self.expected_output);
    }
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
    /// Cached model — loaded once, reused across all calls within this process.
    /// Uses OnceLock for thread-safety (MCP server is async multi-threaded).
    #[cfg(feature = "modernbert")]
    cached_model: std::sync::OnceLock<ModernBertModel>,
    /// Separate cache for the LFM2.5 model (each model gets its own slot).
    #[cfg(feature = "mamba")]
    cached_lfm25: std::sync::OnceLock<Lfm25Model>,
    /// Measured timing profile for this machine (self-calibrated, persisted).
    #[cfg(feature = "mamba")]
    timing_profile: std::sync::RwLock<TimingProfile>,
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

        #[cfg(feature = "mamba")]
        let timing_profile = load_timing_profile(&model_cache_dir);

        Ok(Self { 
            model_cache_dir,
            project_root: project_root.to_path_buf(),
            #[cfg(feature = "modernbert")]
            cached_model: std::sync::OnceLock::new(),
            #[cfg(feature = "mamba")]
            cached_lfm25: std::sync::OnceLock::new(),
            #[cfg(feature = "mamba")]
            timing_profile: std::sync::RwLock::new(timing_profile),
        })
    }

    /// Current timing profile for this machine.
    #[cfg(feature = "mamba")]
    pub fn timing(&self) -> TimingProfile {
        self.timing_profile
            .read()
            .map(|p| p.clone())
            .unwrap_or_default()
    }

    /// Run a short self-calibration (a few prefill + decode calls) and persist
    /// the measured profile so time estimates reflect this machine.
    #[cfg(feature = "mamba")]
    pub fn calibrate_timing(&self) -> Result<TimingProfile> {
        let model = self.load_lfm25_model()?;
        let mut gen = model
            .model
            .lock()
            .map_err(|_| anyhow::anyhow!("LFM2.5 model lock poisoned"))?;

        // Measure prefill at a small size (fast) and decode (single token).
        let prof = measure_profile(&mut gen, &model.device)?;
        drop(gen);

        let profile = TimingProfile {
            prefill_s_per_token: prof.prefill_s_per_token,
            decode_s_per_token: prof.decode_s_per_token,
            call_overhead_s: prof.call_overhead_s,
            measured_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        if let Ok(mut w) = self.timing_profile.write() {
            *w = profile.clone();
        }
        save_timing_profile(&self.model_cache_dir, &profile);
        Ok(profile)
    }

    #[cfg(feature = "modernbert")]
    pub fn load_model(&self, model_type: AiModel, device_type: DeviceType) -> Result<&ModernBertModel> {
        // OnceLock doesn't have get_or_try_init on stable Rust yet.
        // Use get_or_init with interior error handling — if model fails to load,
        // we panic (this is acceptable: missing model = broken installation).
        if let Some(model) = self.cached_model.get() {
            return Ok(model);
        }
        
        // Load the model (not cached yet)
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
        let loaded = ModernBertModel { model, tokenizer, device };
        
        // Store in cache (get_or_init for the first call wins; subsequent calls reuse)
        // If another thread loaded meanwhile, that's fine — we just return the cached one
        self.cached_model.set(loaded).ok().expect("Model cache already set");
        
        Ok(self.cached_model.get().unwrap())
    }

    /// Load (or fetch cached) LFM2.5 model from a GGUF file + tokenizer.json.
    #[cfg(feature = "mamba")]
    pub fn load_lfm25_model(&self) -> Result<&Lfm25Model> {
        if let Some(model) = self.cached_lfm25.get() {
            return Ok(model);
        }

        let model_dir = self.get_model_path(&AiModel::Lfm25);
        let weights_path = model_dir.join("model.gguf");
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !weights_path.exists() {
            return Err(anyhow::anyhow!(
                "Missing LFM2.5 weights: {:?}. Run 'gnawtreewriter ai setup --model lfm25'",
                weights_path
            ));
        }
        if !tokenizer_path.exists() {
            return Err(anyhow::anyhow!(
                "Missing LFM2.5 tokenizer: {:?}. Run 'gnawtreewriter ai setup --model lfm25'",
                tokenizer_path
            ));
        }

        let device = candle_core::Device::Cpu;
        let mut file = std::fs::File::open(&weights_path)
            .map_err(|e| anyhow::anyhow!("failed to open {}: {}", weights_path.display(), e))?;
        let content = candle_core::quantized::gguf_file::Content::read(&mut file)
            .map_err(|e| anyhow::anyhow!("failed to read GGUF header: {e}"))?;
        let model = candle_transformers::models::quantized_lfm2::ModelWeights::from_gguf(
            content,
            &mut file,
            &device,
        )
        .map_err(|e| anyhow::anyhow!("failed to load LFM2.5 weights: {e}"))?;

        let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
            .map_err(anyhow::Error::msg)?;

        let loaded = Lfm25Model {
            model: std::sync::Mutex::new(model),
            tokenizer,
            device,
        };
        self.cached_lfm25
            .set(loaded)
            .ok()
            .expect("LFM2.5 cache already set");
        Ok(self.cached_lfm25.get().unwrap())
    }

    /// Estimate the token count of `text`: exact via the model tokenizer if
    /// loaded, otherwise a chars/4 heuristic. Used for pre-task budgeting.
    #[cfg(feature = "mamba")]
    pub fn estimate_tokens(&self, text: &str) -> usize {
        if let Some(model) = self.cached_lfm25.get() {
            if let Ok(enc) = model.tokenizer.encode(text, false) {
                return enc.get_ids().len();
            }
        }
        text.len() / 4 + 1
    }

    /// Generate text with the LFM2.5 model. `prompt` is encoded with the
    /// model tokenizer; returns the decoded continuation plus whether the
    /// max-token budget was hit (truncated) and how many tokens were produced.
    #[cfg(feature = "mamba")]
    pub fn generate_lfm25(
        &self,
        prompt: &str,
        max_tokens: usize,
        temperature: f32,
    ) -> Result<Generation> {
        let model = self.load_lfm25_model()?;

        // LFM2.5-Instruct is a chat model (Qwen-style im_start/im_end).
        let chat_prompt = format!(
            "<|im_start|>user\n{}\n<|im_start|>assistant\n",
            prompt.trim()
        );
        let encoding = model
            .tokenizer
            .encode(chat_prompt.as_str(), true)
            .map_err(|e| anyhow::anyhow!("tokenizer encode failed: {e}"))?;
        let prompt_ids: Vec<u32> = encoding.get_ids().to_vec();
        if prompt_ids.is_empty() {
            anyhow::bail!("empty prompt after tokenization");
        }

        // LFM2.5 context is large (32768); cap prompt to leave room for gen.
        let ctx = 8192usize;
        let keep = ctx.saturating_sub(max_tokens).min(prompt_ids.len());
        let prompt_ids: Vec<u32> = prompt_ids[prompt_ids.len() - keep..].to_vec();

        let mut gen = model
            .model
            .lock()
            .map_err(|_| anyhow::anyhow!("LFM2.5 model lock poisoned"))?;

        // Prefill: run the whole prompt through once.
        let input = candle_core::Tensor::new(prompt_ids.as_slice(), &model.device)?.unsqueeze(0)?;
        let logits = gen.forward(&input, 0)?;

        let eos: u32 = 7; // LFM2.5-Instruct uses <|im_end|> as EOS
        let eos_bias: f32 = 3.0; // logit bonus so the model can stop naturally
        let mut next = sample_token(&logits, &prompt_ids, temperature, Some((eos, eos_bias)))?;
        let mut generated: Vec<u32> = Vec::with_capacity(max_tokens);
        let mut truncated = true;

        for i in 0..max_tokens {
            if next == eos {
                truncated = false;
                break;
            }
            generated.push(next);
            let input = candle_core::Tensor::new(&[next][..], &model.device)?.unsqueeze(0)?;
            let logits = gen.forward(&input, prompt_ids.len() + i)?;
            next = sample_token(&logits, &generated, temperature, Some((eos, eos_bias)))?;
        }
        drop(gen);

        let decoded = model
            .tokenizer
            .decode(&generated, true)
            .map_err(|e| anyhow::anyhow!("tokenizer decode failed: {e}"))?;
        // Safety trim: stop at the first chat-end marker even if EOS sampling missed.
        let decoded = decoded
            .split("<|im_end|>")
            .next()
            .unwrap_or(&decoded)
            .trim()
            .to_string();

        Ok(Generation {
            text: decoded,
            truncated,
            tokens: generated.len(),
        })
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
            let model_dir = self.get_model_path(&AiModel::ModernBert);
            if !model_dir.exists() { fs::create_dir_all(&model_dir)?; }
            for file in ["config.json", "model.safetensors", "tokenizer.json"] {
                let dest = model_dir.join(file);
                if !dest.exists() || _force {
                    let url = format!("https://huggingface.co/{}/resolve/main/{}", model_id, file);
                    println!("  Downloading {}...", file);
                    let resp = ureq::get(&url).call()
                        .map_err(|e| anyhow::anyhow!("HTTP download failed: {}", e))?;
                    let mut reader = resp.into_reader();
                    let mut out = std::fs::File::create(&dest)
                        .map_err(|e| anyhow::anyhow!("Failed to create {:?}: {}", dest, e))?;
                    std::io::copy(&mut reader, &mut out)?;
                }
            }
        }

        #[cfg(feature = "mamba")]
        {
            // LFM2.5-1.2B-Instruct, QAD Q4_0 (quantization-aware distilled:
            // ~97% of BF16 quality at Q4_0 speed/size).
            let model_id = "LiquidAI/LFM2.5-1.2B-Instruct-GGUF";
            let gguf_file = "LFM2.5-1.2B-Instruct-QAD-Q4_0.gguf";
            let tok_id = "LiquidAI/LFM2.5-1.2B-Instruct";
            let model_dir = self.get_model_path(&AiModel::Lfm25);
            if !model_dir.exists() { fs::create_dir_all(&model_dir)?; }

            let weights_path = model_dir.join("model.gguf");
            let tokenizer_path = model_dir.join("tokenizer.json");

            if !weights_path.exists() || _force {
                let url = format!("https://huggingface.co/{}/resolve/main/{}", model_id, gguf_file);
                println!("  Downloading LFM2.5 QAD-Q4_0 GGUF (~700 MB)...");
                download_file(&url, &weights_path)?;
            } else {
                println!("  model.gguf already present.");
            }
            if !tokenizer_path.exists() || _force {
                let url = format!("https://huggingface.co/{}/resolve/main/tokenizer.json", tok_id);
                println!("  Downloading tokenizer.json...");
                download_file(&url, &tokenizer_path)?;
            }
        }
        Ok(())
    }


    pub fn get_status(&self) -> Result<AiStatus> {
        let modern_bert_installed = self.get_model_path(&AiModel::ModernBert).join("config.json").exists();
        #[cfg(feature = "mamba")]
        let lfm25_installed = self.get_model_path(&AiModel::Lfm25).join("model.gguf").exists();
        #[cfg(not(feature = "mamba"))]
        let lfm25_installed = false;
        Ok(AiStatus { 
            modern_bert_installed, 
            lfm25_installed,
            cache_dir: self.model_cache_dir.clone(), 
            available_devices: vec![DeviceType::Cpu] 
        })
    }

    fn get_model_path(&self, model: &AiModel) -> PathBuf {
        match model {
            AiModel::ModernBert => self.model_cache_dir.join("modernbert"),
            #[cfg(feature = "mamba")]
            AiModel::Lfm25 => self.model_cache_dir.join("lfm25"),
        }
    }
}

/// Sample the next token id from logits using temperature.
/// `eos_bias` (if Some) adds a logit bonus to that token id so the model can
/// naturally choose to stop (LFM2.5's <|im_end|>). Accepts either
/// [1, seq, vocab] or [1, vocab] logits.
#[cfg(feature = "mamba")]
fn sample_token(
    logits: &candle_core::Tensor,
    _seen: &[u32],
    temperature: f32,
    eos_bias: Option<(u32, f32)>,
) -> Result<u32> {
    use candle_core::IndexOp;
    // Reduce to [vocab]: handle [1, seq, vocab] and [1, vocab].
    let logits = if logits.dims().len() == 3 {
        logits.i((0, logits.dim(1)? - 1, ..))?
    } else if logits.dims().len() == 2 {
        logits.i(0)?
    } else {
        logits.clone()
    };
    let mut logits = if temperature > f32::EPSILON {
        logits.affine(1.0 / temperature as f64, 0.0)?
    } else {
        logits
    };
    // Apply EOS bias: boost the stop token so the model can end naturally.
    if let Some((eos_id, bias)) = eos_bias {
        let eos_id = eos_id as usize;
        let vocab = logits.dim(0)?;
        if eos_id < vocab {
            let mut vals: Vec<f32> = logits.to_vec1()?;
            vals[eos_id] += bias;
            logits = candle_core::Tensor::new(vals.as_slice(), logits.device())?;
        }
    }
    // Softmax over vocab.
    let mut probs = candle_nn::ops::softmax(&logits, 0)?;
    let sum: f32 = probs.sum_all()?.to_scalar()?;
    if sum > 0.0 {
        probs = probs.broadcast_div(&candle_core::Tensor::new(sum, logits.device())?)?;
    }
    // Sample from the distribution.
    let probs_vec: Vec<f32> = probs.to_vec1()?;
    let mut rng = fastrand::Rng::new();
    let r: f32 = rng.f32();
    let mut acc = 0.0f32;
    for (i, &p) in probs_vec.iter().enumerate() {
        acc += p;
        if r < acc {
            return Ok(i as u32);
        }
    }
    Ok((probs_vec.len() - 1) as u32)
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
    pub lfm25_installed: bool,
    pub cache_dir: PathBuf,
    pub available_devices: Vec<DeviceType>,
}

// ── Helpers ───────────────────────────────────────────────────

/// Download a URL to a file (streamed, no extra deps).
#[cfg(feature = "mamba")]
fn download_file(url: &str, dest: &std::path::Path) -> Result<()> {
    let resp = ureq::get(url)
        .call()
        .map_err(|e| anyhow::anyhow!("HTTP download failed: {}", e))?;
    let mut reader = resp.into_reader();
    let mut out = std::fs::File::create(dest)
        .map_err(|e| anyhow::anyhow!("Failed to create {:?}: {}", dest, e))?;
    std::io::copy(&mut reader, &mut out)?;
    Ok(())
}

/// Config file holding the machine's measured timing profile.
#[cfg(feature = "mamba")]
fn timing_profile_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join("timing_profile.json")
}

/// Load a saved timing profile, or the defaults if none exists yet.
#[cfg(feature = "mamba")]
fn load_timing_profile(cache_dir: &Path) -> TimingProfile {
    let path = timing_profile_path(cache_dir);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist the timing profile next to the models.
#[cfg(feature = "mamba")]
fn save_timing_profile(cache_dir: &Path, profile: &TimingProfile) {
    let path = timing_profile_path(cache_dir);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(s) = serde_json::to_string_pretty(profile) {
        let _ = fs::write(&path, s);
    }
}

/// Measure the machine's prefill and decode speed with a couple of small
/// forward passes, fitting a linear per-token model.
#[cfg(feature = "mamba")]
fn measure_profile(
    gen: &mut candle_transformers::models::quantized_lfm2::ModelWeights,
    device: &candle_core::Device,
) -> Result<TimingProfile> {
    // Two prefill sizes to fit the linear term + fixed overhead.
    let sizes = [64usize, 256usize];
    let mut prefill_times: Vec<(usize, f64)> = Vec::new();
    for &seq in &sizes {
        let ids: Vec<u32> = (1..=seq as u32).collect();
        let input = candle_core::Tensor::new(ids.as_slice(), device)?.unsqueeze(0)?;
        // Warmup once, then measure.
        let _ = gen.forward(&input, 0)?;
        let t = std::time::Instant::now();
        let _ = gen.forward(&input, 0)?;
        prefill_times.push((seq, t.elapsed().as_secs_f64()));
    }

    // Fit: t(n) = overhead + n * slope  =>  slope = (t2 - t1) / (n2 - n1)
    let (n1, t1) = prefill_times[0];
    let (n2, t2) = prefill_times[1];
    let prefill_s_per_token = if n2 > n1 { (t2 - t1) / (n2 - n1) as f64 } else { 0.065 };
    let call_overhead_s = (t1 - n1 as f64 * prefill_s_per_token).max(0.5);

    // Measure decode: 8 single-token steps.
    let input1 = candle_core::Tensor::new(&[1u32][..], device)?.unsqueeze(0)?;
    let t = std::time::Instant::now();
    for i in 0..8usize {
        let _ = gen.forward(&input1, i)?;
    }
    let dec = t.elapsed().as_secs_f64() / 8.0;
    let decode_s_per_token = if dec > 0.0 { dec } else { 0.015 };

    Ok(TimingProfile {
        prefill_s_per_token,
        decode_s_per_token,
        call_overhead_s,
        measured_at: 0,
    })
}
