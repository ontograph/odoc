//! Retrieval systems over a loaded generation.
//!
//! - B0 ([`baseline`]): prior-attempt substring baseline.
//! - B1 ([`Mode::Lexical`]): BM25 over each chunk's heading path plus text.
//! - B2 ([`Mode::Graph`]): B1 fused with PPR over `LINKS_TO` + `NEXT`.
//! - B3 ([`Mode::Hier`]): B1 fused with BM25 of the chunk's document
//!   pseudo-document.
//!
//! Pipeline: optional path-prefix scope zeroes out-of-scope BM25 scores; B1
//! ranks positive scores (ties by chunk id); abstention is decided on raw BM25
//! before fusion, `s = top_bm25 / Σ idf(unique query terms)`. B2 and B3 seed
//! PPR with the top [`ppr::SEEDS`] B1 chunks (teleport ∝ BM25) and fuse over
//! the pool `B1 top max(50, top_k) ∪ in-scope PPR top 50`.

pub mod baseline;
pub mod bm25;
pub mod fusion;
pub mod ppr;
pub mod tokenize;

use std::collections::HashMap;

use serde::Serialize;

use crate::store::{ChunkRecord, Generation, StoreError};

use bm25::Bm25;
use ppr::ChunkGraph;
use tokenize::{query_terms, tokenize};

/// Candidates taken from each of B1 and PPR.
pub const CANDIDATES: usize = 50;

const MAX_TOKENIZER_INPUT_BYTES: usize = 512 * 1024 * 1024;

// Chunk text selects the first document with a path and hierarchy scoring the
// last. The two differ only when documents share a path: Generation::validate
// refuses that for every stored generation, but SearchIndex::new and try_new
// also accept unvalidated input. Keeps both indices without cloning paths.
type DocumentMap<'a> = HashMap<&'a str, (usize, usize)>;

fn document_map(generation: &Generation) -> DocumentMap<'_> {
    let mut documents = HashMap::new();
    for (i, document) in generation.documents.iter().enumerate() {
        documents
            .entry(document.path.as_str())
            .and_modify(|entry: &mut (usize, usize)| entry.1 = i)
            .or_insert((i, i));
    }
    documents
}

fn chunk_text<'a>(
    generation: &'a Generation,
    documents: &DocumentMap<'_>,
    chunk: &ChunkRecord,
) -> &'a str {
    documents
        .get(chunk.path.as_str())
        .and_then(|&(first, _)| {
            generation.documents[first]
                .text
                .get(chunk.span.byte_start..chunk.span.byte_end)
        })
        .unwrap_or("")
}

fn charge_input(total: &mut usize, bytes: usize, cap: usize) -> Result<(), StoreError> {
    *total = total
        .checked_add(bytes)
        .filter(|&sum| sum <= cap)
        .ok_or(StoreError::CapExceeded {
            what: "tokenizer input bytes",
            cap,
        })?;
    Ok(())
}

fn validate_input_with_limit(
    generation: &Generation,
    documents: &DocumentMap<'_>,
    cap: usize,
) -> Result<(), StoreError> {
    let mut total = 0;
    for chunk in &generation.chunks {
        for heading in &chunk.heading_path {
            charge_input(&mut total, heading.len(), cap)?;
        }
        charge_input(&mut total, chunk.heading_path.len().saturating_sub(1), cap)?;
        charge_input(
            &mut total,
            chunk_text(generation, documents, chunk).len(),
            cap,
        )?;
    }
    for document in &generation.documents {
        charge_input(&mut total, document.pseudo_doc.len(), cap)?;
    }
    Ok(())
}

/// Check actual tokenizer input before constructing indexes from decoded data.
pub(crate) fn validate_input(generation: &Generation) -> Result<(), StoreError> {
    validate_input_with_limit(
        generation,
        &document_map(generation),
        MAX_TOKENIZER_INPUT_BYTES,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AbstainReason {
    NoLexicalMatch,
    BelowThreshold,
    ScopeEmpty,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    /// B1.
    Lexical,
    /// B2 with PPR weight `alpha`.
    Graph { alpha: f64 },
    /// B3 with document weight `beta`.
    Hier { beta: f64 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Params {
    pub mode: Mode,
    pub top_k: usize,
    /// Path prefix; `None` searches everything.
    pub scope: Option<String>,
    /// Abstention threshold; `None` disables abstention (evaluation runs).
    pub threshold: Option<f64>,
    pub explain: bool,
}

impl Params {
    pub fn new(mode: Mode, top_k: usize) -> Self {
        Self {
            mode,
            top_k,
            scope: None,
            threshold: None,
            explain: false,
        }
    }
}

/// One ranked chunk; `chunk` indexes `Generation::chunks`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    pub chunk: usize,
    pub score: f64,
}

/// Score components of one hit (debugging only, no quality claim).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Components {
    pub bm25: f64,
    pub ppr: f64,
    pub doc_bm25: f64,
    /// 1-based B1 rank before fusion; `None` when outside the B1 pool.
    pub bm25_rank: Option<usize>,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ranked {
    pub hits: Vec<Hit>,
    pub abstained: Option<AbstainReason>,
    /// `top_bm25 / Σ idf`, or 0 when there is no lexical match.
    pub abstention_score: f64,
    /// Parallel to `hits` when explain was requested, else empty.
    pub explain: Vec<Components>,
}

impl Ranked {
    fn empty(abstained: Option<AbstainReason>, abstention_score: f64) -> Self {
        Self {
            hits: Vec::new(),
            abstained,
            abstention_score,
            explain: Vec::new(),
        }
    }
}

/// A generation with its in-memory indexes.
pub struct SearchIndex {
    pub generation: Generation,
    bm25: Bm25,
    doc_bm25: Bm25,
    /// Document index of each chunk.
    chunk_doc: Vec<usize>,
    graph: ChunkGraph,
}

impl SearchIndex {
    /// Trusted compatibility constructor without tokenizer-input admission.
    /// Service callers must use [`Self::try_new`] for bounded construction.
    pub fn new(generation: Generation) -> Self {
        let documents = document_map(&generation);
        let (bm25, doc_bm25, chunk_doc) = Self::indexes(&generation, &documents);
        Self::from_indexes(generation, bm25, doc_bm25, chunk_doc)
    }

    /// Refuse excessive aggregate tokenizer input before joins or BM25 work.
    pub fn try_new(generation: Generation) -> Result<Self, StoreError> {
        Self::with_limit(generation, MAX_TOKENIZER_INPUT_BYTES)
    }

    fn with_limit(generation: Generation, cap: usize) -> Result<Self, StoreError> {
        let documents = document_map(&generation);
        validate_input_with_limit(&generation, &documents, cap)?;
        let (bm25, doc_bm25, chunk_doc) = Self::indexes(&generation, &documents);
        Ok(Self::from_indexes(generation, bm25, doc_bm25, chunk_doc))
    }

    fn indexes(generation: &Generation, documents: &DocumentMap<'_>) -> (Bm25, Bm25, Vec<usize>) {
        let bm25 = Bm25::new(generation.chunks.iter().map(|c| {
            let mut tokens = tokenize(&c.heading_path.join("\n"));
            tokens.extend(tokenize(chunk_text(generation, documents, c)));
            tokens
        }));
        let doc_bm25 = Bm25::new(generation.documents.iter().map(|d| tokenize(&d.pseudo_doc)));
        let chunk_doc = generation
            .chunks
            .iter()
            .map(|c| {
                documents
                    .get(c.path.as_str())
                    .map_or(usize::MAX, |&(_, last)| last)
            })
            .collect();
        (bm25, doc_bm25, chunk_doc)
    }

    fn from_indexes(
        generation: Generation,
        bm25: Bm25,
        doc_bm25: Bm25,
        chunk_doc: Vec<usize>,
    ) -> Self {
        let chunk_of: HashMap<&str, usize> = generation
            .chunks
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id.as_str(), i))
            .collect();
        let graph = ChunkGraph::new(
            generation.chunks.len(),
            generation
                .edges
                .iter()
                .filter(|e| e.kind == "LINKS_TO" || e.kind == "NEXT")
                .filter_map(|e| {
                    Some((
                        *chunk_of.get(e.from.as_str())?,
                        *chunk_of.get(e.to.as_str())?,
                    ))
                }),
        );
        Self {
            generation,
            bm25,
            doc_bm25,
            chunk_doc,
            graph,
        }
    }

    pub fn bm25(&self) -> &Bm25 {
        &self.bm25
    }

    /// B0 baseline.
    pub fn b0(&self, query: &str, top_k: usize) -> Vec<Hit> {
        baseline::b0(&self.generation, query, top_k)
    }

    /// BM25 of each document's pseudo-document.
    pub fn doc_scores(&self, query: &str) -> Vec<f64> {
        self.doc_bm25.scores(&query_terms(query))
    }

    pub fn search(&self, query: &str, params: &Params) -> Ranked {
        let chunks = &self.generation.chunks;
        let in_scope: Vec<bool> = chunks
            .iter()
            .map(|c| {
                params
                    .scope
                    .as_deref()
                    .is_none_or(|s| c.path.starts_with(s))
            })
            .collect();
        if !in_scope.contains(&true) {
            return Ranked::empty(Some(AbstainReason::ScopeEmpty), 0.0);
        }
        let terms = query_terms(query);
        let mut bm25 = self.bm25.scores(&terms);
        for (score, keep) in bm25.iter_mut().zip(&in_scope) {
            if !keep {
                *score = 0.0;
            }
        }
        let lexical = rank(&bm25, chunks, params.top_k.max(CANDIDATES));
        let top = lexical.first().map_or(0.0, |h| h.score);
        let (abstention_score, abstained) = abstention(top, &terms, &self.bm25, params.threshold);
        if abstained.is_some() {
            return Ranked::empty(abstained, abstention_score);
        }

        let weight = match params.mode {
            Mode::Lexical => {
                let hits: Vec<Hit> = lexical.iter().take(params.top_k).copied().collect();
                let explain = if params.explain {
                    hits.iter()
                        .enumerate()
                        .map(|(i, h)| Components {
                            bm25: h.score,
                            ppr: 0.0,
                            doc_bm25: 0.0,
                            bm25_rank: Some(i + 1),
                            score: h.score,
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                return Ranked {
                    hits,
                    abstained: None,
                    abstention_score,
                    explain,
                };
            }
            Mode::Graph { alpha } => alpha,
            Mode::Hier { beta } => beta,
        };

        let seeds: Vec<(usize, f64)> = lexical
            .iter()
            .take(ppr::SEEDS)
            .map(|h| (h.chunk, h.score))
            .collect();
        let ppr = self.graph.ppr(&seeds);
        // Mode::Graph and Mode::Hier fuse over this one pool, as the service ADR requires.
        let mut pool: Vec<usize> = lexical.iter().map(|h| h.chunk).collect();
        let scoped_ppr: Vec<f64> = ppr
            .iter()
            .zip(&in_scope)
            .map(|(&s, &keep)| if keep { s } else { 0.0 })
            .collect();
        for hit in rank(&scoped_ppr, chunks, CANDIDATES) {
            if !pool.contains(&hit.chunk) {
                pool.push(hit.chunk);
            }
        }
        let doc_scores = match params.mode {
            Mode::Hier { .. } => self.doc_scores(query),
            _ => Vec::new(),
        };
        let doc_of = |chunk: usize| {
            doc_scores
                .get(self.chunk_doc[chunk])
                .copied()
                .unwrap_or(0.0)
        };
        let pool_bm25: Vec<f64> = pool.iter().map(|&c| bm25[c]).collect();
        let secondary: Vec<f64> = match params.mode {
            Mode::Hier { .. } => pool.iter().map(|&c| doc_of(c)).collect(),
            _ => pool.iter().map(|&c| ppr[c]).collect(),
        };
        let hits = fusion::fuse(&pool, &pool_bm25, &secondary, weight, chunks, params.top_k);
        let explain = if params.explain {
            hits.iter()
                .map(|h| Components {
                    bm25: bm25[h.chunk],
                    ppr: ppr[h.chunk],
                    doc_bm25: doc_of(h.chunk),
                    bm25_rank: lexical
                        .iter()
                        .position(|l| l.chunk == h.chunk)
                        .map(|p| p + 1),
                    score: h.score,
                })
                .collect()
        } else {
            Vec::new()
        };
        Ranked {
            hits,
            abstained: None,
            abstention_score,
            explain,
        }
    }
}

/// Normalised top score and the abstention decision.
pub fn abstention(
    top_bm25: f64,
    terms: &[String],
    bm25: &Bm25,
    threshold: Option<f64>,
) -> (f64, Option<AbstainReason>) {
    let idf_sum: f64 = terms.iter().map(|t| bm25.idf(t)).sum();
    if top_bm25 <= 0.0 || idf_sum <= 0.0 {
        return (0.0, threshold.map(|_| AbstainReason::NoLexicalMatch));
    }
    let s = top_bm25 / idf_sum;
    let below = threshold.is_some_and(|t| s < t);
    (s, below.then_some(AbstainReason::BelowThreshold))
}

/// Positive scores, descending, ties by chunk id; at most `top_k`.
pub fn rank(scores: &[f64], chunks: &[ChunkRecord], top_k: usize) -> Vec<Hit> {
    let mut hits: Vec<Hit> = scores
        .iter()
        .enumerate()
        .filter(|(_, s)| **s > 0.0)
        .map(|(chunk, &score)| Hit { chunk, score })
        .collect();
    hits.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| chunks[a.chunk].id.cmp(&chunks[b.chunk].id))
    });
    hits.truncate(top_k);
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{IndexConfig, build};
    use pretty_assertions::assert_eq;
    use std::fs;
    use std::path::Path;

    fn index() -> (tempfile::TempDir, SearchIndex) {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        fs::write(
            r.join("alpha.md"),
            "# Alpha\n\nThe parseConfig function reads settings.\n\n## Usage\n\nCall it once.\n",
        )
        .unwrap();
        fs::write(r.join("beta.md"), "# Beta\n\nConfig files live in etc.\n").unwrap();
        fs::write(
            r.join("gamma.md"),
            "# Gamma\n\nUnrelated text about cats.\n",
        )
        .unwrap();
        let generation = build(r, &IndexConfig::default()).unwrap();
        (dir, SearchIndex::new(generation))
    }

    fn graph_index() -> SearchIndex {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graph");
        SearchIndex::new(build(&root, &IndexConfig::default()).unwrap())
    }

    fn ids(index: &SearchIndex, hits: &[Hit]) -> Vec<String> {
        hits.iter()
            .map(|h| index.generation.chunks[h.chunk].id.clone())
            .collect()
    }

    fn lexical(top_k: usize) -> Params {
        Params::new(Mode::Lexical, top_k)
    }

    fn bounded_generation() -> Generation {
        let mut generation = graph_index().generation;
        generation.documents.truncate(1);
        generation.chunks.truncate(1);
        generation.edges.clear();
        let document = &mut generation.documents[0];
        document.text = "éabc".into();
        document.pseudo_doc = "pq".into();
        let chunk = &mut generation.chunks[0];
        chunk.path = document.path.clone();
        chunk.heading_path = vec!["é".into(), String::new(), "x".into()];
        chunk.span.byte_start = 0;
        chunk.span.byte_end = 2;
        generation
    }

    fn assert_input_boundary(generation: &Generation, bytes: usize) {
        assert!(SearchIndex::with_limit(generation.clone(), bytes).is_ok());
        assert!(matches!(
            SearchIndex::with_limit(generation.clone(), bytes - 1),
            Err(StoreError::CapExceeded {
                what: "tokenizer input bytes",
                cap,
            }) if cap == bytes - 1
        ));
    }

    #[test]
    fn tokenizer_input_counts_actual_utf8_headings_separators_spans_and_pseudo_docs() {
        let generation = bounded_generation();
        // Heading join "é\n\nx" = 5 bytes, span "é" = 2, pseudo-doc = 2.
        assert_input_boundary(&generation, 9);
        let mut headings = generation.clone();
        headings.documents[0].pseudo_doc.clear();
        headings.chunks[0].span.byte_end = 0;
        assert_input_boundary(&headings, 5);
        let mut spans = generation.clone();
        spans.documents[0].pseudo_doc.clear();
        spans.chunks[0].heading_path.clear();
        assert_input_boundary(&spans, 2);
        let mut pseudo_docs = generation.clone();
        pseudo_docs.chunks.clear();
        // Every pseudo-doc counts, even without a referring chunk.
        pseudo_docs.documents.push(pseudo_docs.documents[0].clone());
        assert_input_boundary(&pseudo_docs, 4);
        let mut duplicates = generation.clone();
        let mut last = duplicates.documents[0].clone();
        last.text = "different last text".into();
        last.pseudo_doc = "xyz".into();
        duplicates.documents.push(last);
        // Keep the first span's two bytes; charge both pseudo-docs (2 + 3).
        assert_input_boundary(&duplicates, 12);
        let index = SearchIndex::with_limit(duplicates, 12).unwrap();
        assert_eq!(index.chunk_doc, vec![1]);
        assert!(validate_input(&generation).is_ok());
        assert!(SearchIndex::try_new(generation).is_ok());
    }

    #[test]
    fn tokenizer_input_counts_overlapping_and_repeated_spans_each_time() {
        let mut generation = bounded_generation();
        generation.documents[0].pseudo_doc.clear();
        generation.chunks[0].heading_path.clear();
        generation.chunks[0].span.byte_end = 4; // "éab", four bytes.
        let mut overlap = generation.chunks[0].clone();
        overlap.span.byte_start = 2;
        overlap.span.byte_end = 5; // "abc", overlaps the first span.
        generation.chunks.push(overlap);
        generation.chunks.push(generation.chunks[0].clone());
        assert_input_boundary(&generation, 11);
    }

    #[test]
    fn tokenizer_input_missing_and_invalid_spans_are_empty() {
        let mut generation = bounded_generation();
        generation.documents[0].pseudo_doc.clear();
        generation.chunks[0].heading_path.clear();
        for (start, end) in [(1, 2), (0, 6), (4, 2), (usize::MAX, usize::MAX)] {
            generation.chunks[0].span.byte_start = start;
            generation.chunks[0].span.byte_end = end;
            assert!(SearchIndex::with_limit(generation.clone(), 0).is_ok());
        }
        generation.chunks[0].path = "missing.md".into();
        generation.chunks[0].span.byte_start = 0;
        generation.chunks[0].span.byte_end = 2;
        let index = SearchIndex::with_limit(generation, 0).unwrap();
        assert_eq!(index.chunk_doc, vec![usize::MAX]);
    }

    #[test]
    fn tokenizer_input_checked_sum_refuses_overflow_without_changing_total() {
        let mut total = usize::MAX;
        assert!(matches!(
            charge_input(&mut total, 1, usize::MAX),
            Err(StoreError::CapExceeded {
                what: "tokenizer input bytes",
                cap: usize::MAX,
            })
        ));
        assert_eq!(total, usize::MAX);
        assert!(charge_input(&mut total, 0, usize::MAX).is_ok());
    }

    #[test]
    fn bounded_construction_matches_old_first_text_last_document_and_rankings() {
        let mut generation = graph_index().generation;
        // Same path, different content and pseudo-doc: chunk text must still
        // come from the first document, hierarchy mapping from the last.
        let mut duplicate = generation.documents[0].clone();
        duplicate.text = "different text".into();
        duplicate.pseudo_doc = "cfg cfg unique config".into();
        generation.documents.push(duplicate);
        let old_bm25 = Bm25::new(generation.chunks.iter().map(|chunk| {
            let mut tokens = tokenize(&chunk.heading_path.join("\n"));
            tokens.extend(tokenize(generation.chunk_text(chunk).unwrap_or("")));
            tokens
        }));
        let old_doc_bm25 = Bm25::new(
            generation
                .documents
                .iter()
                .map(|document| tokenize(&document.pseudo_doc)),
        );
        let old_documents: HashMap<&str, usize> = generation
            .documents
            .iter()
            .enumerate()
            .map(|(i, document)| (document.path.as_str(), i))
            .collect();
        let old_chunk_doc = generation
            .chunks
            .iter()
            .map(|chunk| {
                old_documents
                    .get(chunk.path.as_str())
                    .copied()
                    .unwrap_or(usize::MAX)
            })
            .collect();
        let old =
            SearchIndex::from_indexes(generation.clone(), old_bm25, old_doc_bm25, old_chunk_doc);
        let trusted = SearchIndex::new(generation.clone());
        let bounded = SearchIndex::try_new(generation).unwrap();
        for index in [&trusted, &bounded] {
            assert_eq!(index.bm25, old.bm25);
            assert_eq!(index.doc_bm25, old.doc_bm25);
            assert_eq!(index.chunk_doc, old.chunk_doc);
            for mode in [
                Mode::Lexical,
                Mode::Graph { alpha: 0.3 },
                Mode::Hier { beta: 0.5 },
            ] {
                for query in ["cfg", "alpha", "unique config", "no-match"] {
                    let params = Params {
                        explain: true,
                        ..Params::new(mode, 10)
                    };
                    assert_eq!(index.search(query, &params), old.search(query, &params));
                }
            }
        }
    }

    #[test]
    fn b1_ranks_by_bm25_and_is_repeatable() {
        let (_dir, index) = index();
        let ranked = index.search("config", &lexical(10));
        // "config" occurs in alpha.md#alpha~0 (as a camelCase part) and
        // beta.md#beta~0; beta's chunk is shorter, so it scores higher.
        assert_eq!(
            ids(&index, &ranked.hits),
            vec!["beta.md#beta~0", "alpha.md#alpha~0"]
        );
        assert!(ranked.hits[0].score > ranked.hits[1].score);
        assert_eq!(index.search("config", &lexical(10)), ranked);
        assert_eq!(index.search("parseConfig", &lexical(1)).hits.len(), 1);
    }

    #[test]
    fn ties_break_by_chunk_id() {
        let (_dir, index) = index();
        let chunks = &index.generation.chunks;
        let scores = vec![1.0; chunks.len()];
        let hits = rank(&scores, chunks, 10);
        let got: Vec<&str> = hits.iter().map(|h| chunks[h.chunk].id.as_str()).collect();
        let mut sorted = got.clone();
        sorted.sort();
        assert_eq!(got, sorted);
    }

    #[test]
    fn abstention_matches_hand_computed_score_and_reasons() {
        let bm25 = bm25::tests::toy();
        let terms = vec!["a".to_string(), "c".to_string()];
        // top = d0 = 1.3486402228911236; Σ idf = 0.9808292530117263 + 0.47000362924573563
        // = 1.450832882257462; s = 1.3486402228911236 / 1.450832882257462 = 0.9295627631437958
        let top = 1.3486402228911236;
        let (s, reason) = abstention(top, &terms, &bm25, Some(0.9));
        assert!((s - 0.9295627631437958).abs() < 1e-9);
        assert_eq!(reason, None);
        assert_eq!(
            abstention(top, &terms, &bm25, Some(0.95)).1,
            Some(AbstainReason::BelowThreshold)
        );
        assert_eq!(
            abstention(0.0, &terms, &bm25, Some(0.1)),
            (0.0, Some(AbstainReason::NoLexicalMatch))
        );
        // Disabled for evaluation: never abstains.
        assert_eq!(abstention(top, &terms, &bm25, None).1, None);
    }

    #[test]
    fn search_abstains_with_typed_reason_in_every_mode() {
        let (_dir, index) = index();
        for mode in [
            Mode::Lexical,
            Mode::Graph { alpha: 0.3 },
            Mode::Hier { beta: 0.3 },
        ] {
            let with = |threshold: Option<f64>| Params {
                threshold,
                ..Params::new(mode, 10)
            };
            let none = index.search("zebra", &with(Some(0.1)));
            assert_eq!(none.abstained, Some(AbstainReason::NoLexicalMatch));
            assert!(none.hits.is_empty());
            let low = index.search("config", &with(Some(5.0)));
            assert_eq!(low.abstained, Some(AbstainReason::BelowThreshold));
            assert!(low.hits.is_empty());
            assert!(index.search("zebra", &with(None)).abstained.is_none());
        }
    }

    #[test]
    fn b0_matches_whole_query_substring_in_path_order() {
        let (_dir, index) = index();
        // "config" is in the first 1 KiB of alpha.md and beta.md; path order.
        assert_eq!(
            ids(&index, &index.b0("CONFIG", 10)),
            vec!["alpha.md#alpha~0", "beta.md#beta~0"]
        );
        // Whole-query substring: word order matters, no term matching.
        assert!(index.b0("config parse", 10).is_empty());
        assert_eq!(
            ids(&index, &index.b0("gamma", 10)),
            vec!["gamma.md#gamma~0"]
        );
        assert!(index.b0("   ", 10).is_empty());
        assert_eq!(index.b0("config", 1).len(), 1);
    }

    #[test]
    fn zero_weight_fusion_reproduces_b1_exactly() {
        let (_dir, small) = index();
        let graph = graph_index();
        let queries = [
            "config",
            "alpha",
            "beta setup",
            "gamma run_all",
            "notes",
            "cfg",
            "zzz",
        ];
        for index in [&small, &graph] {
            for q in queries {
                for top_k in [1, 3, 10] {
                    let b1 = index.search(q, &lexical(top_k));
                    for mode in [Mode::Graph { alpha: 0.0 }, Mode::Hier { beta: 0.0 }] {
                        let fused = index.search(q, &Params::new(mode, top_k));
                        assert_eq!(
                            ids(index, &fused.hits),
                            ids(index, &b1.hits),
                            "{q} {mode:?} {top_k}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn graph_mode_surfaces_linked_chunks_with_explain() {
        let index = graph_index();
        // "cfg" only occurs in b.md's preamble; a.md#alpha~0 links to it.
        let b1 = index.search("cfg", &lexical(10));
        assert_eq!(ids(&index, &b1.hits), vec!["b.md#~0"]);
        let params = Params {
            explain: true,
            ..Params::new(Mode::Graph { alpha: 0.5 }, 10)
        };
        let b2 = index.search("cfg", &params);
        let got = ids(&index, &b2.hits);
        assert_eq!(got[0], "b.md#~0");
        let pos = got.iter().position(|id| id == "a.md#alpha~0").unwrap();
        let c = b2.explain[pos];
        assert_eq!((c.bm25, c.bm25_rank), (0.0, None));
        assert!(c.ppr > 0.0 && c.score > 0.0);
        assert_eq!(b2.explain[0].bm25_rank, Some(1));
        assert_eq!(b2.explain.len(), b2.hits.len());
    }

    #[test]
    fn hier_doc_scores_match_hand_computed_values() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("x.md"), "# Term\n\nTerm here.\n").unwrap();
        fs::write(dir.path().join("y.md"), "# Other\n\nTerm is one more.\n").unwrap();
        let index = SearchIndex::new(build(dir.path(), &IndexConfig::default()).unwrap());
        // Pseudo-docs: x = "Term\nTerm here." (3 tokens, tf 2), y = "Other\nTerm is
        // one more." (5 tokens, tf 1); N = 2, n = 2, avgdl = 4, idf = ln(1.2).
        // x: K = 1.2·(0.25 + 0.75·3/4) = 0.975; ln1.2·2·2.2/2.975 = 0.2696520503843362
        // y: K = 1.2·(0.25 + 0.75·5/4) = 1.425; ln1.2·1·2.2/2.425 = 0.1654051236893609
        let s = index.doc_scores("term");
        assert!((s[0] - 0.2696520503843362).abs() < 1e-9, "{}", s[0]);
        assert!((s[1] - 0.1654051236893609).abs() < 1e-9, "{}", s[1]);
        let params = Params {
            explain: true,
            ..Params::new(Mode::Hier { beta: 0.5 }, 10)
        };
        let ranked = index.search("term", &params);
        assert_eq!(
            ids(&index, &ranked.hits),
            vec!["x.md#term~0", "y.md#other~0"]
        );
        assert!((ranked.explain[0].doc_bm25 - 0.2696520503843362).abs() < 1e-9);
    }

    #[test]
    fn scope_restricts_candidates_and_seeds() {
        let index = graph_index();
        let scoped = |scope: &str, mode: Mode| Params {
            scope: Some(scope.to_string()),
            threshold: Some(0.0),
            ..Params::new(mode, 10)
        };
        let empty = index.search("alpha", &scoped("nowhere/", Mode::Lexical));
        assert_eq!(empty.abstained, Some(AbstainReason::ScopeEmpty));
        // "cfg" only matches b.md; inside a.md there is no lexical match.
        let none = index.search("cfg", &scoped("a.md", Mode::Graph { alpha: 0.5 }));
        assert_eq!(none.abstained, Some(AbstainReason::NoLexicalMatch));
        let ranked = index.search("alpha", &scoped("b.md", Mode::Graph { alpha: 0.5 }));
        assert!(!ranked.hits.is_empty());
        for hit in &ranked.hits {
            assert_eq!(index.generation.chunks[hit.chunk].path, "b.md");
        }
    }
}
