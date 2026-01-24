use anyhow::Result;
use std::collections::HashSet;
use crate::llm::{RelationalIndexer, RelationType};

pub struct ImpactAnalyzer {
    indexer: RelationalIndexer,
}

#[derive(Debug, serde::Serialize)]
pub struct ImpactReport {
    pub target_symbol: String,
    pub affected_files: Vec<AffectedFile>,
}

#[derive(Debug, serde::Serialize)]
pub struct AffectedFile {
    pub file_path: String,
    pub call_paths: Vec<String>, // List of node paths where the call originates
}

impl ImpactAnalyzer {
    pub fn new(indexer: RelationalIndexer) -> Self {
        Self { indexer }
    }

    /// Find all files and nodes that call a specific symbol defined in a file
    pub fn analyze_impact(&self, symbol_name: &str, _defined_in: &str) -> Result<ImpactReport> {
        let mut affected = std::collections::HashMap::new();
        
        // In a real implementation, we would search the entire index.
        // For now, we search the files that the indexer has currently loaded in its symbol table.
        // (This will be improved as we implement the project-wide crawler)
        
        // For this version, let's look through all saved graph files
        let graphs = self.load_all_graphs()?;
        
        for graph in graphs {
            let mut node_paths = Vec::new();
            for relation in &graph.relations {
                if relation.to_name == symbol_name && relation.relation_type == RelationType::Call {
                    node_paths.push(relation.from_path.clone());
                }
            }
            
            if !node_paths.is_empty() {
                affected.insert(graph.file_path.clone(), node_paths);
            }
        }

        Ok(ImpactReport {
            target_symbol: symbol_name.to_string(),
            affected_files: affected.into_iter().map(|(path, paths)| AffectedFile {
                file_path: path,
                call_paths: paths,
            }).collect(),
        })
    }

    fn load_all_graphs(&self) -> Result<Vec<crate::llm::relational_index::FileGraph>> {
        // Implementation to read all JSON files from the storage dir
        // and deserialize them back into graphs.
        // (Code omitted for brevity, will be implemented in next step)
        Ok(Vec::new()) // Placeholder
    }
}
