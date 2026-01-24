#[cfg(feature = "modernbert")]
mod tests {
    use gnawtreewriter::llm::{GnawSenseBroker, AiModel, DeviceType, AiManager};
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_gnaw_sense_basic_navigation() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let test_files_dir = dir.path();
        
        // Use the current working directory as project root to find the models
        let project_root = std::env::current_dir()?;
        let ai_manager = AiManager::new(&project_root)?;
        
        // Ensure models are present (should skip download if already there)
        ai_manager.setup(AiModel::ModernBert, DeviceType::Cpu, false).await?;

        // 2. Create a test file in the temp dir
        let test_file_path = test_files_dir.join("test_logic.py");
        let test_code = r#"
def calculate_sum(a, b):
    """Adds two numbers together."""
    return a + b

def save_to_database(data):
    """Persists information to the storage layer."""
    print(f"Saving {data}")

def handle_git_commit(message):
    """Manages version control operations."""
    print(f"Committing with: {message}")
"#;
        fs::write(&test_file_path, test_code)?;

        // 3. Initialize GnawSense Broker
        let broker = GnawSenseBroker::new(&project_root)?;

        // 4. Perform a "Zoom" search (within the file) using a "weak" description
        let query = "how do I save data to the disk?";
        let response = broker.sense(query, Some(test_file_path.to_str().unwrap())).await?;

        if let gnawtreewriter::llm::SenseResponse::Zoom { nodes, .. } = response {
            assert!(!nodes.is_empty(), "Should find at least one matching node");
            
            // The top match should be 'save_to_database'
            let top_match = &nodes[0];
            println!("Top match: {} (score: {})", top_match.path, top_match.score);
            assert!(top_match.preview.contains("save_to_database"));
        } else {
            panic!("Expected a Zoom response when file context is provided");
        }

        // 5. Test another query for git
        let git_query = "version control stuff";
        let git_response = broker.sense(git_query, Some(test_file_path.to_str().unwrap())).await?;
        
        if let gnawtreewriter::llm::SenseResponse::Zoom { nodes, .. } = git_response {
            let top_match = &nodes[0];
            println!("Git match: {} (score: {})", top_match.path, top_match.score);
            assert!(top_match.preview.contains("handle_git_commit"));
        }

        Ok(())
    }
}
