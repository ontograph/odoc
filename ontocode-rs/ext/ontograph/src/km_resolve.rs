use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;

use crate::km::KmFact;
use crate::km::KmManifest;
use crate::km::KmProvenance;
use crate::km::load_facts;
use crate::km::load_manifest;
use crate::km_store::KmIndexOrigin;
use crate::km_store::KmStore;
use crate::km_store::KmStoreError;

pub(crate) const MAX_EXAMPLE_BYTES: usize = 4096;
const MAX_MAPPING_BYTES: u64 = 512 * 1024;
const MAX_MAPPING_ROWS: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KmRetrievalResult {
    pub local: Vec<KmRetrievalCandidate>,
    pub global: Vec<KmRetrievalCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KmRetrievalCandidate {
    pub id: String,
    pub version: String,
    pub path: String,
    pub matched_terms: Vec<String>,
    pub reason: KmRetrievalReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KmRetrievalReason {
    Local,
    Summary,
    ReferenceNeighbor { referenced_by: String },
}

struct RankedCandidate {
    candidate: KmRetrievalCandidate,
    occurrence_count: usize,
}

struct TermMatch {
    matched_terms: Vec<String>,
    occurrence_count: usize,
}

#[derive(Deserialize)]
struct MappingTermsRow {
    kind: String,
    source_concept: String,
    target_concept: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedKm {
    pub id: String,
    pub version: String,
    pub summary: String,
    pub sources: Vec<String>,
    pub licenses: Vec<String>,
    pub origin: Option<KmIndexOrigin>,
    pub provenance: KmProvenance,
    pub facts: Vec<KmFact>,
    pub examples: Vec<KmResolvedExample>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KmResolvedExample {
    pub path: String,
    pub excerpt: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KmResolveError {
    Store(KmStoreError),
    MissingSummary,
    MissingSources,
    MissingLicenses,
    MissingProvenance,
    Manifest(String),
    Facts(String),
    Examples,
}

pub(crate) fn resolve_active_km(
    store: &KmStore,
    id: &str,
) -> Result<Option<ResolvedKm>, KmResolveError> {
    let Some(record) = store.resolve_active(id).map_err(KmResolveError::Store)? else {
        return Ok(None);
    };
    // Never trust the stored path: validate cache layout, containment, and
    // content hash before reading anything (PR-021).
    let root = store
        .validated_module_root(&record)
        .map_err(KmResolveError::Store)?;
    let manifest = load_manifest(&root).map_err(KmResolveError::Manifest)?;
    let facts = load_facts(&root).map_err(KmResolveError::Facts)?;
    let examples = load_examples(&root)?;
    build_resolved_km(manifest, facts, examples, record.origin)
}

pub(crate) fn retrieve_active_kms(
    store: &KmStore,
    query: &str,
) -> Result<KmRetrievalResult, KmResolveError> {
    let query_terms = collect_query_terms(query);
    if query_terms.is_empty() {
        return Ok(KmRetrievalResult {
            local: Vec::new(),
            global: Vec::new(),
        });
    }

    let mut local = Vec::new();
    let mut summary_matches = Vec::new();
    let mut reference_edges = Vec::new();
    let records = store
        .list(/*include_obsolete*/ false)
        .map_err(KmResolveError::Store)?;
    for record in &records {
        // Skip any record whose cache cannot be validated rather than reading a
        // tampered or escaped path (PR-021).
        let Ok(root) = store.validated_module_root(record) else {
            continue;
        };
        let manifest = load_manifest(&root).map_err(KmResolveError::Manifest)?;
        let facts = load_facts(&root).map_err(KmResolveError::Facts)?;

        let mut local_source_terms = record.topic_words.clone();
        for fact in &facts {
            local_source_terms.extend(collect_terms(&fact.claim));
        }
        local_source_terms.extend(collect_mapping_terms(&root));
        let local_match = matching_terms(&query_terms, &local_source_terms);
        if !local_match.matched_terms.is_empty() {
            local.push(ranked_candidate(
                record,
                &root,
                local_match,
                KmRetrievalReason::Local,
            ));
        }

        let summary_match = matching_terms(
            &query_terms,
            &manifest
                .summary
                .as_deref()
                .map(collect_terms)
                .unwrap_or_default(),
        );
        if !summary_match.matched_terms.is_empty() {
            summary_matches.push(ranked_candidate(
                record,
                &root,
                summary_match,
                KmRetrievalReason::Summary,
            ));
        }

        for source in manifest.sources.unwrap_or_default() {
            if let Some(reference) = source.strip_prefix("km:") {
                reference_edges.push((record.id.clone(), reference_id(reference)));
            }
        }
    }

    let mut global = summary_matches;
    for (referencer, reference) in reference_edges {
        if !local
            .iter()
            .any(|candidate| candidate.candidate.id == referencer)
            && !global
                .iter()
                .any(|candidate| candidate.candidate.id == referencer)
        {
            continue;
        }
        let Some(record) = records.iter().find(|record| record.id == reference) else {
            continue;
        };
        let Ok(root) = store.validated_module_root(record) else {
            continue;
        };
        if global
            .iter()
            .any(|candidate| candidate.candidate.id == record.id)
        {
            continue;
        }
        global.push(RankedCandidate {
            candidate: candidate(
                record,
                &root,
                vec![reference.clone()],
                KmRetrievalReason::ReferenceNeighbor {
                    referenced_by: referencer,
                },
            ),
            occurrence_count: 0,
        });
    }

    sort_candidates(&mut local);
    sort_candidates(&mut global);
    Ok(KmRetrievalResult {
        local: local
            .into_iter()
            .map(|candidate| candidate.candidate)
            .collect(),
        global: global
            .into_iter()
            .map(|candidate| candidate.candidate)
            .collect(),
    })
}

fn collect_mapping_terms(root: &Path) -> Vec<String> {
    let Ok(text) = crate::artifact_io::read_to_string_beneath(
        root,
        Path::new("mappings.jsonl"),
        MAX_MAPPING_BYTES as usize,
    ) else {
        return Vec::new();
    };
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .take(MAX_MAPPING_ROWS)
        .filter_map(|line| serde_json::from_str::<MappingTermsRow>(line).ok())
        .filter(|mapping| mapping.kind != "conflict_record")
        .flat_map(|mapping| {
            let mut terms = collect_terms(&mapping.source_concept);
            terms.extend(collect_terms(&mapping.target_concept));
            terms
        })
        .collect()
}

fn ranked_candidate(
    record: &crate::km_store::KmIndexRecord,
    root: &Path,
    term_match: TermMatch,
    reason: KmRetrievalReason,
) -> RankedCandidate {
    RankedCandidate {
        candidate: candidate(record, root, term_match.matched_terms, reason),
        occurrence_count: term_match.occurrence_count,
    }
}

fn sort_candidates(candidates: &mut [RankedCandidate]) {
    candidates.sort_by(|left, right| {
        right
            .candidate
            .matched_terms
            .len()
            .cmp(&left.candidate.matched_terms.len())
            .then_with(|| {
                if reason_rank(&left.candidate.reason) == reason_rank(&right.candidate.reason) {
                    right.occurrence_count.cmp(&left.occurrence_count)
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .then_with(|| left.candidate.id.cmp(&right.candidate.id))
            .then_with(|| left.candidate.version.cmp(&right.candidate.version))
            .then_with(|| {
                reason_rank(&left.candidate.reason).cmp(&reason_rank(&right.candidate.reason))
            })
            .then_with(|| {
                referenced_by(&left.candidate.reason).cmp(referenced_by(&right.candidate.reason))
            })
    });
}

fn matching_terms(query_terms: &[String], terms: &[String]) -> TermMatch {
    let mut occurrence_counts = BTreeMap::new();
    for term in terms {
        *occurrence_counts.entry(normalize_term(term)).or_insert(0) += 1;
    }

    let mut matched_terms = Vec::new();
    let mut occurrence_count = 0;
    let mut matched_normalized_terms = BTreeSet::new();
    for query_term in query_terms {
        let normalized = normalize_term(query_term);
        if !matched_normalized_terms.insert(normalized.clone()) {
            continue;
        }
        let Some(count) = occurrence_counts.get(&normalized) else {
            continue;
        };
        matched_terms.push(query_term.clone());
        occurrence_count += count;
    }

    TermMatch {
        matched_terms,
        occurrence_count,
    }
}

fn normalize_term(term: &str) -> String {
    if let Some(stem) = term.strip_suffix("ing")
        && stem.len() >= 4
    {
        return stem.to_string();
    }
    if let Some(stem) = term.strip_suffix('s')
        && stem.len() >= 3
        && !stem.ends_with('s')
    {
        return stem.to_string();
    }
    term.to_string()
}

fn collect_terms(text: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut current = String::new();
    let mut has_non_ascii = false;
    for character in text.chars() {
        if character.is_alphanumeric() {
            if !character.is_ascii() {
                has_non_ascii = true;
                current.clear();
            } else if !has_non_ascii {
                current.push(character.to_ascii_lowercase());
            }
            continue;
        }
        if !has_non_ascii {
            push_term(&mut current, &mut terms);
        } else {
            current.clear();
            has_non_ascii = false;
        }
    }
    if !has_non_ascii {
        push_term(&mut current, &mut terms);
    }
    terms
}

fn collect_query_terms(text: &str) -> Vec<String> {
    let mut terms = collect_terms(text);
    terms.sort();
    terms.dedup();
    terms
}

fn candidate(
    record: &crate::km_store::KmIndexRecord,
    root: &Path,
    matched_terms: Vec<String>,
    reason: KmRetrievalReason,
) -> KmRetrievalCandidate {
    KmRetrievalCandidate {
        id: record.id.clone(),
        version: record.version.clone(),
        path: root.display().to_string(),
        matched_terms,
        reason,
    }
}

fn build_resolved_km(
    manifest: KmManifest,
    facts: Vec<KmFact>,
    examples: Vec<KmResolvedExample>,
    origin: Option<KmIndexOrigin>,
) -> Result<Option<ResolvedKm>, KmResolveError> {
    let summary = manifest.summary.ok_or(KmResolveError::MissingSummary)?;
    let sources = manifest.sources.ok_or(KmResolveError::MissingSources)?;
    let licenses = manifest.licenses.ok_or(KmResolveError::MissingLicenses)?;
    let provenance = manifest
        .provenance
        .ok_or(KmResolveError::MissingProvenance)?;
    Ok(Some(ResolvedKm {
        id: manifest.id,
        version: manifest.version,
        summary,
        sources,
        licenses,
        origin,
        provenance,
        facts,
        examples,
    }))
}

fn load_examples(root: &Path) -> Result<Vec<KmResolvedExample>, KmResolveError> {
    let examples_root = root.join("examples");
    let mut paths = collect_files(&examples_root).map_err(|_| KmResolveError::Examples)?;
    paths.sort();

    let mut examples = Vec::with_capacity(paths.len());
    for path in paths {
        let relative = path
            .strip_prefix(root)
            .map_err(|_| KmResolveError::Examples)?
            .to_string_lossy()
            .replace('\\', "/");
        let mut bytes = Vec::with_capacity(MAX_EXAMPLE_BYTES);
        crate::artifact_io::open_regular(&path)
            .map_err(|_| KmResolveError::Examples)?
            .take(MAX_EXAMPLE_BYTES as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| KmResolveError::Examples)?;
        let excerpt = String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_EXAMPLE_BYTES)])
            .trim()
            .to_string();
        examples.push(KmResolvedExample {
            path: relative,
            excerpt,
        });
    }
    Ok(examples)
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = fs::read_dir(&current)?.collect::<Result<Vec<_>, _>>()?;
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
    Ok(files)
}

fn reason_rank(reason: &KmRetrievalReason) -> u8 {
    match reason {
        KmRetrievalReason::Local => 0,
        KmRetrievalReason::Summary => 1,
        KmRetrievalReason::ReferenceNeighbor { .. } => 2,
    }
}

fn referenced_by(reason: &KmRetrievalReason) -> &str {
    match reason {
        KmRetrievalReason::ReferenceNeighbor { referenced_by } => referenced_by,
        KmRetrievalReason::Local | KmRetrievalReason::Summary => "",
    }
}

fn push_term(current: &mut String, terms: &mut Vec<String>) {
    if current.len() >= 3 {
        terms.push(std::mem::take(current));
    } else {
        current.clear();
    }
}

fn reference_id(reference: &str) -> String {
    reference
        .split_once('@')
        .map(|(id, _)| id)
        .unwrap_or(reference)
        .to_string()
}

#[cfg(test)]
#[path = "km_resolve_tests.rs"]
mod tests;
