//! KM-backed lineage retrieval for the turn-input seam (LRS-3.0).
//!
//! Grounding resolves a span against real file content, so a candidate passage
//! is only useful when it carries a file path and 1-based line numbers. KM
//! records themselves carry no spans: `KmFact` holds a claim and a source
//! label, not a location. KM example files, however, are real files on disk
//! under the store's validated module root, so a line that matches a demand
//! term yields a genuine path, a real span, and a content hash.
//!
//! Every span here is computed from file content. Nothing is defaulted, so a
//! source edited after retrieval fails the stale-source check rather than
//! grounding against text that has moved.

use std::collections::BTreeMap;

use sha2::Digest;
use sha2::Sha256;

use crate::evidence_planner::SourceLocation;
use crate::km_resolve::ResolvedKm;
use crate::km_resolve::resolve_active_km;
use crate::km_resolve::retrieve_active_kms;
use crate::km_store::KmStore;
use crate::lineage_grounding::SourceGeneration;
use crate::lineage_grounding::SourceGenerationFile;
use crate::lineage_records::CandidatePassage;
use crate::lineage_records::EvidenceDemand;
use crate::lineage_records::LineageLimits;
use crate::lineage_records::merge_candidates;
use crate::lineage_routing::LineageAttempt;
use crate::lineage_routing::LineageCost;
use crate::lineage_selection::SelectionBudget;
use crate::lineage_selection::select_evidence;

/// Maximum KM modules inspected for one turn.
const MAX_RETRIEVED_KMS: usize = 4;
/// Maximum candidate passages produced per demand.
const MAX_CANDIDATES_PER_DEMAND: usize = 8;
/// Lines of context included on each side of a matched line.
const SPAN_CONTEXT_LINES: usize = 1;
/// Byte budget for a Markdown section span's lines, leaving room for the
/// rendered path header so admission's per-span cap still holds.
const MAX_SECTION_SPAN_BYTES: usize = crate::lineage_admission::MAX_SPAN_BYTES - 256;

/// Retrieves KM example spans for the demands induced from a question.
///
/// Returns `Selected`, since grounding cannot run at the turn-input seam.
/// Every failure is a typed attempt variant, so the caller records a reason
/// rather than treating an empty result as success.
pub(crate) fn km_lineage_attempt(store: &KmStore, question: &str) -> LineageAttempt {
    let started = std::time::Instant::now();
    let demands = induce_demands(question);
    if demands.is_empty() {
        return LineageAttempt::InductionFailed(
            "question produced no searchable demand terms".to_string(),
        );
    }

    let mut scanned_files = 0usize;
    let mut generation_files = Vec::new();
    let mut candidates = Vec::new();
    let mut token_costs = BTreeMap::new();

    let all_terms = demands
        .iter()
        .map(|demand| demand.description.clone())
        .collect::<Vec<_>>();
    for demand in &demands {
        let retrieved = match retrieve_active_kms(store, &demand.description) {
            Ok(retrieved) => retrieved,
            Err(error) => {
                return LineageAttempt::RetrievalUnavailable(format!(
                    "km retrieval failed for demand `{}`: {error:?}",
                    demand.demand_id
                ));
            }
        };

        let mut rank = 0usize;
        // One KM can appear in both the local and global result sets, and two
        // KMs can expose the same example path. Merging rejects a demand that
        // retrieves the same passage twice, so admit each passage id once.
        let mut seen_passages = BTreeMap::new();
        for candidate in retrieved
            .local
            .iter()
            .chain(retrieved.global.iter())
            .take(MAX_RETRIEVED_KMS)
        {
            let resolved = match resolve_active_km(store, &candidate.id) {
                Ok(Some(resolved)) => resolved,
                Ok(None) => continue,
                Err(error) => {
                    return LineageAttempt::RetrievalUnavailable(format!(
                        "km resolution failed for `{}`: {error:?}",
                        candidate.id
                    ));
                }
            };
            scanned_files += resolved.examples.len();
            for matched in matching_spans(&resolved, &demand.description, &all_terms) {
                if rank >= MAX_CANDIDATES_PER_DEMAND {
                    break;
                }
                if seen_passages
                    .insert(matched.passage_id.clone(), ())
                    .is_some()
                {
                    continue;
                }
                token_costs.insert(matched.passage_id.clone(), matched.token_cost);
                if !generation_files
                    .iter()
                    .any(|file: &SourceGenerationFile| file.path == matched.path)
                {
                    generation_files.push(SourceGenerationFile {
                        path: matched.path.clone(),
                        generation_id: matched.generation_id.clone(),
                        lines: matched.lines,
                    });
                }
                candidates.push(CandidatePassage {
                    passage_id: matched.passage_id,
                    demand_id: demand.demand_id.clone(),
                    rank,
                    score: matched.score,
                    source_location: SourceLocation {
                        path: matched.path,
                        start_line: Some(matched.start_line),
                        end_line: Some(matched.end_line),
                    },
                });
                rank += 1;
            }
        }
    }

    if candidates.is_empty() {
        return LineageAttempt::RetrievalUnavailable(format!(
            "no km example span matched any of {} demand(s) across {scanned_files} example file(s)",
            demands.len()
        ));
    }

    let ledger = match merge_candidates(&demands, &candidates, LineageLimits::default()) {
        Ok(ledger) => ledger,
        Err(error) => return LineageAttempt::RetrievalFailed(error),
    };
    let selection = match select_evidence(&ledger, &token_costs, SelectionBudget::default()) {
        Ok(selection) => selection,
        Err(error) => return LineageAttempt::SelectionFailed(error),
    };
    let token_cost = selection.total_token_cost;

    LineageAttempt::Selected {
        ledger,
        selection,
        generation: SourceGeneration::from_files(generation_files),
        cost: LineageCost {
            elapsed_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            token_cost,
        },
    }
}

/// One example-file line that matched a demand term, with its computed span.
struct MatchedSpan {
    passage_id: String,
    path: String,
    generation_id: String,
    lines: Vec<String>,
    start_line: usize,
    end_line: usize,
    score: usize,
    token_cost: usize,
}

/// Finds, for each example file, the line containing `term` that also contains
/// the most distinct demand terms (`all_terms`), and returns its context-padded
/// span. Ties go to the longer line, then the earlier one.
///
/// Taking the first matching line per file made a file contribute whatever line
/// happened to come first, and scoring by line length let long unrelated lines
/// outrank short lines that matched several question terms. Preferring the line
/// that covers the most demands lets one passage satisfy several demands, which
/// the selector already rewards through marginal coverage.
fn matching_spans(resolved: &ResolvedKm, term: &str, all_terms: &[String]) -> Vec<MatchedSpan> {
    let needle = term.to_ascii_lowercase();
    let mut spans = Vec::new();
    for example in &resolved.examples {
        let lines = example
            .excerpt
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>();
        if lines.is_empty() {
            continue;
        }
        let generation_id = content_hash(&example.excerpt);
        // (distinct demand terms, length, line index) of the best matching line.
        let mut best: Option<(usize, usize, usize)> = None;
        for (index, line) in lines.iter().enumerate() {
            let lowered = line.to_ascii_lowercase();
            if !lowered.contains(&needle) {
                continue;
            }
            let distinct = all_terms
                .iter()
                .filter(|candidate| lowered.contains(candidate.as_str()))
                .count();
            let candidate = (distinct, line.len(), index);
            let better = match best {
                None => true,
                Some((best_distinct, best_len, _)) => {
                    (distinct, line.len()) > (best_distinct, best_len)
                }
            };
            if better {
                best = Some(candidate);
            }
        }
        let Some((distinct, line_len, index)) = best else {
            continue;
        };
        let (start_line, end_line) = span_bounds(&lines, index);
        spans.push(MatchedSpan {
            // The id names only the start; `span_bounds` gives every span
            // with one start the same end, so merging never sees a conflict.
            passage_id: format!("{}:{}:{start_line}", resolved.id, example.path),
            path: km_source_key(&resolved.id, &resolved.version, &example.path),
            generation_id: generation_id.clone(),
            lines: lines.clone(),
            start_line,
            end_line,
            // Distinct demand terms dominate; length only breaks ties.
            score: distinct * 10_000 + line_len.min(9_999),
            token_cost: lines[start_line - 1..end_line]
                .iter()
                .map(String::len)
                .sum::<usize>()
                .max(1),
        });
    }
    spans
}

/// 1-based inclusive span for the matched line at 0-based `index`.
///
/// Inside a Markdown section the span starts at the section heading and runs
/// to the line before the next heading, cut at `MAX_SECTION_SPAN_BYTES`. Notes
/// put the section's subject (often an `Implementation: path` line) right
/// under the heading, which a narrow window around a later match drops. The
/// end depends only on the section, so every match in it yields one span.
/// Without a heading, when only the heading fits, or when the match lies past
/// the cut, the span is a window of `SPAN_CONTEXT_LINES` on each side. A
/// window can start on the heading line only for a match right below it, and
/// that match falls back only when the whole section does, so a start line
/// (the passage id) never maps to two different ends.
fn span_bounds(lines: &[String], index: usize) -> (usize, usize) {
    let is_heading =
        |line: &String| line.starts_with('#') && line.trim_start_matches('#').starts_with(' ');
    if let Some(start) = lines[..=index].iter().rposition(is_heading) {
        let mut end = start;
        let mut bytes = 0usize;
        for (offset, line) in lines[start..].iter().enumerate() {
            bytes += line.len() + 1;
            if offset > 0 && (is_heading(line) || bytes > MAX_SECTION_SPAN_BYTES) {
                break;
            }
            end = start + offset;
        }
        if end > start && end >= index {
            return (start + 1, end + 1);
        }
    }
    let start_line = index.saturating_sub(SPAN_CONTEXT_LINES) + 1;
    let end_line = (start_line + 2 * SPAN_CONTEXT_LINES).min(lines.len());
    (start_line, end_line)
}

/// Demand terms induced from a question, in stable order.
///
/// Deliberately the same shape as the automatic KM word filter: lowercase
/// alphanumeric words of three characters or more, deduplicated.
fn induce_demands(question: &str) -> Vec<EvidenceDemand> {
    let mut words = question
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| word.len() >= 3)
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    words.sort();
    words.dedup();
    words
        .into_iter()
        .take(LineageLimits::default().max_demands)
        .enumerate()
        .map(|(index, word)| EvidenceDemand {
            demand_id: format!("d-{index}"),
            description: word,
            bindings: BTreeMap::new(),
        })
        .collect()
}

/// Content hash identifying the generation a span was read from.
fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

/// Source identity encoded as `<id byte length>:<id>@<semver>/<relative path>`.
/// KM ids may contain `/`, `@` or `:`; the byte length fixes their boundary.
/// Strict semver contains no `/`, so the remaining path is unambiguous.
fn km_source_key(id: &str, version: &str, relative_path: &str) -> String {
    let id_bytes = id.len();
    format!("{id_bytes}:{id}@{version}/{relative_path}")
}

/// Removes the KM namespace for filesystem reads and display only.
/// Ordinary source paths are preserved for non-KM admission callers.
pub(crate) fn km_source_relative_path(key: &str) -> &str {
    let relative_path = || {
        let (id_bytes, rest) = key.split_once(':')?;
        let id_bytes = id_bytes.parse::<usize>().ok()?;
        let suffix = rest.get(id_bytes..)?.strip_prefix('@')?;
        let (version, path) = suffix.split_once('/')?;
        semver::Version::parse(version).ok()?;
        Some(path)
    };
    match relative_path() {
        Some(path) => path,
        None => key,
    }
}

#[cfg(test)]
#[path = "lineage_km_retrieval_tests.rs"]
mod tests;

/// Re-reads selected examples from validated, admitted KM packages for grounding.
pub(crate) fn refresh_selected(
    store: &KmStore,
    scope: &str,
    selection: &crate::lineage_selection::SelectionReport,
    ledger: &crate::lineage_records::LineageLedger,
    admits: impl Fn(&crate::km_store::KmIndexRecord) -> bool,
) -> Result<SourceGeneration, crate::lineage_admission::AdmissionRejection> {
    use std::collections::BTreeSet;
    use std::collections::btree_map::Entry;
    use std::io::Read;

    use crate::km_resolve::MAX_EXAMPLE_BYTES;
    use crate::lineage_admission::AdmissionRejection;
    use crate::lineage_admission::MAX_ADMITTED_SPANS;

    let mut seen = BTreeSet::new();
    let mut selected_ids = Vec::new();
    for passage in &selection.selected {
        if seen.insert(passage.passage_id.as_str()) {
            if seen.len() > MAX_ADMITTED_SPANS {
                return Err(AdmissionRejection::ByteCapExceeded);
            }
            selected_ids.push(passage.passage_id.as_str());
        }
    }

    let records = store
        .list(/*include_obsolete*/ false)
        .map_err(|_| AdmissionRejection::SourceUnavailable)?;
    let mut key_owners = BTreeMap::new();
    let mut roots = BTreeMap::new();
    let mut files = Vec::new();
    for id in selected_ids {
        let location = ledger
            .lineage_for(id)
            .map(|lineage| &lineage.source_location)
            .ok_or(AdmissionRejection::SchemaInvalid)?;
        let (Some(start), Some(end)) = (location.start_line, location.end_line) else {
            return Err(AdmissionRejection::SchemaInvalid);
        };
        if location.path.is_empty() || start == 0 || start > end {
            return Err(AdmissionRejection::SchemaInvalid);
        }
        let path = km_source_relative_path(&location.path);
        let suffix = format!(":{path}:{start}");
        let mut owners = records
            .iter()
            .filter(|record| id.strip_prefix(record.id.as_str()) == Some(suffix.as_str()));
        let record = owners.next().ok_or(AdmissionRejection::SourceUnavailable)?;
        if owners.next().is_some() {
            return Err(AdmissionRejection::SchemaInvalid);
        }
        if record.project_scope.as_deref() != Some(scope) || !admits(record) {
            return Err(AdmissionRejection::ScopeInvalid);
        }
        let key = km_source_key(&record.id, &record.version, path);
        if key != location.path {
            return Err(AdmissionRejection::SchemaInvalid);
        }
        let owner = (record.id.as_str(), record.version.as_str());
        if let Some(previous_owner) = key_owners.get(&key) {
            if *previous_owner != owner {
                return Err(AdmissionRejection::SchemaInvalid);
            }
            continue;
        }

        let root = match roots.entry(record.id.as_str()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(
                store
                    .validated_module_root(record)
                    .map_err(|_| AdmissionRejection::SourceUnavailable)?,
            ),
        };
        let examples_root = std::fs::canonicalize(root.join("examples"))
            .map_err(|_| AdmissionRejection::SourceUnavailable)?;
        let example_path = std::fs::canonicalize(root.join(path))
            .map_err(|_| AdmissionRejection::SourceUnavailable)?;
        if !examples_root.starts_with(root.as_path())
            || !example_path.starts_with(&examples_root)
            || std::path::Path::new(path).is_absolute()
        {
            return Err(AdmissionRejection::ScopeInvalid);
        }
        let relative = example_path
            .strip_prefix(root.as_path())
            .map_err(|_| AdmissionRejection::ScopeInvalid)?;
        let file = crate::artifact_io::open_beneath(root.as_path(), relative)
            .map_err(|_| AdmissionRejection::SourceUnavailable)?;
        let mut bytes = Vec::with_capacity(MAX_EXAMPLE_BYTES);
        file.take(MAX_EXAMPLE_BYTES as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| AdmissionRejection::SourceUnavailable)?;
        let excerpt = String::from_utf8_lossy(&bytes).trim().to_string();
        files.push(SourceGenerationFile {
            path: key.clone(),
            generation_id: content_hash(&excerpt),
            lines: excerpt.lines().map(str::to_string).collect(),
        });
        key_owners.insert(key, owner);
    }
    Ok(SourceGeneration::from_files(files))
}

#[cfg(test)]
mod refresh_tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::km_resolve::MAX_EXAMPLE_BYTES;
    use crate::lineage_admission::AdmissionRejection;
    use crate::lineage_admission::MAX_ADMITTED_SPANS;

    const EXAMPLE: &str = "intro line\nthe dispatcher validates every lease\ntrailing line";

    // Mirrors the existing retrieval fixture, whose helper is private to tests.
    fn seed_refresh_store(
        example: &[u8],
    ) -> Result<(tempfile::TempDir, KmStore), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let store = KmStore::new(dir.path().join("km-cache"));
        let root = dir.path().join("auto-km-lineage-0.1.0");
        std::fs::create_dir_all(root.join("examples"))?;
        std::fs::create_dir_all(root.join("tests"))?;
        std::fs::write(
            root.join("km.toml"),
            "id = \"codebase-graph/lineage\"\nversion = \"0.1.0\"\nkind = \"codebase-graph\"\nsummary = \"Dispatcher lease rules\"\nsources = [\"silver:doc\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n",
        )?;
        std::fs::write(
            root.join("facts.jsonl"),
            "{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"the dispatcher validates every lease\",\"source\":\"silver:doc\",\"confidence\":0.9}\n",
        )?;
        std::fs::write(root.join("examples/README.md"), example)?;
        std::fs::write(root.join("tests/README.md"), "test")?;
        std::fs::write(dir.path().join("doc"), "doc")?;
        store
            .import_for_project(&root, &dir.path().display().to_string())
            .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
        Ok((dir, store))
    }

    #[test]
    fn unchanged_examples_match_the_snapshot_with_bounded_lossy_excerpts()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut example = format!("  {EXAMPLE}\n").into_bytes();
        example.resize(MAX_EXAMPLE_BYTES - 1, b' ');
        example.extend_from_slice("é ignored suffix".as_bytes());
        let (dir, store) = seed_refresh_store(&example)?;
        let LineageAttempt::Selected {
            ledger,
            mut selection,
            generation,
            ..
        } = km_lineage_attempt(&store, "dispatcher")
        else {
            return Err("fixture must produce a selection".into());
        };
        let selected = selection
            .selected
            .first()
            .ok_or("fixture must select a passage")?
            .clone();
        selection
            .selected
            .extend(vec![selected; MAX_ADMITTED_SPANS]);
        let scope = dir.path().display().to_string();
        assert_eq!(
            refresh_selected(&store, &scope, &selection, &ledger, |_| true),
            Ok(generation)
        );
        Ok(())
    }

    #[test]
    fn a_modified_cached_example_cannot_reuse_the_retrieval_snapshot()
    -> Result<(), Box<dyn std::error::Error>> {
        let (dir, store) = seed_refresh_store(EXAMPLE.as_bytes())?;
        let LineageAttempt::Selected {
            ledger, selection, ..
        } = km_lineage_attempt(&store, "dispatcher")
        else {
            return Err("fixture must produce a selection".into());
        };
        let records = store
            .list(/*include_obsolete*/ false)
            .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
        let record = records.first().ok_or("fixture must contain a record")?;
        std::fs::write(
            std::path::Path::new(&record.path).join("examples/README.md"),
            "changed example",
        )?;
        let scope = dir.path().display().to_string();
        assert_eq!(
            refresh_selected(&store, &scope, &selection, &ledger, |_| true),
            Err(AdmissionRejection::SourceUnavailable)
        );
        Ok(())
    }

    #[test]
    fn invalid_scope_provenance_and_distinct_passage_counts_fail_closed()
    -> Result<(), Box<dyn std::error::Error>> {
        let (dir, store) = seed_refresh_store(EXAMPLE.as_bytes())?;
        let LineageAttempt::Selected {
            mut ledger,
            mut selection,
            ..
        } = km_lineage_attempt(&store, "dispatcher")
        else {
            return Err("fixture must produce a selection".into());
        };
        let scope = dir.path().display().to_string();
        assert_eq!(
            refresh_selected(&store, "other-project", &selection, &ledger, |_| true),
            Err(AdmissionRejection::ScopeInvalid)
        );
        assert_eq!(
            refresh_selected(&store, &scope, &selection, &ledger, |_| false),
            Err(AdmissionRejection::ScopeInvalid)
        );
        let passage = ledger
            .passages
            .first_mut()
            .ok_or("fixture must contain a lineage")?;
        passage.source_location.start_line = Some(0);
        assert_eq!(
            refresh_selected(&store, &scope, &selection, &ledger, |_| true),
            Err(AdmissionRejection::SchemaInvalid)
        );
        let selected = selection
            .selected
            .first()
            .ok_or("fixture must select a passage")?
            .clone();
        selection.selected = (0..=MAX_ADMITTED_SPANS)
            .map(|index| {
                let mut passage = selected.clone();
                passage.passage_id = format!("distinct-{index}");
                passage
            })
            .collect();
        assert_eq!(
            refresh_selected(&store, &scope, &selection, &ledger, |_| true),
            Err(AdmissionRejection::ByteCapExceeded)
        );
        Ok(())
    }
}
