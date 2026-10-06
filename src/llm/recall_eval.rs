//! Recall@k harness for the semantic index — measurement only, no
//! behavior change. Gives the baseline number for "how good is the
//! ModernBERT + JSON store solution" and verifies the SQLite migration
//! later (docs/GUARDIAN_V2_PLAN.md culture: measure before swaps).
//!
//! Usage: gnawtreewriter ai recall-eval evals/sense_recall.json
//! Eval set: JSON array of {query, expect_file, expect_preview?}. A hit
//! counts when a top-k result's stored path ends with expect_file and
//! (when given) its content_preview contains expect_preview
//! (case-insensitive).

use crate::core::find_project_root;
use crate::llm::relational_index::RelationalIndexer;
use crate::llm::semantic_index::{rerank_satellite_with_graphs, NodeEmbedding};
use crate::llm::{indexing_device, AiManager, AiModel, SemanticIndexManager};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct RecallCase {
    pub query: String,
    /// File the answer must come from (suffix match on the stored path).
    pub expect_file: String,
    /// Case-insensitive substring expected in the hit's content_preview.
    pub expect_preview: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CaseResult {
    pub query: String,
    /// Rank in the RAW cosine top-k (model diagnostic); None = miss.
    pub hit_at_rank: Option<usize>,
    /// Rank via the sense pipeline — wide cosine window + rerank
    /// (lexical signals, decl/impl priors, per-file diversity), i.e.
    /// what `gnawtreewriter sense` actually serves.
    pub hit_at_rank_pipeline: Option<usize>,
    pub top_cosine: f32,
    /// Adjusted (post-rerank) score of the top pipeline hit.
    pub top_adjusted: Option<f32>,
    /// Rank of the expected entry in the RAW cosine window (2000 cap)
    /// — model-side diagnostic. None = the model never surfaced it.
    pub expected_cosine_rank: Option<usize>,
    /// Rank of the expected entry in the FULL reranked order (not
    /// capped at k) — reranker-side diagnostic. > k means the reranker
    /// pushed the answer below the served window.
    pub expected_pipeline_rank: Option<usize>,
    /// file_paths of the 10 highest pipeline hits (displacement audit).
    pub top_pipeline_files: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RecallReport {
    pub cases: usize,
    /// Hits via the sense pipeline (what users get).
    pub hits: usize,
    pub recall_at_1: f64,
    pub recall_at_k: f64,
    pub mrr: f64,
    /// Raw-cosine diagnostics (the embedding model alone).
    pub raw_hits: usize,
    pub raw_recall_at_1: f64,
    pub raw_recall_at_k: f64,
    pub raw_mrr: f64,
    pub mean_embed_ms: f64,
    pub mean_search_ms: f64,
    pub k: usize,
    pub per_case: Vec<CaseResult>,
}

pub fn run_recall_eval(eval_path: &Path, k: usize, json_out: bool) -> Result<()> {
    let raw = std::fs::read_to_string(eval_path)
        .with_context(|| format!("Failed to read eval set {}", eval_path.display()))?;
    let cases: Vec<RecallCase> = serde_json::from_str(&raw).context(
        "Failed to parse eval set — expected a JSON array of {query, expect_file, expect_preview?}",
    )?;
    if cases.is_empty() {
        anyhow::bail!("Eval set has no cases.");
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let project_root = find_project_root(&cwd);

    let index_manager = SemanticIndexManager::new(&project_root);
    let index = index_manager.load_project_index()?;
    if index.entries.is_empty() {
        anyhow::bail!("Semantic index is empty — run `gnawtreewriter ai index` first.");
    }
    let dim = index.entries[0].vector.len();
    if let Ok(Some(info)) = index_manager.get_model_info() {
        if info.dimension != dim {
            eprintln!(
                "⚠️  index dim {} != model_info dim {} — the index may be stale; re-run `ai index`.",
                dim, info.dimension
            );
        }
    }

    let manager = AiManager::new(&project_root)?;
    let model = manager.load_model(AiModel::Bge, indexing_device())?;

    let mut per_case = Vec::new();
    let mut sum_embed_ms = 0.0f64;
    let mut sum_search_ms = 0.0f64;
    // Knowledge graphs for the third rerank channel (graph proximity)
    // — loaded once; absent graphs degrade to the two-channel floor.
    let graphs = RelationalIndexer::new(&project_root)
        .load_all_graphs()
        .unwrap_or_default();
    let mut reciprocal_sum = 0.0f64;
    let mut raw_reciprocal_sum = 0.0f64;

    for case in &cases {
        let t0 = std::time::Instant::now();
        let query_vector = model.get_query_embedding(&case.query)?.to_vec1()?;
        let embed_ms = t0.elapsed().as_secs_f64() * 1000.0;

        let t1 = std::time::Instant::now();
        let hits = index.search(&query_vector, k);
        let search_ms = t1.elapsed().as_secs_f64() * 1000.0;

        sum_embed_ms += embed_ms;
        sum_search_ms += search_ms;

        // The wide window + rerank the sense pipeline actually serves
        // (same shape as gnaw_sense: 2000 candidates, threshold 0.1,
        // graph-proximity channel from the knowledge graphs).
        let wide = index.search_with_threshold(&query_vector, 2000, 0.1);
        let is_hit = |e: &NodeEmbedding| {
            e.file_path.ends_with(&case.expect_file)
                && case
                    .expect_preview
                    .as_deref()
                    .map(|p| e.content_preview.to_lowercase().contains(&p.to_lowercase()))
                    .unwrap_or(true)
        };
        // Calibration diagnostics: where does the expected entry sit in
        // the RAW window, and in the FULL reranked order (not capped at
        // k)? This separates "the model never surfaced it" from "the
        // reranker pushed it out of the served window".
        let expected_cosine_rank = wide.iter().position(|(e, _)| is_hit(e)).map(|i| i + 1);
        let window = wide.len();
        let reranked = rerank_satellite_with_graphs(&case.query, wide, window, Some(&graphs));
        let hit_at_rank = hits.iter().position(|(e, _)| is_hit(e)).map(|i| i + 1);
        let expected_pipeline_rank = reranked.iter().position(|h| is_hit(h.entry)).map(|i| i + 1);
        let hit_at_rank_pipeline = expected_pipeline_rank.filter(|r| *r <= k);
        let top_pipeline_files: Vec<String> = reranked
            .iter()
            .take(10)
            .map(|h| h.entry.file_path.clone())
            .collect();

        if let Some(rank) = hit_at_rank_pipeline {
            reciprocal_sum += 1.0 / rank as f64;
        }
        if let Some(rank) = hit_at_rank {
            raw_reciprocal_sum += 1.0 / rank as f64;
        }

        per_case.push(CaseResult {
            query: case.query.clone(),
            hit_at_rank,
            hit_at_rank_pipeline,
            top_cosine: hits.first().map(|(_, s)| *s).unwrap_or(0.0),
            top_adjusted: reranked.first().map(|h| h.adjusted),
            expected_cosine_rank,
            expected_pipeline_rank,
            top_pipeline_files,
        });
    }

    let n = cases.len() as f64;
    let hits = per_case
        .iter()
        .filter(|r| r.hit_at_rank_pipeline.is_some())
        .count();
    let raw_hits = per_case.iter().filter(|r| r.hit_at_rank.is_some()).count();
    let report = RecallReport {
        cases: cases.len(),
        hits,
        recall_at_1: per_case
            .iter()
            .filter(|r| r.hit_at_rank_pipeline == Some(1))
            .count() as f64
            / n,
        recall_at_k: hits as f64 / n,
        mrr: reciprocal_sum / n,
        raw_hits,
        raw_recall_at_1: per_case.iter().filter(|r| r.hit_at_rank == Some(1)).count() as f64 / n,
        raw_recall_at_k: raw_hits as f64 / n,
        raw_mrr: raw_reciprocal_sum / n,
        mean_embed_ms: sum_embed_ms / n,
        mean_search_ms: sum_search_ms / n,
        k,
        per_case,
    };

    if json_out {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "Recall@{} eval — {} queries against {} entries",
            report.k,
            report.cases,
            index.entries.len()
        );
        println!(
            "  pipeline  recall@1: {:5.1}%   recall@{}: {:5.1}%   MRR: {:.3}",
            report.recall_at_1 * 100.0,
            report.k,
            report.recall_at_k * 100.0,
            report.mrr
        );
        println!(
            "  raw cos   recall@1: {:5.1}%   recall@{}: {:5.1}%   MRR: {:.3}",
            report.raw_recall_at_1 * 100.0,
            report.k,
            report.raw_recall_at_k * 100.0,
            report.raw_mrr
        );
        println!(
            "  mean embed: {:.0} ms   mean search: {:.1} ms",
            report.mean_embed_ms, report.mean_search_ms
        );
        println!();
        for r in &report.per_case {
            let pipe = match r.hit_at_rank_pipeline {
                Some(1) => "hit (1)".to_string(),
                Some(rank) => format!("hit ({rank})"),
                None => "MISS    ".to_string(),
            };
            let raw = match r.hit_at_rank {
                Some(rank) => format!("raw({rank})"),
                None => "raw(-)  ".to_string(),
            };
            println!("  {} {} cos={:.2}  {}", pipe, raw, r.top_cosine, r.query);
        }
    }
    Ok(())
}
