use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use ontocode_utils_path::write_atomically;
use serde::Deserialize;
use serde::Serialize;

use crate::km::load_manifest;
use crate::km_llm_client::KmLlmModelContext;
use crate::km_llm_client::run_model_text;

const MAX_RECIPE_SUMMARY_CHARS: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmLlmRecipeRequest {
    pub package_dir: PathBuf,
    pub generated_outputs: Vec<String>,
    pub project_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KmLlmRecipeResult {
    pub recipe_path: String,
    pub generated_outputs: usize,
    pub conflicts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KmLlmRecipeError {
    MissingPackage,
    PackageOutsideProjectRoot,
    InvalidPackage(String),
    ReadMappings,
    WriteRecipe,
    SerializeRecipe,
}

pub fn run_km_llm_recipe(
    request: &KmLlmRecipeRequest,
    model_ctx: Option<KmLlmModelContext<'_>>,
) -> Result<KmLlmRecipeResult, KmLlmRecipeError> {
    let project_root = fs::canonicalize(&request.project_root)
        .map_err(|_| KmLlmRecipeError::PackageOutsideProjectRoot)?;
    let package_dir =
        fs::canonicalize(&request.package_dir).map_err(|_| KmLlmRecipeError::MissingPackage)?;
    if !package_dir.is_dir() {
        return Err(KmLlmRecipeError::MissingPackage);
    }
    if !package_dir.starts_with(&project_root) {
        return Err(KmLlmRecipeError::PackageOutsideProjectRoot);
    }
    let manifest = load_manifest(&package_dir).map_err(KmLlmRecipeError::InvalidPackage)?;
    let generated_outputs = generated_outputs(request);
    let conflicts = load_conflict_ids(&package_dir)?;
    let summary = model_ctx
        .and_then(|ctx| {
            run_model_text(
                ctx.config,
                &assemble_recipe_prompt(
                    &manifest.id,
                    &manifest.version,
                    &generated_outputs,
                    &conflicts,
                ),
                ctx.model,
            )
        })
        .and_then(|summary| normalize_summary(&summary))
        .unwrap_or_else(|| {
            format!(
                "File-only recipe metadata for {}@{}.",
                manifest.id, manifest.version
            )
        });
    let recipe = RecipeFile {
        id: format!("recipe-{}-v1", manifest.id.replace('/', "-")),
        kind: "km_transformation_recipe".to_string(),
        summary,
        composition: RecipeComposition {
            generated_outputs: generated_outputs.iter().cloned().collect(),
            conflicts: conflicts
                .iter()
                .cloned()
                .map(|id| RecipeConflict { id })
                .collect(),
        },
    };

    let recipe_path = package_dir.join("recipe.json");
    let contents =
        serde_json::to_string_pretty(&recipe).map_err(|_| KmLlmRecipeError::SerializeRecipe)?;
    write_atomically(&recipe_path, &format!("{contents}\n"))
        .map_err(|_| KmLlmRecipeError::WriteRecipe)?;
    Ok(KmLlmRecipeResult {
        recipe_path: recipe_path.display().to_string(),
        generated_outputs: generated_outputs.len(),
        conflicts: conflicts.len(),
    })
}

impl std::fmt::Display for KmLlmRecipeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPackage => write!(f, "KM package directory is missing"),
            Self::PackageOutsideProjectRoot => {
                write!(
                    f,
                    "KM package directory must be inside the canonical project root"
                )
            }
            Self::InvalidPackage(error) => write!(f, "KM package is invalid: {error}"),
            Self::ReadMappings => write!(f, "failed to read mappings.jsonl"),
            Self::WriteRecipe => write!(f, "failed to write recipe.json"),
            Self::SerializeRecipe => write!(f, "failed to serialize recipe.json"),
        }
    }
}

impl std::error::Error for KmLlmRecipeError {}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct RecipeFile {
    id: String,
    kind: String,
    summary: String,
    composition: RecipeComposition,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct RecipeComposition {
    generated_outputs: Vec<String>,
    conflicts: Vec<RecipeConflict>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct RecipeConflict {
    id: String,
}

#[derive(Debug, Deserialize)]
struct MappingRow {
    id: String,
    kind: String,
}

fn assemble_recipe_prompt(
    id: &str,
    version: &str,
    generated_outputs: &BTreeSet<String>,
    conflicts: &BTreeSet<String>,
) -> String {
    let generated_outputs = generated_outputs
        .iter()
        .map(|output| format!("- {output}"))
        .collect::<Vec<_>>()
        .join("\n");
    let conflicts = conflicts
        .iter()
        .map(|conflict| format!("- {conflict}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Write one plain-text summary sentence for recipe.json for KM package {id}@{version}. Keep it under {MAX_RECIPE_SUMMARY_CHARS} characters. Mention that this is authoring metadata only. Do not mention providers, models, secrets, or credentials.\n\nGenerated outputs:\n{generated_outputs}\n\nConflict ids:\n{conflicts}\n"
    )
}

fn normalize_summary(summary: &str) -> Option<String> {
    let summary = summary.split_whitespace().collect::<Vec<_>>().join(" ");
    if summary.is_empty() || summary.chars().count() > MAX_RECIPE_SUMMARY_CHARS {
        return None;
    }
    Some(summary)
}

fn generated_outputs(request: &KmLlmRecipeRequest) -> BTreeSet<String> {
    if request.generated_outputs.is_empty() {
        return [
            "km.toml",
            "facts.jsonl",
            "mappings.jsonl",
            "recipe.json",
            "examples/README.md",
            "tests/README.md",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
    }
    request
        .generated_outputs
        .iter()
        .filter(|output| !output.trim().is_empty())
        .cloned()
        .collect()
}

fn load_conflict_ids(root: &Path) -> Result<BTreeSet<String>, KmLlmRecipeError> {
    let path = root.join("mappings.jsonl");
    if !path.exists() {
        return Ok(BTreeSet::new());
    }
    let contents = fs::read_to_string(path).map_err(|_| KmLlmRecipeError::ReadMappings)?;
    let mut conflicts = BTreeSet::new();
    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let row =
            serde_json::from_str::<MappingRow>(line).map_err(|_| KmLlmRecipeError::ReadMappings)?;
        if row.kind == "conflict_record" {
            conflicts.insert(row.id);
        }
    }
    Ok(conflicts)
}

#[cfg(test)]
#[path = "km_llm_recipe_tests.rs"]
mod tests;
