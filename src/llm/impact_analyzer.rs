use crate::llm::{RelationType, RelationalIndexer};
use anyhow::Result;
use std::path::Path;

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

    /// Convenience: default RelationalIndexer rooted at project_root
    /// (reads .gnawtreewriter_ai/graph/).
    pub fn new_with_root(project_root: &Path) -> Self {
        Self::new(RelationalIndexer::new(project_root))
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
            affected_files: affected
                .into_iter()
                .map(|(path, paths)| AffectedFile {
                    file_path: path,
                    call_paths: paths,
                })
                .collect(),
        })
    }

    /// Fas 4 (docs/GUARDIAN_V2_PLAN.md): read all saved graphs from the
    /// index directory (.gnawtreewriter_ai/graph/*.json). Missing or empty
    /// index yields an empty vec — callers must treat that as "no impact
    /// data", never an error.
    fn load_all_graphs(&self) -> Result<Vec<crate::llm::relational_index::FileGraph>> {
        self.indexer.load_all_graphs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::relational_index::{Relation, RelationType};
    use std::collections::{HashMap, HashSet};
    use std::path::Path;

    #[test]
    fn analyze_impact_finds_callers_from_saved_graphs() {
        let dir = tempfile::tempdir().unwrap();
        let indexer = RelationalIndexer::new(dir.path());
        let mut relations = HashSet::new();
        relations.insert(Relation {
            from_file: "src/b.rs".to_string(),
            from_path: "0.1.2".to_string(),
            to_file: Some("src/a.rs".to_string()),
            to_name: "target".to_string(),
            relation_type: RelationType::Call,
        });
        indexer
            .save_graph(&crate::llm::relational_index::FileGraph {
                file_path: "src/b.rs".to_string(),
                relations,
                definitions: HashMap::new(),
            })
            .unwrap();

        let analyzer = ImpactAnalyzer::new(RelationalIndexer::new(dir.path()));
        let report = analyzer.analyze_impact("target", "src/a.rs").unwrap();
        assert_eq!(report.affected_files.len(), 1);
        assert_eq!(report.affected_files[0].file_path, "src/b.rs");
        assert_eq!(report.affected_files[0].call_paths, vec!["0.1.2"]);
    }

    #[test]
    fn analyze_impact_without_index_is_empty_not_error() {
        let dir = tempfile::tempdir().unwrap();
        let analyzer = ImpactAnalyzer::new(RelationalIndexer::new(dir.path()));
        let report = analyzer.analyze_impact("ghost", "src/a.rs").unwrap();
        assert!(report.affected_files.is_empty());
    }

    #[test]
    fn new_with_root_builds_default_analyzer() {
        let dir = tempfile::tempdir().unwrap();
        let analyzer = ImpactAnalyzer::new_with_root(Path::new(dir.path()));
        let report = analyzer.analyze_impact("x", "f.rs").unwrap();
        assert!(report.affected_files.is_empty());
    }
}
