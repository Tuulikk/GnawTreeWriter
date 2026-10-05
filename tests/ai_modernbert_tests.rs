use anyhow::Result;
use gnawtreewriter::llm::AiManager;
#[cfg(feature = "modernbert")]
use std::path::Path;

/// Helper to check if ModernBERT is actually installed before running heavy tests
#[cfg(feature = "modernbert")]
fn is_modernbert_installed(project_root: &Path) -> bool {
    project_root
        .join(".gnawtreewriter_ai/models/modernbert/model.safetensors")
        .exists()
}

// NOTE: The previous semantic search, completion, and refactor tests in this file
// were removed as they were written for an older, deprecated version of AiManager.
// Modern semantic search is now tested in tests/gnaw_sense_integration.rs.

#[test]
fn test_ai_status_detection() -> Result<()> {
    let project_root = std::env::current_dir()?;
    let manager = AiManager::new(&project_root)?;
    let status = manager.get_status()?;

    let local = project_root.join(".gnawtreewriter_ai/models");
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let global = std::path::PathBuf::from(&home).join(".gnawtreewriter_ai/models");
    assert!(
        status.cache_dir == local || status.cache_dir == global,
        "cache_dir mismatch: {:?}",
        status.cache_dir
    );

    #[cfg(feature = "modernbert")]
    {
        if is_modernbert_installed(&project_root) {
            assert!(status.modern_bert_installed);
        }
    }

    Ok(())
}

/// Batched embeddings (get_embeddings) must agree row-for-row with the
/// sequential get_embedding — same math, only float reduction order
/// differs. Covers padding (ragged lengths) and the batch-size boundary
/// (20 generated texts > EMBED_BATCH=16 → two forward passes). Skips
/// when the model is not installed.
///
/// `#[ignore]`: debug-mode ModernBERT forwards cost ~200 s — run
/// explicitly when touching get_embeddings:
///   cargo test --test ai_modernbert_tests -- --ignored --nocapture
#[cfg(feature = "modernbert")]
#[test]
#[ignore = "heavy: debug ModernBERT forwards (~3 min); run when touching get_embeddings"]
fn batch_embeddings_match_singles() -> Result<()> {
    let project_root = std::env::current_dir()?;
    if !is_modernbert_installed(&project_root) {
        eprintln!("skipping batch test: modernbert model not installed");
        return Ok(());
    }
    let manager = AiManager::new(&project_root)?;
    let model = manager.load_model(
        gnawtreewriter::llm::AiModel::ModernBert,
        gnawtreewriter::llm::DeviceType::Cpu,
    )?;

    let mut texts: Vec<String> = vec![
        "fn alpha() { let x = 1; }".to_string(),
        "pub struct Beta { field: u32 }".to_string(),
        "x".to_string(), // deliberately tiny — heavy padding on its row
        "use std::collections::HashMap;".to_string(),
    ];
    for i in 0..20 {
        texts.push(format!(
            "fn generated_{i}(x: u32) -> u32 {{ let scale = {i}; x.wrapping_mul(scale).wrapping_add(scale) }}"
        ));
    }
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();

    // Sequential baseline (timed) vs one batched call — the throughput
    // claim in a nutshell: forwards go from N to ceil(N / 16).
    let t0 = std::time::Instant::now();
    let mut singles: Vec<Vec<f32>> = Vec::with_capacity(texts.len());
    for text in &texts {
        singles.push(model.get_embedding(text)?.to_vec1()?);
    }
    let sequential = t0.elapsed();

    let t1 = std::time::Instant::now();
    let batch = model.get_embeddings(&refs)?;
    let batched = t1.elapsed();
    eprintln!(
        "timing: {} texts — sequential {:?} vs batched {:?} ({:.1}x)",
        texts.len(),
        sequential,
        batched,
        sequential.as_secs_f64() / batched.as_secs_f64().max(1e-9)
    );

    assert_eq!(batch.len(), texts.len(), "one vector per input, in order");
    for (i, single) in singles.iter().enumerate() {
        assert_eq!(batch[i].len(), single.len(), "row {i} dimension");
        let dot: f32 = batch[i].iter().zip(single).map(|(a, b)| a * b).sum();
        let na: f32 = batch[i].iter().map(|a| a * a).sum::<f32>().sqrt();
        let nb: f32 = single.iter().map(|b| b * b).sum::<f32>().sqrt();
        let cos = dot / (na * nb);
        assert!(
            cos > 0.999,
            "batch row {i} diverged from sequential embedding: cos={cos}"
        );
    }
    Ok(())
}
