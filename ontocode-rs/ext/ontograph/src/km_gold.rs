use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use ontocode_utils_path::write_atomically;
use serde::Deserialize;
use serde::Serialize;

use crate::artifact_io::read_to_string;
use crate::km::KmFact;
use crate::km::KmManifest;
use crate::km::KmProvenance;
use crate::km::load_facts;
use crate::km::load_manifest;
use crate::km_store::KmStore;
use crate::km_store::KmStoreError;
use crate::manifest::ArtifactGeneration;
use crate::manifest::ArtifactPublishError;
use crate::manifest::ArtifactVersion;
use crate::manifest::GenerationMetadata;
use crate::manifest::GraphCountSummary;
use crate::manifest::KM_GOLD_FILE;
use crate::manifest::OntographArtifactRoot;
use crate::manifest::OntographManifest;
use crate::manifest::RepositoryIdentity;
use crate::query::BoundedItems;
use crate::query::QueryBudget;

const KM_GOLD_SCHEMA_VERSION: u32 = 1;
const MAX_KM_GOLD_POINTER_BYTES: usize = 4 * 1024;
const MAX_KM_GOLD_ARTIFACT_BYTES: usize = 4 * 1024 * 1024;
const MAX_KM_GOLD_OPTIONAL_JSON_BYTES: usize = 512 * 1024;
const MAX_KM_GOLD_FILE_ENTRIES: usize = 2_000;
const MAX_KM_GOLD_FILE_DEPTH: usize = 32;
const MAX_KM_GOLD_REFERENCED_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KmGoldBuildRequest {
    pub repo_root: PathBuf,
    pub km_store_root: PathBuf,
    pub generation_id: String,
    pub target_head: String,
    pub implementation_version: String,
    pub timestamp_unix_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct KmGoldArtifact {
    pub schema_version: u32,
    pub modules: Vec<KmGoldModule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct KmGoldModule {
    pub id: String,
    pub version: String,
    pub kind: String,
    pub summary: String,
    pub licenses: Vec<String>,
    pub provenance: KmProvenance,
    pub facts: Vec<KmFact>,
    pub examples: Vec<KmGoldFileRef>,
    pub tests: Vec<KmGoldFileRef>,
    pub sources: Vec<KmGoldSource>,
    pub recipe: Option<KmGoldRecipe>,
    pub mappings: Vec<KmGoldMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct KmGoldFileRef {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct KmGoldSource {
    pub id: String,
    pub kind: String,
    pub path: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct KmGoldRecipe {
    pub id: String,
    pub kind: String,
    pub summary: String,
    pub generated_outputs: Vec<String>,
    pub conflict_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct KmGoldMapping {
    pub id: String,
    pub kind: String,
    pub source_concept: String,
    pub target_concept: String,
    pub relation: String,
    pub source: String,
    pub confidence: f64,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct KmGoldModuleSummary {
    pub id: String,
    pub version: String,
    pub kind: String,
    pub facts: usize,
    pub sources: usize,
    pub mappings: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct KmGoldFactSummary {
    pub module_id: String,
    pub fact_id: String,
    pub kind: String,
    pub source: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct KmGoldSourceSummary {
    pub module_id: String,
    pub source_id: String,
    pub kind: String,
    pub path: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct KmGoldMappingSummary {
    pub module_id: String,
    pub mapping_id: String,
    pub kind: String,
    pub relation: String,
    pub source_concept: String,
    pub target_concept: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KmGoldError {
    Store(KmStoreError),
    Manifest,
    Facts,
    Sources,
    Recipe,
    Mappings,
    Examples,
    Tests,
    Serialize,
    WriteArtifact,
    PublishManifest(ArtifactPublishError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KmGoldLoadError {
    MissingCurrentPointer,
    InvalidCurrentPointer,
    MissingArtifact,
    ReadArtifact,
    InvalidArtifact,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KmGoldStore {
    artifact: KmGoldArtifact,
}

pub(crate) fn build_active_km_gold(
    request: KmGoldBuildRequest,
) -> Result<KmGoldArtifact, KmGoldError> {
    let store = KmStore::new(request.km_store_root.clone());
    let active_modules = store
        .list(/*include_obsolete*/ false)
        .map_err(KmGoldError::Store)?;
    let mut modules = active_modules
        .into_iter()
        .map(|record| load_gold_module(Path::new(&record.path)))
        .collect::<Result<Vec<_>, _>>()?;
    modules.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then(left.version.cmp(&right.version))
    });

    let artifact = KmGoldArtifact {
        schema_version: KM_GOLD_SCHEMA_VERSION,
        modules,
    };
    publish_gold_artifact(&request, &artifact)?;
    Ok(artifact)
}

impl KmGoldStore {
    pub(crate) fn load_active_generation(
        root: &OntographArtifactRoot,
    ) -> Result<Self, KmGoldLoadError> {
        let generation_id = read_to_string(
            &root.km_gold_current_pointer_path(),
            MAX_KM_GOLD_POINTER_BYTES,
        )
        .map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => KmGoldLoadError::MissingCurrentPointer,
            _ => KmGoldLoadError::InvalidCurrentPointer,
        })?;
        let artifact_path = root
            .km_gold_generation_file_path(generation_id.trim(), KM_GOLD_FILE)
            .map_err(|_| KmGoldLoadError::InvalidCurrentPointer)?;
        let contents =
            read_to_string(&artifact_path, MAX_KM_GOLD_ARTIFACT_BYTES).map_err(|err| match err
                .kind()
            {
                io::ErrorKind::NotFound => KmGoldLoadError::MissingArtifact,
                _ => KmGoldLoadError::ReadArtifact,
            })?;
        let artifact = serde_json::from_str::<KmGoldArtifact>(&contents)
            .map_err(|_| KmGoldLoadError::InvalidArtifact)?;
        Ok(Self { artifact })
    }

    pub(crate) fn module_summaries(
        &self,
        budget: QueryBudget,
    ) -> BoundedItems<KmGoldModuleSummary> {
        let items = self
            .artifact
            .modules
            .iter()
            .map(|module| KmGoldModuleSummary {
                id: module.id.clone(),
                version: module.version.clone(),
                kind: module.kind.clone(),
                facts: module.facts.len(),
                sources: module.sources.len(),
                mappings: module.mappings.len(),
            })
            .collect::<Vec<_>>();
        BoundedItems::new(items, budget)
    }

    pub(crate) fn fact_summaries(
        &self,
        module_id: &str,
        budget: QueryBudget,
    ) -> BoundedItems<KmGoldFactSummary> {
        let items = self
            .artifact
            .modules
            .iter()
            .filter(|module| module.id == module_id)
            .flat_map(|module| {
                module.facts.iter().map(|fact| KmGoldFactSummary {
                    module_id: module.id.clone(),
                    fact_id: fact.id.clone(),
                    kind: fact.kind.clone(),
                    source: fact.source.clone(),
                    confidence: fact.confidence,
                })
            })
            .collect::<Vec<_>>();
        BoundedItems::new(items, budget)
    }

    pub(crate) fn source_summaries(
        &self,
        module_id: &str,
        budget: QueryBudget,
    ) -> BoundedItems<KmGoldSourceSummary> {
        let items = self
            .artifact
            .modules
            .iter()
            .filter(|module| module.id == module_id)
            .flat_map(|module| {
                module.sources.iter().map(|source| KmGoldSourceSummary {
                    module_id: module.id.clone(),
                    source_id: source.id.clone(),
                    kind: source.kind.clone(),
                    path: source.path.clone(),
                    role: source.role.clone(),
                })
            })
            .collect::<Vec<_>>();
        BoundedItems::new(items, budget)
    }

    pub(crate) fn mapping_summaries(
        &self,
        module_id: &str,
        budget: QueryBudget,
    ) -> BoundedItems<KmGoldMappingSummary> {
        let items = self
            .artifact
            .modules
            .iter()
            .filter(|module| module.id == module_id)
            .flat_map(|module| {
                module.mappings.iter().map(|mapping| KmGoldMappingSummary {
                    module_id: module.id.clone(),
                    mapping_id: mapping.id.clone(),
                    kind: mapping.kind.clone(),
                    relation: mapping.relation.clone(),
                    source_concept: mapping.source_concept.clone(),
                    target_concept: mapping.target_concept.clone(),
                })
            })
            .collect::<Vec<_>>();
        BoundedItems::new(items, budget)
    }
}

fn load_gold_module(root: &Path) -> Result<KmGoldModule, KmGoldError> {
    let manifest = load_manifest(root).map_err(|_| KmGoldError::Manifest)?;
    let facts = load_facts(root).map_err(|_| KmGoldError::Facts)?;
    let examples = collect_relative_files(root, "examples").map_err(|_| KmGoldError::Examples)?;
    let tests = collect_relative_files(root, "tests").map_err(|_| KmGoldError::Tests)?;
    let sources = load_optional_jsonl::<KmGoldSource>(&root.join("sources.jsonl"))
        .map_err(|_| KmGoldError::Sources)?;
    let recipe =
        load_optional_recipe(&root.join("recipe.json")).map_err(|_| KmGoldError::Recipe)?;
    let mappings = load_optional_jsonl::<KmGoldMapping>(&root.join("mappings.jsonl"))
        .map_err(|_| KmGoldError::Mappings)?;
    build_gold_module(manifest, facts, examples, tests, sources, recipe, mappings)
}

fn build_gold_module(
    manifest: KmManifest,
    mut facts: Vec<KmFact>,
    mut examples: Vec<KmGoldFileRef>,
    mut tests: Vec<KmGoldFileRef>,
    mut sources: Vec<KmGoldSource>,
    recipe: Option<KmGoldRecipe>,
    mut mappings: Vec<KmGoldMapping>,
) -> Result<KmGoldModule, KmGoldError> {
    facts.sort_by(|left, right| left.id.cmp(&right.id));
    examples.sort_by(|left, right| left.path.cmp(&right.path));
    tests.sort_by(|left, right| left.path.cmp(&right.path));
    sources.sort_by(|left, right| left.id.cmp(&right.id).then(left.path.cmp(&right.path)));
    mappings.sort_by(|left, right| left.id.cmp(&right.id));

    Ok(KmGoldModule {
        id: manifest.id,
        version: manifest.version,
        kind: manifest.kind,
        summary: manifest.summary.ok_or(KmGoldError::Manifest)?,
        licenses: manifest.licenses.ok_or(KmGoldError::Manifest)?,
        provenance: manifest.provenance.ok_or(KmGoldError::Manifest)?,
        facts,
        examples,
        tests,
        sources,
        recipe,
        mappings,
    })
}

fn collect_relative_files(root: &Path, child: &str) -> Result<Vec<KmGoldFileRef>, io::Error> {
    let child_root = root.join(child);
    let mut files = Vec::new();
    let mut entry_count = 0;
    let mut referenced_bytes = 0;
    collect_files_from(
        &child_root,
        &mut files,
        0,
        &mut entry_count,
        &mut referenced_bytes,
    )?;
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| invalid_data("collected files should remain under root"))?;
            Ok(KmGoldFileRef {
                path: relative.to_string_lossy().replace('\\', "/"),
            })
        })
        .collect()
}

fn collect_files_from(
    path: &Path,
    files: &mut Vec<PathBuf>,
    depth: usize,
    entry_count: &mut usize,
    referenced_bytes: &mut u64,
) -> Result<(), io::Error> {
    if depth > MAX_KM_GOLD_FILE_DEPTH {
        return Err(invalid_data("KM gold file tree exceeds the depth limit"));
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        *entry_count = entry_count
            .checked_add(1)
            .ok_or_else(|| invalid_data("KM gold file entry count overflow"))?;
        if *entry_count > MAX_KM_GOLD_FILE_ENTRIES {
            return Err(invalid_data("KM gold file tree exceeds the entry limit"));
        }
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(invalid_data("KM gold file tree contains a symlink"));
        }
        if file_type.is_dir() {
            collect_files_from(&path, files, depth + 1, entry_count, referenced_bytes)?;
        } else if file_type.is_file() {
            *referenced_bytes = referenced_bytes
                .checked_add(entry.metadata()?.len())
                .ok_or_else(|| invalid_data("KM gold referenced byte count overflow"))?;
            if *referenced_bytes > MAX_KM_GOLD_REFERENCED_BYTES {
                return Err(invalid_data(
                    "KM gold referenced files exceed the byte limit",
                ));
            }
            files.push(path);
        }
    }
    Ok(())
}

fn load_optional_jsonl<T>(path: &Path) -> Result<Vec<T>, io::Error>
where
    T: for<'de> Deserialize<'de>,
{
    let contents = match read_to_string(path, MAX_KM_GOLD_OPTIONAL_JSON_BYTES) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };
    contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<T>(line).map_err(invalid_data))
        .collect()
}

fn load_optional_recipe(path: &Path) -> Result<Option<KmGoldRecipe>, io::Error> {
    let contents = match read_to_string(path, MAX_KM_GOLD_OPTIONAL_JSON_BYTES) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };
    let recipe = serde_json::from_str::<RecipeFile>(&contents).map_err(invalid_data)?;
    let mut generated_outputs = recipe.composition.generated_outputs;
    generated_outputs.sort();
    let mut conflict_ids = recipe
        .composition
        .conflicts
        .into_iter()
        .map(|conflict| conflict.id)
        .collect::<Vec<_>>();
    conflict_ids.sort();
    Ok(Some(KmGoldRecipe {
        id: recipe.id,
        kind: recipe.kind,
        summary: recipe.summary,
        generated_outputs,
        conflict_ids,
    }))
}

fn invalid_data(err: impl ToString) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err.to_string())
}

fn publish_gold_artifact(
    request: &KmGoldBuildRequest,
    artifact: &KmGoldArtifact,
) -> Result<(), KmGoldError> {
    let root = OntographArtifactRoot::new(&request.repo_root);
    let artifact_path = root
        .km_gold_generation_file_path(&request.generation_id, KM_GOLD_FILE)
        .map_err(KmGoldError::PublishManifest)?;
    let contents = serde_json::to_string_pretty(artifact).map_err(|_| KmGoldError::Serialize)?;
    write_atomically(&artifact_path, &format!("{contents}\n"))
        .map_err(|_| KmGoldError::WriteArtifact)?;
    root.publish_km_gold_manifest(&gold_manifest_for_request(request, artifact))
        .map_err(KmGoldError::PublishManifest)
}

fn gold_manifest_for_request(
    request: &KmGoldBuildRequest,
    artifact: &KmGoldArtifact,
) -> OntographManifest {
    let module_count = artifact.modules.len() as u64;
    let version_count = module_count;
    let fact_count = artifact
        .modules
        .iter()
        .map(|module| module.facts.len() as u64)
        .sum::<u64>();
    let example_count = artifact
        .modules
        .iter()
        .map(|module| module.examples.len() as u64)
        .sum::<u64>();
    let test_count = artifact
        .modules
        .iter()
        .map(|module| module.tests.len() as u64)
        .sum::<u64>();
    let source_count = artifact
        .modules
        .iter()
        .map(|module| module.sources.len() as u64)
        .sum::<u64>();
    let recipe_count = artifact
        .modules
        .iter()
        .filter(|module| module.recipe.is_some())
        .count() as u64;
    let mapping_count = artifact
        .modules
        .iter()
        .map(|module| module.mappings.len() as u64)
        .sum::<u64>();
    let nodes = module_count
        + version_count
        + fact_count
        + example_count
        + test_count
        + source_count
        + recipe_count
        + mapping_count;
    let edges =
        fact_count + example_count + test_count + source_count + mapping_count + recipe_count;

    OntographManifest {
        repository: RepositoryIdentity {
            root: request.repo_root.display().to_string(),
            vcs: "git".to_string(),
            remote_url: None,
        },
        version: ArtifactVersion {
            schema_version: artifact.schema_version,
            implementation_version: request.implementation_version.clone(),
        },
        active_generation_id: request.generation_id.clone(),
        completed_generations: vec![ArtifactGeneration {
            metadata: GenerationMetadata {
                generation_id: request.generation_id.clone(),
                target_head: request.target_head.clone(),
                created_at_unix_seconds: request.timestamp_unix_seconds,
                completed_at_unix_seconds: request.timestamp_unix_seconds,
                manifest_hash: format!(
                    "km-gold-v{}-modules-{}-facts-{}-mappings-{}",
                    artifact.schema_version, module_count, fact_count, mapping_count
                ),
            },
            counts: GraphCountSummary {
                files: module_count,
                symbols: fact_count + mapping_count,
                relationships: edges,
                nodes,
                edges,
            },
        }],
    }
}

#[derive(Debug, Deserialize)]
struct RecipeFile {
    id: String,
    kind: String,
    summary: String,
    composition: RecipeComposition,
}

#[derive(Debug, Deserialize)]
struct RecipeComposition {
    #[serde(default)]
    generated_outputs: Vec<String>,
    #[serde(default)]
    conflicts: Vec<RecipeConflict>,
}

#[derive(Debug, Deserialize)]
struct RecipeConflict {
    id: String,
}

#[cfg(test)]
#[path = "km_gold_tests.rs"]
mod tests;
