//! Bounded read-only snapshot comparison for two retained versions of one KM
//! module.
//!
//! Per the ADR Version History And Diff Contract this is snapshot comparison
//! only: it adds no as-of query engine, no fact-level validity intervals, no
//! supersession edges, no event log, and no schema version. Both versions must
//! be named explicitly by the caller, every collection is capped, truncation is
//! reported, and ordering is deterministic by stable id.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::KmLibrarianVersionDiffError;
use super::KmLibrarianVersionDiffRequest;
use super::KmLibrarianVersionDiffResult;
use super::KmLibrarianVersionEntry;
use super::KmLibrarianVersionFactChange;
use super::KmLibrarianVersionFactEntry;
use super::KmLibrarianVersionMappingChange;
use super::KmLibrarianVersionMappingEntry;
use crate::km::load_facts;
use crate::km_store::KmIndexRecord;
use crate::km_store::KmStore;

/// Caps every emitted collection so one report cannot grow without bound.
const MAX_CHANGED_ITEMS: usize = 50;
const MAX_MAPPING_BYTES: u64 = 512 * 1024;
const MAX_MAPPING_ROWS: usize = 200;

#[derive(Clone, Deserialize)]
struct MappingRow {
    id: String,
    kind: String,
    source_concept: String,
    target_concept: String,
    relation: String,
    source: String,
    confidence: f64,
}

/// Loaded and validated snapshot of exactly one retained module version.
struct Snapshot {
    entry: KmLibrarianVersionEntry,
    facts: BTreeMap<String, KmLibrarianVersionFactEntry>,
    mappings: BTreeMap<String, KmLibrarianVersionMappingEntry>,
}

pub(super) fn run(
    request: &KmLibrarianVersionDiffRequest,
) -> Result<KmLibrarianVersionDiffResult, KmLibrarianVersionDiffError> {
    if request.module_id.trim().is_empty() {
        return Err(KmLibrarianVersionDiffError::InvalidRequest(
            "module id must not be empty".to_string(),
        ));
    }
    if request.left_version.trim() == request.right_version.trim() {
        return Err(KmLibrarianVersionDiffError::InvalidRequest(
            "left and right versions must differ".to_string(),
        ));
    }

    let store = KmStore::new(&request.km_store_root);
    // Obsolete records are included on purpose: retained older versions are the
    // point of this report, and they stay invisible to ordinary retrieval.
    let records = store.list(/*include_obsolete*/ true)?;

    let left = load_snapshot(&store, &records, &request.module_id, &request.left_version)?;
    let right = load_snapshot(&store, &records, &request.module_id, &request.right_version)?;

    let (added_facts, removed_facts, changed_facts, facts_truncated) =
        diff_facts(&left.facts, &right.facts);
    let (added_mappings, removed_mappings, changed_mappings, mappings_truncated) =
        diff_mappings(&left.mappings, &right.mappings);

    Ok(KmLibrarianVersionDiffResult {
        module_id: request.module_id.clone(),
        left: left.entry,
        right: right.entry,
        added_facts,
        removed_facts,
        changed_facts,
        added_mappings,
        removed_mappings,
        changed_mappings,
        truncated: facts_truncated || mappings_truncated,
    })
}

fn load_snapshot(
    store: &KmStore,
    records: &[KmIndexRecord],
    module_id: &str,
    version: &str,
) -> Result<Snapshot, KmLibrarianVersionDiffError> {
    let record = records
        .iter()
        .find(|record| record.id == module_id && record.version == version)
        .ok_or_else(|| {
            KmLibrarianVersionDiffError::VersionNotFound(format!("{module_id}@{version}"))
        })?;

    // Fails closed on a tampered or escaped cache: the stored path is never
    // trusted and the cached content hash is verified before any read.
    let root = store.validated_module_root(record).map_err(|error| {
        KmLibrarianVersionDiffError::InvalidSnapshot(format!(
            "{module_id}@{version} cached snapshot failed validation: {error:?}"
        ))
    })?;

    let facts = load_facts(&root)
        .map_err(KmLibrarianVersionDiffError::InvalidSnapshot)?
        .into_iter()
        .map(|fact| {
            (
                fact.id.clone(),
                KmLibrarianVersionFactEntry {
                    fact_id: fact.id,
                    kind: fact.kind,
                    claim: fact.claim,
                    source: fact.source,
                    confidence: fact.confidence,
                },
            )
        })
        .collect();

    let mappings = load_mappings(&root)
        .map_err(KmLibrarianVersionDiffError::InvalidSnapshot)?
        .into_iter()
        .map(|mapping| {
            (
                mapping.id.clone(),
                KmLibrarianVersionMappingEntry {
                    mapping_id: mapping.id,
                    source_concept: mapping.source_concept,
                    target_concept: mapping.target_concept,
                    relation: mapping.relation,
                    source: mapping.source,
                    confidence: mapping.confidence,
                },
            )
        })
        .collect();

    Ok(Snapshot {
        entry: KmLibrarianVersionEntry {
            version: record.version.clone(),
            status: record.status.clone(),
            content_hash: record.content_hash.clone(),
            imported_at_unix_seconds: record.imported_at_unix_seconds,
        },
        facts,
        mappings,
    })
}

fn diff_facts(
    left: &BTreeMap<String, KmLibrarianVersionFactEntry>,
    right: &BTreeMap<String, KmLibrarianVersionFactEntry>,
) -> (
    Vec<KmLibrarianVersionFactEntry>,
    Vec<KmLibrarianVersionFactEntry>,
    Vec<KmLibrarianVersionFactChange>,
    bool,
) {
    let mut added: Vec<_> = right
        .iter()
        .filter(|(id, _)| !left.contains_key(*id))
        .map(|(_, entry)| entry.clone())
        .collect();
    let mut removed: Vec<_> = left
        .iter()
        .filter(|(id, _)| !right.contains_key(*id))
        .map(|(_, entry)| entry.clone())
        .collect();
    let mut changed: Vec<_> = left
        .iter()
        .filter_map(|(id, before)| {
            let after = right.get(id)?;
            if before == after {
                return None;
            }
            Some(KmLibrarianVersionFactChange {
                fact_id: id.clone(),
                before: before.clone(),
                after: after.clone(),
            })
        })
        .collect();

    let truncated = cap(&mut added) | cap(&mut removed) | cap(&mut changed);
    (added, removed, changed, truncated)
}

fn diff_mappings(
    left: &BTreeMap<String, KmLibrarianVersionMappingEntry>,
    right: &BTreeMap<String, KmLibrarianVersionMappingEntry>,
) -> (
    Vec<KmLibrarianVersionMappingEntry>,
    Vec<KmLibrarianVersionMappingEntry>,
    Vec<KmLibrarianVersionMappingChange>,
    bool,
) {
    let mut added: Vec<_> = right
        .iter()
        .filter(|(id, _)| !left.contains_key(*id))
        .map(|(_, entry)| entry.clone())
        .collect();
    let mut removed: Vec<_> = left
        .iter()
        .filter(|(id, _)| !right.contains_key(*id))
        .map(|(_, entry)| entry.clone())
        .collect();
    let mut changed: Vec<_> = left
        .iter()
        .filter_map(|(id, before)| {
            let after = right.get(id)?;
            if before == after {
                return None;
            }
            Some(KmLibrarianVersionMappingChange {
                mapping_id: id.clone(),
                before: before.clone(),
                after: after.clone(),
            })
        })
        .collect();

    let truncated = cap(&mut added) | cap(&mut removed) | cap(&mut changed);
    (added, removed, changed, truncated)
}

/// Truncates one collection to the shared cap, reporting whether it was cut.
/// Input order is already deterministic because both sides are keyed by stable
/// id in a `BTreeMap`.
fn cap<T>(items: &mut Vec<T>) -> bool {
    if items.len() <= MAX_CHANGED_ITEMS {
        return false;
    }
    items.truncate(MAX_CHANGED_ITEMS);
    true
}

fn load_mappings(root: &Path) -> Result<Vec<MappingRow>, String> {
    let path = root.join("mappings.jsonl");
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err("failed to read mappings.jsonl metadata".to_string()),
    };
    if metadata.len() > MAX_MAPPING_BYTES {
        return Err(format!(
            "mappings.jsonl exceeds the {MAX_MAPPING_BYTES}-byte diff cap"
        ));
    }
    let text = fs::read_to_string(&path).map_err(|_| "failed to read mappings.jsonl")?;
    let mut mappings = Vec::new();
    for (index, line) in text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
    {
        if index == MAX_MAPPING_ROWS {
            return Err(format!(
                "mappings.jsonl exceeds the {MAX_MAPPING_ROWS}-row diff cap"
            ));
        }
        let mapping = serde_json::from_str::<MappingRow>(line)
            .map_err(|error| format!("mappings.jsonl line {} is malformed: {error}", index + 1))?;
        if mapping.kind == "conflict_record" {
            continue;
        }
        if mapping.id.trim().is_empty()
            || mapping.source_concept.trim().is_empty()
            || mapping.target_concept.trim().is_empty()
            || mapping.relation.trim().is_empty()
            || mapping.source.trim().is_empty()
            || !mapping.confidence.is_finite()
            || !(0.0..=1.0).contains(&mapping.confidence)
        {
            return Err(format!(
                "mappings.jsonl line {} has invalid required fields",
                index + 1
            ));
        }
        mappings.push(mapping);
    }
    Ok(mappings)
}

#[cfg(test)]
#[path = "version_diff_tests.rs"]
mod tests;
