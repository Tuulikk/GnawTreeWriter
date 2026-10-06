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
    /// 1-based rank of the expected hit in top-k; None = miss.
    pub hit_at_rank: Option<usize>,
    pub top_cosine: f32,
}

#[derive(Debug, Serialize)]
pub struct RecallReport {
    pub cases: usize,
    pub hits: usize,
    pub recall_at_1: f64,
    pub recall_at_k: f64,
    pub mrr: f64,
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
    let mut reciprocal_sum = 0.0f64;

    for case in &cases {
        let t0 = std::time::Instant::now();
        let query_vector = model.get_query_embedding(&case.query)?.to_vec1()?;
        let embed_ms = t0.elapsed().as_secs_f64() * 1000.0;

        let t1 = std::time::Instant::now();
        let hits = index.search(&query_vector, k);
        let search_ms = t1.elapsed().as_secs_f64() * 1000.0;

        sum_embed_ms += embed_ms;
        sum_search_ms += search_ms;

        let hit_at_rank = hits
            .iter()
            .position(|(e, _)| {
                e.file_path.ends_with(&case.expect_file)
                    && case
                        .expect_preview
                        .as_deref()
                        .map(|p| e.content_preview.to_lowercase().contains(&p.to_lowercase()))
                        .unwrap_or(true)
            })
            .map(|i| i + 1);

        if let Some(rank) = hit_at_rank {
            reciprocal_sum += 1.0 / rank as f64;
        }

        per_case.push(CaseResult {
            query: case.query.clone(),
            hit_at_rank,
            top_cosine: hits.first().map(|(_, s)| *s).unwrap_or(0.0),
        });
    }

    let n = cases.len() as f64;
    let hits = per_case.iter().filter(|r| r.hit_at_rank.is_some()).count();
    let report = RecallReport {
        cases: cases.len(),
        hits,
        recall_at_1: per_case.iter().filter(|r| r.hit_at_rank == Some(1)).count() as f64 / n,
        recall_at_k: hits as f64 / n,
        mrr: reciprocal_sum / n,
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
            "  recall@1: {:5.1}%   recall@{}: {:5.1}%   MRR: {:.3}",
            report.recall_at_1 * 100.0,
            report.k,
            report.recall_at_k * 100.0,
            report.mrr
        );
        println!(
            "  mean embed: {:.0} ms   mean search: {:.1} ms",
            report.mean_embed_ms, report.mean_search_ms
        );
        println!();
        for r in &report.per_case {
            let status = match r.hit_at_rank {
                Some(1) => "hit (1)".to_string(),
                Some(rank) => format!("hit ({rank})"),
                None => "MISS    ".to_string(),
            };
            println!("  {} cos={:.2}  {}", status, r.top_cosine, r.query);
        }
    }
    Ok(())
}
