#[cfg(feature = "modernbert")]
mod tests {
    use gnawtreewriter::llm::RelationalIndexer;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_cross_file_relations() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        // 1. Create two related files
        let file_a = path.join("logic.rs");
        let code_a = "pub fn calculate_price() { return 100; }";
        fs::write(&file_a, code_a)?;

        let file_b = path.join("main.rs");
        let code_b = "fn run() { let p = calculate_price(); }";
        fs::write(&file_b, code_b)?;

        // 2. Index the directory
        let mut indexer = RelationalIndexer::new(path);
        let graphs = indexer.index_directory(path)?;

        // 3. Verify relations
        assert_eq!(graphs.len(), 2);
        
        let main_graph = graphs.iter().find(|g| g.file_path.contains("main.rs")).unwrap();
        
        // Find the call relation
        let call = main_graph.relations.iter().find(|r| r.to_name == "calculate_price").unwrap();
        
        println!("Found cross-file relation: {} -> {}:{}", 
            call.from_file, 
            call.to_file.as_ref().unwrap_or(&"unknown".into()), 
            call.to_name
        );

        assert!(call.to_file.is_some(), "Should have found the file for calculate_price");
        assert!(call.to_file.as_ref().unwrap().contains("logic.rs"));

        Ok(())
    }
}
