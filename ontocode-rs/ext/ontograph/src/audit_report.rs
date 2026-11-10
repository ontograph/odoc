use std::io;

use ontocode_extension_api::FunctionCallError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::artifact_io::read_to_string;
use crate::audit_build::AuditAction;
use crate::indexer::SourceIndexArtifact;
use crate::indexer::SourceIndexLoadError;
use crate::indexer::SourceIndexStore;
use crate::manifest::OntographArtifactRoot;
use crate::manifest::SOURCE_INDEX_FILE;
use crate::tool::DONOR_ACTIONS_IMPLEMENTED;
use crate::tool::DONOR_ACTIONS_INTENTIONAL_DIVERGENCE;
use crate::tool::DONOR_ACTIONS_TOTAL;
use crate::tool::OntographThreadState;
use crate::tool::discovered_pack_summaries;
use crate::tool::donor_actions_missing;
use crate::tool::ensure_current_source_index_ready;

const MISSING_AUDIT_ACTIONS: [&str; 2] = ["audit.resource_trace", "audit.taint"];

const INTENTIONAL_EXCLUSIONS: [&str; 6] = [
    "graph database",
    "donor runtime",
    "scheduler/planner/router/registry",
    "execution fan-out",
    "host exposure",
    "raw Cypher",
];

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditReportResult {
    pub action: AuditAction,
    pub source_index: AuditReportSourceIndexSummary,
    pub audit_surface: AuditReportSurfaceSummary,
    pub graph: AuditReportGraphSummary,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditReportSourceIndexSummary {
    pub ready: bool,
    pub status: AuditReportSourceIndexStatus,
    pub files: usize,
    pub symbols: usize,
    pub relationships: usize,
    pub routes: usize,
    pub unresolved_calls: usize,
    pub processes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditReportSourceIndexStatus {
    Ready,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditReportSurfaceSummary {
    pub implemented_native_audit_actions: Vec<String>,
    pub missing_audit_actions: Vec<String>,
    pub donor_actions_total: usize,
    pub donor_actions_implemented: usize,
    pub donor_actions_intentional_divergence: usize,
    pub donor_actions_missing: usize,
    pub intentional_exclusions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditReportGraphSummary {
    pub files: usize,
    pub symbols: usize,
    pub relationships: usize,
    pub routes: usize,
    pub unresolved_calls: usize,
    pub processes: usize,
}

pub(crate) fn run_report_audit(
    thread_state: &OntographThreadState,
) -> Result<AuditReportResult, FunctionCallError> {
    let artifact_root = thread_state.current_artifact_root().ok_or_else(|| {
        FunctionCallError::RespondToModel(
            "ontograph.audit workspace context is unavailable for this turn".to_string(),
        )
    })?;
    let source_index = load_report_source_index(thread_state, &artifact_root)?;
    let graph = AuditReportGraphSummary {
        files: source_index.files,
        symbols: source_index.symbols,
        relationships: source_index.relationships,
        routes: source_index.routes,
        unresolved_calls: source_index.unresolved_calls,
        processes: source_index.processes,
    };

    Ok(AuditReportResult {
        action: AuditAction::Report,
        source_index,
        audit_surface: AuditReportSurfaceSummary {
            implemented_native_audit_actions: implemented_native_audit_actions(),
            missing_audit_actions: MISSING_AUDIT_ACTIONS
                .iter()
                .map(|action| (*action).to_string())
                .collect(),
            donor_actions_total: DONOR_ACTIONS_TOTAL,
            donor_actions_implemented: DONOR_ACTIONS_IMPLEMENTED,
            donor_actions_intentional_divergence: DONOR_ACTIONS_INTENTIONAL_DIVERGENCE,
            donor_actions_missing: donor_actions_missing(),
            intentional_exclusions: INTENTIONAL_EXCLUSIONS
                .iter()
                .map(|exclusion| (*exclusion).to_string())
                .collect(),
        },
        graph,
        limits: vec![
            "report reads only the active source-index.json generation".to_string(),
            "audit surface metadata is static and bounded to native Ontograph actions".to_string(),
        ],
        notes: vec![
            "audit.report does not build, refresh, or mutate source-index artifacts".to_string(),
            "missing source-index state is reported as readiness only".to_string(),
            "stale or corrupt source-index state fails closed through existing source-index readiness checks".to_string(),
            "the native source index currently analyzes Rust source only".to_string(),
            "call relationships use a conservative lexical unique-name heuristic".to_string(),
            "unresolved or ambiguous calls remain explicit and must not be treated as proven absent dependencies".to_string(),
            "source inspection remains authority for safety-critical conclusions".to_string(),
        ],
    })
}

fn load_report_source_index(
    thread_state: &OntographThreadState,
    artifact_root: &OntographArtifactRoot,
) -> Result<AuditReportSourceIndexSummary, FunctionCallError> {
    match SourceIndexStore::load_active_generation(artifact_root) {
        Ok(_store) => {
            ensure_current_source_index_ready(thread_state, "ontograph.audit")?;
            let artifact = read_active_source_index_artifact(artifact_root)?;
            Ok(AuditReportSourceIndexSummary {
                ready: true,
                status: AuditReportSourceIndexStatus::Ready,
                files: artifact.files.len(),
                symbols: artifact.files.iter().map(|file| file.symbols.len()).sum(),
                relationships: artifact.relationships.len(),
                routes: artifact.routes.len(),
                unresolved_calls: artifact.unresolved_calls.len(),
                processes: artifact.processes.len(),
            })
        }
        Err(
            SourceIndexLoadError::MissingCurrentPointer | SourceIndexLoadError::MissingSourceIndex,
        ) => Ok(AuditReportSourceIndexSummary {
            ready: false,
            status: AuditReportSourceIndexStatus::Missing,
            files: 0,
            symbols: 0,
            relationships: 0,
            routes: 0,
            unresolved_calls: 0,
            processes: 0,
        }),
        Err(SourceIndexLoadError::InvalidCurrentPointer)
        | Err(SourceIndexLoadError::ReadSourceIndex)
        | Err(SourceIndexLoadError::InvalidSourceIndex) => Err(FunctionCallError::RespondToModel(
            "ontograph.audit failed to load the native source index".to_string(),
        )),
    }
}

pub(crate) fn load_trace_boundary_source_index(
    thread_state: &OntographThreadState,
) -> Result<SourceIndexArtifact, FunctionCallError> {
    let artifact_root = thread_state.current_artifact_root().ok_or_else(|| {
        FunctionCallError::RespondToModel(
            "ontograph.audit workspace context is unavailable for this turn".to_string(),
        )
    })?;

    SourceIndexStore::load_active_generation(&artifact_root).map_err(|_| {
        FunctionCallError::RespondToModel(
            "ontograph.audit active source index is unavailable".to_string(),
        )
    })?;
    ensure_current_source_index_ready(thread_state, "ontograph.audit")?;
    read_active_source_index_artifact(&artifact_root)
}

fn read_active_source_index_artifact(
    artifact_root: &OntographArtifactRoot,
) -> Result<SourceIndexArtifact, FunctionCallError> {
    let generation_id =
        read_to_string(&artifact_root.current_pointer_path(), 4096).map_err(|err| {
            match err.kind() {
                io::ErrorKind::NotFound => FunctionCallError::RespondToModel(
                    "ontograph.audit active source index is missing".to_string(),
                ),
                _ => FunctionCallError::RespondToModel(
                    "ontograph.audit active source index pointer is invalid".to_string(),
                ),
            }
        })?;
    let path = artifact_root
        .generation_file_path(generation_id.trim(), SOURCE_INDEX_FILE)
        .map_err(|_| {
            FunctionCallError::RespondToModel(
                "ontograph.audit active source index pointer is invalid".to_string(),
            )
        })?;
    let contents = read_to_string(&path, 64 * 1024 * 1024).map_err(|err| match err.kind() {
        io::ErrorKind::NotFound => FunctionCallError::RespondToModel(
            "ontograph.audit source-index.json is missing".to_string(),
        ),
        _ => FunctionCallError::RespondToModel(
            "ontograph.audit failed to read source-index.json".to_string(),
        ),
    })?;
    serde_json::from_str::<SourceIndexArtifact>(&contents).map_err(|_| {
        FunctionCallError::RespondToModel("ontograph.audit source index is corrupt".to_string())
    })
}

fn implemented_native_audit_actions() -> Vec<String> {
    discovered_pack_summaries()
        .into_iter()
        .flat_map(|pack| pack.actions)
        .filter(|action| action.starts_with("audit."))
        .collect()
}
