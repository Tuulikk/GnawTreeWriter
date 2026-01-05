use anyhow::Result;
use gnawtreewriter::llm::{AiManager, DeviceType};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

/// Helper to check if ModernBERT is actually installed before running heavy tests
fn is_modernbert_installed(project_root: &Path) -> bool {
    project_root
        .join(".gnawtreewriter_ai/models/modernbert/model.safetensors")
        .exists()
}

#[tokio::test]
#[cfg(feature = "modernbert")]
async fn test_modernbert_semantic_search() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    // We assume the test is run from the project root
    let project_root = current_dir.clone();

    if !is_modernbert_installed(&project_root) {
        println!("Skipping test: ModernBERT model not found in .gnawtreewriter_ai/");
        return Ok(());
    }

    let manager = AiManager::new(&project_root)?;

    // Create dummy nodes for testing
    let nodes = vec![
        gnawtreewriter::TreeNode {
            path: "0".to_string(),
            node_type: "function".to_string(),
            content: "def login(username, password):\n    return auth.verify(username, password)"
                .to_string(),
            ..Default::default()
        },
        gnawtreewriter::TreeNode {
            path: "1".to_string(),
            node_type: "function".to_string(),
            content: "def calculate_sum(a, b):\n    return a + b".to_string(),
            ..Default::default()
        },
    ];

    // Search for authentication related code
    let results = manager
        .semantic_search("authentication and security", &nodes, DeviceType::Cpu)
        .await?;

    assert!(!results.is_empty(), "Should return at least one result");
    assert_eq!(
        results[0].path, "0",
        "The login function should be the top match for 'authentication'"
    );
    assert!(
        results[0].score > results[1].score,
        "Login should have higher score than math function"
    );

    Ok(())
}

#[tokio::test]
#[cfg(feature = "modernbert")]
async fn test_modernbert_code_completion_masking() -> Result<()> {
    let temp = tempdir()?;
    let project_root = std::env::current_dir()?;

    if !is_modernbert_installed(&project_root) {
        return Ok(());
    }

    let test_file = temp.path().join("test.py");
    let content = "def hello():\n    print('world')\n\ndef goodbye():\n    print('bye')";
    fs::write(&test_file, content)?;

    let manager = AiManager::new(&project_root)?;

    // Test completion at a specific node path
    // Note: This test verifies the integration of the masking logic with the model
    let suggestions = manager
        .complete_code(test_file.to_str().unwrap(), "1")
        .await?;

    assert!(
        !suggestions.is_empty(),
        "Should return completion suggestions"
    );
    // ModernBERT should suggest something plausible for a print statement or function body
    println!("Top suggestion: {}", suggestions[0].text);

    Ok(())
}

#[tokio::test]
#[cfg(feature = "modernbert")]
async fn test_modernbert_refactor_suggestions() -> Result<()> {
    let temp = tempdir()?;
    let project_root = std::env::current_dir()?;

    if !is_modernbert_installed(&project_root) {
        return Ok(());
    }

    let test_file = temp.path().join("complex.py");
    // A moderately complex function to trigger the heuristic
    let content = "def complex_logic(data):\n".to_owned() + &"    x = data * 2\n".repeat(5);
    fs::write(&test_file, content)?;

    let manager = AiManager::new(&project_root)?;

    let suggestions = manager
        .suggest_refactor(test_file.to_str().unwrap(), None)
        .await?;

    // Our new implementation uses embedding norm to find complex nodes
    // This should trigger a suggestion for the root or the function node
    assert!(
        !suggestions.is_empty(),
        "Should find refactoring opportunities in complex code"
    );
    assert!(suggestions[0].title.contains("Simplify") || suggestions[0].title.contains("Complex"));

    Ok(())
}

#[test]
fn test_ai_status_detection() -> Result<()> {
    let project_root = std::env::current_dir()?;
    let manager = AiManager::new(&project_root)?;
    let status = manager.get_status()?;

    assert_eq!(
        status.cache_dir,
        project_root.join(".gnawtreewriter_ai/models")
    );
    // If the model is there, status should reflect it
    if is_modernbert_installed(&project_root) {
        assert!(status.modern_bert_installed);
    }

    Ok(())
}
