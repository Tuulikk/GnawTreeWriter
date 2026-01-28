use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::{HashSet, HashMap};
use std::path::{Path, PathBuf};
use std::fs;
use crate::parser::TreeNode;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RelationType {
    Call,       // Function or method call
    Definition, // Where a symbol is defined
    Reference,  // General usage/reference
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Relation {
    pub from_file: String,
    pub from_path: String,
    pub to_file: Option<String>, // None if unknown (external or not yet indexed)
    pub to_name: String,
    pub relation_type: RelationType,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FileGraph {
    pub file_path: String,
    pub relations: HashSet<Relation>,
    pub definitions: HashMap<String, String>, // Name -> Path within file
}

pub struct RelationalIndexer {
    storage_dir: PathBuf,
    symbol_table: HashMap<String, Vec<String>>, // Name -> List of files where defined
}

impl RelationalIndexer {
    pub fn new(project_root: &Path) -> Self {
        let storage_dir = project_root.join(".gnawtreewriter_ai").join("graph");
        if !storage_dir.exists() {
            let _ = fs::create_dir_all(&storage_dir);
        }
        Self { 
            storage_dir,
            symbol_table: HashMap::new(),
        }
    }

    /// Scan a directory and build relations between files recursively
    pub fn index_directory(&mut self, dir_path: &Path) -> Result<Vec<FileGraph>> {
        let mut graphs = Vec::new();
        use walkdir::WalkDir;
        
        // 1. First pass: Collect all definitions in the directory recursively
        for entry in WalkDir::new(dir_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            
            // Skip hidden directories and common build/environment folders
            let is_ignored = path.components().any(|c| {
                let s = c.as_os_str().to_str().unwrap_or("");
                s.starts_with('.') || s == "venv" || s == "node_modules" || s == "target" || s == "__pycache__" || s == "env"
            });
            
            if is_ignored {
                continue;
            }

            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(parser) = crate::parser::get_parser(path) {
                    if let Ok(tree) = parser.parse(&content) {
                        let mut defs = HashMap::new();
                        self.collect_definitions(&tree, &mut defs);
                        
                        let file_str = path.to_string_lossy().to_string();
                        for name in defs.keys() {
                            self.symbol_table.entry(name.clone())
                                .or_default()
                                .push(file_str.clone());
                        }
                        
                        graphs.push((path.to_path_buf(), tree, defs));
                    }
                }
            }
        }

        // 2. Second pass: Map calls to discovered definitions
        let mut final_graphs = Vec::new();
        for (path, tree, defs) in graphs {
            let file_str = path.to_string_lossy().to_string();
            let mut relations = HashSet::new();
            self.extract_relations(&tree, &file_str, &mut relations);
            
            let graph = FileGraph {
                file_path: file_str,
                relations,
                definitions: defs,
            };
            
            self.save_graph(&graph)?;
            final_graphs.push(graph);
        }

        Ok(final_graphs)
    }

    fn collect_definitions(&self, node: &TreeNode, acc: &mut HashMap<String, String>) {
        if node.node_type.contains("definition") || node.node_type.contains("item") {
            if let Some(name) = node.get_name() {
                acc.insert(name, node.path.clone());
            }
        }
        for child in &node.children {
            self.collect_definitions(child, acc);
        }
    }

    fn extract_relations(&self, node: &TreeNode, current_file: &str, acc: &mut HashSet<Relation>) {
        if node.node_type.contains("call") || node.node_type.contains("usage") {
            if let Some(name) = node.get_name() {
                // Check if we know where this is defined
                let to_file = self.symbol_table.get(&name)
                    .and_then(|files| files.first()) // Simplified: take first match
                    .cloned();

                acc.insert(Relation {
                    from_file: current_file.to_string(),
                    from_path: node.path.clone(),
                    to_file,
                    to_name: name,
                    relation_type: RelationType::Call,
                });
            }
        }

        for child in &node.children {
            self.extract_relations(child, current_file, acc);
        }
    }

    pub fn save_graph(&self, graph: &FileGraph) -> Result<()> {
        let file_hash = crate::core::transaction_log::calculate_content_hash(&graph.file_path);
        let save_path = self.storage_dir.join(format!("{}.json", file_hash));
        let data = serde_json::to_string_pretty(graph)?;
        fs::write(save_path, data)?;
        Ok(())
    }

    pub fn load_all_graphs(&self) -> Result<Vec<FileGraph>> {
        let mut graphs = Vec::new();
        if !self.storage_dir.exists() { return Ok(graphs); }

        for entry in fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let data = fs::read_to_string(path)?;
                if let Ok(graph) = serde_json::from_str::<FileGraph>(&data) {
                    graphs.push(graph);
                }
            }
        }
        Ok(graphs)
    }
}