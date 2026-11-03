//! Phase 3 completion tests — the knowledge graph reads from the
//! code-index SQLite store; JSON files become the fallback ladder.

use gnawtreewriter::core::code_index::CodeIndexOptions;
use gnawtreewriter::core::code_index_db;
use gnawtreewriter::llm::{ImpactAnalyzer, RelationalIndexer};

fn opts() -> CodeIndexOptions {
    CodeIndexOptions {
        extensions: vec!["rs".to_string()],
        ..Default::default()
    }
}

fn make_store(dir: &std::path::Path) {
    std::fs::write(dir.join(".git"), "gitdir: nowhere\n").unwrap();
    std::fs::write(dir.join("lib.rs"), "pub fn shared() -> u8 { 1 }\n").unwrap();
    std::fs::write(dir.join("other.rs"), "pub fn caller() -> u8 { shared() }\n").unwrap();
    code_index_db::generate_with_db(dir, &opts()).unwrap();
}

#[test]
fn graphs_load_from_db_store() {
    let dir = tempfile::tempdir().unwrap();
    make_store(dir.path());

    let indexer = RelationalIndexer::new(dir.path());
    let graphs = indexer.load_all_graphs().unwrap();
    assert!(!graphs.is_empty(), "DB-backed graphs must be non-empty");

    // definitions carry dot-paths (the read_node chain).
    let lib = graphs.iter().find(|g| g.file_path == "lib.rs").unwrap();
    let shared_path = lib.definitions.get("shared").expect("shared defined");
    assert!(!shared_path.is_empty(), "definition must carry a dot-path");

    // The cross-file call edge is resolved with to_file.
    let other = graphs.iter().find(|g| g.file_path == "other.rs").unwrap();
    let call = other
        .relations
        .iter()
        .find(|r| {
            r.to_name == "shared" && r.relation_type == gnawtreewriter::llm::RelationType::Call
        })
        .expect("cross-file call edge present");
    assert_eq!(
        call.to_file.as_deref(),
        Some("lib.rs"),
        "to_file resolved via to_id join: {call:?}"
    );
}

#[test]
fn resolution_stats_work_over_db_graphs() {
    let dir = tempfile::tempdir().unwrap();
    make_store(dir.path());
    let indexer = RelationalIndexer::new(dir.path());
    let stats = indexer.resolution_stats().unwrap();
    assert!(stats.calls_total >= 1, "calls counted: {stats:?}");
    assert!(
        stats.calls_resolved >= 1,
        "cross-file call resolved: {stats:?}"
    );
    assert_eq!(stats.calls_unresolved, 0);
}

#[test]
fn impact_analysis_reads_db_graphs() {
    let dir = tempfile::tempdir().unwrap();
    make_store(dir.path());
    let analyzer = ImpactAnalyzer::new_with_root(dir.path());
    let report = analyzer
        .analyze_impact("shared", "lib.rs")
        .expect("impact analysis over DB graphs");
    assert!(
        report
            .affected_files
            .iter()
            .any(|af| af.file_path.contains("other.rs")),
        "caller in other.rs found: {report:?}"
    );
}

#[test]
fn index_directory_uses_store_when_present() {
    let dir = tempfile::tempdir().unwrap();
    make_store(dir.path());
    let mut indexer = RelationalIndexer::new(dir.path());
    let graphs = indexer.index_directory(dir.path()).unwrap();
    assert!(!graphs.is_empty());
    assert!(
        graphs.iter().any(|g| !g.relations.is_empty()),
        "DB path serves relations without the legacy JSON pipeline"
    );
}

#[test]
fn legacy_json_pipeline_still_works_without_db() {
    // Capability ladder: no store → the struktur level degrades to the
    // legacy parse+JSON path (pre-Phase-3 behavior preserved).
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("solo.rs"), "pub fn solo() -> u8 { 1 }\n").unwrap();
    let mut indexer = RelationalIndexer::new(dir.path());
    let graphs = indexer.index_directory(dir.path()).unwrap();
    assert!(!graphs.is_empty(), "legacy path indexes the dir");
    assert!(graphs[0].definitions.contains_key("solo"));
}
