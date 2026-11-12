#![cfg(feature = "ai")]

use crate::core::file_walker::walk_source_files;
use crate::llm::{GnawSenseBroker, NodeEmbedding, SemanticIndexManager};
use crate::parser::{get_parser, TreeNode};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ProjectIndexer {
    project_root: PathBuf,
    broker: GnawSenseBroker,
    index_manager: SemanticIndexManager,
}

/// Outcome of an index run — three numbers that must add up.
#[derive(Debug, Default, Clone, Copy, serde::Serialize)]
pub struct IndexReport {
    /// Files the walker reached and read.
    pub visited: usize,
    /// Files whose vectors were (re)built this run.
    pub embedded: usize,
    /// Files skipped because their vectors already match this content.
    pub unchanged: usize,
    /// Files whose vectors were removed because the file no longer exists.
    pub pruned: usize,
}

impl ProjectIndexer {
    pub fn new(project_root: &Path) -> Result<Self> {
        Ok(Self {
            project_root: project_root.to_path_buf(),
            broker: GnawSenseBroker::new(project_root)?,
            index_manager: SemanticIndexManager::new(project_root),
        })
    }

    /// Crawl the project and index supported source files starting from target_path
    /// What an index run actually did. `visited == embedded + unchanged` — the
    /// old code reported only `visited` as "Successfully indexed N files", which
    /// reads as work done even when every file was skipped by hash (2026-11-11).
    pub async fn index_all_report(&self, target_path: &Path) -> Result<IndexReport> {
        let mut report = IndexReport::default();
        // Dimension is read from the FIRST vector actually produced, so the
        // recorded model_info describes this model rather than a constant.
        let mut dimension: Option<usize> = None;
        // GPU only here (indexing), behind the 20%-VRAM gate; sense/query
        // paths below stay on CPU by design — see ai_manager::indexing_device.
        let model = self.broker.get_manager().load_model(
            crate::llm::AiManager::index_model(),
            crate::llm::ai_manager::indexing_device(),
        )?;

        // Canonicalize target_path to ensure strip_prefix works
        let target_path = if target_path.is_relative() {
            fs::canonicalize(target_path).unwrap_or(target_path.to_path_buf())
        } else {
            target_path.to_path_buf()
        };

        for path in walk_source_files(&target_path) {
            // A generated artifact duplicates the source it was rendered from:
            // embedding it doubles the noise floor and re-embeds on every build.
            if crate::core::file_walker::is_generated_artifact(&path) {
                continue;
            }
            if let Ok(parser) = get_parser(&path) {
                // Try to strip prefix safely
                let file_path_str = path
                    .strip_prefix(&self.project_root)
                    .unwrap_or(&path) // Fallback to full path if prefix doesn't match
                    .to_string_lossy()
                    .to_string();

                if let Ok(content) = fs::read_to_string(&path) {
                    Self::trace_file(&report, &file_path_str);
                    // SMART RE-INDEXING: Check if file changed
                    // SMART RE-INDEXING, done honestly: ask the store whether
                    // these vectors were built from THIS content. The old check
                    // asked whether a shard file existed — its name was a hash of
                    // the PATH, so it existed from the first run onward and every
                    // later edit was silently skipped (2026-11-11).
                    let already = self
                        .index_manager
                        .has_content_hash(&file_path_str, &content)
                        .unwrap_or(false);
                    if already {
                        report.visited += 1;
                        report.unchanged += 1;
                        Self::report_progress(&report);
                        continue;
                    }

                    if let Ok(tree) = parser.parse(&content) {
                        // Two phases: walk collects (node_path, preview, text)
                        // triples, then ONE batched embedding call per file
                        // (16 nodes per forward instead of one per node).
                        let mut pending: Vec<(String, String, String)> = Vec::new();
                        Self::collect_pending(&tree, &mut pending);
                        Self::cap_pending(&mut pending);
                        // The file's own //! description bridges agent
                        // queries ("avoid reparsing unchanged files") to
                        // implementation symbols (get_or_parse) — prepend it
                        // to both the embedded text and the stored preview.
                        let module_doc = Self::extract_module_doc(&content);
                        if !module_doc.is_empty() {
                            for t in pending.iter_mut() {
                                t.2 = format!("{}\n\n{}", t.2, module_doc);
                            }
                        }
                        if !pending.is_empty() {
                            let texts: Vec<&str> = pending.iter().map(|t| t.2.as_str()).collect();
                            let vectors = model.get_embeddings(&texts)?;
                            if dimension.is_none() {
                                dimension = vectors.first().map(|v| v.len());
                            }
                            let entries = pending
                                .into_iter()
                                .zip(vectors)
                                .map(|((node_path, content_preview, _), vector)| NodeEmbedding {
                                    file_path: file_path_str.clone(),
                                    node_path,
                                    content_preview,
                                    vector,
                                })
                                .collect();
                            self.index_manager
                                .save_index(&file_path_str, &content, entries)?;
                            report.visited += 1;
                            report.embedded += 1;
                            Self::report_progress(&report);
                        }
                    }
                }
            }
        }

        // Durable cleanup: vectors for files that no longer exist. Done AFTER
        // the walk so a file that just appeared is never pruned, and reported
        // so the operator sees the store shrink for a reason.
        report.pruned = self.index_manager.prune_missing_files()?;

        // Model metadata must name the model that ACTUALLY embedded. The old
        // call hardcoded "bge-base-en-v1.5"/768, which would record a lie the
        // moment another index model is configured — and `store_model_mismatch`
        // trusts this row.
        if let Some(dim) = dimension {
            self.index_manager
                .save_model_info(crate::llm::AiManager::index_model().id(), dim)?;
        }

        Ok(report)
    }

    /// A 40-minute run must not be silent (measured CPU cost on this repo:
    /// ~7 files/min, 2026-11-11). Every 25 files, to stderr so `--json`/pipes
    /// keep working. `GTW_INDEX_TRACE=1` names every file BEFORE it is processed
    /// — the only way to see which file a run is sitting on rather than which
    /// one it just finished.
    fn trace_file(report: &IndexReport, file: &str) {
        if std::env::var_os("GTW_INDEX_TRACE").is_some() {
            eprintln!("  → [{}] {}", report.visited + 1, file);
        }
    }

    fn report_progress(report: &IndexReport) {
        if std::env::var_os("GTW_INDEX_TRACE").is_some() {
            return;
        }
        if report.visited.is_multiple_of(25) {
            eprintln!(
                "  … {} file(s): {} embedded, {} unchanged",
                report.visited, report.embedded, report.unchanged
            );
        }
    }

    /// Backwards-compatible entry point: returns the number of files VISITED.
    /// Kept so existing callers (and path-dependents) keep compiling.
    pub async fn index_all(&self, target_path: &Path) -> Result<usize> {
        Ok(self.index_all_report(target_path).await?.visited)
    }

    /// Walk the tree collecting (node_path, preview, text) triples for
    /// every embeddable node/chunk — embedding happens afterwards in ONE
    /// batched call per file (see ModernBertModel::get_embeddings). No
    /// model in this phase, so the walk is trivially cheap.
    fn extract_module_doc(content: &str) -> String {
        let mut doc = String::new();
        for line in content.lines() {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("//!") {
                doc.push_str(rest.trim());
                doc.push(' ');
                if doc.chars().count() >= 200 {
                    break;
                }
            } else if !trimmed.is_empty() {
                break;
            }
        }
        doc.trim().to_string()
    }
    /// A single file may not own the run. `src/cli.rs` (7 027 lines) produced
    /// 485 chunks — ~248k tokens, about six minutes at the ~700 tok/s this CPU
    /// manages — which is what turned a full rebuild into a 40-minute job.
    /// Same policy as `gnaw_sense::MAX_EMBED_NODES`: keep the LARGEST units,
    /// because size correlates with "has something worth finding"; a huge file
    /// is then represented by its biggest definitions instead of by all of them.
    const MAX_CHUNKS_PER_FILE: usize = 96;

    /// Keep the biggest `MAX_CHUNKS_PER_FILE` units (in place).
    fn cap_pending(pending: &mut Vec<(String, String, String)>) -> usize {
        if pending.len() <= Self::MAX_CHUNKS_PER_FILE {
            return 0;
        }
        let before = pending.len();
        pending.sort_by_key(|t| std::cmp::Reverse(t.2.len()));
        pending.truncate(Self::MAX_CHUNKS_PER_FILE);
        before - Self::MAX_CHUNKS_PER_FILE
    }

    fn collect_pending(node: &TreeNode, acc: &mut Vec<(String, String, String)>) {
        // Index functions, classes, and important definitions
        if node.node_type.contains("definition") || node.node_type.contains("item") {
            // CHUNKING LOGIC: If node is too large, split it.
            // ModernBERT safe limit is roughly 8192 tokens. Chunking
            // thresholds must respect the 8192-token rope context; worst
            // case is ~1 token/char (CJK), so chunks stay at 4000 chars and
            // nodes over 8000 chars are chunked.
            if node.content.chars().count() > 8000 {
                let chunks = Self::chunk_text(&node.content, 4000, 500);
                for (i, chunk) in chunks.into_iter().enumerate() {
                    let preview = format!(
                        "(Chunk {}) {}",
                        i,
                        crate::llm::gnaw_sense::truncate_preview(chunk.trim(), 240)
                    );
                    acc.push((format!("{}[chunk:{}]", node.path, i), preview, chunk));
                }
            } else {
                let preview = crate::llm::gnaw_sense::truncate_preview(&node.content, 240);
                acc.push((node.path.clone(), preview, node.content.clone()));
            }
        }

        for child in &node.children {
            Self::collect_pending(child, acc);
        }
    }

    /// Split `text` into overlapping chunks of at most `size` CHARACTERS
    /// (not bytes — byte arithmetic panics on multibyte UTF-8).
    fn chunk_text(text: &str, size: usize, overlap: usize) -> Vec<String> {
        let mut chunks = Vec::new();
        if text.is_empty() {
            return chunks;
        }

        let chars: Vec<char> = text.chars().collect();
        let mut start = 0;
        while start < chars.len() {
            let end = (start + size).min(chars.len());
            chunks.push(chars[start..end].iter().collect());
            if end == chars.len() {
                break;
            }
            start += size - overlap;
        }
        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectIndexer;

    /// The cap this module exists for: one huge file must not own the run.
    /// `src/cli.rs` produced 485 chunks (≈248k tokens ≈ 6 min on CPU); the cap
    /// keeps the 96 largest and drops the rest, so the worst case per file is
    /// bounded regardless of file size.
    #[test]
    fn cap_pending_bounds_one_file() {
        let mut pending: Vec<(String, String, String)> = (0..500)
            .map(|i| (format!("{i}"), "p".to_string(), "x".repeat(i + 1)))
            .collect();
        let dropped = ProjectIndexer::cap_pending(&mut pending);
        assert_eq!(pending.len(), ProjectIndexer::MAX_CHUNKS_PER_FILE);
        assert_eq!(dropped, 500 - ProjectIndexer::MAX_CHUNKS_PER_FILE);
        // Largest kept: the first element is the longest text in the input.
        assert_eq!(pending[0].2.len(), 500);

        // Under the cap nothing moves or disappears.
        let mut small: Vec<(String, String, String)> = vec![("a".into(), "p".into(), "t".into())];
        assert_eq!(ProjectIndexer::cap_pending(&mut small), 0);
        assert_eq!(small.len(), 1);
    }

    /// Regression: byte-based chunking (`text[start..end]`) panicked on
    /// multibyte UTF-8 at chunk boundaries.
    #[test]
    fn chunk_text_is_char_boundary_safe() {
        let text = format!("{}{}", "函".repeat(3000), "🦀".repeat(3000));
        let chunks = ProjectIndexer::chunk_text(&text, 4000, 500);
        assert!(!chunks.is_empty());
        for c in &chunks {
            assert!(c.chars().count() <= 4000);
        }
        // Reassembly with overlap must preserve all characters.
        let total: usize = chunks.iter().map(|c| c.chars().count()).sum();
        assert!(total >= text.chars().count());
    }

    #[test]
    fn chunk_text_edge_cases() {
        assert!(ProjectIndexer::chunk_text("", 100, 10).is_empty());
        let short = "fn main() {}";
        assert_eq!(
            ProjectIndexer::chunk_text(short, 100, 10),
            vec![short.to_string()]
        );
    }
}
