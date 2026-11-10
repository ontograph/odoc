use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;

use crate::km::KmValidationStatus;
use crate::km::validate_km;
use crate::km_llm_client::DraftFact;
use crate::km_llm_client::DraftMapping;
use crate::km_llm_client::KmLlmModelContext;
use crate::km_llm_client::MODEL_EXTRACT_INSTRUCTION;
use crate::km_llm_client::canonical_row_source;
use crate::km_llm_client::is_reserved_dir;
use crate::km_llm_client::run_model_extract;
use crate::km_llm_client::validate_draft_fact;
use crate::km_llm_client::validate_draft_mapping;

const MAX_MODEL_PROMPT_BYTES: usize = 10_000;
/// Headroom reserved for the instruction header that the client prepends to the
/// assembled prompt before its own truncation. Full source coverage is only
/// claimed when the prompt still fits once that header is added, so a complete
/// prompt is never silently trimmed on the way to the model.
///
/// Derived from the client's own instruction rather than duplicated, so the two
/// cannot drift apart if that instruction is ever reworded or lengthened.
const MODEL_INSTRUCTION_HEADROOM_BYTES: usize = MODEL_EXTRACT_INSTRUCTION.len();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmLlmExtractRequest {
    pub sources: Vec<String>,
    pub output_dir: PathBuf,
    pub project_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KmLlmExtractResult {
    pub output_dir: String,
    pub facts: usize,
    pub mappings: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KmLlmExtractError {
    EmptySources,
    OutputDirNotReserved,
    OutputDirOutsideProjectRoot,
    OutputDirExists,
    InvalidSourcePrefix(String),
    MissingSource(String),
    CreateOutputDir,
    WriteFile,
    SerializeRow,
    InvalidDraft(Vec<String>),
}

/// Run the deterministic KM draft extractor, optionally consulting a
/// model-backed path first.
///
/// When `model_ctx` is `Some` and the stage is enabled, offline-only policy is
/// satisfied, and prompt caps hold, the extractor writes validated
/// model-derived draft rows. In every other case (including any model error) it
/// falls back to the deterministic scaffold, so a model call is never required.
pub fn run_km_llm_extract(
    request: &KmLlmExtractRequest,
    model_ctx: Option<KmLlmModelContext<'_>>,
) -> Result<KmLlmExtractResult, KmLlmExtractError> {
    if request.sources.is_empty() {
        return Err(KmLlmExtractError::EmptySources);
    }
    let project_root = fs::canonicalize(&request.project_root).map_err(|_| {
        KmLlmExtractError::MissingSource(request.project_root.display().to_string())
    })?;
    let output_dir = if request.output_dir.is_absolute() {
        request.output_dir.clone()
    } else {
        project_root.join(&request.output_dir)
    };
    if !is_reserved_dir(&output_dir) {
        return Err(KmLlmExtractError::OutputDirNotReserved);
    }
    let output_parent = output_dir
        .parent()
        .and_then(|parent| fs::canonicalize(parent).ok())
        .ok_or(KmLlmExtractError::OutputDirOutsideProjectRoot)?;
    if !output_parent.starts_with(&project_root) {
        return Err(KmLlmExtractError::OutputDirOutsideProjectRoot);
    }
    let output_dir = output_parent.join(
        output_dir
            .file_name()
            .ok_or(KmLlmExtractError::OutputDirNotReserved)?,
    );
    if output_dir.exists() {
        return Err(KmLlmExtractError::OutputDirExists);
    }

    let sources = request
        .sources
        .iter()
        .map(|source| parse_source(source, &project_root))
        .collect::<Result<Vec<_>, _>>()?;

    let prompt = assemble_prompt_text(&sources, &project_root)?;
    // Bounded Authoring Contract: a model draft is accepted only when every
    // requested source is fully represented in the bounded prompt. Partial
    // coverage skips the model call entirely and uses the deterministic path.
    let model_draft = model_ctx
        .filter(|_| prompt.covers_all_sources)
        .and_then(|ctx| run_model_extract(ctx.config, &prompt.text, ctx.model))
        .filter(|draft| !draft.facts.is_empty())
        .and_then(|mut draft| {
            for row in &mut draft.facts {
                row.source = canonical_row_source(&row.source, &project_root)?;
            }
            for row in &mut draft.mappings {
                row.source = canonical_row_source(&row.source, &project_root)?;
            }
            Some(draft)
        });

    let staging = staging_path(&output_dir);
    let mut staging_created = false;
    let result = (|| {
        fs::create_dir(&staging).map_err(|_| KmLlmExtractError::CreateOutputDir)?;
        staging_created = true;
        fs::create_dir(staging.join("examples")).map_err(|_| KmLlmExtractError::CreateOutputDir)?;
        fs::create_dir(staging.join("tests")).map_err(|_| KmLlmExtractError::CreateOutputDir)?;

        write_manifest(&staging, &sources)?;

        let (facts, mappings) = match model_draft {
            Some(draft) => {
                let facts = write_model_facts(&staging, &draft.facts, &project_root)?;
                let mappings = write_model_mappings(&staging, &draft.mappings, &project_root)?;
                (facts, mappings)
            }
            None => {
                write_facts(&staging, &sources)?;
                write_mappings(&staging, &sources)?;
                (sources.len(), sources.len())
            }
        };

        fs::write(
            staging.join("examples/README.md"),
            "# Draft extractor example\n",
        )
        .map_err(|_| KmLlmExtractError::WriteFile)?;
        fs::write(staging.join("tests/README.md"), "# Draft extractor test\n")
            .map_err(|_| KmLlmExtractError::WriteFile)?;

        let validation = validate_km(&staging);
        if validation.status != KmValidationStatus::Valid {
            return Err(KmLlmExtractError::InvalidDraft(validation.errors));
        }
        fs::rename(&staging, &output_dir).map_err(|_| KmLlmExtractError::WriteFile)?;
        Ok((facts, mappings))
    })();
    let (facts, mappings) = result.inspect_err(|_| {
        if staging_created {
            let _ = fs::remove_dir_all(&staging);
        }
    })?;

    Ok(KmLlmExtractResult {
        output_dir: output_dir.display().to_string(),
        facts,
        mappings,
    })
}

impl std::fmt::Display for KmLlmExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySources => write!(f, "at least one source is required"),
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
            Self::InvalidSourcePrefix(source) => {
                write!(f, "source must start with bronze: or silver: {source}")
            }
            Self::MissingSource(source) => write!(f, "source path does not exist: {source}"),
            Self::CreateOutputDir => write!(f, "failed to create output directory"),
            Self::WriteFile => write!(f, "failed to write draft KM file"),
            Self::SerializeRow => write!(f, "failed to serialize draft KM row"),
            Self::InvalidDraft(errors) if !errors.is_empty() => {
                write!(f, "generated draft KM is invalid: {}", errors.join("; "))
            }
            Self::InvalidDraft(_) => write!(f, "generated draft KM is invalid"),
        }
    }
}

impl std::error::Error for KmLlmExtractError {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedSource {
    raw: String,
    relative_path: String,
}

fn parse_source(source: &str, project_root: &Path) -> Result<ParsedSource, KmLlmExtractError> {
    let Some((prefix, relative_path)) = source.split_once(':') else {
        return Err(KmLlmExtractError::InvalidSourcePrefix(source.to_string()));
    };
    if prefix != "bronze" && prefix != "silver" {
        return Err(KmLlmExtractError::InvalidSourcePrefix(source.to_string()));
    }
    if relative_path.trim().is_empty()
        || Path::new(relative_path).is_absolute()
        || Path::new(relative_path).components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(KmLlmExtractError::InvalidSourcePrefix(source.to_string()));
    }
    let Some(canonical_source) = canonical_row_source(source, project_root) else {
        return Err(KmLlmExtractError::MissingSource(source.to_string()));
    };
    let Some((_, canonical_relative_path)) = canonical_source.split_once(':') else {
        return Err(KmLlmExtractError::InvalidSourcePrefix(source.to_string()));
    };
    let canonical_relative_path = canonical_relative_path.to_string();

    Ok(ParsedSource {
        raw: canonical_source,
        relative_path: canonical_relative_path,
    })
}

/// The bounded authoring prompt plus whether it represents every requested
/// source in full.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AssembledPrompt {
    text: String,
    covers_all_sources: bool,
}

/// Assemble the authoring prompt text from the resolved sources without ever
/// reading or retaining more than the aggregate prompt byte cap.
///
/// `covers_all_sources` is true only when every source was appended whole and
/// the result still leaves room for the client-side instruction header, so a
/// partially represented source can never back a model-derived draft.
fn assemble_prompt_text(
    sources: &[ParsedSource],
    project_root: &Path,
) -> Result<AssembledPrompt, KmLlmExtractError> {
    let mut prompt = String::from(
        "Extract codebase graph facts and concept mappings from the following source files.\n\n",
    );
    let mut covers_all_sources = true;
    for source in sources {
        let header = format!("--- Source: {} ---\n", source.raw);
        covers_all_sources &= push_bounded(&mut prompt, &header) == header.len();
        if prompt.len() >= MAX_MODEL_PROMPT_BYTES {
            return Ok(AssembledPrompt {
                text: prompt,
                covers_all_sources: false,
            });
        }
        let remaining = MAX_MODEL_PROMPT_BYTES - prompt.len();
        let path = project_root.join(&source.relative_path);
        let file = fs::File::open(&path)
            .map_err(|_| KmLlmExtractError::MissingSource(source.raw.clone()))?;
        let mut bytes = Vec::with_capacity(remaining.min(8 * 1024));
        // Read one byte past the budget: a short read proves the source ended
        // inside the budget, so it is fully represented.
        file.take(remaining as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| KmLlmExtractError::MissingSource(source.raw.clone()))?;
        covers_all_sources &= bytes.len() <= remaining;
        let body = String::from_utf8_lossy(&bytes);
        covers_all_sources &= push_bounded(&mut prompt, &body) == body.len();
        covers_all_sources &= push_bounded(&mut prompt, "\n\n") == 2;
    }
    // The client prepends its instruction header and truncates to the same cap,
    // so a prompt without that headroom would reach the model incomplete.
    covers_all_sources &= prompt.len() + MODEL_INSTRUCTION_HEADROOM_BYTES <= MAX_MODEL_PROMPT_BYTES;
    Ok(AssembledPrompt {
        text: prompt,
        covers_all_sources,
    })
}

/// Append as much of `value` as the cap allows, returning the bytes appended.
fn push_bounded(output: &mut String, value: &str) -> usize {
    let remaining = MAX_MODEL_PROMPT_BYTES.saturating_sub(output.len());
    let mut end = remaining.min(value.len());
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    output.push_str(&value[..end]);
    end
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

fn write_manifest(output_dir: &Path, sources: &[ParsedSource]) -> Result<(), KmLlmExtractError> {
    let sources = sources
        .iter()
        .map(|source| format!("  \"{}\",", source.raw))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(
        output_dir.join("km.toml"),
        format!(
            "id = \"codebase-graph/draft-extract\"\nversion = \"0.1.0\"\nkind = \"codebase-graph\"\nsummary = \"Draft KM facts emitted by the local deterministic extractor stage.\"\nsources = [\n{sources}\n]\nlicenses = [\"MIT\"]\n\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-06\"\nsource_commit = \"HEAD\"\n"
        ),
    )
    .map_err(|_| KmLlmExtractError::WriteFile)
}

fn write_facts(output_dir: &Path, sources: &[ParsedSource]) -> Result<(), KmLlmExtractError> {
    let mut file = fs::File::create(output_dir.join("facts.jsonl"))
        .map_err(|_| KmLlmExtractError::WriteFile)?;
    for (index, source) in sources.iter().enumerate() {
        let row = DraftFact {
            id: format!("draft-extract-fact-{:03}", index + 1),
            kind: "draft_source_reference".to_string(),
            claim: format!(
                "Draft extractor selected {} as KM source evidence.",
                source.relative_path
            ),
            source: source.raw.clone(),
            confidence: 0.5,
        };
        write_jsonl(&mut file, &row)?;
    }
    Ok(())
}

fn write_mappings(output_dir: &Path, sources: &[ParsedSource]) -> Result<(), KmLlmExtractError> {
    let mut file = fs::File::create(output_dir.join("mappings.jsonl"))
        .map_err(|_| KmLlmExtractError::WriteFile)?;
    for (index, source) in sources.iter().enumerate() {
        let row = DraftMapping {
            id: format!("draft-extract-mapping-{:03}", index + 1),
            kind: "draft_source_mapping".to_string(),
            source_concept: source.relative_path.clone(),
            target_concept: "ontograph_km_fact".to_string(),
            relation: "candidate_source_for".to_string(),
            source: source.raw.clone(),
            confidence: 0.5,
        };
        write_jsonl(&mut file, &row)?;
    }
    Ok(())
}

/// Write model-derived fact rows, dropping any row whose `source` no longer
/// resolves to a real prefixed project path.
fn write_model_facts(
    output_dir: &Path,
    facts: &[DraftFact],
    project_root: &Path,
) -> Result<usize, KmLlmExtractError> {
    let mut file = fs::File::create(output_dir.join("facts.jsonl"))
        .map_err(|_| KmLlmExtractError::WriteFile)?;
    let mut written = 0;
    for row in facts {
        if validate_draft_fact(row)
            && let Some(source) = canonical_row_source(&row.source, project_root)
        {
            let mut row = row.clone();
            row.source = source;
            write_jsonl(&mut file, &row)?;
            written += 1;
        }
    }
    Ok(written)
}

/// Write model-derived mapping rows, dropping any row whose `source` no longer
/// resolves to a real prefixed project path.
fn write_model_mappings(
    output_dir: &Path,
    mappings: &[DraftMapping],
    project_root: &Path,
) -> Result<usize, KmLlmExtractError> {
    let mut file = fs::File::create(output_dir.join("mappings.jsonl"))
        .map_err(|_| KmLlmExtractError::WriteFile)?;
    let mut written = 0;
    for row in mappings {
        if validate_draft_mapping(row)
            && let Some(source) = canonical_row_source(&row.source, project_root)
        {
            let mut row = row.clone();
            row.source = source;
            write_jsonl(&mut file, &row)?;
            written += 1;
        }
    }
    Ok(written)
}

fn write_jsonl(file: &mut fs::File, row: &impl Serialize) -> Result<(), KmLlmExtractError> {
    let line = serde_json::to_string(row).map_err(|_| KmLlmExtractError::SerializeRow)?;
    writeln!(file, "{line}").map_err(|_| KmLlmExtractError::WriteFile)
}

#[cfg(test)]
#[path = "km_llm_extract_tests.rs"]
mod tests;
