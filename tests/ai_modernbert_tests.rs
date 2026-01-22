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

    assert_eq!(
        status.cache_dir,
        project_root.join(".gnawtreewriter_ai/models")
    );
    
    #[cfg(feature = "modernbert")]
    {
        if is_modernbert_installed(&project_root) {
            assert!(status.modern_bert_installed);
        }
    }

    Ok(())
}