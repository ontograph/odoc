use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::KmLibrarianMatchedFact;
use super::KmLibrarianOntologyPath;
use super::KmLibrarianQueryCandidate;
use crate::km::load_facts;

const MAX_MAPPING_BYTES: u64 = 512 * 1024;
const MAX_MAPPING_ROWS: usize = 200;
const MAX_DEPTH: usize = 2;
const MAX_PATHS: usize = 25;
const MAX_MATCHED_FACTS: usize = 25;

pub(super) struct OntologyExpansion {
    pub(super) paths: Vec<KmLibrarianOntologyPath>,
    pub(super) matched_facts: Vec<KmLibrarianMatchedFact>,
    pub(super) truncated: bool,
}

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

#[derive(Clone)]
struct Edge {
    mapping: MappingRow,
    source_concept: String,
    target_concept: String,
}

pub(super) fn expand(
    candidates: &[KmLibrarianQueryCandidate],
) -> Result<OntologyExpansion, String> {
    let mut paths = Vec::new();
    let mut matched_facts = Vec::new();
    let mut truncated = false;
    let mut expanded_packages = BTreeSet::new();

    for candidate in candidates {
        if candidate.matched_terms.is_empty()
            || !expanded_packages.insert((candidate.id.clone(), candidate.version.clone()))
        {
            continue;
        }
        let package_root = Path::new(&candidate.path);
        let mappings = load_mappings(package_root)?;
        let adjacency = build_adjacency(&mappings);
        let mut reached_concepts = BTreeSet::new();
        let mut pending = VecDeque::new();

        for source_concept in adjacency.keys() {
            if concept_matches_seed(&candidate.matched_terms, source_concept)
                && reached_concepts.insert(source_concept.clone())
            {
                pending.push_back((source_concept.clone(), 0));
            }
        }

        while let Some((concept, depth)) = pending.pop_front() {
            if depth >= MAX_DEPTH {
                continue;
            }
            let Some(edges) = adjacency.get(&concept) else {
                continue;
            };
            for edge in edges {
                let normalized_target = normalize(&edge.target_concept);
                if reached_concepts.contains(&normalized_target) {
                    continue;
                }
                if paths.len() == MAX_PATHS {
                    truncated = true;
                    break;
                }
                let next_depth = depth + 1;
                paths.push(KmLibrarianOntologyPath {
                    package_id: candidate.id.clone(),
                    package_version: candidate.version.clone(),
                    mapping_id: edge.mapping.id.clone(),
                    depth: next_depth,
                    source_concept: edge.source_concept.clone(),
                    target_concept: edge.target_concept.clone(),
                    relation: edge.mapping.relation.clone(),
                    source: edge.mapping.source.clone(),
                    confidence: edge.mapping.confidence,
                });
                if reached_concepts.insert(normalized_target.clone()) && next_depth < MAX_DEPTH {
                    pending.push_back((normalized_target, next_depth));
                }
            }
            if truncated {
                break;
            }
        }

        let facts = load_facts(package_root)?;
        for fact in facts {
            let normalized_claim = normalize(&fact.claim);
            if !seed_matches_text(&candidate.matched_terms, &normalized_claim)
                && !reached_concepts
                    .iter()
                    .any(|concept| contains_concept(&normalized_claim, concept))
            {
                continue;
            }
            if matched_facts.len() == MAX_MATCHED_FACTS {
                truncated = true;
                break;
            }
            matched_facts.push(KmLibrarianMatchedFact {
                package_id: candidate.id.clone(),
                package_version: candidate.version.clone(),
                fact_id: fact.id,
                kind: fact.kind,
                claim: fact.claim,
                source: fact.source,
                confidence: fact.confidence,
            });
        }
    }

    paths.sort_by(|left, right| {
        (
            &left.package_id,
            left.depth,
            &left.mapping_id,
            &left.source_concept,
        )
            .cmp(&(
                &right.package_id,
                right.depth,
                &right.mapping_id,
                &right.source_concept,
            ))
    });
    matched_facts.sort_by(|left, right| {
        (&left.package_id, &left.fact_id).cmp(&(&right.package_id, &right.fact_id))
    });

    Ok(OntologyExpansion {
        paths,
        matched_facts,
        truncated,
    })
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
            "mappings.jsonl exceeds the {MAX_MAPPING_BYTES}-byte query cap"
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
                "mappings.jsonl exceeds the {MAX_MAPPING_ROWS}-row query cap"
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
    mappings.sort_by(|left, right| {
        (
            normalize(&left.source_concept),
            normalize(&left.target_concept),
            &left.relation,
            &left.id,
        )
            .cmp(&(
                normalize(&right.source_concept),
                normalize(&right.target_concept),
                &right.relation,
                &right.id,
            ))
    });
    Ok(mappings)
}

fn build_adjacency(mappings: &[MappingRow]) -> BTreeMap<String, Vec<Edge>> {
    let mut adjacency = BTreeMap::<String, Vec<Edge>>::new();
    for mapping in mappings {
        add_edge(
            &mut adjacency,
            mapping,
            &mapping.source_concept,
            &mapping.target_concept,
        );
        if matches!(mapping.relation.as_str(), "same_as" | "equivalent_to") {
            add_edge(
                &mut adjacency,
                mapping,
                &mapping.target_concept,
                &mapping.source_concept,
            );
        }
    }
    for edges in adjacency.values_mut() {
        edges.sort_by(|left, right| {
            (&left.mapping.id, &left.source_concept, &left.target_concept).cmp(&(
                &right.mapping.id,
                &right.source_concept,
                &right.target_concept,
            ))
        });
    }
    adjacency
}

fn add_edge(
    adjacency: &mut BTreeMap<String, Vec<Edge>>,
    mapping: &MappingRow,
    source_concept: &str,
    target_concept: &str,
) {
    adjacency
        .entry(normalize(source_concept))
        .or_default()
        .push(Edge {
            mapping: mapping.clone(),
            source_concept: source_concept.to_string(),
            target_concept: target_concept.to_string(),
        });
}

fn concept_matches_seed(seed_terms: &[String], concept: &str) -> bool {
    let seed_terms = seed_terms
        .iter()
        .map(|term| normalize_term(term))
        .collect::<BTreeSet<_>>();
    let concept_terms = normalized_terms(concept);
    !concept_terms.is_empty()
        && concept_terms
            .iter()
            .all(|concept_term| seed_terms.contains(concept_term))
}

fn seed_matches_text(seed_terms: &[String], text: &str) -> bool {
    let text_terms = normalized_terms(text).into_iter().collect::<BTreeSet<_>>();
    seed_terms
        .iter()
        .map(|term| normalize_term(term))
        .all(|seed_term| text_terms.contains(&seed_term))
}

fn normalized_terms(text: &str) -> Vec<String> {
    normalize(text)
        .split_whitespace()
        .map(normalize_term)
        .collect()
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

fn contains_concept(text: &str, concept: &str) -> bool {
    format!(" {text} ").contains(&format!(" {concept} "))
}

fn normalize(text: &str) -> String {
    let mut normalized = String::new();
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
            if !current.is_empty() {
                if !normalized.is_empty() {
                    normalized.push(' ');
                }
                normalized.push_str(&current);
                current.clear();
            }
        } else {
            current.clear();
            has_non_ascii = false;
        }
    }
    if !has_non_ascii && !current.is_empty() {
        if !normalized.is_empty() {
            normalized.push(' ');
        }
        normalized.push_str(&current);
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_drops_non_ascii_words_and_preserves_ascii() {
        assert_eq!(normalize("café latte résumé"), "latte");
        assert_eq!(normalize("Graph STORE"), "graph store");
    }
}
