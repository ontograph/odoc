use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::km::KmFact;
use crate::km::KmValidationStatus;
use crate::km::load_facts;
use crate::km::load_manifest;
use crate::km::validate_km;
use crate::km_llm_client::DraftMapping;
use crate::km_llm_client::KmLlmModelContext;
use crate::km_llm_client::is_reserved_dir;
use crate::km_llm_client::run_model_extract;
use crate::km_llm_client::validate_draft_mapping;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmLlmConsolidateRequest {
    pub input_dirs: Vec<PathBuf>,
    pub output_dir: PathBuf,
    pub project_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KmLlmConsolidateResult {
    pub output_dir: String,
    pub facts: usize,
    pub mappings: usize,
    pub conflicts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KmLlmConsolidateError {
    EmptyInputs,
    OutputDirNotReserved,
    OutputDirOutsideProjectRoot,
    OutputDirExists,
    MissingInput(String),
    InvalidInput(String),
    CreateOutputDir,
    WriteFile,
    ReadMappings,
    SerializeRow,
    InvalidDraft(Vec<String>),
}

pub fn run_km_llm_consolidate(
    request: &KmLlmConsolidateRequest,
    model_ctx: Option<KmLlmModelContext<'_>>,
) -> Result<KmLlmConsolidateResult, KmLlmConsolidateError> {
    if request.input_dirs.is_empty() {
        return Err(KmLlmConsolidateError::EmptyInputs);
    }
    let project_root = fs::canonicalize(&request.project_root)
        .map_err(|_| KmLlmConsolidateError::OutputDirOutsideProjectRoot)?;
    let output_dir = if request.output_dir.is_absolute() {
        request.output_dir.clone()
    } else {
        project_root.join(&request.output_dir)
    };
    if !is_reserved_dir(&output_dir) {
        return Err(KmLlmConsolidateError::OutputDirNotReserved);
    }
    let output_parent = output_dir
        .parent()
        .and_then(|parent| fs::canonicalize(parent).ok())
        .ok_or(KmLlmConsolidateError::OutputDirOutsideProjectRoot)?;
    if !output_parent.starts_with(&project_root) {
        return Err(KmLlmConsolidateError::OutputDirOutsideProjectRoot);
    }
    let output_dir = output_parent.join(
        output_dir
            .file_name()
            .ok_or(KmLlmConsolidateError::OutputDirNotReserved)?,
    );
    if output_dir.exists() {
        return Err(KmLlmConsolidateError::OutputDirExists);
    }

    let mut sources = BTreeSet::new();
    let mut facts: BTreeMap<String, KmFact> = BTreeMap::new();
    let mut mappings: BTreeMap<String, KmMapping> = BTreeMap::new();
    let mut conflicts = Vec::new();
    let mut conflict_relations: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for input_dir in &request.input_dirs {
        if !input_dir.is_dir() {
            return Err(KmLlmConsolidateError::MissingInput(
                input_dir.display().to_string(),
            ));
        }
        let manifest = load_manifest(input_dir).map_err(KmLlmConsolidateError::InvalidInput)?;
        sources.insert(format!("km:{}@{}", manifest.id, manifest.version));
        for source in manifest.sources.unwrap_or_default() {
            sources.insert(source);
        }
        for fact in load_facts(input_dir).map_err(KmLlmConsolidateError::InvalidInput)? {
            let key = normalized_fact_key(&fact);
            facts.entry(key).or_insert(fact);
        }
        for mapping in load_mappings(input_dir)? {
            if mapping.kind == "conflict_record" {
                conflicts.push(mapping);
                continue;
            }
            let key = normalized_mapping_key(&mapping);
            match mappings.get(&key) {
                Some(existing) if existing.relation != mapping.relation => {
                    conflict_relations
                        .entry(key.clone())
                        .or_default()
                        .extend([existing.relation.clone(), mapping.relation.clone()]);
                    conflicts.push(conflict_for(existing, &mapping, conflicts.len() + 1));
                }
                Some(_) => {}
                None => {
                    mappings.insert(key, mapping);
                }
            }
        }
    }

    // The model adjudicates only ambiguous deterministic clusters. When no
    // conflict was found there is nothing to adjudicate, so the deterministic
    // result is used unchanged. `run_model_extract` enforces the enable gate,
    // offline-only policy, and token caps, and returns `None` on any failure so
    // consolidation always falls back to the deterministic merge.
    if let Some(ctx) = model_ctx
        && !conflicts.is_empty()
    {
        let prompt = assemble_conflict_prompt(&conflicts);
        if let Some(draft) = run_model_extract(ctx.config, &prompt, ctx.model) {
            apply_model_resolutions(
                &mut mappings,
                &mut conflicts,
                &conflict_relations,
                &draft.mappings,
            );
        }
    }

    let staging = staging_path(&output_dir);
    let mut staging_created = false;
    let result = (|| {
        fs::create_dir(&staging).map_err(|_| KmLlmConsolidateError::CreateOutputDir)?;
        staging_created = true;
        fs::create_dir(staging.join("examples"))
            .map_err(|_| KmLlmConsolidateError::CreateOutputDir)?;
        fs::create_dir(staging.join("tests"))
            .map_err(|_| KmLlmConsolidateError::CreateOutputDir)?;

        write_manifest(&staging, &sources)?;
        write_facts(&staging, facts.values())?;
        write_mappings(&staging, mappings.values(), &conflicts)?;
        fs::write(
            staging.join("examples/README.md"),
            "# Consolidated graph example\n",
        )
        .map_err(|_| KmLlmConsolidateError::WriteFile)?;
        fs::write(
            staging.join("tests/README.md"),
            "# Consolidated graph test\n",
        )
        .map_err(|_| KmLlmConsolidateError::WriteFile)?;

        let validation = validate_km(&staging);
        if validation.status != KmValidationStatus::Valid {
            return Err(KmLlmConsolidateError::InvalidDraft(validation.errors));
        }
        fs::rename(&staging, &output_dir).map_err(|_| KmLlmConsolidateError::WriteFile)
    })();
    result.inspect_err(|_| {
        if staging_created {
            let _ = fs::remove_dir_all(&staging);
        }
    })?;

    Ok(KmLlmConsolidateResult {
        output_dir: output_dir.display().to_string(),
        facts: facts.len(),
        mappings: mappings.len() + conflicts.len(),
        conflicts: conflicts.len(),
    })
}

impl std::fmt::Display for KmLlmConsolidateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyInputs => write!(f, "at least one input KM directory is required"),
            Self::OutputDirNotReserved => {
                write!(
                    f,
                    "output directory must be a reserved underscore directory"
                )
            }
            Self::OutputDirOutsideProjectRoot => {
                write!(
                    f,
                    "output directory must be inside the canonical project root"
                )
            }
            Self::OutputDirExists => write!(f, "output directory already exists"),
            Self::MissingInput(path) => write!(f, "input KM directory is missing: {path}"),
            Self::InvalidInput(error) => write!(f, "input KM package is invalid: {error}"),
            Self::CreateOutputDir => write!(f, "failed to create output directory"),
            Self::WriteFile => write!(f, "failed to write consolidated KM file"),
            Self::ReadMappings => write!(f, "failed to read mappings.jsonl"),
            Self::SerializeRow => write!(f, "failed to serialize consolidated KM row"),
            Self::InvalidDraft(errors) if !errors.is_empty() => {
                write!(
                    f,
                    "generated consolidated KM is invalid: {}",
                    errors.join("; ")
                )
            }
            Self::InvalidDraft(_) => write!(f, "generated consolidated KM is invalid"),
        }
    }
}

impl std::error::Error for KmLlmConsolidateError {}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct KmMapping {
    id: String,
    kind: String,
    source_concept: String,
    target_concept: String,
    relation: String,
    source: String,
    confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolution: Option<String>,
}

fn load_mappings(root: &Path) -> Result<Vec<KmMapping>, KmLlmConsolidateError> {
    let path = root.join("mappings.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path).map_err(|_| KmLlmConsolidateError::ReadMappings)?;
    text.lines()
        .map(|line| serde_json::from_str(line).map_err(|_| KmLlmConsolidateError::ReadMappings))
        .collect()
}

fn normalized_fact_key(fact: &KmFact) -> String {
    format!(
        "{}:{}:{}",
        normalize_token(&fact.kind),
        normalize_sentence(&fact.claim),
        normalize_sentence(&fact.source)
    )
}

fn normalized_mapping_key(mapping: &KmMapping) -> String {
    endpoint_key(&mapping.source_concept, &mapping.target_concept)
}

fn endpoint_key(source_concept: &str, target_concept: &str) -> String {
    format!(
        "{}:{}",
        normalize_token(source_concept),
        normalize_token(target_concept)
    )
}

/// Assemble a metadata-only prompt describing the competing mappings the model
/// should adjudicate. No source bodies, secrets, or endpoints are included.
fn assemble_conflict_prompt(conflicts: &[KmMapping]) -> String {
    let mut prompt =
        String::from("Choose one relation for each competing concept mapping below.\n\n");
    for conflict in conflicts {
        prompt.push_str(&format!(
            "- {} -> {}: {}\n",
            conflict.source_concept,
            conflict.target_concept,
            conflict
                .resolution
                .as_deref()
                .unwrap_or("competing relations")
        ));
    }
    prompt
}

/// Apply model-chosen relations to conflicting clusters. A model row is honored
/// only when its normalized endpoints match a cluster that is actually in
/// conflict, so the model cannot introduce new endpoints outside the
/// deterministic candidate set.
fn apply_model_resolutions(
    mappings: &mut BTreeMap<String, KmMapping>,
    conflicts: &mut Vec<KmMapping>,
    conflict_relations: &BTreeMap<String, BTreeSet<String>>,
    model_mappings: &[DraftMapping],
) {
    let conflict_keys: BTreeSet<String> = conflicts
        .iter()
        .map(|conflict| endpoint_key(&conflict.source_concept, &conflict.target_concept))
        .collect();
    for row in model_mappings {
        if !validate_draft_mapping(row) {
            continue;
        }
        let key = endpoint_key(&row.source_concept, &row.target_concept);
        if !conflict_keys.contains(&key) {
            continue;
        }
        if !conflict_relations
            .get(&key)
            .is_some_and(|relations| relations.contains(&row.relation))
        {
            continue;
        }
        if let Some(surviving) = mappings.get_mut(&key) {
            surviving.relation = row.relation.clone();
            surviving.resolution = Some(format!(
                "Model adjudication selected relation `{}` for the competing cluster.",
                row.relation
            ));
        }
        conflicts.retain(|conflict| {
            endpoint_key(&conflict.source_concept, &conflict.target_concept) != key
        });
    }
}

fn normalize_token(value: &str) -> String {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join("_")
}

fn normalize_sentence(value: &str) -> String {
    value
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

fn conflict_for(existing: &KmMapping, incoming: &KmMapping, index: usize) -> KmMapping {
    KmMapping {
        id: format!("draft-consolidate-conflict-{index:03}"),
        kind: "conflict_record".to_string(),
        source_concept: existing.source_concept.clone(),
        target_concept: existing.target_concept.clone(),
        relation: "conflicts_with".to_string(),
        source: existing.source.clone(),
        confidence: existing.confidence.min(incoming.confidence),
        resolution: Some(format!(
            "Deterministic consolidation found competing relations `{}` and `{}` for the same normalized endpoints.",
            existing.relation, incoming.relation
        )),
    }
}

fn staging_path(output_dir: &Path) -> PathBuf {
    let name = output_dir
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    output_dir.with_file_name(format!(
        ".{name}.staging-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ))
}

fn write_manifest(
    output_dir: &Path,
    sources: &BTreeSet<String>,
) -> Result<(), KmLlmConsolidateError> {
    let sources = sources
        .iter()
        .map(|source| format!("  \"{source}\","))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(
        output_dir.join("km.toml"),
        format!(
            "id = \"codebase-graph/draft-consolidated\"\nversion = \"0.1.0\"\nkind = \"codebase-graph\"\nsummary = \"Draft KM package emitted by the local deterministic graph consolidator stage.\"\nsources = [\n{sources}\n]\nlicenses = [\"MIT\"]\n\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-06\"\nsource_commit = \"HEAD\"\n"
        ),
    )
    .map_err(|_| KmLlmConsolidateError::WriteFile)
}

fn write_facts<'a>(
    output_dir: &Path,
    facts: impl Iterator<Item = &'a KmFact>,
) -> Result<(), KmLlmConsolidateError> {
    let mut file = fs::File::create(output_dir.join("facts.jsonl"))
        .map_err(|_| KmLlmConsolidateError::WriteFile)?;
    for fact in facts {
        write_jsonl(&mut file, fact)?;
    }
    Ok(())
}

fn write_mappings<'a>(
    output_dir: &Path,
    mappings: impl Iterator<Item = &'a KmMapping>,
    conflicts: &[KmMapping],
) -> Result<(), KmLlmConsolidateError> {
    let mut file = fs::File::create(output_dir.join("mappings.jsonl"))
        .map_err(|_| KmLlmConsolidateError::WriteFile)?;
    for mapping in mappings {
        write_jsonl(&mut file, mapping)?;
    }
    for conflict in conflicts {
        write_jsonl(&mut file, conflict)?;
    }
    Ok(())
}

fn write_jsonl(file: &mut fs::File, row: &impl Serialize) -> Result<(), KmLlmConsolidateError> {
    let line = serde_json::to_string(row).map_err(|_| KmLlmConsolidateError::SerializeRow)?;
    writeln!(file, "{line}").map_err(|_| KmLlmConsolidateError::WriteFile)
}

#[cfg(test)]
#[path = "km_llm_consolidate_tests.rs"]
mod tests;
