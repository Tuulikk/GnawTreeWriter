use crate::core::file_walker::walk_source_files;
use crate::parser::TreeNode;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

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
    /// Identifier tokens from this file's use/import statements (Fas 4.2
    /// step 3) — file stems appearing here narrow ambiguous call targets.
    /// `default` keeps older graph JSON files loadable.
    #[serde(default)]
    pub imports: HashSet<String>,
}

/// Resolution quality of the knowledge graph (Fas 4.2 step 1):
/// how much of the call graph is unambiguously resolvable. This is the
/// trust level of impact reports — surfaced via `doctor` and stats.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexStats {
    pub files_indexed: usize,
    pub definitions: usize,
    /// Names defined in exactly one file.
    pub unique_symbols: usize,
    /// Names defined in more than one file (same-name ambiguity).
    pub ambiguous_symbols: usize,
    pub calls_total: usize,
    /// Call relations with a definite definition site.
    pub calls_resolved: usize,
    /// Unknown or ambiguous sites — impact counts these conservatively.
    pub calls_unresolved: usize,
    /// Top ambiguous names with their defining files (max 10).
    pub top_ambiguous: Vec<AmbiguousName>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AmbiguousName {
    pub name: String,
    pub files: Vec<String>,
}

/// Fas 4.2 step 3: collect identifier tokens from use/import statements
/// (rust `use_statement`, python `import_from_statement`, js/ts
/// `import_statement`, ...). Tokens include BOTH module segments
/// (crate::utils::parse -> crate, utils, parse) and imported names, so
/// file-stem matching and future symbol-level narrowing share one set.
/// Known noise: language keywords (filtered) and the calling file's own
/// imported names — harmless for stem matching.
fn collect_imports(node: &TreeNode, acc: &mut HashSet<String>) {
    const KEYWORDS: [&str; 9] = [
        "use", "crate", "self", "super", "pub", "as", "from", "import", "extern",
    ];
    let t = node.node_type.as_str();
    if (t.contains("use") || t.contains("import")) && !t.contains("call") {
        for tok in node
            .content
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        {
            if tok.len() > 1
                && !tok.chars().next().unwrap().is_ascii_digit()
                && !KEYWORDS.contains(&tok)
            {
                acc.insert(tok.to_string());
            }
        }
    }
    for child in &node.children {
        collect_imports(child, acc);
    }
}

pub struct RelationalIndexer {
    storage_dir: PathBuf,
    symbol_table: HashMap<String, Vec<String>>, // Name -> List of files where defined
    /// Stats from the last index_directory run (Fas 4.2 step 1).
    last_stats: Option<IndexStats>,
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
            last_stats: None,
        }
    }

    /// Stats from the last `index_directory` run, if any. For on-demand
    /// stats over an existing store (also older ones), use
    /// `resolution_stats`.
    pub fn last_index_stats(&self) -> Option<&IndexStats> {
        self.last_stats.as_ref()
    }

    /// Fas 4.2 step 1: recompute resolution quality from the stored
    /// graphs on demand — works for indexes built before the same-file
    /// rule too.
    pub fn resolution_stats(&self) -> Result<IndexStats> {
        let graphs = self.load_all_graphs()?;
        let mut def_files: HashMap<String, std::collections::BTreeSet<String>> = HashMap::new();
        let mut definitions = 0usize;
        for g in &graphs {
            definitions += g.definitions.len();
            for name in g.definitions.keys() {
                def_files
                    .entry(name.clone())
                    .or_default()
                    .insert(g.file_path.clone());
            }
        }
        let mut calls_total = 0usize;
        let mut calls_resolved = 0usize;
        for g in &graphs {
            for r in &g.relations {
                if r.relation_type == RelationType::Call {
                    calls_total += 1;
                    if r.to_file.is_some() {
                        calls_resolved += 1;
                    }
                }
            }
        }
        let mut ambiguous: Vec<AmbiguousName> = def_files
            .iter()
            .filter(|(_, files)| files.len() > 1)
            .map(|(name, files)| AmbiguousName {
                name: name.clone(),
                files: files.iter().cloned().collect(),
            })
            .collect();
        ambiguous.sort_by(|a, b| b.files.len().cmp(&a.files.len()).then(a.name.cmp(&b.name)));
        ambiguous.truncate(10);
        Ok(IndexStats {
            files_indexed: graphs.len(),
            definitions,
            unique_symbols: def_files.values().filter(|f| f.len() == 1).count(),
            ambiguous_symbols: def_files.values().filter(|f| f.len() > 1).count(),
            calls_total,
            calls_resolved,
            calls_unresolved: calls_total.saturating_sub(calls_resolved),
            top_ambiguous: ambiguous,
        })
    }

    /// Scan a directory and build relations between files recursively
    pub fn index_directory(&mut self, dir_path: &Path) -> Result<Vec<FileGraph>> {
        let mut graphs = Vec::new();

        // 1. First pass: Collect all definitions in the directory recursively
        for path in walk_source_files(dir_path) {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(parser) = crate::parser::get_parser(&path) {
                    if let Ok(tree) = parser.parse(&content) {
                        let mut defs = HashMap::new();
                        self.collect_definitions(&tree, &mut defs);

                        let mut imports = HashSet::new();
                        collect_imports(&tree, &mut imports);

                        let file_str = path.to_string_lossy().to_string();
                        for name in defs.keys() {
                            self.symbol_table
                                .entry(name.clone())
                                .or_default()
                                .push(file_str.clone());
                        }

                        graphs.push((path.to_path_buf(), tree, defs, imports));
                    }
                }
            }
        }

        // 2. Second pass: Map calls to discovered definitions
        let mut final_graphs = Vec::new();
        for (path, tree, defs, imports) in graphs {
            let file_str = path.to_string_lossy().to_string();
            let mut relations = HashSet::new();
            self.extract_relations(&tree, &file_str, &defs, &imports, &mut relations);

            let graph = FileGraph {
                file_path: file_str,
                relations,
                definitions: defs,
                imports,
            };

            self.save_graph(&graph)?;
            final_graphs.push(graph);
        }

        // Fas 4.2 step 1: snapshot resolution quality of this run.
        self.last_stats = self.resolution_stats().ok();

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

    fn extract_relations(
        &self,
        node: &TreeNode,
        current_file: &str,
        defs: &HashMap<String, String>,
        imports: &HashSet<String>,
        acc: &mut HashSet<Relation>,
    ) {
        if node.node_type.contains("call") || node.node_type.contains("usage") {
            if let Some(name) = node.get_name() {
                // Same-module precedence (Fas 4.2 step 2): a definition in
                // the CURRENT file wins even when other files define the
                // same name — resolves most ambiguity with zero path
                // parsing. (Known trade-off: bare-name matching can still
                // conflate a same-named method call.)
                let to_file = if defs.contains_key(&name) {
                    Some(current_file.to_string())
                } else {
                    match self.symbol_table.get(&name) {
                        // Unambiguous by definition.
                        Some(files) if files.len() == 1 => files.first().cloned(),
                        // Fas 4.2 step 3: import-aware narrowing — among the
                        // candidate files, exactly one whose module name
                        // appears in this file's import tokens wins. A
                        // candidate's module names = file stem + parent
                        // dir (src/core/mod.rs is module "core", so stem
                        // alone would miss the dominant Rust layout).
                        Some(files) => {
                            let hits: Vec<&String> = files
                                .iter()
                                .filter(|f| {
                                    let p = Path::new(f);
                                    let stem_hit = p
                                        .file_stem()
                                        .and_then(|s| s.to_str())
                                        .map(|stem| imports.contains(stem))
                                        .unwrap_or(false);
                                    let dir_hit = p
                                        .parent()
                                        .and_then(|d| d.file_name())
                                        .and_then(|s| s.to_str())
                                        .map(|dir| imports.contains(dir))
                                        .unwrap_or(false);
                                    stem_hit || dir_hit
                                })
                                .collect();
                            if hits.len() == 1 {
                                Some(hits[0].to_string())
                            } else {
                                None
                            }
                        }
                        None => None,
                    }
                };

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
            self.extract_relations(child, current_file, defs, imports, acc);
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
        if !self.storage_dir.exists() {
            return Ok(graphs);
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// a.rs defines dup AND calls its own dup; b.rs defines the same
    /// name — the same-file rule must still resolve the caller.
    fn write_project(dir: &Path) {
        std::fs::create_dir(dir.join(".git")).unwrap();
        std::fs::write(
            dir.join("a.rs"),
            "fn dup(x: u32) -> u32 {\n    x\n}\n\nfn call_a() -> u32 {\n    dup(1)\n}\n",
        )
        .unwrap();
        std::fs::write(dir.join("b.rs"), "fn dup(x: u32) -> u32 {\n    x + 1\n}\n").unwrap();
    }

    #[test]
    fn same_file_definition_resolves_despite_ambiguity() {
        let dir = tempdir().unwrap();
        write_project(dir.path());
        let mut indexer = RelationalIndexer::new(dir.path());
        let graphs = indexer.index_directory(dir.path()).unwrap();
        let a = graphs
            .iter()
            .find(|g| g.file_path.ends_with("a.rs"))
            .unwrap();
        let rel = a
            .relations
            .iter()
            .find(|r| r.to_name == "dup" && r.relation_type == RelationType::Call)
            .expect("dup call relation");
        assert_eq!(
            rel.to_file.as_deref(),
            Some(a.file_path.as_str()),
            "same-file rule wins over cross-file ambiguity"
        );
    }

    #[test]
    fn rust_use_stem_narrows_same_name_candidates() {
        // parse is defined in BOTH a.rs and utils.rs; caller.rs imports
        // it from utils — the use-stem match must resolve the caller.
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        std::fs::write(
            dir.path().join("a.rs"),
            "pub fn parse(x: u32) -> u32 {\n    x\n}\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("utils.rs"),
            "pub fn parse(x: u32) -> u32 {\n    x * 2\n}\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("caller.rs"),
            "use crate::utils::parse;\n\npub fn run() -> u32 {\n    parse(1)\n}\n",
        )
        .unwrap();
        let mut indexer = RelationalIndexer::new(dir.path());
        let graphs = indexer.index_directory(dir.path()).unwrap();
        let caller = graphs
            .iter()
            .find(|g| g.file_path.ends_with("caller.rs"))
            .unwrap();
        let rel = caller
            .relations
            .iter()
            .find(|r| r.to_name == "parse" && r.relation_type == RelationType::Call)
            .expect("parse call relation");
        assert_eq!(
            rel.to_file
                .as_deref()
                .map(|f| Path::new(f).file_stem().and_then(|s| s.to_str())),
            Some(Some("utils")),
            "import stem must pick utils.rs: {rel:?}"
        );
    }

    #[test]
    fn bare_call_without_imports_stays_ambiguous() {
        // Same two definitions, but the caller has NO use statement —
        // must stay None (no first-match guessing).
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        std::fs::write(
            dir.path().join("a.rs"),
            "pub fn parse(x: u32) -> u32 {\n    x\n}\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("utils.rs"),
            "pub fn parse(x: u32) -> u32 {\n    x * 2\n}\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("caller.rs"),
            "pub fn run() -> u32 {\n    parse(1)\n}\n",
        )
        .unwrap();
        let mut indexer = RelationalIndexer::new(dir.path());
        let graphs = indexer.index_directory(dir.path()).unwrap();
        let caller = graphs
            .iter()
            .find(|g| g.file_path.ends_with("caller.rs"))
            .unwrap();
        let rel = caller
            .relations
            .iter()
            .find(|r| r.to_name == "parse" && r.relation_type == RelationType::Call)
            .expect("parse call relation");
        assert_eq!(rel.to_file, None, "no import = no guess");
    }

    #[test]
    fn python_from_import_narrows_candidates() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        std::fs::write(dir.path().join("utils.py"), "def parse(x):\n    return x\n").unwrap();
        std::fs::write(dir.path().join("a.py"), "def parse(x):\n    return x + 1\n").unwrap();
        std::fs::write(
            dir.path().join("caller.py"),
            "from utils import parse\n\n\ndef run():\n    return parse(1)\n",
        )
        .unwrap();
        let mut indexer = RelationalIndexer::new(dir.path());
        let graphs = indexer.index_directory(dir.path()).unwrap();
        let caller = graphs
            .iter()
            .find(|g| g.file_path.ends_with("caller.py"))
            .unwrap();
        let rel = caller
            .relations
            .iter()
            .find(|r| r.to_name == "parse" && r.relation_type == RelationType::Call)
            .expect("parse call relation");
        assert_eq!(
            rel.to_file
                .as_deref()
                .map(|f| Path::new(f).file_stem().and_then(|s| s.to_str())),
            Some(Some("utils")),
            "from-import must pick utils.py: {rel:?}"
        );
    }

    #[test]
    fn resolution_stats_reports_ambiguity_and_calls() {
        let dir = tempdir().unwrap();
        write_project(dir.path());
        let mut indexer = RelationalIndexer::new(dir.path());
        indexer.index_directory(dir.path()).unwrap();
        let stats = indexer.resolution_stats().unwrap();
        assert_eq!(stats.files_indexed, 2);
        assert_eq!(stats.ambiguous_symbols, 1, "dup is defined in 2 files");
        assert!(
            stats
                .top_ambiguous
                .iter()
                .any(|a| a.name == "dup" && a.files.len() == 2),
            "top_ambiguous names dup: {stats:?}"
        );
        assert!(stats.calls_total >= 1);
        assert_eq!(
            stats.calls_resolved + stats.calls_unresolved,
            stats.calls_total
        );
        assert_eq!(
            indexer.last_index_stats().map(|s| s.calls_total),
            Some(stats.calls_total),
            "last_index_stats mirrors the last run"
        );
    }
}
