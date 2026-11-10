use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use ontocode_utils_path::write_atomically;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::artifact_io::read_to_string;

pub(crate) const ONTOGRAPH_SCHEMA_VERSION: u32 = 1;
pub(crate) const ONTOCODE_DIR: &str = ".ontocode";
pub(crate) const ONTOGRAPH_DIR: &str = "ontograph";
const GENERATIONS_DIR: &str = "generations";
pub(crate) const RETAINED_GENERATIONS: usize = 2;
const CURRENT_GENERATION_FILE: &str = "current-generation";
const INDEX_REBUILD_LOCK_FILE: &str = "index.lock";
const MANIFEST_FILE: &str = "manifest.json";
const MAX_ARTIFACT_READ_BYTES: usize = 64 * 1024 * 1024;
const MAX_POINTER_READ_BYTES: usize = 4096;
pub(crate) const SOURCE_INDEX_FILE: &str = "source-index.json";
#[cfg(test)]
const KM_GOLD_DIR: &str = "km-gold";
#[cfg(test)]
const KM_GOLD_GENERATIONS_DIR: &str = "generations";
#[cfg(test)]
const KM_GOLD_CURRENT_GENERATION_FILE: &str = "current-generation";
#[cfg(test)]
const KM_GOLD_MANIFEST_FILE: &str = "manifest.json";
#[cfg(test)]
pub(crate) const KM_GOLD_FILE: &str = "km-gold.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RepositoryIdentity {
    pub root: String,
    pub vcs: String,
    pub remote_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ArtifactVersion {
    pub schema_version: u32,
    pub implementation_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct GenerationMetadata {
    pub generation_id: String,
    pub target_head: String,
    pub created_at_unix_seconds: i64,
    pub completed_at_unix_seconds: i64,
    pub manifest_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct GraphCountSummary {
    pub files: u64,
    pub symbols: u64,
    pub relationships: u64,
    pub nodes: u64,
    pub edges: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ArtifactGeneration {
    pub metadata: GenerationMetadata,
    pub counts: GraphCountSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct OntographManifest {
    pub repository: RepositoryIdentity,
    pub version: ArtifactVersion,
    pub active_generation_id: String,
    pub completed_generations: Vec<ArtifactGeneration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManifestExpectation {
    pub repository: RepositoryIdentity,
    pub version: ArtifactVersion,
    pub target_head: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OntographArtifactRoot {
    repository_root: PathBuf,
    root_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RebuildReason {
    SchemaVersionMismatch,
    ImplementationVersionMismatch,
    RepositoryMismatch,
    MissingActiveGeneration,
    TargetHeadMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactReadiness {
    Ready,
    RebuildRequired(ArtifactRebuildReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactRebuildReason {
    MissingCurrentPointer,
    CorruptCurrentPointer,
    MissingManifest,
    UnreadableManifest,
    InvalidManifest,
    ManifestMismatch(RebuildReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactPublishError {
    MissingActiveGeneration,
    InvalidGenerationId,
    SerializeManifest,
    WriteManifest,
    WriteCurrentPointer,
}

impl OntographManifest {
    pub(crate) fn rebuild_reason(
        &self,
        expectation: &ManifestExpectation,
    ) -> Option<RebuildReason> {
        if self.version.schema_version != expectation.version.schema_version {
            return Some(RebuildReason::SchemaVersionMismatch);
        }
        if self.version.implementation_version != expectation.version.implementation_version {
            return Some(RebuildReason::ImplementationVersionMismatch);
        }
        if self.repository != expectation.repository {
            return Some(RebuildReason::RepositoryMismatch);
        }

        let Some(active_generation) = self.active_generation() else {
            return Some(RebuildReason::MissingActiveGeneration);
        };
        if active_generation.metadata.target_head != expectation.target_head {
            return Some(RebuildReason::TargetHeadMismatch);
        }

        None
    }

    fn active_generation(&self) -> Option<&ArtifactGeneration> {
        self.completed_generations
            .iter()
            .find(|generation| generation.metadata.generation_id == self.active_generation_id)
    }
}

impl OntographArtifactRoot {
    pub(crate) fn new(repo_root: impl Into<PathBuf>) -> Self {
        let repository_root = repo_root.into();
        Self {
            root_path: repo_ontograph_root(&repository_root),
            repository_root,
        }
    }

    pub(crate) fn root_path(&self) -> PathBuf {
        self.root_path.clone()
    }

    pub(crate) fn repository_root(&self) -> &Path {
        &self.repository_root
    }

    pub(crate) fn generations_root(&self) -> PathBuf {
        self.root_path().join(GENERATIONS_DIR)
    }

    #[cfg(test)]
    pub(crate) fn km_gold_root_path(&self) -> PathBuf {
        self.root_path().join(KM_GOLD_DIR)
    }

    #[cfg(test)]
    pub(crate) fn km_gold_generations_root(&self) -> PathBuf {
        self.km_gold_root_path().join(KM_GOLD_GENERATIONS_DIR)
    }

    pub(crate) fn current_pointer_path(&self) -> PathBuf {
        self.root_path().join(CURRENT_GENERATION_FILE)
    }

    pub(crate) fn index_rebuild_lock_path(&self) -> PathBuf {
        self.root_path().join(INDEX_REBUILD_LOCK_FILE)
    }

    #[cfg(test)]
    pub(crate) fn km_gold_current_pointer_path(&self) -> PathBuf {
        self.km_gold_root_path()
            .join(KM_GOLD_CURRENT_GENERATION_FILE)
    }

    pub(crate) fn generation_dir(
        &self,
        generation_id: &str,
    ) -> Result<PathBuf, ArtifactPublishError> {
        validate_generation_id(generation_id)?;
        Ok(self.generations_root().join(generation_id))
    }

    #[cfg(test)]
    pub(crate) fn km_gold_generation_dir(
        &self,
        generation_id: &str,
    ) -> Result<PathBuf, ArtifactPublishError> {
        validate_generation_id(generation_id)?;
        Ok(self.km_gold_generations_root().join(generation_id))
    }

    pub(crate) fn generation_manifest_path(
        &self,
        generation_id: &str,
    ) -> Result<PathBuf, ArtifactPublishError> {
        self.generation_file_path(generation_id, MANIFEST_FILE)
    }

    pub(crate) fn generation_file_path(
        &self,
        generation_id: &str,
        file_name: &str,
    ) -> Result<PathBuf, ArtifactPublishError> {
        validate_generation_id(file_name)?;
        Ok(self.generation_dir(generation_id)?.join(file_name))
    }

    #[cfg(test)]
    pub(crate) fn km_gold_generation_manifest_path(
        &self,
        generation_id: &str,
    ) -> Result<PathBuf, ArtifactPublishError> {
        self.km_gold_generation_file_path(generation_id, KM_GOLD_MANIFEST_FILE)
    }

    #[cfg(test)]
    pub(crate) fn km_gold_generation_file_path(
        &self,
        generation_id: &str,
        file_name: &str,
    ) -> Result<PathBuf, ArtifactPublishError> {
        validate_generation_id(file_name)?;
        Ok(self.km_gold_generation_dir(generation_id)?.join(file_name))
    }

    pub(crate) fn publish_manifest(
        &self,
        manifest: &OntographManifest,
    ) -> Result<(), ArtifactPublishError> {
        let active_generation = manifest
            .active_generation()
            .ok_or(ArtifactPublishError::MissingActiveGeneration)?;
        let manifest_path =
            self.generation_manifest_path(&active_generation.metadata.generation_id)?;
        publish_manifest_at(manifest, manifest_path, self.current_pointer_path())
    }

    /// Removes older generation directories until at most [`RETAINED_GENERATIONS`]
    /// remain beside the active one.
    ///
    /// Victims are ordered by the `completed_at_unix_seconds` recorded in each
    /// generation's own manifest, with the generation id as a stable tie-breaker.
    /// A generation whose manifest is missing or unreadable sorts first, so an
    /// incomplete publish is reclaimed before a completed one. Removal failures
    /// are returned as warnings; pruning never fails publication.
    pub(crate) fn prune_generations(&self, active_generation_id: &str) -> Vec<String> {
        let entries = match fs::read_dir(self.generations_root()) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Vec::new(),
            Err(_) => return Vec::new(),
        };
        let mut candidates = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let generation_id = entry.file_name().into_string().ok()?;
                if generation_id == active_generation_id
                    || validate_generation_id(&generation_id).is_err()
                    || !entry.file_type().ok()?.is_dir()
                {
                    return None;
                }
                let path = entry.path();
                let completed_at = generation_completed_at(&path, &generation_id);
                Some((completed_at, generation_id, path))
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

        let mut warnings = Vec::new();
        let prune_count = candidates.len().saturating_sub(RETAINED_GENERATIONS);
        for (_, generation_id, path) in candidates.into_iter().take(prune_count) {
            if let Err(error) = fs::remove_dir_all(path) {
                warnings.push(format!(
                    "failed to prune generation {generation_id}: {:?}",
                    error.kind()
                ));
            }
        }
        warnings
    }

    #[cfg(test)]
    pub(crate) fn publish_km_gold_manifest(
        &self,
        manifest: &OntographManifest,
    ) -> Result<(), ArtifactPublishError> {
        let active_generation = manifest
            .active_generation()
            .ok_or(ArtifactPublishError::MissingActiveGeneration)?;
        let manifest_path =
            self.km_gold_generation_manifest_path(&active_generation.metadata.generation_id)?;
        publish_manifest_at(manifest, manifest_path, self.km_gold_current_pointer_path())
    }

    pub(crate) fn current_manifest_readiness(
        &self,
        expectation: &ManifestExpectation,
    ) -> ArtifactReadiness {
        let generation_id = match read_current_generation_id(&self.current_pointer_path()) {
            Ok(generation_id) => generation_id,
            Err(reason) => return ArtifactReadiness::RebuildRequired(reason),
        };
        let Ok(manifest_path) = self.generation_manifest_path(&generation_id) else {
            return ArtifactReadiness::RebuildRequired(
                ArtifactRebuildReason::CorruptCurrentPointer,
            );
        };
        let contents = match read_to_string(&manifest_path, MAX_ARTIFACT_READ_BYTES) {
            Ok(contents) => contents,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return ArtifactReadiness::RebuildRequired(ArtifactRebuildReason::MissingManifest);
            }
            Err(_) => {
                return ArtifactReadiness::RebuildRequired(
                    ArtifactRebuildReason::UnreadableManifest,
                );
            }
        };
        let manifest = match serde_json::from_str::<OntographManifest>(&contents) {
            Ok(manifest) => manifest,
            Err(_) => {
                return ArtifactReadiness::RebuildRequired(ArtifactRebuildReason::InvalidManifest);
            }
        };

        match manifest.rebuild_reason(expectation) {
            Some(reason) => {
                ArtifactReadiness::RebuildRequired(ArtifactRebuildReason::ManifestMismatch(reason))
            }
            None => ArtifactReadiness::Ready,
        }
    }
}

pub(crate) fn repo_ontograph_root(repo_root: &Path) -> PathBuf {
    repo_root.join(ONTOCODE_DIR).join(ONTOGRAPH_DIR)
}

fn publish_manifest_at(
    manifest: &OntographManifest,
    manifest_path: PathBuf,
    current_pointer_path: PathBuf,
) -> Result<(), ArtifactPublishError> {
    let active_generation = manifest
        .active_generation()
        .ok_or(ArtifactPublishError::MissingActiveGeneration)?;
    let contents = serde_json::to_string_pretty(manifest)
        .map_err(|_| ArtifactPublishError::SerializeManifest)?;
    write_atomically(&manifest_path, &format!("{contents}\n"))
        .map_err(|_| ArtifactPublishError::WriteManifest)?;
    write_atomically(
        &current_pointer_path,
        &format!("{}\n", active_generation.metadata.generation_id),
    )
    .map_err(|_| ArtifactPublishError::WriteCurrentPointer)
}

fn read_current_generation_id(path: &Path) -> Result<String, ArtifactRebuildReason> {
    let contents = match read_to_string(path, MAX_POINTER_READ_BYTES) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(ArtifactRebuildReason::MissingCurrentPointer);
        }
        Err(_) => return Err(ArtifactRebuildReason::CorruptCurrentPointer),
    };
    let generation_id = contents.trim();
    validate_generation_id(generation_id)
        .map_err(|_| ArtifactRebuildReason::CorruptCurrentPointer)?;
    Ok(generation_id.to_string())
}

/// Reads the completion timestamp a generation recorded for itself.
///
/// Returns `None` when the manifest is absent, unreadable, malformed, or lists
/// no generation, so pruning treats it as the oldest candidate.
fn generation_completed_at(generation_dir: &Path, generation_id: &str) -> Option<i64> {
    let contents =
        read_to_string(&generation_dir.join(MANIFEST_FILE), MAX_ARTIFACT_READ_BYTES).ok()?;
    let manifest = serde_json::from_str::<OntographManifest>(&contents).ok()?;
    let generation = manifest
        .completed_generations
        .iter()
        .find(|generation| generation.metadata.generation_id == generation_id)?;
    Some(generation.metadata.completed_at_unix_seconds)
}

fn validate_generation_id(generation_id: &str) -> Result<(), ArtifactPublishError> {
    let is_valid = !generation_id.is_empty()
        && generation_id != "."
        && generation_id != ".."
        && generation_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'));
    if is_valid {
        Ok(())
    } else {
        Err(ArtifactPublishError::InvalidGenerationId)
    }
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;
