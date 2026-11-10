use std::fs;
use std::path::Path;
use std::time::Instant;

use pretty_assertions::assert_eq;
use tempfile::TempDir;

use crate::km_resolve::KmRetrievalCandidate;
use crate::km_resolve::KmRetrievalReason;
use crate::km_resolve::KmRetrievalResult;
use crate::km_resolve::MAX_MAPPING_BYTES;
use crate::km_resolve::collect_mapping_terms;
use crate::km_resolve::collect_query_terms;
use crate::km_resolve::normalize_term;
use crate::km_resolve::resolve_active_km;
use crate::km_resolve::retrieve_active_kms;
use crate::km_store::KmStore;

#[test]
fn retrieve_active_kms_returns_empty_for_empty_query() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));

    assert_eq!(
        retrieve_active_kms(&store, "  ").expect("retrieve"),
        KmRetrievalResult {
            local: Vec::new(),
            global: Vec::new(),
        }
    );
}

#[test]
fn retrieve_active_kms_returns_local_summary_and_reference_neighbors() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let local = write_valid_km(
        tempdir.path(),
        "local",
        "Local graph package.",
        "Alpha retrieval fact.",
        &["km:codebase-graph/neighbor@0.1.0"],
    );
    let summary = write_valid_km(
        tempdir.path(),
        "summary",
        "Alpha summary package.",
        "Unrelated fact.",
        &["silver:summary"],
    );
    let neighbor = write_valid_km(
        tempdir.path(),
        "neighbor",
        "Reference neighbor package.",
        "Unrelated fact.",
        &["silver:neighbor"],
    );
    let local_record = store.import(&local).expect("import local");
    let summary_record = store.import(&summary).expect("import summary");
    let neighbor_record = store.import(&neighbor).expect("import neighbor");

    assert_eq!(
        retrieve_active_kms(&store, "alpha retrieval").expect("retrieve"),
        KmRetrievalResult {
            local: vec![
                KmRetrievalCandidate {
                    id: "codebase-graph/local".to_string(),
                    version: "0.1.0".to_string(),
                    path: fs::canonicalize(local_record.path)
                        .expect("canonical package path")
                        .display()
                        .to_string(),
                    matched_terms: vec!["alpha".to_string(), "retrieval".to_string()],
                    reason: KmRetrievalReason::Local,
                },
                KmRetrievalCandidate {
                    id: "codebase-graph/summary".to_string(),
                    version: "0.1.0".to_string(),
                    path: fs::canonicalize(&summary_record.path)
                        .expect("canonical package path")
                        .display()
                        .to_string(),
                    matched_terms: vec!["alpha".to_string()],
                    reason: KmRetrievalReason::Local,
                },
            ],
            global: vec![
                KmRetrievalCandidate {
                    id: "codebase-graph/neighbor".to_string(),
                    version: "0.1.0".to_string(),
                    path: fs::canonicalize(neighbor_record.path)
                        .expect("canonical package path")
                        .display()
                        .to_string(),
                    matched_terms: vec!["codebase-graph/neighbor".to_string()],
                    reason: KmRetrievalReason::ReferenceNeighbor {
                        referenced_by: "codebase-graph/local".to_string(),
                    },
                },
                KmRetrievalCandidate {
                    id: "codebase-graph/summary".to_string(),
                    version: "0.1.0".to_string(),
                    path: fs::canonicalize(summary_record.path)
                        .expect("canonical package path")
                        .display()
                        .to_string(),
                    matched_terms: vec!["alpha".to_string()],
                    reason: KmRetrievalReason::Summary,
                },
            ],
        }
    );
}

#[test]
fn retrieve_active_kms_ranks_same_score_candidates_by_id() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let later = write_valid_km(
        tempdir.path(),
        "zeta",
        "Zeta package.",
        "Alpha fact.",
        &["silver:zeta"],
    );
    let earlier = write_valid_km(
        tempdir.path(),
        "alpha",
        "Alpha package.",
        "Alpha fact.",
        &["silver:alpha"],
    );
    let later_record = store.import(&later).expect("import later");
    let earlier_record = store.import(&earlier).expect("import earlier");

    assert_eq!(
        retrieve_active_kms(&store, "alpha")
            .expect("retrieve")
            .local,
        vec![
            KmRetrievalCandidate {
                id: "codebase-graph/alpha".to_string(),
                version: "0.1.0".to_string(),
                path: fs::canonicalize(earlier_record.path)
                    .expect("canonical package path")
                    .display()
                    .to_string(),
                matched_terms: vec!["alpha".to_string()],
                reason: KmRetrievalReason::Local,
            },
            KmRetrievalCandidate {
                id: "codebase-graph/zeta".to_string(),
                version: "0.1.0".to_string(),
                path: fs::canonicalize(later_record.path)
                    .expect("canonical package path")
                    .display()
                    .to_string(),
                matched_terms: vec!["alpha".to_string()],
                reason: KmRetrievalReason::Local,
            },
        ]
    );
}

#[derive(Debug, PartialEq, Eq)]
struct RetrievalBaselineObservation {
    query: &'static str,
    expected_relevant_ids: Vec<&'static str>,
    returned_ids: Vec<String>,
    disposition: &'static str,
}

#[test]
fn retrieve_active_kms_baseline_covers_lexical_and_curated_mapping_recall() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let pooling = write_valid_km(
        tempdir.path(),
        "database-session-reuse",
        "Database session reuse.",
        "Connection pooling reuses database sessions.",
        &["silver:pooling"],
    );
    let single_cache = write_valid_km(
        tempdir.path(),
        "a-single-cache",
        "Single cache mention.",
        "Cache computed results.",
        &["silver:cache-single"],
    );
    let repeated_cache = write_valid_km(
        tempdir.path(),
        "z-repeated-cache",
        "Repeated cache mentions.",
        "Cache cache cache computed results.",
        &["silver:cache-repeated"],
    );
    for source in [&single_cache, &repeated_cache] {
        fs::write(
            source.join("mappings.jsonl"),
            "{\"id\":\"map-memoization-cache\",\"kind\":\"concept_mapping\",\"source_concept\":\"memoization\",\"target_concept\":\"cache\",\"relation\":\"same_as\",\"source\":\"silver:cache\",\"confidence\":0.95}\n",
        )
        .expect("mapping");
    }
    for source in [pooling, single_cache, repeated_cache] {
        store.import(&source).expect("import baseline package");
    }

    let started = Instant::now();
    let observations = [
        (
            "connection pooling",
            vec!["codebase-graph/database-session-reuse"],
            "exact lexical recall succeeds",
        ),
        (
            "connections pools",
            vec!["codebase-graph/database-session-reuse"],
            "lexical morphology normalization succeeds",
        ),
        (
            "cache",
            vec![
                "codebase-graph/z-repeated-cache",
                "codebase-graph/a-single-cache",
            ],
            "term frequency breaks equal-term ties",
        ),
        (
            "memoization",
            vec![
                "codebase-graph/a-single-cache",
                "codebase-graph/z-repeated-cache",
            ],
            "curated mapping recall succeeds",
        ),
    ]
    .map(|(query, expected_relevant_ids, disposition)| {
        let returned_ids = retrieve_active_kms(&store, query)
            .expect("retrieve baseline query")
            .local
            .into_iter()
            .map(|candidate| candidate.id)
            .collect();
        RetrievalBaselineObservation {
            query,
            expected_relevant_ids,
            returned_ids,
            disposition,
        }
    });
    let elapsed = started.elapsed();

    assert_eq!(
        observations,
        [
            RetrievalBaselineObservation {
                query: "connection pooling",
                expected_relevant_ids: vec!["codebase-graph/database-session-reuse"],
                returned_ids: vec!["codebase-graph/database-session-reuse".to_string()],
                disposition: "exact lexical recall succeeds",
            },
            RetrievalBaselineObservation {
                query: "connections pools",
                expected_relevant_ids: vec!["codebase-graph/database-session-reuse"],
                returned_ids: vec!["codebase-graph/database-session-reuse".to_string()],
                disposition: "lexical morphology normalization succeeds",
            },
            RetrievalBaselineObservation {
                query: "cache",
                expected_relevant_ids: vec![
                    "codebase-graph/z-repeated-cache",
                    "codebase-graph/a-single-cache",
                ],
                returned_ids: vec![
                    "codebase-graph/z-repeated-cache".to_string(),
                    "codebase-graph/a-single-cache".to_string(),
                ],
                disposition: "term frequency breaks equal-term ties",
            },
            RetrievalBaselineObservation {
                query: "memoization",
                expected_relevant_ids: vec![
                    "codebase-graph/a-single-cache",
                    "codebase-graph/z-repeated-cache",
                ],
                returned_ids: vec![
                    "codebase-graph/a-single-cache".to_string(),
                    "codebase-graph/z-repeated-cache".to_string(),
                ],
                disposition: "curated mapping recall succeeds",
            },
        ]
    );
    eprintln!(
        "retrieval baseline: corpus=3 queries=4 elapsed_us={}",
        elapsed.as_micros()
    );
}

#[test]
fn collect_mapping_terms_is_bounded_and_ignores_invalid_rows() {
    let tempdir = TempDir::new().expect("temp dir");
    fs::write(
        tempdir.path().join("mappings.jsonl"),
        concat!(
            "{\"kind\":\"concept_mapping\",\"source_concept\":\"memoization\",\"target_concept\":\"cache\"}\n",
            "{\n",
            "{\"kind\":\"conflict_record\",\"source_concept\":\"ignored source\",\"target_concept\":\"ignored target\"}\n"
        ),
    )
    .expect("mappings");
    assert_eq!(
        collect_mapping_terms(tempdir.path()),
        vec!["memoization".to_string(), "cache".to_string()]
    );

    fs::write(
        tempdir.path().join("mappings.jsonl"),
        vec![b'x'; MAX_MAPPING_BYTES as usize + 1],
    )
    .expect("oversized mappings");
    assert_eq!(collect_mapping_terms(tempdir.path()), Vec::<String>::new());
}

fn write_valid_km(
    root: &Path,
    name: &str,
    summary: &str,
    claim: &str,
    sources: &[&str],
) -> std::path::PathBuf {
    write_valid_km_impl(root, name, summary, claim, sources)
}

#[test]
fn resolve_active_km_rejects_tampered_cache_content() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(
        tempdir.path(),
        "local",
        "Local graph package.",
        "Alpha retrieval fact.",
        &["silver:doc"],
    );
    let imported = store.import(&source).expect("import local");

    // A record that resolves cleanly before tampering...
    assert!(
        resolve_active_km(&store, &imported.id)
            .expect("resolve ok")
            .is_some()
    );

    // ...must fail closed once the cached content no longer matches its hash.
    fs::write(Path::new(&imported.path).join("facts.jsonl"), "{}\n").expect("tamper cache");
    assert!(resolve_active_km(&store, &imported.id).is_err());
}

#[test]
fn retrieve_active_kms_skips_tampered_cache_content() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(
        tempdir.path(),
        "local",
        "Local graph package.",
        "Alpha retrieval fact.",
        &["silver:doc"],
    );
    let imported = store.import(&source).expect("import local");

    // Tampered cache content is skipped rather than injected.
    fs::write(Path::new(&imported.path).join("facts.jsonl"), "{}\n").expect("tamper cache");
    let result = retrieve_active_kms(&store, "alpha retrieval").expect("retrieve");
    assert_eq!(
        result,
        KmRetrievalResult {
            local: Vec::new(),
            global: Vec::new(),
        }
    );
}

#[test]
fn retrieve_active_kms_skips_tampered_reference_neighbors() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let local = write_valid_km(
        tempdir.path(),
        "local",
        "Local graph package.",
        "Alpha retrieval fact.",
        &["km:codebase-graph/neighbor@0.1.0"],
    );
    let neighbor = write_valid_km(
        tempdir.path(),
        "neighbor",
        "Reference neighbor package.",
        "Unrelated fact.",
        &["silver:neighbor"],
    );
    let local_record = store.import(&local).expect("import local");
    let neighbor_record = store.import(&neighbor).expect("import neighbor");
    fs::write(Path::new(&neighbor_record.path).join("facts.jsonl"), "{}\n")
        .expect("tamper neighbor cache");

    assert_eq!(
        retrieve_active_kms(&store, "alpha retrieval").expect("retrieve"),
        KmRetrievalResult {
            local: vec![KmRetrievalCandidate {
                id: "codebase-graph/local".to_string(),
                version: "0.1.0".to_string(),
                path: fs::canonicalize(local_record.path)
                    .expect("canonical package path")
                    .display()
                    .to_string(),
                matched_terms: vec!["alpha".to_string(), "retrieval".to_string()],
                reason: KmRetrievalReason::Local,
            }],
            global: Vec::new(),
        }
    );
}

fn write_valid_km_impl(
    root: &Path,
    name: &str,
    summary: &str,
    claim: &str,
    sources: &[&str],
) -> std::path::PathBuf {
    let km_root = root.join(format!("{name}-km"));
    fs::create_dir_all(km_root.join("examples")).expect("examples dir");
    fs::create_dir_all(km_root.join("tests")).expect("tests dir");
    fs::write(
        km_root.join("km.toml"),
        valid_manifest(name, summary, sources),
    )
    .expect("manifest");
    fs::write(
        km_root.join("facts.jsonl"),
        format!(
            "{{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"{claim}\",\"source\":\"silver:doc\",\"confidence\":0.9}}\n"
        ),
    )
    .expect("facts");
    fs::write(
        km_root.join("examples/README.md"),
        "# Example\n\nExample body.\n",
    )
    .expect("example");
    fs::write(km_root.join("tests/README.md"), "# Test\n\nTest body.\n").expect("test");
    km_root
}

fn valid_manifest(name: &str, summary: &str, sources: &[&str]) -> String {
    let sources = sources
        .iter()
        .map(|source| format!("\"{source}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "id = \"codebase-graph/{name}\"\nversion = \"0.1.0\"\nkind = \"codebase-graph\"\nsummary = \"{summary}\"\nsources = [{sources}]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n"
    )
}

// Backlog 25: the "ing" suffix is stripped only when the stem keeps at least
// four characters.
#[test]
fn normalize_term_strips_ing_only_when_stem_is_four_plus_chars() {
    assert_eq!(normalize_term("testing"), "test");
    assert_eq!(normalize_term("ring"), "ring");
    assert_eq!(normalize_term("king"), "king");
    assert_eq!(normalize_term("caching"), "cach");
}

// Backlog 26: plural "s" is stripped unless the stem itself ends in "s".
#[test]
fn normalize_term_strips_plural_s_but_not_double_s() {
    assert_eq!(normalize_term("graphs"), "graph");
    assert_eq!(normalize_term("class"), "class");
    assert_eq!(normalize_term("as"), "as");
    assert_eq!(normalize_term("facts"), "fact");
}

// OGRC-09 / 2026-09-10 amendment: non-ASCII fragment rule decided in
// ADR_ONTOGRAPH_RETRIEVAL_NORMALIZATION_POLICY.md ("Amendment 2026-09-10: Non-ASCII Fragment Rule").
// A candidate term is a maximal run of alphanumeric characters; if that run contains any
// non-ASCII character the entire term is dropped, yielding no misleading ASCII fragments.
#[test]
fn non_ascii_terms_are_dropped_by_the_ascii_only_tokenizer() {
    // Cyrillic, CJK, and accented Latin produce no terms: every character fails
    // the filter or marks the word non-ASCII.
    assert_eq!(collect_query_terms("граф хранилище"), Vec::<String>::new());
    assert_eq!(collect_query_terms("知識モジュール"), Vec::<String>::new());

    // Accented words containing non-ASCII characters yield no terms instead of
    // misleading ASCII fragments like "sum" or "caf".
    assert_eq!(collect_query_terms("résumé"), Vec::<String>::new());
    assert_eq!(collect_query_terms("naïve"), Vec::<String>::new());
    assert_eq!(collect_query_terms("café latte"), vec!["latte".to_string()]);

    // Mixed input keeps only all-ASCII terms, so non-ASCII words are dropped entirely.
    assert_eq!(collect_query_terms("граф graph"), vec!["graph".to_string()]);
}

// OGRC-09: the ASCII path itself is unaffected by the above, including
// case folding and the three-character floor.
#[test]
fn ascii_tokenizer_behavior_is_unchanged_by_the_unicode_measurement() {
    assert_eq!(
        collect_query_terms("Graph STORE store"),
        vec!["graph".to_string(), "store".to_string()]
    );
    // Fragments shorter than three characters are discarded.
    assert_eq!(collect_query_terms("a bc graph"), vec!["graph".to_string()]);
}

// Backlog 27: query terms and index terms run through the same normalizer, so
// inflection differences on either side still match.
#[test]
fn query_and_index_term_stemming_are_symmetric() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(
        tempdir.path(),
        "local",
        "Local graph package.",
        "Fact about caches and testing pipelines.",
        &["silver:doc"],
    );
    store.import(&source).expect("import local");

    // "cache"/"caches" and "tests"/"testing" collapse to the same stems.
    let result = retrieve_active_kms(&store, "cache tests").expect("retrieve");

    assert_eq!(result.local.len(), 1);
    assert_eq!(
        result.local[0].matched_terms,
        vec!["cache".to_string(), "tests".to_string()]
    );
}

// Backlog 28: matching is case-insensitive on the query side.
#[test]
fn retrieval_is_case_insensitive_for_mixed_case_queries() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(
        tempdir.path(),
        "local",
        "Local graph package.",
        "Alpha retrieval fact.",
        &["silver:doc"],
    );
    store.import(&source).expect("import local");

    let result = retrieve_active_kms(&store, "ALPHA Retrieval").expect("retrieve");

    assert_eq!(result.local.len(), 1);
}

// Backlog 29: the tokenizer is ASCII-only, so non-ASCII query words produce no
// terms and cannot match. Pinned documented limitation.
#[test]
fn multibyte_terms_are_dropped_by_ascii_tokenizer() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(
        tempdir.path(),
        "local",
        "Local graph package.",
        "Alpha retrieval fact.",
        &["silver:doc"],
    );
    store.import(&source).expect("import local");

    let result = retrieve_active_kms(&store, "\u{65e5}\u{672c}\u{8a9e} \u{e9}\u{e9}\u{e9}")
        .expect("retrieve");

    assert_eq!(result.local, Vec::new());
    assert_eq!(result.global, Vec::new());
}

// Backlog 30: with equal matched-term counts and the same reason rank, the
// candidate with more term occurrences ranks first, beating lexical id order.
#[test]
fn occurrence_count_breaks_ties_only_within_same_reason_rank() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let sparse = write_valid_km(
        tempdir.path(),
        "aaa-sparse",
        "Sparse package.",
        "Alpha retrieval fact.",
        &["silver:doc"],
    );
    let dense = write_valid_km(
        tempdir.path(),
        "zzz-dense",
        "Dense package.",
        "Alpha alpha alpha retrieval retrieval fact.",
        &["silver:doc"],
    );
    store.import(&sparse).expect("import sparse");
    store.import(&dense).expect("import dense");

    let result = retrieve_active_kms(&store, "alpha retrieval").expect("retrieve");

    assert_eq!(result.local.len(), 2);
    assert_eq!(result.local[0].id, "codebase-graph/zzz-dense");
    assert_eq!(result.local[1].id, "codebase-graph/aaa-sparse");
}

// Backlog 31 (pinned actual behavior): summary words also become topic words,
// so a summary match is simultaneously a local match; the km: reference is
// still pulled into the global list.
#[test]
fn reference_neighbor_from_global_only_match_is_included() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let referencer = write_valid_km(
        tempdir.path(),
        "referencer",
        "Alpha retrieval summary.",
        "Unrelated fact.",
        &["km:codebase-graph/neighbor@0.1.0"],
    );
    let neighbor = write_valid_km(
        tempdir.path(),
        "neighbor",
        "Neighbor package.",
        "Neighbor fact.",
        &["silver:doc"],
    );
    store.import(&referencer).expect("import referencer");
    store.import(&neighbor).expect("import neighbor");

    let result = retrieve_active_kms(&store, "alpha retrieval summary").expect("retrieve");

    // The summary feeds topic words, so the referencer matches locally too.
    assert!(
        result
            .local
            .iter()
            .any(|candidate| candidate.id == "codebase-graph/referencer")
    );
    assert!(
        result
            .global
            .iter()
            .any(|candidate| candidate.id == "codebase-graph/referencer")
    );
    assert!(result.global.iter().any(|candidate| {
        candidate.id == "codebase-graph/neighbor"
            && candidate.reason
                == KmRetrievalReason::ReferenceNeighbor {
                    referenced_by: "codebase-graph/referencer".to_string(),
                }
    }));
}

// Backlog 32 (pinned actual behavior): reference expansion is one pass over
// edges in record order, so a chain a->b->c expands transitively when b's
// edge is processed after b was admitted. Limiting this to one hop would be a
// behavior change requiring its own decision.
#[test]
fn reference_chain_expands_transitively_in_edge_order() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let a = write_valid_km(
        tempdir.path(),
        "aaa",
        "Package a.",
        "Alpha retrieval fact.",
        &["km:codebase-graph/bbb@0.1.0"],
    );
    let b = write_valid_km(
        tempdir.path(),
        "bbb",
        "Package b.",
        "Beta fact.",
        &["km:codebase-graph/ccc@0.1.0"],
    );
    let c = write_valid_km(
        tempdir.path(),
        "ccc",
        "Package c.",
        "Gamma fact.",
        &["silver:doc"],
    );
    store.import(&a).expect("import a");
    store.import(&b).expect("import b");
    store.import(&c).expect("import c");

    let result = retrieve_active_kms(&store, "alpha retrieval").expect("retrieve");

    assert!(
        result
            .global
            .iter()
            .any(|candidate| candidate.id == "codebase-graph/bbb")
    );
    assert!(
        result
            .global
            .iter()
            .any(|candidate| candidate.id == "codebase-graph/ccc"),
        "second-hop reference is expanded because edges are processed in order"
    );
}

// Backlog 33: a versioned km: reference resolves by id alone; the version
// suffix is ignored.
#[test]
fn km_reference_with_version_suffix_resolves_by_id_only() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let local = write_valid_km(
        tempdir.path(),
        "local",
        "Local package.",
        "Alpha retrieval fact.",
        &["km:codebase-graph/neighbor@9.9.9"],
    );
    let neighbor = write_valid_km(
        tempdir.path(),
        "neighbor",
        "Neighbor package.",
        "Neighbor fact.",
        &["silver:doc"],
    );
    store.import(&local).expect("import local");
    store.import(&neighbor).expect("import neighbor");

    let result = retrieve_active_kms(&store, "alpha retrieval").expect("retrieve");

    assert!(
        result
            .global
            .iter()
            .any(|candidate| candidate.id == "codebase-graph/neighbor"),
        "reference must resolve by id even when the pinned version differs"
    );
}

// Backlog 34 (pinned current behavior): a self-referencing KM that matched
// locally reappears in global as its own reference neighbor, because the
// dedupe check only scans the global list. Changing that is a behavior
// decision, not a test fix.
#[test]
fn self_referencing_km_reappears_in_global_as_its_own_neighbor() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(
        tempdir.path(),
        "selfref",
        "Self referencing package.",
        "Alpha retrieval fact.",
        &["km:codebase-graph/selfref@0.1.0"],
    );
    store.import(&source).expect("import selfref");

    let result = retrieve_active_kms(&store, "alpha retrieval").expect("retrieve");

    assert_eq!(result.local.len(), 1);
    assert_eq!(result.global.len(), 1);
    assert_eq!(result.global[0].id, "codebase-graph/selfref");
    assert_eq!(
        result.global[0].reason,
        KmRetrievalReason::ReferenceNeighbor {
            referenced_by: "codebase-graph/selfref".to_string(),
        }
    );
}

// Backlog 35: an oversized mappings.jsonl is silently ignored, never an error.
#[test]
fn mappings_over_byte_cap_are_ignored_not_erred() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = tempdir.path().join("pkg");
    fs::create_dir_all(&root).expect("pkg dir");
    let row = "{\"kind\":\"concept_mapping\",\"source_concept\":\"uniqueterm\",\"target_concept\":\"otherterm\"}\n";
    let repeats = (MAX_MAPPING_BYTES as usize / row.len()) + 2;
    fs::write(root.join("mappings.jsonl"), row.repeat(repeats)).expect("oversized mappings");

    assert_eq!(collect_mapping_terms(&root), Vec::<String>::new());
}

// Backlog 36: only the first 200 mapping rows contribute terms.
#[test]
fn mapping_row_201_is_ignored_by_row_cap() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = tempdir.path().join("pkg");
    fs::create_dir_all(&root).expect("pkg dir");
    let mut rows = String::new();
    for index in 0..200 {
        rows.push_str(&format!(
            "{{\"kind\":\"concept_mapping\",\"source_concept\":\"common{index}\",\"target_concept\":\"shared\"}}\n"
        ));
    }
    rows.push_str(
        "{\"kind\":\"concept_mapping\",\"source_concept\":\"beyondcap\",\"target_concept\":\"beyondcap\"}\n",
    );
    fs::write(root.join("mappings.jsonl"), rows).expect("mappings");

    let terms = collect_mapping_terms(&root);

    assert!(terms.contains(&"common0".to_string()));
    assert!(terms.contains(&"common199".to_string()));
    assert!(!terms.contains(&"beyondcap".to_string()));
}

// Backlog 37: example excerpts truncate at 4096 bytes and handle a multibyte
// character split at the boundary lossily instead of panicking.
#[test]
fn example_excerpt_truncates_at_4096_bytes_lossy_on_multibyte() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(
        tempdir.path(),
        "local",
        "Local package.",
        "Alpha retrieval fact.",
        &["silver:doc"],
    );
    // 1366 three-byte chars = 4098 bytes; byte 4096 splits the final char.
    fs::write(source.join("examples/README.md"), "\u{754c}".repeat(1366))
        .expect("multibyte example");
    store.import(&source).expect("import local");

    let resolved = resolve_active_km(&store, "codebase-graph/local")
        .expect("resolve succeeds")
        .expect("resolved module");

    let excerpt = &resolved.examples[0].excerpt;
    assert!(excerpt.chars().filter(|c| *c == '\u{754c}').count() == 1365);
    assert!(excerpt.ends_with('\u{fffd}'));
}

// Backlog 38: resolving an unknown id is Ok(None), not an error.
#[test]
fn resolve_active_km_for_unknown_id_returns_ok_none() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));

    assert_eq!(
        resolve_active_km(&store, "codebase-graph/absent").expect("resolve succeeds"),
        None
    );
}

// Backlog 97: fixed retrieval-measurement corpus (ADR Retrieval Measurement
// Contract). This is a fixture only. It authorizes no ranking-algorithm
// change; a failing case here is the evidence a later correction would need.
#[test]
fn retrieval_measurement_corpus_v1() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    // Three packages with deliberately distinct vocabularies.
    for (name, summary, claim) in [
        (
            "graph",
            "Graph traversal package.",
            "Graph traversal walks nodes and edges for retrieval.",
        ),
        (
            "storage",
            "Storage layout package.",
            "Storage layout keeps immutable snapshots on disk.",
        ),
        (
            "ranking",
            "Ranking policy package.",
            "Ranking policy orders candidates by matched terms.",
        ),
    ] {
        let source = write_valid_km(tempdir.path(), name, summary, claim, &["silver:doc"]);
        store.import(&source).expect("import corpus package");
    }

    // (query, expected top local package id). Queries are chosen so exactly
    // one package should lead.
    let corpus = [
        ("graph traversal nodes edges", "codebase-graph/graph"),
        ("immutable snapshots disk layout", "codebase-graph/storage"),
        ("ranking orders candidates", "codebase-graph/ranking"),
        ("walks nodes edges retrieval", "codebase-graph/graph"),
        ("storage snapshots", "codebase-graph/storage"),
        ("matched terms ordering policy", "codebase-graph/ranking"),
        ("traversal retrieval", "codebase-graph/graph"),
        ("keeps immutable storage", "codebase-graph/storage"),
    ];

    for (query, expected_top) in corpus {
        let result = retrieve_active_kms(&store, query).expect("retrieve");
        let top = result
            .local
            .first()
            .unwrap_or_else(|| panic!("query `{query}` returned no local candidate"));
        assert_eq!(
            top.id, expected_top,
            "query `{query}` ranked `{}` ahead of `{expected_top}`",
            top.id
        );
    }

    // Negative controls: unrelated vocabulary retrieves nothing.
    for query in ["quantum teleportation protocol", "baking sourdough bread"] {
        assert!(
            retrieve_active_kms(&store, query)
                .expect("retrieve")
                .local
                .is_empty(),
            "unrelated query `{query}` must not match"
        );
    }
}

// GDK-7.0: fixed retrieval-ranking corpus, frozen before any ranking change.
//
// Method, per the ADR Measurement Amendment: the fixture packages are authored
// first, and each query's expected answer is stated from that authored text
// using one intent rule that names no scoring mechanism. Agreement with current
// output is a passing result, never the justification. If a row disagrees with
// current behavior it is recorded as a failing corpus row and closes as a
// finding; this test authorizes no ranking change and touches no production
// source.
fn ranking_corpus_store(tempdir: &TempDir) -> KmStore {
    let store = KmStore::new(tempdir.path().join("km-cache"));
    for (name, summary, claim) in [
        (
            "database-session-reuse",
            "Database session reuse package.",
            "Connection pooling reuses database sessions.",
        ),
        (
            "a-single-cache",
            "Computed results package.",
            "Cache computed results.",
        ),
        (
            "z-repeated-cache",
            "Caching subject package.",
            "Cache cache cache computed results.",
        ),
    ] {
        let source = write_valid_km(tempdir.path(), name, summary, claim, &["silver:doc"]);
        store.import(&source).expect("import corpus package");
    }
    store
}

fn corpus_local_ids(store: &KmStore, query: &str) -> Vec<String> {
    retrieve_active_kms(store, query)
        .expect("retrieve")
        .local
        .into_iter()
        .map(|candidate| candidate.id)
        .collect()
}

// Intent rule: exact-term. The query words appear verbatim in exactly one
// authored package text and in no other, so a user asking this intends that
// package.
#[test]
fn ranking_corpus_exact_term_query_returns_only_the_verbatim_package() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = ranking_corpus_store(&tempdir);

    assert_eq!(
        corpus_local_ids(&store, "connection pooling"),
        vec!["codebase-graph/database-session-reuse".to_string()]
    );
}

// Intent rule: morphological. This is the plural inflection of the exact-term
// query above. A user does not change which package they want by pluralizing,
// so it must return the same package.
#[test]
fn ranking_corpus_morphological_query_matches_the_exact_term_row() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = ranking_corpus_store(&tempdir);

    assert_eq!(
        corpus_local_ids(&store, "connections pools"),
        corpus_local_ids(&store, "connection pooling"),
    );
}

// Intent rule: specificity. z-repeated-cache was authored to be ABOUT caching,
// repeating the subject term as its dominant content; a-single-cache was
// authored to mention caching once while its subject is computed results. A
// user asking the bare subject word intends the package that is about that
// subject. Stated in authored terms deliberately: occurrence count is the
// current mechanism and must not appear in this justification.
#[test]
fn ranking_corpus_bare_subject_query_ranks_the_subject_package_first() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = ranking_corpus_store(&tempdir);

    let ids = corpus_local_ids(&store, "cache");
    let about = ids
        .iter()
        .position(|id| id == "codebase-graph/z-repeated-cache")
        .expect("the package about caching is retrieved");
    let mentions = ids
        .iter()
        .position(|id| id == "codebase-graph/a-single-cache")
        .expect("the package mentioning caching is retrieved");
    assert!(
        about < mentions,
        "package about the subject must outrank the one merely mentioning it: {ids:?}"
    );
}

// Intent rule: absence. No authored package text contains this term or any
// lexical form of it. The semantic relationship to caching is real for a human
// but is not present in the authored fixture, so lexical retrieval is expected
// to return nothing. This row exists so a future lexical change cannot claim
// semantic coverage.
#[test]
fn ranking_corpus_absent_term_query_returns_empty_and_must_stay_empty() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = ranking_corpus_store(&tempdir);

    let result = retrieve_active_kms(&store, "memoization").expect("retrieve");
    assert_eq!(result.local, Vec::new());
    assert_eq!(result.global, Vec::new());
}
