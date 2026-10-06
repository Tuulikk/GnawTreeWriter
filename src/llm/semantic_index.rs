use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::llm::relational_index::FileGraph;

/// Vector BLOB codec: little-endian f32 sequence (768 dims = 3 KB/row).
fn vector_to_blob(vector: &[f32]) -> Vec<u8> {
    vector.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn blob_to_vector(blob: &[u8]) -> Vec<f32> {
    blob.as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEmbedding {
    pub file_path: String,
    pub node_path: String,
    pub content_preview: String,
    pub vector: Vec<f32>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SemanticIndex {
    pub entries: Vec<NodeEmbedding>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub model_name: String,
    pub dimension: usize,
    pub created_at: DateTime<Utc>,
}

pub struct SemanticIndexManager {
    storage_dir: PathBuf,
}

impl SemanticIndexManager {
    pub fn new(project_root: &Path) -> Self {
        let storage_dir = project_root.join(".gnawtreewriter_ai").join("index");
        if !storage_dir.exists() {
            let _ = fs::create_dir_all(&storage_dir);
        }
        Self { storage_dir }
    }

    pub fn get_storage_dir(&self) -> &Path {
        &self.storage_dir
    }

    /// Open (and initialize) the embeddings DB. One file
    /// (`embeddings.db`, WAL mode) replaces the per-file JSON shards.
    fn open_db(&self) -> Result<Connection> {
        let conn = Connection::open(self.storage_dir.join("embeddings.db"))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS model_info (
                 id INTEGER PRIMARY KEY CHECK (id = 1),
                 model_name TEXT NOT NULL,
                 dimension INTEGER NOT NULL,
                 created_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS embeddings (
                 file_hash TEXT NOT NULL,
                 file_path TEXT NOT NULL,
                 node_path TEXT NOT NULL,
                 content_preview TEXT NOT NULL,
                 vector BLOB NOT NULL,
                 PRIMARY KEY (file_path, node_path)
             );
             CREATE INDEX IF NOT EXISTS idx_embeddings_hash ON embeddings(file_hash);",
        )?;
        Ok(conn)
    }

    pub fn save_model_info(&self, model_name: &str, dimension: usize) -> Result<()> {
        let conn = self.open_db()?;
        conn.execute(
            "INSERT INTO model_info (id, model_name, dimension, created_at)
             VALUES (1, ?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET model_name = ?1, dimension = ?2, created_at = ?3",
            rusqlite::params![model_name, dimension as i64, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn get_model_info(&self) -> Result<Option<ModelInfo>> {
        let conn = self.open_db()?;
        let mut stmt =
            conn.prepare("SELECT model_name, dimension, created_at FROM model_info WHERE id = 1")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(ModelInfo {
                model_name: row.get(0)?,
                dimension: row.get::<_, i64>(1)? as usize,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)?
                    .with_timezone(&Utc),
            }));
        }
        // Legacy fallback: pre-SQLite projects keep model_info.json.
        let load_path = self.storage_dir.join("model_info.json");
        if load_path.exists() {
            let info: ModelInfo = serde_json::from_str(&fs::read_to_string(load_path)?)?;
            return Ok(Some(info));
        }
        Ok(None)
    }

    pub fn save_index(&self, file_path: &str, entries: Vec<NodeEmbedding>) -> Result<()> {
        let file_hash = crate::core::transaction_log::calculate_content_hash(file_path);
        let mut conn = self.open_db()?;
        // Upsert semantics: a re-indexed file REPLACES all its previous
        // rows. The per-file JSON storage kept stale rows forever whenever
        // a file's node set changed (the orphan defect).
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM embeddings WHERE file_path = ?1", [file_path])?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO embeddings
                     (file_hash, file_path, node_path, content_preview, vector)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for e in &entries {
                stmt.execute(rusqlite::params![
                    file_hash,
                    e.file_path,
                    e.node_path,
                    e.content_preview,
                    vector_to_blob(&e.vector)
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn load_project_index(&self) -> Result<SemanticIndex> {
        if !self.storage_dir.exists() {
            return Ok(SemanticIndex::default());
        }
        let mut conn = self.open_db()?;
        Self::migrate_legacy_json(&mut conn, &self.storage_dir)?;
        let mut index = SemanticIndex::default();
        let mut stmt =
            conn.prepare("SELECT file_path, node_path, content_preview, vector FROM embeddings")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let vector: Vec<u8> = row.get(3)?;
            index.entries.push(NodeEmbedding {
                file_path: row.get(0)?,
                node_path: row.get(1)?,
                content_preview: row.get(2)?,
                vector: blob_to_vector(&vector),
            });
        }
        Ok(index)
    }

    /// One-time import of the legacy per-file JSON storage
    /// (`<file_hash>.json` + `model_info.json`): rows move into the
    /// embeddings DB, source files are renamed `*.migrated` so the
    /// migration is idempotent and never imports twice.
    fn migrate_legacy_json(conn: &mut Connection, storage_dir: &Path) -> Result<()> {
        let legacy: Vec<(PathBuf, Vec<NodeEmbedding>)> = fs::read_dir(storage_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension().and_then(|s| s.to_str()) == Some("json")
                    && p.file_name().and_then(|s| s.to_str()) != Some("model_info.json")
            })
            .filter_map(|p| {
                let data = fs::read_to_string(&p).ok()?;
                let entries: Vec<NodeEmbedding> = serde_json::from_str(&data).ok()?;
                Some((p, entries))
            })
            .collect();
        let legacy_info = storage_dir.join("model_info.json");
        if legacy.is_empty() && !legacy_info.exists() {
            return Ok(());
        }
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO embeddings
                     (file_hash, file_path, node_path, content_preview, vector)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for (path, entries) in &legacy {
                let hash = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                for e in entries {
                    stmt.execute(rusqlite::params![
                        hash,
                        e.file_path,
                        e.node_path,
                        e.content_preview,
                        vector_to_blob(&e.vector)
                    ])?;
                }
            }
        }
        let has_model: i64 = tx.query_row("SELECT COUNT(*) FROM model_info", [], |r| r.get(0))?;
        if has_model == 0 && legacy_info.exists() {
            if let Ok(info) = serde_json::from_str::<ModelInfo>(&fs::read_to_string(&legacy_info)?)
            {
                tx.execute(
                    "INSERT OR REPLACE INTO model_info (id, model_name, dimension, created_at)
                     VALUES (1, ?1, ?2, ?3)",
                    rusqlite::params![
                        info.model_name,
                        info.dimension as i64,
                        info.created_at.to_rfc3339()
                    ],
                )?;
            }
        }
        tx.commit()?;
        for (path, _) in &legacy {
            let mut renamed = path.clone().into_os_string();
            renamed.push(".migrated");
            let _ = fs::rename(path, renamed);
        }
        if legacy_info.exists() {
            let mut renamed = legacy_info.clone().into_os_string();
            renamed.push(".migrated");
            let _ = fs::rename(&legacy_info, renamed);
        }
        Ok(())
    }
}

impl SemanticIndex {
    /// Search for entries most similar to query_vector.
    /// Results with cosine similarity below 0.2 are filtered out.
    pub fn search(&self, query_vector: &[f32], limit: usize) -> Vec<(&NodeEmbedding, f32)> {
        self.search_with_threshold(query_vector, limit, 0.2)
    }

    /// Search with explicit minimum score threshold.
    pub fn search_with_threshold(
        &self,
        query_vector: &[f32],
        limit: usize,
        min_score: f32,
    ) -> Vec<(&NodeEmbedding, f32)> {
        let mut results: Vec<(&NodeEmbedding, f32)> = self
            .entries
            .iter()
            .map(|entry| {
                let score = cosine_similarity(query_vector, &entry.vector);
                (entry, score)
            })
            .filter(|(_, score)| *score >= min_score)
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

/// Multiplicative kind priors for satellite rerank (see
/// rerank_satellite): declaration heads demoted, implementation heads
/// nudged — pure cosine alone ranks `pub mod x;` above `impl`s for
/// name-heavy queries: short decls are lexically tight (their names
/// ARE the query terms) while real implementations are long and
/// diffuse. Declarations are pointers; implementations are answers.
/// Applied MULTIPLICATIVELY on the fused RRF score so the adjustment
/// scales with the entry's own standing — absolute additions let a
/// lexically perfect tail entry leapfrog the whole ranking (measured:
/// pipeline@100 72% vs raw@100 90% before this change).
const DECL_PENALTY: f32 = 0.15;
const IMPL_BONUS: f32 = 0.03;
/// Reciprocal-rank-fusion constants: RRF_K is the standard smoothing
/// constant; the lexical channel is weighted at half the cosine channel.
const RRF_K: f32 = 60.0;
const LEX_CHANNEL_WEIGHT: f32 = 0.5;
/// Third channel: call-graph proximity. Weight per rrf unit — a rank-0
/// graph neighbor adds GRAPH_CHANNEL_WEIGHT/(RRF_K) ≈ 0.0033, enough to
/// climb ~5 cosine positions. Eval-calibrated: 0.4 hurt head recall
/// (hub adjacency promoted generic files), 3.0 destroyed the ranking
/// outright; 0.2 keeps the head intact and adds +2 recall@100.
const GRAPH_CHANNEL_WEIGHT: f32 = 0.2;
/// How many top files (two-channel order) seed the graph expansion.
const GRAPH_SEED_K: usize = 8;
/// Files with more resolved edges than this are hubs (core/mod.rs,
/// cli.rs): adjacency to them is topical noise, so they are excluded
/// both as seeds and as promoted neighbors.
const GRAPH_HUB_DEGREE: usize = 100;

#[derive(PartialEq, Clone, Copy, Debug)]
enum PreviewKind {
    Decl,
    ImplLike,
    Other,
}

fn classify_preview(preview: &str) -> PreviewKind {
    let first = preview
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    // Chunked entries prefix their head ("(Chunk 0) impl ...") — classify
    // on the real code after the marker, or everything lands in Other.
    let head = match first.find(')') {
        Some(pos) if first.starts_with("(Chunk") => first[pos + 1..].trim(),
        _ => first,
    };
    const DECLS: &[&str] = &[
        "pub mod",
        "mod ",
        "use ",
        "pub use",
        "pub(crate) use",
        "extern crate",
        "import ",
        "export ",
        "from ",
    ];
    const IMPLS: &[&str] = &[
        "fn ",
        "pub fn",
        "async fn",
        "pub async fn",
        "pub(crate) fn",
        "impl ",
        "impl<",
        "pub impl",
        "struct ",
        "pub struct",
        "enum ",
        "pub enum",
        "trait ",
        "pub trait",
        "macro_rules",
        "type ",
        "pub type",
        "const ",
        "pub const",
        "static ",
        "pub static",
        "def ",
        "class ",
        "function ",
    ];
    if head.starts_with("#[") || head.starts_with("#![") {
        // Attribute/test-attrs are metadata, not the thing being asked about.
        PreviewKind::Decl
    } else if DECLS.iter().any(|p| head.starts_with(p)) {
        PreviewKind::Decl
    } else if IMPLS.iter().any(|p| head.starts_with(p)) {
        PreviewKind::ImplLike
    } else {
        PreviewKind::Other
    }
}

/// A satellite hit after reranking: the entry plus BOTH scores — the raw
/// cosine (what the model actually measured) and the adjusted ranking
/// value (RRF fusion of cosine and lexical rank, scaled by the kind
/// prior). Consumers must never mistake the adjusted value for a
/// cosine: it lives on the reciprocal-rank scale (roughly 0.001–0.04
/// for realistic windows) and only orders hits WITHIN one search.
pub struct RerankedHit<'a> {
    pub entry: &'a NodeEmbedding,
    pub cosine: f32,
    pub adjusted: f32,
}

/// Index entries may be chunked (`8[chunk:2]`) — such paths do not exist
/// in the tree and would fail `read_node` (diagnostic-chain break).
/// Responses carry the PARENT node path; the chunk identity stays visible
/// in content_preview ("(Chunk 2) …").
pub fn parent_node_path(path: &str) -> &str {
    match path.find("[chunk:") {
        Some(idx) => &path[..idx],
        None => path,
    }
}

/// Light symmetric normalization for lexical matching: plural 's' trimmed
/// on BOTH sides of the comparison ("modules" == "module") as long as at
/// least 3 chars remain — symmetric, so it can never create a mismatch.
fn lex_norm(word: &str) -> &str {
    let trimmed = word.trim_end_matches('s');
    if word.len() > 3 && trimmed.len() >= 3 {
        trimmed
    } else {
        word
    }
}

/// One file must not crowd out the whole top list: an agent asking
/// "where does X live" wants breadth, not 10 nodes of cli.rs.
const MAX_PER_FILE: usize = 3;

/// Fuse two satellite searches over the SAME index by taking, per entry,
/// the higher cosine (dedup on file_path+node_path). Used for query
/// expansion: the natural-language query and its expanded form each run
/// their own search; the fused set feeds rerank_satellite. Order is not
/// meaningful — rerank sorts anyway.
pub fn fuse_by_max<'a>(
    a: Vec<(&'a NodeEmbedding, f32)>,
    b: Vec<(&'a NodeEmbedding, f32)>,
) -> Vec<(&'a NodeEmbedding, f32)> {
    let mut best: std::collections::HashMap<(&'a str, &'a str), (&'a NodeEmbedding, f32)> =
        std::collections::HashMap::new();
    for (entry, score) in a.into_iter().chain(b) {
        let key = (entry.file_path.as_str(), entry.node_path.as_str());
        match best.get(&key) {
            Some((_, existing)) if *existing >= score => {}
            _ => {
                best.insert(key, (entry, score));
            }
        }
    }
    best.into_values().collect()
}

/// Rerank satellite hits by RECIPROCAL-RANK FUSION of channels — cosine
/// and lexical overlap (optionally graph proximity, see the
/// graph-channel variant) — instead of absolute score additions:
///
///   fused = rrf(cos_rank) + LEX_CHANNEL_WEIGHT * rrf(lex_rank)
///   adjusted = fused * (1 + kind_prior)
///
/// with rrf(r) = 1/(RRF_K + r). Every channel contributes a BOUNDED
/// term, so a lexically perfect tail entry can climb a few places but
/// can no longer leapfrog the whole ranking. Equal cosines tie-break
/// by lexical overlap (substring noise must not claim the better
/// rank); remaining ties keep the raw order. The kind prior is
/// multiplicative — it scales with the entry's own standing. In
/// inventory mode ("which modules/files exist …") declarations ARE
/// the answer and get the boost instead of the penalty.
///
/// Fetch a WIDER raw window than you serve (e.g. search(.., 2000) then
/// rerank(.., top=10)) so lexical channel candidates exist at all.
pub fn rerank_satellite<'a>(
    query: &str,
    hits: Vec<(&'a NodeEmbedding, f32)>,
    top: usize,
) -> Vec<RerankedHit<'a>> {
    rerank_satellite_with_graphs(query, hits, top, None)
}

/// Canonical path key: graph JSONs store ABSOLUTE paths while the
/// semantic index stores project-relative ones — compare both from the
/// first "src/" segment so the two spellings collide correctly.
fn graph_path_key(p: &str) -> &str {
    match p.find("src/") {
        Some(i) => &p[i..],
        None => p,
    }
}

/// As [rerank_satellite], plus an OPTIONAL third channel: call-graph
/// proximity. Seeds = files of the best two-channel entries; a seed's
/// callers and callees get a bounded GRAPH_CHANNEL_WEIGHT-weighted rrf
/// term, best-seed-first. Structs co-designed with a strong hit rise
/// even when their own text never mentions the query — exactly the
/// model's blind spot. Pass None for the plain two-channel ranking.
pub fn rerank_satellite_with_graphs<'a>(
    query: &str,
    hits: Vec<(&'a NodeEmbedding, f32)>,
    top: usize,
    graphs: Option<&[FileGraph]>,
) -> Vec<RerankedHit<'a>> {
    let q_tokens: Vec<String> = query
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3)
        .map(|t| lex_norm(t).to_string())
        .collect();

    let q_lower = query.to_lowercase();
    let inventory_intent = [
        "module", "modules", "import", "imports", "file", "files", "symbol", "symbols", "export",
        "exports", "class", "classes",
    ]
    .iter()
    .any(|w| q_lower.contains(w));

    // Lexical overlap per entry (word-boundary rules unchanged).
    let mut entries: Vec<(&NodeEmbedding, f32, f32, PreviewKind)> = hits
        .into_iter()
        .map(|(entry, cosine)| {
            let haystack = format!(
                "{} {} {}",
                entry.content_preview.to_lowercase(),
                entry.file_path.to_lowercase(),
                entry.node_path.to_lowercase()
            );
            let words: std::collections::HashSet<String> = haystack
                .split(|c: char| !c.is_alphanumeric())
                .filter(|w| !w.is_empty())
                .map(|w| lex_norm(w).to_string())
                .collect();
            let matched = q_tokens
                .iter()
                .filter(|t| {
                    words.contains(t.as_str())
                        || (t.chars().count() >= 4 && words.iter().any(|w| w.contains(t.as_str())))
                })
                .count();
            let lex = if q_tokens.is_empty() {
                0.0
            } else {
                matched as f32 / q_tokens.len() as f32
            };
            let kind = classify_preview(&entry.content_preview);
            (entry, cosine, lex, kind)
        })
        .collect();

    // Primary channel: cosine rank. Equal cosines break by lexical
    // overlap so noise cannot claim the better rank; rest is stable.
    entries.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal))
    });

    // Secondary channel: lexical rank — all entries with ANY overlap
    // compete, but the channel contribution is CONTINUOUS
    // (LEX_CHANNEL_WEIGHT * lex * rrf(rank)): a 1/3 overlap gets a
    // third of a full match's boost, so weak noise cannot buy the
    // climb a perfect identifier match earns (binary membership made
    // the channel outrank cosine positions 100+ deep — the tail leak).
    let mut lex_order: Vec<usize> = (0..entries.len()).filter(|&i| entries[i].2 > 0.0).collect();
    lex_order.sort_by(|&a, &b| {
        entries[b]
            .2
            .partial_cmp(&entries[a].2)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut lex_rank = vec![usize::MAX; entries.len()];
    for (place, &i) in lex_order.iter().enumerate() {
        lex_rank[i] = place;
    }

    let rrf = |rank: usize| 1.0 / (RRF_K + rank as f32);

    let mut scored: Vec<(&NodeEmbedding, f32, f32)> = entries
        .iter()
        .enumerate()
        .map(|(i, (entry, cosine, _lex, kind))| {
            let mut fused = rrf(i)
                + if lex_rank[i] == usize::MAX {
                    0.0
                } else {
                    LEX_CHANNEL_WEIGHT * entries[i].2 * rrf(lex_rank[i])
                };
            let prior = match kind {
                PreviewKind::Decl => {
                    if inventory_intent {
                        2.0 * IMPL_BONUS
                    } else {
                        -DECL_PENALTY
                    }
                }
                PreviewKind::ImplLike => {
                    if inventory_intent {
                        0.0
                    } else {
                        IMPL_BONUS
                    }
                }
                PreviewKind::Other => 0.0,
            };
            fused *= 1.0 + prior;
            (*entry, *cosine, fused)
        })
        .collect();

    scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    // Third channel (optional): graph proximity over the two-channel
    // order. Top GRAPH_SEED_K distinct files seed a BFS over the call
    // graph — out-edges (seed calls target) and in-edges (target calls
    // seed) — and each neighbor gets rrf(its best seed rank) weighted
    // by GRAPH_CHANNEL_WEIGHT, then the fused order re-sorts. The
    // channel is deliberately silent when graphs are absent or carry
    // no edges for the seeds: two-channel behavior is the floor.
    if let Some(graphs) = graphs {
        let seed_files: Vec<String> = {
            let mut seen = std::collections::HashSet::new();
            scored
                .iter()
                .filter(|(e, _, _)| seen.insert(e.file_path.clone()))
                .take(GRAPH_SEED_K)
                .map(|(e, _, _)| e.file_path.clone())
                .collect()
        };
        if !seed_files.is_empty() {
            let seeds: std::collections::HashSet<&str> =
                seed_files.iter().map(|s| s.as_str()).collect();
            // Hub exclusion: files wired to everything (core/mod.rs,
            // cli.rs) make adjacency meaningless — a call to them says
            // nothing about topical kinship. Skip them as seeds AND as
            // neighbors; degree = resolved in+out edges over the graph.
            let mut degree: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for graph in graphs {
                let fk = graph_path_key(&graph.file_path).to_string();
                *degree.entry(fk).or_insert(0) += graph.relations.len();
                for rel in &graph.relations {
                    if let Some(t) = &rel.to_file {
                        *degree.entry(graph_path_key(t).to_string()).or_insert(0) += 1;
                    }
                }
            }
            // neighbor file -> best (lowest) seed order adjacent to it
            let mut neighbor_rank: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            let bump =
                |file: &str, order: usize, map: &mut std::collections::HashMap<String, usize>| {
                    map.entry(file.to_string())
                        .and_modify(|r| *r = (*r).min(order))
                        .or_insert(order);
                };
            for (order, seed) in seed_files.iter().enumerate() {
                for graph in graphs {
                    let gk = graph_path_key(&graph.file_path);
                    if degree.get(gk).copied().unwrap_or(0) > GRAPH_HUB_DEGREE {
                        continue;
                    }
                    if gk == seed.as_str() {
                        for rel in &graph.relations {
                            if let Some(target) = &rel.to_file {
                                let key = graph_path_key(target).to_string();
                                if !seeds.contains(key.as_str())
                                    && degree.get(key.as_str()).copied().unwrap_or(0)
                                        <= GRAPH_HUB_DEGREE
                                {
                                    bump(&key, order, &mut neighbor_rank);
                                }
                            }
                        }
                    } else if !seeds.contains(gk)
                        && graph.relations.iter().any(|rel| {
                            rel.to_file.as_deref().map(graph_path_key) == Some(seed.as_str())
                        })
                    {
                        bump(gk, order, &mut neighbor_rank);
                    }
                }
            }
            if !neighbor_rank.is_empty() {
                for (entry, _, fused) in scored.iter_mut() {
                    if let Some(&g_rank) = neighbor_rank.get(&entry.file_path) {
                        *fused += GRAPH_CHANNEL_WEIGHT * rrf(g_rank);
                    }
                }
                scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
            }
        }
    }

    // Per-file diversification: greedy over the fused ranking.
    let mut per_file: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut out: Vec<RerankedHit<'a>> = Vec::with_capacity(top);
    for (entry, cosine, fused) in scored {
        let count = per_file.entry(entry.file_path.as_str()).or_insert(0);
        if *count >= MAX_PER_FILE {
            continue;
        }
        *count += 1;
        out.push(RerankedHit {
            entry,
            cosine,
            adjusted: fused,
        });
        if out.len() >= top {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, preview: &str) -> NodeEmbedding {
        NodeEmbedding {
            file_path: "src/x.rs".to_string(),
            node_path: path.to_string(),
            content_preview: preview.to_string(),
            vector: vec![0.0],
        }
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![0.0, 1.0];
        assert!(cosine_similarity(&a, &c).abs() < 1e-6);
    }

    #[test]
    fn rerank_demotes_module_decls_below_impls() {
        // Raw cosine ties them — declarations must lose for a name-heavy query.
        let e1 = entry("1", "pub mod undo_redo;");
        let e2 = entry("2", "impl UndoRedoManager {");
        let e3 = entry("3", "use crate::llm::Source;");
        let hits = vec![(&e1, 0.90), (&e2, 0.89), (&e3, 0.88)];
        let out = rerank_satellite("how is undo implemented", hits, 10);
        assert_eq!(out[0].entry.node_path, "2", "impl must outrank mod decl");
        assert_eq!(out[1].entry.node_path, "1");
        assert_eq!(out[2].entry.node_path, "3");
    }

    #[test]
    fn rerank_rewards_query_terms() {
        let e1 = entry("1", "fn unrelated_thing() {}");
        let e2 = entry("2", "fn undo_transaction() {}");
        let hits = vec![(&e1, 0.90), (&e2, 0.88)];
        let out = rerank_satellite("undo the transaction", hits, 10);
        assert_eq!(out[0].entry.node_path, "2", "lexical overlap must win");
    }

    #[test]
    fn rerank_respects_top_limit() {
        let e1 = entry("1", "fn a() {}");
        let e2 = entry("2", "fn b() {}");
        let e3 = entry("3", "fn c() {}");
        let hits = vec![(&e1, 0.5), (&e2, 0.4), (&e3, 0.3)];
        let out = rerank_satellite("anything", hits, 2);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn rerank_empty_query_keeps_cosine_order() {
        let e1 = entry("1", "fn a() {}");
        let e2 = entry("2", "pub mod m;");
        let hits = vec![(&e1, 0.5), (&e2, 0.4)];
        let out = rerank_satellite("", hits, 10);
        assert_eq!(out[0].entry.node_path, "1");
    }

    #[test]
    fn rerank_inventory_query_keeps_decls_competitive() {
        // "Which modules …" is answered by declarations: soft penalty must
        // not let an unrelated impl/struct crowd out the mod entries.
        let e1 = entry("1", "pub struct BlueprintEngine {");
        let e2 = entry("2", "pub mod transaction_log;");
        let hits = vec![(&e1, 0.80), (&e2, 0.80)];
        let out = rerank_satellite("which modules exist in this project", hits, 10);
        assert_eq!(
            out[0].entry.node_path, "2",
            "decl must win an inventory query at equal cosine"
        );
    }

    #[test]
    fn classify_kinds() {
        assert_eq!(classify_preview("pub mod undo_redo;"), PreviewKind::Decl);
        assert_eq!(classify_preview("  use std::io;"), PreviewKind::Decl);
        assert_eq!(
            classify_preview("impl UndoRedoManager {"),
            PreviewKind::ImplLike
        );
        assert_eq!(
            classify_preview("pub async fn load() {}"),
            PreviewKind::ImplLike
        );
        assert_eq!(classify_preview("let x = 1;"), PreviewKind::Other);
        // Chunk markers must not hide the real head.
        assert_eq!(
            classify_preview("(Chunk 0) impl UndoRedoManager {"),
            PreviewKind::ImplLike
        );
        assert_eq!(
            classify_preview("(Chunk 3) pub mod whatever;"),
            PreviewKind::Decl
        );
    }

    #[test]
    fn parent_node_path_strips_chunk_marker() {
        // Critical: chunk paths do not exist in the tree — read_node on
        // them would fail (diagnostic-chain break).
        assert_eq!(parent_node_path("8[chunk:2]"), "8");
        assert_eq!(parent_node_path("14.2.11[chunk:0]"), "14.2.11");
        assert_eq!(parent_node_path("41"), "41", "plain paths unchanged");
        assert_eq!(parent_node_path("1.2"), "1.2");
    }

    #[test]
    fn rerank_diversifies_across_files() {
        // 5 high hits from file A must not crowd out file B entirely.
        let mk = |path: &str, file: &str, preview: &str| NodeEmbedding {
            file_path: file.to_string(),
            node_path: path.to_string(),
            content_preview: preview.to_string(),
            vector: vec![0.0],
        };
        let a: Vec<NodeEmbedding> = (0..5)
            .map(|i| mk(&format!("a{i}"), "src/a.rs", "fn alpha() {}"))
            .collect();
        let b = vec![
            mk("b0", "src/b.rs", "fn beta() {}"),
            mk("b1", "src/b.rs", "fn gamma() {}"),
        ];
        let mut hits: Vec<(&NodeEmbedding, f32)> = a.iter().map(|e| (e, 0.9)).collect();
        hits.extend(b.iter().map(|e| (e, 0.5)));
        let out = rerank_satellite("anything specific", hits, 10);
        let files: std::collections::HashSet<&str> =
            out.iter().map(|h| h.entry.file_path.as_str()).collect();
        assert!(
            files.contains("src/b.rs"),
            "file B must survive the per-file cap: {:?}",
            files
        );
        let a_count = out
            .iter()
            .filter(|h| h.entry.file_path == "src/a.rs")
            .count();
        assert!(a_count <= 3, "file A capped at 3, got {a_count}");
    }

    #[test]
    fn graph_channel_promotes_call_neighbor() {
        use crate::llm::relational_index::{Relation, RelationType};
        // 11 files: f00..f08 (cosine ranks 0-8), nb (9), tgt (10).
        // tgt calls f00. GRAPH_SEED_K = 8: seeds = f00..f07, so tgt is
        // NOT a seed and earns the graph term — overtaking nb and f08
        // despite the lowest cosine.
        let mk = |file: &str, path: &str| NodeEmbedding {
            file_path: file.to_string(),
            node_path: path.to_string(),
            content_preview: "fn generic_body() {}".to_string(),
            vector: vec![0.0],
        };
        let entries: Vec<NodeEmbedding> = (0..9)
            .map(|i| mk(&format!("f0{i}.rs"), &format!("{i}")))
            .chain(std::iter::once(mk("nb.rs", "9")))
            .chain(std::iter::once(mk("tgt.rs", "10")))
            .collect();
        let hits: Vec<(&NodeEmbedding, f32)> = entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e, 0.60 - 0.001 * i as f32))
            .collect();
        let graphs = vec![FileGraph {
            file_path: "tgt.rs".to_string(),
            relations: std::iter::once(Relation {
                from_file: "tgt.rs".to_string(),
                from_path: "0".to_string(),
                to_file: Some("f00.rs".to_string()),
                to_name: "g".to_string(),
                relation_type: RelationType::Call,
            })
            .collect(),
            definitions: Default::default(),
            imports: Default::default(),
        }];
        let out = rerank_satellite_with_graphs("anything specific", hits, 11, Some(&graphs));
        // The neighbor overtakes the ENTIRE seed block - that is the
        // channel contract (silent co-designed code rises to the top
        // with its anchor). Whether overtaking even the top seed is
        // the right strength is a weight question for the eval.
        assert_eq!(
            out[0].entry.file_path, "tgt.rs",
            "call neighbor of the top seed outranks every seed"
        );
        assert_eq!(out[1].entry.file_path, "f00.rs", "seeds keep their order");
        assert_eq!(
            out[10].entry.file_path, "nb.rs",
            "unrelated files sink to the tail"
        );
    }

    #[test]
    fn graph_channel_none_or_empty_matches_plain_rerank() {
        let a = entry("1", "impl UndoRedoManager {");
        let b = entry("2", "fn other() {}");
        let hits: Vec<(&NodeEmbedding, f32)> = vec![(&a, 0.9), (&b, 0.4)];
        let plain = rerank_satellite("undo the thing", hits.clone(), 10);
        let none = rerank_satellite_with_graphs("undo the thing", hits.clone(), 10, None);
        let empty = rerank_satellite_with_graphs("undo the thing", hits, 10, Some(&[]));
        for ((p, n), e) in plain.iter().zip(&none).zip(&empty) {
            assert_eq!(p.entry.node_path, n.entry.node_path);
            assert_eq!(p.entry.node_path, e.entry.node_path);
            assert!((p.adjusted - n.adjusted).abs() < 1e-9);
            assert!((p.adjusted - e.adjusted).abs() < 1e-9);
        }
    }

    #[test]
    fn rerank_lexical_uses_word_boundaries() {
        // "log" must not match "catalog" (substring-before); CamelCase
        // still matches via >=4-char in-word check.
        let e1 = entry("1", "fn handle_catalog() {}");
        let e2 = entry("2", "fn transaction_log_writer() {}");
        let hits = vec![(&e1, 0.7), (&e2, 0.7)];
        let out = rerank_satellite("transaction log", hits, 10);
        assert_eq!(
            out[0].entry.node_path, "2",
            "word match must win over substring noise"
        );

        // RRF scale: the lexical match shows up as a fused-rank WIN over
        // an equal-cosine entry with no overlap — not as adjusted > cosine
        // (adjusted is a reciprocal-rank value, a different unit).
        let c1 = entry("1", "impl UndoRedoManager {");
        let c2 = entry("2", "fn completely_different_thing() {}");
        let hits = vec![(&c1, 0.7), (&c2, 0.7)];
        let out = rerank_satellite("how is undo handled", hits, 10);
        assert_eq!(
            out[0].entry.node_path, "1",
            "CamelCase identifier still lex-matches (wins the fused rank)"
        );
    }

    #[test]
    fn rerank_exposes_raw_cosine_and_fused_rank() {
        let e_impl = entry("1", "impl UndoRedoManager {");
        let e_decl = entry("2", "pub mod unrelated_thing;");
        // Equal cosines: the impl head with lexical overlap must win the
        // fused ranking; the raw cosine stays verbatim on the hit.
        let hits = vec![(&e_impl, 0.80), (&e_decl, 0.80)];
        let out = rerank_satellite("undo the thing", hits, 10);
        assert_eq!(out[0].cosine, 0.80, "raw cosine preserved verbatim");
        assert_eq!(
            out[0].entry.node_path, "1",
            "impl+lex beats a decl at equal cosine"
        );
        assert!(
            out[0].adjusted > 0.0 && out[0].adjusted < 1.0,
            "adjusted is an RRF-scale ranking value, not a cosine"
        );
    }

    #[test]
    fn fuse_by_max_dedups_and_keeps_best_score() {
        let e1 = entry("1", "fn alpha() {}");
        let e2 = entry("2", "fn beta() {}");
        // Same entry from both channels (query + expansion): max wins.
        let fused = fuse_by_max(vec![(&e1, 0.60)], vec![(&e1, 0.85), (&e2, 0.40)]);
        assert_eq!(fused.len(), 2, "deduped on file+node");
        let scores: std::collections::HashMap<&str, f32> = fused
            .iter()
            .map(|(e, s)| (e.node_path.as_str(), *s))
            .collect();
        assert_eq!(scores["1"], 0.85, "higher cosine kept");
        assert_eq!(scores["2"], 0.40, "entry only in second channel kept");
    }

    #[test]
    fn fuse_by_max_empty_channels() {
        let e1 = entry("1", "fn alpha() {}");
        assert!(fuse_by_max(vec![], vec![]).is_empty());
        assert_eq!(fuse_by_max(vec![(&e1, 0.5)], vec![]).len(), 1);
    }

    // ---- SQLite storage ----

    fn sqlite_entry(file: &str, node: &str, vector: Vec<f32>) -> NodeEmbedding {
        NodeEmbedding {
            file_path: file.to_string(),
            node_path: node.to_string(),
            content_preview: format!("fn node_{node}() {{}}"),
            vector,
        }
    }

    #[test]
    fn sqlite_roundtrip_preserves_entries() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SemanticIndexManager::new(dir.path());
        let v: Vec<f32> = (0..768).map(|i| i as f32 * 0.001).collect();
        mgr.save_index(
            "src/a.rs",
            vec![
                sqlite_entry("src/a.rs", "1", v.clone()),
                sqlite_entry("src/a.rs", "2", vec![0.5]),
            ],
        )
        .unwrap();
        let loaded = mgr.load_project_index().unwrap();
        assert_eq!(loaded.entries.len(), 2);
        let e = loaded
            .entries
            .iter()
            .find(|e| e.node_path == "1")
            .expect("node 1 present");
        assert_eq!(e.file_path, "src/a.rs");
        assert_eq!(e.vector, v, "f32 bytes must round-trip exactly");
    }

    #[test]
    fn sqlite_upsert_deletes_stale_rows() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SemanticIndexManager::new(dir.path());
        mgr.save_index(
            "src/a.rs",
            vec![
                sqlite_entry("src/a.rs", "1", vec![0.1]),
                sqlite_entry("src/a.rs", "2", vec![0.2]),
            ],
        )
        .unwrap();
        // File re-indexed with a DIFFERENT node set: old node 1 must die.
        mgr.save_index(
            "src/a.rs",
            vec![
                sqlite_entry("src/a.rs", "2", vec![0.2]),
                sqlite_entry("src/a.rs", "3", vec![0.3]),
            ],
        )
        .unwrap();
        let loaded = mgr.load_project_index().unwrap();
        let nodes: Vec<&str> = loaded
            .entries
            .iter()
            .map(|e| e.node_path.as_str())
            .collect();
        assert_eq!(nodes, vec!["2", "3"], "no orphans from the old node set");
    }

    #[test]
    fn sqlite_migrates_legacy_json_idempotently() {
        let dir = tempfile::tempdir().unwrap();
        let storage = dir.path().join(".gnawtreewriter_ai").join("index");
        fs::create_dir_all(&storage).unwrap();
        let entries = vec![sqlite_entry("src/old.rs", "7", vec![0.25, 0.75])];
        fs::write(
            storage.join("abc123.json"),
            serde_json::to_string(&entries).unwrap(),
        )
        .unwrap();
        fs::write(
            storage.join("model_info.json"),
            serde_json::to_string(&ModelInfo {
                model_name: "bge-base-en-v1.5".into(),
                dimension: 768,
                created_at: Utc::now(),
            })
            .unwrap(),
        )
        .unwrap();

        let mgr = SemanticIndexManager::new(dir.path());
        let loaded = mgr.load_project_index().unwrap();
        assert_eq!(loaded.entries.len(), 1, "legacy rows imported");
        assert_eq!(loaded.entries[0].vector, vec![0.25, 0.75]);
        let info = mgr.get_model_info().unwrap().expect("model info imported");
        assert_eq!(info.model_name, "bge-base-en-v1.5");
        // Sources renamed, not deleted — and never re-imported.
        assert!(!storage.join("abc123.json").exists());
        assert!(storage.join("abc123.json.migrated").exists());
        let again = mgr.load_project_index().unwrap();
        assert_eq!(again.entries.len(), 1, "migration is idempotent");
    }

    #[test]
    fn sqlite_model_info_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SemanticIndexManager::new(dir.path());
        assert!(mgr.get_model_info().unwrap().is_none(), "fresh = none");
        mgr.save_model_info("bge-base-en-v1.5", 768).unwrap();
        mgr.save_model_info("bge-base-en-v1.5", 768).unwrap(); // upsert ok
        let info = mgr.get_model_info().unwrap().expect("saved");
        assert_eq!(info.model_name, "bge-base-en-v1.5");
        assert_eq!(info.dimension, 768);
    }
}
