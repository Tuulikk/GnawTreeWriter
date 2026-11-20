//! Pipeline engine for the LFM2.5 command extension.
//!
//! Fixed-step pipelines: control flow lives in Rust, each LLM call is a small
//! bounded step with typed input/output. No chat, no agentic loop.

use crate::llm::ai_manager::{sources_from_evidence, AiManager, Source};
use crate::llm::prompts;
use anyhow::{Context, Result};
use std::path::Path;

/// Parameters for pipeline LLM calls.
#[derive(Debug, Clone, Copy)]
pub struct GenerateParams {
    pub max_tokens: usize,
    pub temperature: f32,
}

impl Default for GenerateParams {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            temperature: 0.3,
        }
    }
}

/// Task-level settings derived from the user's chosen resolution. Larger
/// chunks and output budgets cost more time but give more detail; the user
/// picks what is appropriate for the job (no hard timeout is imposed).
#[derive(Debug, Clone, Copy)]
pub struct TaskParams {
    /// Chunk size in characters fed to the model per call (≈ tokens/4).
    pub chunk_chars: usize,
    /// Output budget for summary-style steps.
    pub summary_tokens: usize,
    /// Output budget for synthesis/explain steps.
    pub synth_tokens: usize,
}

impl TaskParams {
    pub fn from_resolution(r: crate::llm::Resolution) -> Self {
        match r {
            crate::llm::Resolution::Fast => Self {
                chunk_chars: 600, // ~150 tokens prefill ≈ 5-8s/call
                summary_tokens: 96,
                synth_tokens: 256,
            },
            crate::llm::Resolution::Balanced => Self {
                chunk_chars: 1200, // ~300 tokens prefill ≈ 10-15s/call
                summary_tokens: 128,
                synth_tokens: 512,
            },
            crate::llm::Resolution::Thorough => Self {
                chunk_chars: 2400, // ~600 tokens prefill ≈ 25-40s/call
                summary_tokens: 192,
                synth_tokens: 768,
            },
            crate::llm::Resolution::Auto => Self::from_resolution(crate::llm::Resolution::Balanced),
        }
    }
}

/// Low temperature for extraction/ranking steps (deterministic-ish).
const EXTRACT_TEMP: f32 = 0.1;
/// Slightly higher for synthesis.
const SYNTH_TEMP: f32 = 0.4;

/// Extract a JSON array of strings/numbers from model output (lenient).
/// Expand a search question into content terms with the local LFM2.5
/// (same prompt as investigate's step 1) — used by satellite sense when
/// the caller passes expand=true. Empty vec = model produced nothing;
/// callers decide whether that is worth reporting.
pub fn expand_query_terms(mgr: &AiManager, question: &str) -> Result<Vec<String>> {
    let prompt = prompts::expand_query_prompt(question);
    let gen = mgr.generate_lfm25(&prompt, 60, EXTRACT_TEMP)?;
    Ok(parse_json_array(&gen.text).unwrap_or_default())
}

fn parse_json_array(s: &str) -> Option<Vec<String>> {
    let start = s.find('[')?;
    let end = s.rfind(']')?;
    let inner = &s[start + 1..end];
    Some(
        inner
            .split(',')
            .map(|t| t.trim().trim_matches('"').trim_matches('\'').to_string())
            .filter(|t| !t.is_empty())
            .collect(),
    )
}

/// Parse a JSON array of indices.
fn parse_json_indices(s: &str) -> Option<Vec<usize>> {
    parse_json_array(s).map(|v| v.iter().filter_map(|t| t.parse::<usize>().ok()).collect())
}

// ── Commands ─────────────────────────────────────────────────

/// `explain <file> [--node <path>]` — single-pass explanation of one node.
/// Returns (text, budget) where budget accounts for the tokens and time used.
pub fn explain_node(
    mgr: &AiManager,
    file_path: &str,
    node_path: Option<&str>,
    resolution: crate::llm::Resolution,
) -> Result<(String, crate::llm::TokenBudget)> {
    let task = TaskParams::from_resolution(resolution);
    let start = std::time::Instant::now();
    let writer = crate::GnawTreeWriter::new(file_path)
        .with_context(|| format!("failed to parse {file_path}"))?;
    let tree = writer.analyze();

    let node = match node_path {
        Some(p) => tree
            .find_path(p)
            .with_context(|| format!("node path {p} not found in {file_path}"))?,
        None => tree,
    };

    // Cap content length so the prompt stays small.
    let mut content = node.content.clone();
    if content.len() > 4000 {
        content.truncate(4000);
        content.push_str("\n...");
    }
    // Also cap total prompt context (model ctx 2048 tokens).
    let max_prompt_chars = 6000;
    if content.len() > max_prompt_chars {
        content = content[..max_prompt_chars].to_string();
    }

    let prompt = prompts::explain_prompt(file_path, &node.node_type, &content);
    let params = GenerateParams {
        max_tokens: task.synth_tokens,
        temperature: SYNTH_TEMP,
    };

    let mut budget = crate::llm::TokenBudget::default();
    budget.expected_input = mgr.estimate_tokens(&prompt);
    budget.expected_output = params.max_tokens;
    budget.estimate_seconds(&mgr.timing());

    let gen = mgr.generate_lfm25(&prompt, params.max_tokens, params.temperature)?;
    budget.actual_input = mgr.estimate_tokens(&prompt);
    budget.record(&gen);
    budget.actual_seconds = start.elapsed().as_secs_f64();
    if gen.truncated {
        eprintln!(
            "⚠️  answer hit the {}-token budget — may be cut off",
            params.max_tokens
        );
    }

    Ok((clean_output(&gen.text), budget))
}

/// What a `describe` call feeds the model. See `describe_file_persisted`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescribeDetail {
    /// Signatures + structure (default: 3.3x faster, measured).
    Skeleton,
    /// Full source text — when the bodies matter.
    Source,
}

impl DescribeDetail {
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "source" | "full" => DescribeDetail::Source,
            _ => DescribeDetail::Skeleton,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DescribeDetail::Skeleton => "skeleton",
            DescribeDetail::Source => "source",
        }
    }
}

/// `describe <file>` — produce a CLAIM about a file and persist it.
///
/// The producer half of the prose layer (`docs/L2_MODEL_ASSISTED_CLASSIFICATION.md`
/// §8 step 1). It reuses `explain_node`'s generation and adds the three things
/// that make the result a first-class, self-invalidating part of Code Index:
///
/// 1. **An anchor from the STORE**, not from a fresh read: `files.hash` is what
///    `sweep_prose_freshness` compares against, so both agree by construction.
///    A claim whose anchor cannot be found is refused — never written unanchored.
/// 2. **`producer` + `model`** so a model claim can never pass for an extracted
///    fact (ROADMAP: "index facts and prose claims never merge silently").
/// 3. **Optional enrichment.** A missing store or model is REPORTED, never
///    blocking (ROADMAP: "staleness surfaces, it does not punish").
///
/// `detail` controls WHAT the model reads. Measured 2026-11-20 on the same
/// 37 KB file, same machine, same model:
///
/// | mode       | prompt  | wall  | the claim |
/// |------------|---------|-------|-----------|
/// | `skeleton` |  593 t  | 43 s  | one line: what the module handles |
/// | `source`   | 1190 t  | 86 s  | names the types, the strategies, the flow |
///
/// **This is a trade, not a free win**: half the time buys half the content.
/// `skeleton` is the default because a coverage pass wants breadth — one
/// useful line per file beats a rich line for a third of the files — and
/// `source` is one flag away when depth matters more than coverage. The
/// measured numbers are here so the choice is made on evidence, not on the
/// assumption that cheaper is better.
pub fn describe_file_persisted(
    mgr: &AiManager,
    root: &std::path::Path,
    file_path: &str,
    producer: &str,
    model: &str,
    language: &str,
    detail: DescribeDetail,
) -> Result<(String, crate::llm::TokenBudget, serde_json::Value)> {
    use crate::core::code_index_db as db;

    let index_root = db::resolve_index_root(root);
    let mut conn = db::open_existing(&index_root).with_context(|| {
        format!(
            "no code-index store under {} — run `gnawtreewriter code-index --db` first \
             (claims are anchored to store hashes, so there is nothing to anchor to yet)",
            index_root.display()
        )
    })?;

    // The anchor comes from the store: this is the same hash the freshness
    // sweep will compare against, so the claim's lifetime is exact.
    let rel = {
        let abs = std::fs::canonicalize(file_path).unwrap_or_else(|_| file_path.into());
        abs.strip_prefix(&index_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| file_path.to_string())
    };
    let anchor_hash = db::file_hash(&conn, &rel).with_context(|| {
        format!(
            "{rel} is not in the code-index store — refresh it (`code-index --db`) before \
             describing it; an unanchored claim would never go stale"
        )
    })?;

    let (text, budget) = match detail {
        DescribeDetail::Source => explain_node(mgr, file_path, None, crate::llm::Resolution::Auto)?,
        DescribeDetail::Skeleton => {
            // Reuse summarize's cheap path: signatures + structure instead of
            // bodies. Measured 3.3x faster for a claim of the same use.
            let path = std::path::Path::new(file_path);
            let task = TaskParams::from_resolution(crate::llm::Resolution::Auto);
            let mut budget = crate::llm::TokenBudget::default();
            let mut exp_in = 0usize;
            let mut exp_out = 0usize;
            let text = summarize_file_chunked(
                mgr,
                &path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| file_path.to_string()),
                path,
                &task,
                &mut budget,
                &mut exp_in,
                &mut exp_out,
            )?;
            // The chunked path accumulates its expectations in its own out-
            // params; carry them into the budget it also filled so the answer
            // never reports "0 in / 0 out (expected)".
            budget.expected_input = exp_in;
            budget.expected_output = exp_out;
            budget.estimate_seconds(&mgr.timing());
            (text, budget)
        }
    };

    let revision_id = conn
        .query_row(
            "SELECT id FROM index_revisions WHERE status = 'completed' ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get::<_, i64>(0),
        )
        .ok();

    db::upsert_prose(
        &mut conn,
        "file",
        &rel,
        &anchor_hash,
        file_path,
        &text,
        producer,
        Some(model),
        language,
        revision_id,
    )?;

    let summary = db::prose_summary(&conn, true);
    Ok((text, budget, summary))
}

/// `summarize <dir>` — hierarchical map-reduce over files → dirs → project.
/// Returns (result, budget) with full token accounting.
///
/// Prefill in candle's LFM2 is roughly O(seq_len^2), so a single call must
/// stay small (~300 tokens). Large files are chunked: each chunk is
/// summarized in its own cheap call, then chunk summaries are reduced into a
/// file summary (map-reduce).
pub fn summarize_dir(
    mgr: &AiManager,
    dir: &Path,
    max_files: usize,
    resolution: crate::llm::Resolution,
) -> Result<(SummarizeResult, crate::llm::TokenBudget)> {
    let task = TaskParams::from_resolution(resolution);
    let start = std::time::Instant::now();
    let files = crate::core::file_walker::walk_source_files_filtered(
        dir,
        &[
            "rs", "py", "js", "ts", "tsx", "jsx", "go", "java", "c", "cpp", "h", "hpp", "cs",
            "php", "rb", "swift", "kt",
        ],
    );

    let mut budget = crate::llm::TokenBudget::default();
    let mut expected_input = 0usize;
    let mut expected_output = 0usize;

    // Step 1: per-file summaries (each file's AST skeleton chunked+reduced).
    let mut file_summaries: Vec<(String, String)> = Vec::new();
    for path in files.iter().take(max_files) {
        let name = path
            .strip_prefix(dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let summary = summarize_file_chunked(
            mgr,
            &name,
            path,
            &task,
            &mut budget,
            &mut expected_input,
            &mut expected_output,
        )?;
        if !summary.trim().is_empty() {
            file_summaries.push((name, summary));
        }
    }

    // Step 2: directory summary from file summaries.
    let dir_name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| dir.to_string_lossy().to_string());
    let dir_prompt = prompts::summarize_dir_prompt(&dir_name, &file_summaries);
    let dir_params = GenerateParams {
        max_tokens: task.summary_tokens,
        temperature: EXTRACT_TEMP,
    };
    expected_input += mgr.estimate_tokens(&dir_prompt);
    expected_output += dir_params.max_tokens;
    let dir_gen = mgr.generate_lfm25(&dir_prompt, dir_params.max_tokens, dir_params.temperature)?;
    budget.actual_input += mgr.estimate_tokens(&dir_prompt);
    budget.record(&dir_gen);
    let dir_summary = clean_output(&dir_gen.text);

    // Step 3: project summary (only meaningful for the root).
    let project_summary = if dir.parent().is_none() {
        let proj_prompt =
            prompts::summarize_project_prompt(&[(dir_name.clone(), dir_summary.clone())]);
        let proj_params = GenerateParams {
            max_tokens: task.synth_tokens,
            temperature: SYNTH_TEMP,
        };
        expected_input += mgr.estimate_tokens(&proj_prompt);
        expected_output += proj_params.max_tokens;
        let proj_gen = mgr.generate_lfm25(
            &proj_prompt,
            proj_params.max_tokens,
            proj_params.temperature,
        )?;
        budget.actual_input += mgr.estimate_tokens(&proj_prompt);
        budget.record(&proj_gen);
        clean_output(&proj_gen.text)
    } else {
        String::new()
    };

    budget.expected_input = expected_input;
    budget.expected_output = expected_output;
    budget.estimate_seconds(&mgr.timing());
    budget.actual_seconds = start.elapsed().as_secs_f64();

    Ok((
        SummarizeResult {
            directory: dir_name,
            file_summaries,
            directory_summary: dir_summary,
            project_summary: project_summary,
        },
        budget,
    ))
}

/// Build a compact AST skeleton of a file: top-level item names + their
/// first-line signatures. This is ~10-30x smaller than raw source, which
/// matters because LFM2 prefill in candle scales ~O(seq_len^2).
fn file_skeleton(path: &Path) -> String {
    let writer = match crate::GnawTreeWriter::new(path.to_str().unwrap_or("")) {
        Ok(w) => w,
        Err(_) => return String::new(),
    };
    let tree = writer.analyze();
    let mut parts: Vec<String> = Vec::new();
    collect_skeleton(tree, &mut parts, 0);
    parts.join("\n")
}

fn collect_skeleton(node: &crate::parser::TreeNode, out: &mut Vec<String>, depth: usize) {
    if depth > 2 {
        return;
    }
    let interesting = matches!(
        node.node_type.as_str(),
        "function_item"
            | "function_definition"
            | "function_declaration"
            | "struct_item"
            | "struct_declaration"
            | "enum_item"
            | "enum_declaration"
            | "impl_item"
            | "trait_item"
            | "trait_declaration"
            | "class_declaration"
            | "class_definition"
            | "method_definition"
            | "mod_item"
    );
    if interesting {
        let name = node.get_name().unwrap_or_default();
        let sig = node.content.lines().next().unwrap_or("").trim().to_string();
        let line = format!("{}: {}", name, sig);
        if !out.contains(&line) {
            out.push(line);
        }
    }
    for child in &node.children {
        collect_skeleton(child, out, depth + 1);
    }
}

/// Summarize one file by summarizing its compact AST skeleton. If the
/// skeleton is large (huge file), chunk it to resolution-sized pieces.
fn summarize_file_chunked(
    mgr: &AiManager,
    name: &str,
    path: &Path,
    task: &TaskParams,
    budget: &mut crate::llm::TokenBudget,
    expected_input: &mut usize,
    expected_output: &mut usize,
) -> Result<String> {
    let skeleton = file_skeleton(path);
    if skeleton.trim().is_empty() {
        // Fall back to a small raw excerpt for unsupported/empty parses.
        let content = std::fs::read_to_string(path).unwrap_or_default();
        return summarize_text_chunked(
            mgr,
            name,
            &content,
            task,
            budget,
            expected_input,
            expected_output,
        );
    }

    summarize_text_chunked(
        mgr,
        name,
        &skeleton,
        task,
        budget,
        expected_input,
        expected_output,
    )
}

/// Chunk `text` into resolution-sized pieces, summarize each, then reduce
/// into one summary. Keeps every prefill small so O(seq^2) cost stays low.
fn summarize_text_chunked(
    mgr: &AiManager,
    name: &str,
    text: &str,
    task: &TaskParams,
    budget: &mut crate::llm::TokenBudget,
    expected_input: &mut usize,
    expected_output: &mut usize,
) -> Result<String> {
    let chunks: Vec<&str> = if text.len() <= task.chunk_chars {
        vec![text]
    } else {
        text.as_bytes()
            .chunks(task.chunk_chars)
            .map(|c| std::str::from_utf8(c).unwrap_or(""))
            .collect()
    };

    // Map: summarize each chunk in its own small, fast call.
    let mut chunk_summaries: Vec<String> = Vec::with_capacity(chunks.len());
    for (i, chunk) in chunks.iter().enumerate() {
        let prompt = prompts::summarize_file_prompt(
            &format!("{name} (part {}/{})", i + 1, chunks.len()),
            chunk,
            40,
        );
        let params = GenerateParams {
            max_tokens: task.summary_tokens,
            temperature: EXTRACT_TEMP,
        };
        *expected_input += mgr.estimate_tokens(&prompt);
        *expected_output += params.max_tokens;
        let gen = mgr.generate_lfm25(&prompt, params.max_tokens, params.temperature)?;
        budget.actual_input += mgr.estimate_tokens(&prompt);
        budget.record(&gen);
        chunk_summaries.push(clean_output(&gen.text));
    }

    // Reduce: combine chunk summaries into one file summary.
    if chunk_summaries.len() <= 1 {
        return Ok(chunk_summaries.into_iter().next().unwrap_or_default());
    }
    let combined = chunk_summaries
        .iter()
        .map(|s| format!("- {s}"))
        .collect::<Vec<_>>()
        .join("\n");
    let reduce_prompt = format!(
        "Combine these partial summaries of the file '{name}' into one concise \
         summary (under 60 words). Output only the summary.\n\n{combined}\n\nSummary:"
    );
    let params = GenerateParams {
        max_tokens: task.summary_tokens,
        temperature: EXTRACT_TEMP,
    };
    *expected_input += mgr.estimate_tokens(&reduce_prompt);
    *expected_output += params.max_tokens;
    let gen = mgr.generate_lfm25(&reduce_prompt, params.max_tokens, params.temperature)?;
    budget.actual_input += mgr.estimate_tokens(&reduce_prompt);
    budget.record(&gen);
    Ok(clean_output(&gen.text))
}

/// `investigate "question"` — query expansion → index search → ranking → answer.
pub fn investigate(
    mgr: &AiManager,
    question: &str,
    resolution: crate::llm::Resolution,
) -> Result<(InvestigateResult, crate::llm::TokenBudget)> {
    let task = TaskParams::from_resolution(resolution);
    let start = std::time::Instant::now();
    let mut budget = crate::llm::TokenBudget::default();
    let mut expected_input = 0usize;
    let mut expected_output = 0usize;

    // Step 1: expand the question into search terms.
    let expand_prompt = prompts::expand_query_prompt(question);
    let expand_params = GenerateParams {
        max_tokens: task.summary_tokens.min(60),
        temperature: EXTRACT_TEMP,
    };
    expected_input += mgr.estimate_tokens(&expand_prompt);
    expected_output += expand_params.max_tokens;
    let expand_gen = mgr.generate_lfm25(
        &expand_prompt,
        expand_params.max_tokens,
        expand_params.temperature,
    )?;
    budget.actual_input += mgr.estimate_tokens(&expand_prompt);
    budget.record(&expand_gen);
    let terms = parse_json_array(&expand_gen.text).unwrap_or_else(|| vec![question.to_string()]);

    // Step 2: deterministic search over the project index.
    let root = std::env::current_dir().unwrap_or_default();
    let project_root = crate::core::find_project_root(&root);
    let candidates = search_index(&project_root, &terms, 15)?;

    // Step 3: rank candidates.
    let rank_prompt = prompts::rank_candidates_prompt(question, &candidates);
    let rank_params = GenerateParams {
        max_tokens: 40,
        temperature: EXTRACT_TEMP,
    };
    expected_input += mgr.estimate_tokens(&rank_prompt);
    expected_output += rank_params.max_tokens;
    let rank_gen = mgr.generate_lfm25(
        &rank_prompt,
        rank_params.max_tokens,
        rank_params.temperature,
    )?;
    budget.actual_input += mgr.estimate_tokens(&rank_prompt);
    budget.record(&rank_gen);
    let indices = parse_json_indices(&rank_gen.text)
        .unwrap_or_else(|| (0..candidates.len().min(3)).collect());

    // Step 4: read evidence from the top-ranked files and synthesize.
    let mut evidence: Vec<(String, String)> = Vec::new();
    for idx in indices.iter().take(3) {
        if let Some((path, _)) = candidates.get(*idx) {
            if let Ok(content) = std::fs::read_to_string(path) {
                // Char-boundary safe cap: byte slicing panics on multibyte
                // UTF-8 (the same bug class as gnaw_sense preview crash).
                let capped: String = content.chars().take(3000).collect();
                evidence.push((path.clone(), capped));
            }
        }
    }
    if evidence.is_empty() {
        anyhow::bail!("no matching code found for the question");
    }

    let synth_prompt = prompts::synthesize_answer_prompt(question, &evidence);
    let synth_params = GenerateParams {
        max_tokens: task.synth_tokens,
        temperature: SYNTH_TEMP,
    };
    expected_input += mgr.estimate_tokens(&synth_prompt);
    expected_output += synth_params.max_tokens;
    let synth_gen = mgr.generate_lfm25(
        &synth_prompt,
        synth_params.max_tokens,
        synth_params.temperature,
    )?;
    budget.actual_input += mgr.estimate_tokens(&synth_prompt);
    budget.record(&synth_gen);

    budget.expected_input = expected_input;
    budget.expected_output = expected_output;
    budget.estimate_seconds(&mgr.timing());
    budget.actual_seconds = start.elapsed().as_secs_f64();

    Ok((
        InvestigateResult {
            terms,
            candidates: candidates.iter().map(|(p, _)| p.clone()).collect(),
            answer: clean_output(&synth_gen.text),
            sources: sources_from_evidence(&evidence),
        },
        budget,
    ))
}

/// Deterministic candidate search: match terms against entity/relation index
/// file paths + content keywords.
fn search_index(
    project_root: &Path,
    terms: &[String],
    max_results: usize,
) -> Result<Vec<(String, String)>> {
    let mut scored: Vec<(usize, String, String)> = Vec::new(); // (score, path, summary)

    // Walk source files and score by term hits in path + content.
    let files = crate::core::file_walker::walk_source_files_filtered(
        project_root,
        &[
            "rs", "py", "js", "ts", "tsx", "jsx", "go", "java", "c", "cpp", "h", "hpp", "cs",
            "php", "rb", "swift", "kt",
        ],
    );
    for path in files {
        let rel = path
            .strip_prefix(project_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_lowercase();
        let mut score = 0usize;
        for t in terms {
            let tl = t.to_lowercase();
            if rel.contains(&tl) {
                score += 3;
            }
        }
        if score == 0 {
            // Shallow content peek (first 8KB) for term hits.
            if let Ok(content) = std::fs::read_to_string(&path) {
                let cl = content[..content.len().min(8192)].to_lowercase();
                for t in terms {
                    if cl.contains(&t.to_lowercase()) {
                        score += 1;
                    }
                }
            }
        }
        if score > 0 {
            scored.push((score, path.to_string_lossy().to_string(), String::new()));
        }
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(scored
        .into_iter()
        .take(max_results)
        .map(|(_, p, _)| (p, String::new()))
        .collect())
}

/// Trim boilerplate the model may add around its answer.
fn clean_output(s: &str) -> String {
    s.trim()
        .trim_start_matches("Summary:")
        .trim_start_matches("Answer:")
        .trim()
        .to_string()
}

/// Result of `summarize`.
#[derive(Debug, serde::Serialize)]
pub struct SummarizeResult {
    pub directory: String,
    pub file_summaries: Vec<(String, String)>,
    pub directory_summary: String,
    pub project_summary: String,
}

/// Result of `investigate`.
#[derive(Debug, serde::Serialize)]
pub struct InvestigateResult {
    pub terms: Vec<String>,
    pub candidates: Vec<String>,
    pub answer: String,
    /// Files the answer was actually synthesized from — the provenance
    /// contract (ROADMAP 9.4): every LLM answer is auditable via read_node.
    pub sources: Vec<Source>,
}

/// Proposed edit from the model: which node to replace and with what content.
#[derive(Debug)]
pub struct EditProposal {
    pub node_path: String,
    pub content: String,
    pub budget: crate::llm::TokenBudget,
    /// The original source line the model targeted (for multi-edit).
    pub old_line: String,
    /// The replacement line text (for multi-edit).
    pub new_line: String,
}

/// `edit --ask`: the model proposes a minimal old→new change; GTW finds the
/// AST node containing `old`, applies the replacement *inside* that node, and
/// returns the full new node content. The caller runs it through the Duplex
/// Loop (preview_edit) before applying — the AST validates the model's intent.
pub fn propose_edit(
    mgr: &AiManager,
    file_path: &str,
    request: &str,
    resolution: crate::llm::Resolution,
) -> Result<EditProposal> {
    let task = TaskParams::from_resolution(resolution);
    let start = std::time::Instant::now();

    let writer = crate::GnawTreeWriter::new(file_path)
        .with_context(|| format!("failed to parse {file_path}"))?;
    let tree = writer.analyze();
    let source = writer.get_source().to_string();

    // Give the model a bounded, line-numbered preview (small files fully).
    let preview_raw: String = source.chars().take(6000).collect();
    let preview: String = preview_raw
        .lines()
        .enumerate()
        .map(|(i, l)| format!("{:>4} | {}", i + 1, l))
        .collect::<Vec<_>>()
        .join("\n");

    // Rule-injected expertise: annotate known rule violations in the file so
    // the model can avoid introducing/worsening them (spec step 3).
    let lang = std::path::Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let (findings, _, _) = crate::core::rules::check_code_with_builtin(&source, &lang);
    let issues = crate::core::rules::format_findings_for_prompt(&findings);

    let prompt = prompts::edit_ask_prompt(file_path, request, &preview, &issues);
    let params = GenerateParams {
        max_tokens: task.synth_tokens,
        temperature: EXTRACT_TEMP,
    };

    let mut budget = crate::llm::TokenBudget::default();
    budget.expected_input = mgr.estimate_tokens(&prompt);
    budget.expected_output = params.max_tokens;
    budget.estimate_seconds(&mgr.timing());

    let gen = mgr.generate_lfm25(&prompt, params.max_tokens, params.temperature)?;
    budget.actual_input = mgr.estimate_tokens(&prompt);
    budget.record(&gen);
    budget.actual_seconds = start.elapsed().as_secs_f64();

    // Parse the line-based proposal: {line, new}.
    let (line, new) = match parse_edit_proposal(&gen.text) {
        Some(p) => p,
        None => {
            eprintln!("[edit-ask] raw model response:\n{}", gen.text);
            anyhow::bail!("model did not return a valid edit proposal");
        }
    };
    if new.trim().is_empty() {
        anyhow::bail!("model returned an empty edit proposal");
    }

    // Read the target line from the source, then find the smallest AST node
    // containing that line and replace the line's text inside it.
    let source_lines: Vec<&str> = source.lines().collect();
    if line == 0 || line > source_lines.len() {
        anyhow::bail!(
            "model proposed line {line}, but the file has {} lines",
            source_lines.len()
        );
    }
    let old_line = source_lines[line - 1];
    let node = find_node_containing_line(tree, line, old_line.trim())
        .ok_or_else(|| anyhow::anyhow!("could not find a node on line {line} in {file_path}"))?;
    let node_path = node.path.clone();
    let node_content = node.content.clone();
    if !node_content.contains(old_line.trim()) {
        anyhow::bail!(
            "model's target line does not appear in node {node_path}: {}",
            old_line.trim()
        );
    }
    let mut new_content = node_content.replacen(old_line.trim(), new.trim(), 1);
    // Build the multi-edit replacement line, applying the same normalizations.
    let mut new_line = new.trim().to_string();
    // If the replaced line ended with `;` but the new content does not, add it
    // back so the node stays valid (a small model often drops it).
    if old_line.trim().ends_with(';')
        && !new_content.trim_end().ends_with(';')
        && !new_content.trim_end().ends_with('}')
    {
        new_content = format!("{};", new_content.trim_end());
        new_line = format!("{};", new_line.trim_end());
    }
    // If the original line was a `let` declaration and the model only gave the
    // value expression (dropped `let x =`), keep the binding name.
    if old_line.trim().starts_with("let ") && !new_content.trim().starts_with("let ") {
        let binding = old_line
            .trim()
            .strip_prefix("let ")
            .and_then(|s| s.split('=').next())
            .unwrap_or("")
            .trim();
        if !binding.is_empty() {
            new_content = format!(
                "let {} = {};",
                binding,
                new_content.trim().trim_end_matches(';')
            );
            new_line = format!(
                "let {} = {};",
                binding,
                new_line.trim().trim_end_matches(';')
            );
        }
    }

    Ok(EditProposal {
        node_path,
        content: new_content,
        budget,
        old_line: old_line.trim().to_string(),
        new_line,
    })
}

/// `edit --ask --all`: the model gives a recurring snippet (old) and its
/// replacement (new); GTW applies it to every occurrence in the file.
pub fn propose_edit_all(
    mgr: &AiManager,
    file_path: &str,
    request: &str,
    resolution: crate::llm::Resolution,
) -> Result<EditProposal> {
    let task = TaskParams::from_resolution(resolution);
    let start = std::time::Instant::now();

    let writer = crate::GnawTreeWriter::new(file_path)
        .with_context(|| format!("failed to parse {file_path}"))?;
    let source = writer.get_source().to_string();

    let preview_raw: String = source.chars().take(6000).collect();
    let preview: String = preview_raw
        .lines()
        .enumerate()
        .map(|(i, l)| format!("{:>4} | {}", i + 1, l))
        .collect::<Vec<_>>()
        .join("\n");

    let lang = std::path::Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let (findings, _, _) = crate::core::rules::check_code_with_builtin(&source, &lang);
    let issues = crate::core::rules::format_findings_for_prompt(&findings);

    let prompt = prompts::edit_ask_all_prompt(file_path, request, &preview, &issues);
    let params = GenerateParams {
        max_tokens: task.synth_tokens,
        temperature: EXTRACT_TEMP,
    };

    let mut budget = crate::llm::TokenBudget::default();
    budget.expected_input = mgr.estimate_tokens(&prompt);
    budget.expected_output = params.max_tokens;
    budget.estimate_seconds(&mgr.timing());

    let gen = mgr.generate_lfm25(&prompt, params.max_tokens, params.temperature)?;
    budget.actual_input = mgr.estimate_tokens(&prompt);
    budget.record(&gen);
    budget.actual_seconds = start.elapsed().as_secs_f64();

    let (old, new) = match parse_old_new_proposal(&gen.text) {
        Some(p) => p,
        None => {
            eprintln!("[edit-ask-all] raw model response:\n{}", gen.text);
            anyhow::bail!("model did not return a valid edit proposal");
        }
    };
    if old.trim().is_empty() || new.trim().is_empty() {
        anyhow::bail!("model returned an empty old/new edit proposal");
    }
    let count = source.matches(old.trim()).count();
    if count == 0 {
        anyhow::bail!("model's 'old' snippet \"{}\" not found in file", old.trim());
    }

    Ok(EditProposal {
        node_path: String::new(),
        content: new.trim().to_string(),
        budget,
        old_line: old.trim().to_string(),
        new_line: new.trim().to_string(),
    })
}

/// Parse `{"old": "...", "new": "..."}` (for --all multi-edit).
/// Takes the FIRST object only (the model may emit several).
fn parse_old_new_proposal(s: &str) -> Option<(String, String)> {
    let marker = s.find("{\"old\"")?;
    let json_str = &s[marker..];
    // Find the end of the first object: scan forward tracking brace depth.
    let mut depth = 0i32;
    let mut end = None;
    for (i, c) in json_str.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end?;
    let json_str = &json_str[..=end];
    let v: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let old = v.get("old")?.as_str()?.to_string();
    let new = v.get("new")?.as_str()?.to_string();
    Some((old, new))
}

/// Find the smallest node that spans `line` AND whose content contains
/// `line_text`. Prefers the smallest such node (the actual statement).
fn find_node_containing_line<'a>(
    node: &'a crate::parser::TreeNode,
    line: usize,
    line_text: &str,
) -> Option<&'a crate::parser::TreeNode> {
    if line < node.start_line || line > node.end_line {
        return None;
    }
    let mut best: Option<&crate::parser::TreeNode> = None;
    for child in &node.children {
        if let Some(found) = find_node_containing_line(child, line, line_text) {
            best = Some(found);
        }
    }
    // If a child matched, prefer it; otherwise use this node if it contains
    // the line text (meaningful node, not just a spanning container).
    if let Some(b) = best {
        return Some(b);
    }
    if node.content.contains(line_text) && !node.content.trim().is_empty() {
        Some(node)
    } else {
        None
    }
}

/// Parse the model's JSON edit proposal (line-based): `{"line": N, "new": "..."}`.
/// Looks for the `{"line"` marker because `new` may contain braces.
fn parse_edit_proposal(s: &str) -> Option<(usize, String)> {
    let marker = s.find("{\"line\"")?;
    let json_str = &s[marker..];
    // Find the end of the first object (the model may emit several).
    let mut depth = 0i32;
    let mut end = None;
    for (i, c) in json_str.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end?;
    let json_str = &json_str[..=end];
    let v: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let line = match v.get("line") {
        Some(serde_json::Value::Number(n)) => n.as_u64()? as usize,
        Some(serde_json::Value::String(s)) => s.parse::<usize>().ok()?,
        _ => return None,
    };
    let new = v.get("new")?.as_str()?.to_string();
    Some((line, new))
}
