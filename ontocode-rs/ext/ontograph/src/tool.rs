use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::PoisonError;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use ontocode_extension_api::ContextualUserFragment;
use ontocode_extension_api::FunctionCallError;
use ontocode_extension_api::JsonToolOutput;
use ontocode_extension_api::ToolCall;
use ontocode_extension_api::ToolExecutor;
use ontocode_extension_api::ToolName;
use ontocode_extension_api::ToolOutput;
use ontocode_extension_api::ToolSpec;
use ontocode_extension_api::parse_tool_input_schema;
use ontocode_tools::ResponsesApiNamespace;
use ontocode_tools::ResponsesApiNamespaceTool;
use ontocode_tools::ResponsesApiTool;
use ontocode_tools::ToolExposure;
use ontocode_tools::default_namespace_description;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::artifact_io::read_to_string;
use crate::audit_build::AUDIT_TOOL_DESCRIPTION;
use crate::audit_build::AUDIT_TOOL_NAME;
use crate::docs::DocsIndexLoadError;
use crate::docs::DocsIndexStore;
use crate::docs::MAX_DOCS_CONTEXT_ITEMS;
use crate::docs::MAX_DOCS_EXCERPT_BYTES;
use crate::group_state::CURRENT_WORKSPACE_GROUP;
use crate::group_state::CurrentWorkspaceGroupState;
use crate::group_state::GroupStateError;
use crate::group_state::read_current_workspace_group_state;
use crate::group_state::write_current_workspace_group_state;
use crate::indexer::RustIndexError;
use crate::indexer::RustIndexRequest;
use crate::indexer::SourceIndexLoadError;
use crate::indexer::SourceIndexStore;
use crate::indexer::index_rust_workspace;
use crate::indexer::resolve_target_head;
use crate::indexer::rust_workspace_state_key;
use crate::indexer::source_index_counts;
use crate::km::MAX_RENDERED_EXAMPLES;
use crate::km::MAX_RENDERED_FACTS;
use crate::km_context::max_fragment_body_bytes;
use crate::km_context::render_km_context;
use crate::km_resolve::KmResolveError;
use crate::km_resolve::resolve_active_km;
use crate::km_store::KmIndexOrigin;
use crate::km_store::KmStore;
use crate::km_store::KmStoreError;
use crate::manifest::ArtifactReadiness;
use crate::manifest::ManifestExpectation;
use crate::manifest::OntographArtifactRoot;
use crate::manifest::OntographManifest;
use crate::query::DiffImpactRequest;
use crate::query::DocsContextRequest;
use crate::query::DocsDriftRequest;
use crate::query::DocsTraceRequest;
use crate::query::EvidenceRequest;
use crate::query::MAX_QUERY_ITEMS;
use crate::query::MAX_QUERY_TIMEOUT_MS;
use crate::query::MAX_SERIALIZED_BYTES;
use crate::query::ModuleContextRequest;
use crate::query::OntographReadQuery;
use crate::query::QueryBudget;
use crate::query::QueryError;
use crate::query::RepoMapRequest;
use crate::query::RouteImpactRequest;
use crate::query::RouteMapRequest;
use crate::query::RouteShapeRequest;
use crate::query::SymbolImpactRequest;
use crate::query::SymbolSearchRequest;

pub(crate) const ONTOGRAPH_NAMESPACE: &str = "ontograph";
pub(crate) const DISCOVER_TOOL_NAME: &str = "discover";
pub(crate) const DOCS_TOOL_NAME: &str = "docs";
pub(crate) const EXPLAIN_MODULE_TOOL_NAME: &str = "explain_module";
pub(crate) const IMPACT_TOOL_NAME: &str = "impact";
pub(crate) const INSPECT_TOOL_NAME: &str = "inspect";
pub(crate) const KM_CONTEXT_TOOL_NAME: &str = "km_context";
#[cfg(test)]
pub(crate) const PROCEDURAL_CONTEXT_TOOL_NAME: &str = "procedural_context";
pub(crate) const MANAGE_TOOL_NAME: &str = "manage";
pub(crate) const SEARCH_TOOL_NAME: &str = "search";
pub(crate) const EVIDENCE_PLANNER_TOOL_NAME: &str = "evidence_planner";
const EVIDENCE_PLANNER_DESCRIPTION: &str = "Plan and execute bounded, read-only, provenance-bearing graph retrieval over Ontograph evidence.";
/// Marker `resolve_target_head` reports when a checkout has no resolvable `HEAD`.
/// Mirrors the indexer's own unversioned marker; it is never immutable provenance.
const UNVERSIONED_WORKSPACE_HEAD: &str = "workspace-unversioned";
/// Bounded cap on KM procedural records returned by one planner projection.
const MAX_PLANNER_PROCEDURAL_RECORDS: usize = 32;

const DISCOVER_TOOL_DESCRIPTION: &str =
    "Report the live native Ontograph tool surface and checked-in parity metadata.";
const DOCS_DESCRIPTION: &str = "Read bounded docs-only context, trace, drift, and readiness from the active native Ontograph docs sidecar.";
const EXPLAIN_MODULE_DESCRIPTION: &str =
    "Inspect one local UTF-8 file and return bounded module facts without building an index.";
const IMPACT_DESCRIPTION: &str =
    "Read bounded symbol impact from the active native Ontograph source index.";
const INSPECT_DESCRIPTION: &str =
    "Read bounded context, evidence, or route shape from the active native Ontograph source index.";
const KM_CONTEXT_DESCRIPTION: &str =
    "Read one bounded active Knowledge Module context fragment from the native Ontograph KM cache.";
#[cfg(test)]
const PROCEDURAL_CONTEXT_DESCRIPTION: &str = "Read one bounded read-only procedural neighborhood for an active Knowledge Module procedure node from the native Ontograph KM cache.";
const MANAGE_DESCRIPTION: &str = "Build the native Ontograph source index, manage audit sessions, or read bounded route-map records from the active index.";
const SEARCH_DESCRIPTION: &str =
    "Read bounded symbol matches from the active native Ontograph source index.";
const PARITY_MATRIX_PATH: &str = ".memory-bank/knowledge-hub/ONTOGRAPH_DONOR_PARITY_MATRIX.md";
pub(crate) const DONOR_ACTIONS_TOTAL: usize = 62;
pub(crate) const DONOR_ACTIONS_IMPLEMENTED: usize = 56;
pub(crate) const DONOR_ACTIONS_INTENTIONAL_DIVERGENCE: usize = 0;
/// Donor actions tracked in the parity matrix but withdrawn from scope.
///
/// `inspect.ipc` remains a tracked row so its history and reintroduction gate
/// stay visible, but it is neither implemented nor missing work. Without this
/// term the missing count overstates remaining scope by one.
pub(crate) const DONOR_ACTIONS_WITHDRAWN: usize = 1;
pub(crate) const MAX_FILE_BYTES: usize = 1024 * 1024;
const MAX_SYMBOLS: usize = 32;
const IMPLEMENTATION_VERSION: &str = env!("CARGO_PKG_VERSION");
const RUST_SCAN_LIMIT_NOTE: &str = "Rust top_level_symbols use syn when the file parses; malformed Rust and non-Rust files use bounded line-based extraction";

#[derive(Clone, Default)]
pub(crate) struct DiscoverTool {
    thread_state: Arc<OntographThreadState>,
}

#[derive(Clone, Default)]
pub(crate) struct DocsTool {
    thread_state: Arc<OntographThreadState>,
}

#[derive(Clone, Default)]
pub(crate) struct ExplainModuleTool {
    thread_state: Arc<OntographThreadState>,
}

#[derive(Clone, Default)]
pub(crate) struct ImpactTool {
    thread_state: Arc<OntographThreadState>,
}

#[derive(Clone, Default)]
pub(crate) struct InspectTool {
    thread_state: Arc<OntographThreadState>,
}

#[derive(Clone)]
pub(crate) struct KmContextTool {
    km_store: Option<KmStore>,
    thread_state: Option<Arc<OntographThreadState>>,
    /// Host-owned identity of the calling thread, when the host supplies one.
    /// Agent-authored records are readable only by the thread that wrote them.
    thread_id: Option<String>,
}

#[derive(Clone, Default)]
pub(crate) struct ManageTool {
    thread_state: Arc<OntographThreadState>,
}

/// Local read-only inspection surface over already validated KM transitions.
///
/// This tool never mutates the graph and is never registered on the
/// app-server or default host; it mirrors `KmContextTool` ownership rules so
/// agent-authored records stay readable only by their authoring thread.
///
/// Registration is deliberately withheld: `AgentKnowledgeToolContributor`
/// publishes to the app-server embedded host, and exposing this verb there
/// would change an asserted host tool contract without an ADR. Until that ADR
/// exists the surface stays test-only.
#[cfg(test)]
#[derive(Clone)]
pub(crate) struct ProceduralContextTool {
    km_store: Option<KmStore>,
    thread_state: Option<Arc<OntographThreadState>>,
    thread_id: Option<String>,
}

#[derive(Clone, Default)]
pub(crate) struct SearchTool {
    thread_state: Arc<OntographThreadState>,
}

#[derive(Clone, Default)]
pub(crate) struct EvidencePlannerTool {
    thread_state: Arc<OntographThreadState>,
}

impl EvidencePlannerTool {
    pub(crate) fn new(thread_state: Arc<OntographThreadState>) -> Self {
        Self { thread_state }
    }
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct EvidencePlannerArgs {
    pub shape: String,
    pub anchor: String,
    #[serde(default)]
    pub direction: Option<String>,
    /// Optional relation filter for graph and KM traversal evidence, matched
    /// case-insensitively. Source-index relations use uppercase kinds such as
    /// `CALLS` and `TESTS`; KM transitions use author-defined names such as
    /// `next`. The filter narrows traversal evidence only. Textual symbol and
    /// docs evidence is a contract floor and is always returned. When a filter
    /// removes every traversal record, the result carries one
    /// `Blocked:RelationFilter:<anchor>` record listing the requested and
    /// observed relations.
    #[serde(default)]
    pub relation_types: Option<Vec<String>>,
    #[serde(default)]
    pub max_hops: Option<usize>,
    #[serde(default)]
    pub max_records: Option<usize>,
    #[serde(default)]
    pub max_artifacts: Option<usize>,
    #[serde(default)]
    pub max_context_bytes: Option<usize>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema, PartialEq, Eq)]
pub(crate) struct EvidencePlannerResult {
    pub subgraph: serde_json::Value,
    pub advisory_note: String,
}

#[derive(Debug, Default)]
pub(crate) struct OntographThreadState {
    auto_injected_kms: Mutex<BTreeSet<(String, String, String)>>,
    current_cwd: Mutex<Option<PathBuf>>,
}

#[path = "tool_types.rs"]
mod tool_types;

pub(crate) use tool_types::*;

#[cfg(test)]
#[path = "procedural_context_tool.rs"]
mod procedural_context_tool;
#[cfg(test)]
pub(crate) use procedural_context_tool::MAX_PROCEDURAL_CONTEXT_HOPS;
#[cfg(test)]
pub(crate) use procedural_context_tool::MAX_PROCEDURAL_CONTEXT_TRANSITIONS;
#[path = "tool_discover.rs"]
mod tool_discover;
#[path = "tool_docs.rs"]
mod tool_docs;
#[path = "tool_explain.rs"]
mod tool_explain;
#[path = "tool_impact.rs"]
mod tool_impact;
#[path = "tool_inspect.rs"]
mod tool_inspect;
#[path = "tool_km_context.rs"]
mod tool_km_context;
#[path = "tool_manage.rs"]
mod tool_manage;
#[path = "tool_search.rs"]
mod tool_search;

fn search_symbol_summary(item: crate::query::SymbolSearchItem) -> SearchSymbolSummary {
    SearchSymbolSummary {
        symbol: item.symbol,
        kind: item.kind,
        path: item.path,
    }
}

impl ExplainModuleTool {
    pub(crate) fn new(thread_state: Arc<OntographThreadState>) -> Self {
        Self { thread_state }
    }
}

impl DocsTool {
    pub(crate) fn new(thread_state: Arc<OntographThreadState>) -> Self {
        Self { thread_state }
    }
}

impl ImpactTool {
    pub(crate) fn new(thread_state: Arc<OntographThreadState>) -> Self {
        Self { thread_state }
    }
}

impl InspectTool {
    pub(crate) fn new(thread_state: Arc<OntographThreadState>) -> Self {
        Self { thread_state }
    }
}

impl KmContextTool {
    pub(crate) fn new(thread_state: Arc<OntographThreadState>) -> Self {
        Self {
            km_store: None,
            thread_state: Some(thread_state),
            thread_id: None,
        }
    }

    /// Builds the verb with the calling thread's host-owned identity, so
    /// agent-authored records from other threads stay unreadable.
    pub(crate) fn new_for_thread(
        thread_state: Arc<OntographThreadState>,
        thread_id: String,
    ) -> Self {
        Self {
            km_store: None,
            thread_state: Some(thread_state),
            thread_id: Some(thread_id),
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_store(km_store: KmStore) -> Self {
        Self {
            km_store: Some(km_store),
            thread_state: None,
            thread_id: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_store_for_thread(km_store: KmStore, thread_id: String) -> Self {
        Self {
            km_store: Some(km_store),
            thread_state: None,
            thread_id: Some(thread_id),
        }
    }

    fn km_store(&self) -> Result<KmStore, FunctionCallError> {
        self.km_store.clone().map(Ok).unwrap_or_else(|| {
            let current_cwd = self
                .thread_state
                .as_ref()
                .and_then(|thread_state| thread_state.current_cwd())
                .ok_or_else(|| {
                    FunctionCallError::RespondToModel(
                        "ontograph.km_context workspace context is unavailable for this turn"
                            .to_string(),
                    )
                })?;
            KmStore::for_project_path(&current_cwd).ok_or_else(|| {
                FunctionCallError::RespondToModel(
                    "ontograph.km_context workspace context is unavailable for this turn"
                        .to_string(),
                )
            })
        })
    }
}

impl ManageTool {
    pub(crate) fn new(thread_state: Arc<OntographThreadState>) -> Self {
        Self { thread_state }
    }
}

#[cfg(test)]
impl ProceduralContextTool {
    pub(crate) fn new_with_store(km_store: KmStore) -> Self {
        Self {
            km_store: Some(km_store),
            thread_state: None,
            thread_id: None,
        }
    }

    pub(crate) fn new_with_store_for_thread(km_store: KmStore, thread_id: String) -> Self {
        Self {
            km_store: Some(km_store),
            thread_state: None,
            thread_id: Some(thread_id),
        }
    }

    fn km_store(&self) -> Result<KmStore, FunctionCallError> {
        self.km_store.clone().map(Ok).unwrap_or_else(|| {
            let current_cwd = self
                .thread_state
                .as_ref()
                .and_then(|thread_state| thread_state.current_cwd())
                .ok_or_else(|| {
                    FunctionCallError::RespondToModel(
                        "ontograph.procedural_context workspace context is unavailable for this turn"
                            .to_string(),
                    )
                })?;
            KmStore::for_project_path(&current_cwd).ok_or_else(|| {
                FunctionCallError::RespondToModel(
                    "ontograph.procedural_context workspace context is unavailable for this turn"
                        .to_string(),
                )
            })
        })
    }
}

impl SearchTool {
    pub(crate) fn new(thread_state: Arc<OntographThreadState>) -> Self {
        Self { thread_state }
    }
}

impl OntographThreadState {
    pub(crate) fn auto_km_context_was_injected(
        &self,
        project_scope: &str,
        id: &str,
        version: &str,
    ) -> bool {
        self.auto_injected_kms
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(&(
                project_scope.to_string(),
                id.to_string(),
                version.to_string(),
            ))
    }

    pub(crate) fn mark_auto_km_context_injected(
        &self,
        project_scope: &str,
        id: &str,
        version: &str,
    ) {
        self.auto_injected_kms
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert((
                project_scope.to_string(),
                id.to_string(),
                version.to_string(),
            ));
    }

    /// Advances the automatic-KM compaction generation by clearing once-per-generation
    /// injection marks. Called only after the host commits compacted replacement history.
    pub(crate) fn advance_auto_km_compaction_generation(&self) {
        self.auto_injected_kms
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    pub(crate) fn set_current_cwd(&self, cwd: PathBuf) {
        *self
            .current_cwd
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(cwd);
    }

    pub(crate) fn clear_current_cwd(&self) {
        *self
            .current_cwd
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = None;
    }

    pub(crate) fn current_cwd(&self) -> Option<PathBuf> {
        self.current_cwd
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn current_artifact_root(&self) -> Option<OntographArtifactRoot> {
        self.current_repo_root().map(OntographArtifactRoot::new)
    }

    pub(crate) fn current_repo_root(&self) -> Option<PathBuf> {
        self.current_cwd().map(|cwd| resolve_workspace_root(&cwd))
    }

    pub(crate) fn ensure_active_source_index(
        &self,
        tool_name: &str,
    ) -> Result<(), FunctionCallError> {
        let repo_root = self.current_repo_root().ok_or_else(|| {
            FunctionCallError::RespondToModel(format!(
                "{tool_name} workspace context is unavailable for this turn"
            ))
        })?;
        let artifact_root = self.current_artifact_root().ok_or_else(|| {
            FunctionCallError::RespondToModel(format!(
                "{tool_name} workspace context is unavailable for this turn"
            ))
        })?;
        let expectation = manifest_expectation(&repo_root).map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "{tool_name} failed to inspect the Rust workspace state: {}",
                rust_index_error_message(err)
            ))
        })?;
        ensure_source_index_ready(&artifact_root, &expectation, tool_name)
    }

    pub(crate) fn load_current_source_index(
        &self,
        tool_name: &str,
    ) -> Result<SourceIndexStore, FunctionCallError> {
        let repo_root = self.current_repo_root().ok_or_else(|| {
            FunctionCallError::RespondToModel(format!(
                "{tool_name} workspace context is unavailable for this turn"
            ))
        })?;
        let artifact_root = self.current_artifact_root().ok_or_else(|| {
            FunctionCallError::RespondToModel(format!(
                "{tool_name} workspace context is unavailable for this turn"
            ))
        })?;
        let expectation = manifest_expectation(&repo_root).map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "{tool_name} failed to inspect the Rust workspace state: {}",
                rust_index_error_message(err)
            ))
        })?;
        ensure_source_index_ready(&artifact_root, &expectation, tool_name)?;
        SourceIndexStore::load_active_generation(&artifact_root).map_err(|err| match tool_name {
            "ontograph.discover" => discover_load_error(err),
            "ontograph.inspect" => inspect_load_error(err),
            "ontograph.impact" => impact_load_error(err),
            "ontograph.manage" => manage_load_error(err),
            "ontograph.search" => search_load_error(err),
            _ => FunctionCallError::RespondToModel(format!(
                "{tool_name} failed to load the native source index"
            )),
        })
    }
}

pub(crate) fn ensure_current_source_index_ready(
    thread_state: &OntographThreadState,
    tool_name: &str,
) -> Result<(), FunctionCallError> {
    let repo_root = thread_state.current_repo_root().ok_or_else(|| {
        FunctionCallError::RespondToModel(format!(
            "{tool_name} workspace context is unavailable for this turn"
        ))
    })?;
    let artifact_root = thread_state.current_artifact_root().ok_or_else(|| {
        FunctionCallError::RespondToModel(format!(
            "{tool_name} workspace context is unavailable for this turn"
        ))
    })?;
    let expectation = manifest_expectation(&repo_root).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "{tool_name} failed to inspect the Rust workspace state: {}",
            rust_index_error_message(err)
        ))
    })?;
    ensure_source_index_ready(&artifact_root, &expectation, tool_name)
}

fn ensure_source_index_ready(
    artifact_root: &OntographArtifactRoot,
    expectation: &ManifestExpectation,
    tool_name: &str,
) -> Result<(), FunctionCallError> {
    if artifact_root.current_manifest_readiness(expectation) != ArtifactReadiness::Ready {
        return Err(FunctionCallError::RespondToModel(format!(
            "{tool_name} source index is stale for the current workspace state; run ontograph.manage with action `index` to rebuild it"
        )));
    }
    Ok(())
}

pub(crate) fn rebuild_source_index(
    repo_root: &Path,
) -> Result<(String, crate::manifest::GraphCountSummary, Vec<String>), FunctionCallError> {
    let artifact_root = OntographArtifactRoot::new(repo_root.to_path_buf());
    let expectation = manifest_expectation(repo_root).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.manage failed to inspect the Rust workspace state: {}",
            rust_index_error_message(err)
        ))
    })?;
    let generation_id = expectation.target_head.clone();
    let (artifact, limits) = index_rust_workspace(RustIndexRequest {
        repo_root: repo_root.to_path_buf(),
        artifact_root,
        generation_id: generation_id.clone(),
        target_head: expectation.target_head,
        implementation_version: expectation.version.implementation_version,
        timestamp_unix_seconds: current_unix_seconds(),
    })
    .map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.manage failed to build the native source index: {}",
            rust_index_error_message(err)
        ))
    })?;
    Ok((generation_id, source_index_counts(&artifact), limits))
}

pub(crate) fn namespace_function_tool<I: JsonSchema, O: JsonSchema>(
    name: &str,
    description: &str,
) -> ToolSpec {
    let input_schema = serde_json::to_value(schemars::schema_for!(I))
        .unwrap_or_else(|err| panic!("{name} args schema should serialize: {err}"));
    let output_schema = serde_json::to_value(schemars::schema_for!(O))
        .unwrap_or_else(|err| panic!("{name} result schema should serialize: {err}"));
    ToolSpec::Namespace(ResponsesApiNamespace {
        name: ONTOGRAPH_NAMESPACE.to_string(),
        description: default_namespace_description(ONTOGRAPH_NAMESPACE),
        tools: vec![ResponsesApiNamespaceTool::Function(ResponsesApiTool {
            name: name.to_string(),
            description: description.to_string(),
            strict: false,
            defer_loading: None,
            parameters: parse_tool_input_schema(&input_schema)
                .unwrap_or_else(|err| panic!("{name} args schema should parse: {err}")),
            output_schema: Some(output_schema),
        })],
    })
}

pub(crate) fn parse_tool_args<T: for<'de> Deserialize<'de>>(
    call: &ToolCall,
    tool_name: &str,
) -> Result<T, FunctionCallError> {
    let arguments = call.function_arguments()?;
    serde_json::from_str(arguments).map_err(|err| {
        FunctionCallError::RespondToModel(format!("invalid {tool_name} arguments: {err}"))
    })
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn implemented_tool_summaries() -> Vec<ImplementedToolSummary> {
    vec![
        ImplementedToolSummary {
            full_name: format!("{ONTOGRAPH_NAMESPACE}.{EXPLAIN_MODULE_TOOL_NAME}"),
            description: EXPLAIN_MODULE_DESCRIPTION.to_string(),
        },
        ImplementedToolSummary {
            full_name: format!("{ONTOGRAPH_NAMESPACE}.{DISCOVER_TOOL_NAME}"),
            description: DISCOVER_TOOL_DESCRIPTION.to_string(),
        },
        ImplementedToolSummary {
            full_name: format!("{ONTOGRAPH_NAMESPACE}.{DOCS_TOOL_NAME}"),
            description: format!(
                "{DOCS_DESCRIPTION} Requires a docs-index.json sidecar, published by ontograph.manage action `index`."
            ),
        },
        ImplementedToolSummary {
            full_name: format!("{ONTOGRAPH_NAMESPACE}.{AUDIT_TOOL_NAME}"),
            description: AUDIT_TOOL_DESCRIPTION.to_string(),
        },
        ImplementedToolSummary {
            full_name: format!("{ONTOGRAPH_NAMESPACE}.{IMPACT_TOOL_NAME}"),
            description: IMPACT_DESCRIPTION.to_string(),
        },
        ImplementedToolSummary {
            full_name: format!("{ONTOGRAPH_NAMESPACE}.{INSPECT_TOOL_NAME}"),
            description: INSPECT_DESCRIPTION.to_string(),
        },
        ImplementedToolSummary {
            full_name: format!("{ONTOGRAPH_NAMESPACE}.{KM_CONTEXT_TOOL_NAME}"),
            description: KM_CONTEXT_DESCRIPTION.to_string(),
        },
        ImplementedToolSummary {
            full_name: format!("{ONTOGRAPH_NAMESPACE}.{MANAGE_TOOL_NAME}"),
            description: MANAGE_DESCRIPTION.to_string(),
        },
        ImplementedToolSummary {
            full_name: format!("{ONTOGRAPH_NAMESPACE}.{SEARCH_TOOL_NAME}"),
            description: SEARCH_DESCRIPTION.to_string(),
        },
    ]
}

pub(crate) fn discovered_pack_summaries() -> Vec<DiscoveredPackSummary> {
    vec![
        DiscoveredPackSummary {
            name: "residue_domains".to_string(),
            description: "Static native pack for bounded forbidden-domain residue scans."
                .to_string(),
            actions: vec!["audit.build".to_string()],
        },
        DiscoveredPackSummary {
            name: "pattern_checks".to_string(),
            description: "Static native pack for bounded source-pattern scans.".to_string(),
            actions: vec!["audit.patterns".to_string()],
        },
        DiscoveredPackSummary {
            name: "saved_finding_reruns".to_string(),
            description: "Static native pack for rerunning saved audit findings against the current workspace state."
                .to_string(),
            actions: vec!["audit.rerun".to_string()],
        },
        DiscoveredPackSummary {
            name: "rust_tech_debt".to_string(),
            description: "Static native pack for bounded Rust structural tech-debt ranking."
                .to_string(),
            actions: vec!["audit.tech_debt".to_string()],
        },
        DiscoveredPackSummary {
            name: "rust_hotspots".to_string(),
            description: "Static native pack for bounded Rust file hotspot ranking.".to_string(),
            actions: vec!["audit.hotspots".to_string()],
        },
        DiscoveredPackSummary {
            name: "rust_dead_code".to_string(),
            description: "Static native pack for bounded Rust private-orphan scans."
                .to_string(),
            actions: vec!["audit.dead_code".to_string()],
        },
        DiscoveredPackSummary {
            name: "graph_cycles".to_string(),
            description: "Static native pack for bounded graph cycle reports.".to_string(),
            actions: vec!["audit.cycles".to_string()],
        },
        DiscoveredPackSummary {
            name: "graph_coupling".to_string(),
            description: "Static native pack for bounded graph coupling reports.".to_string(),
            actions: vec!["audit.coupling".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_report".to_string(),
            description:
                "Static native pack for bounded source-index and audit-surface reporting."
                    .to_string(),
            actions: vec!["audit.report".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_violations".to_string(),
            description:
                "Static native pack for caller-supplied boundary-rule checks.".to_string(),
            actions: vec!["audit.violations".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_migration".to_string(),
            description:
                "Static native pack for explicit docs-sidecar migration reference reports. Requires a docs-index.json sidecar, published by ontograph.manage action `index`."
                    .to_string(),
            actions: vec!["audit.migration".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_requirements".to_string(),
            description:
                "Static native pack for explicit docs-sidecar requirement reference reports. Requires a docs-index.json sidecar, published by ontograph.manage action `index`."
                    .to_string(),
            actions: vec!["audit.requirements".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_drift".to_string(),
            description:
                "Static native pack for explicit docs-sidecar drift aggregation. Requires a docs-index.json sidecar, published by ontograph.manage action `index`."
                    .to_string(),
            actions: vec!["audit.drift".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_graph_diff".to_string(),
            description:
                "Static native pack for completed source-index graph generation diffs."
                    .to_string(),
            actions: vec!["audit.graph_diff".to_string()],
        },
        DiscoveredPackSummary {
            name: "rust_logic".to_string(),
            description: "Static native pack for bounded Rust systems-logic scans."
                .to_string(),
            actions: vec!["audit.logic".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_session_start".to_string(),
            description:
                "Static native pack for local audit session lock and readback state.".to_string(),
            actions: vec!["audit.session_start".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_session_verify".to_string(),
            description:
                "Static native pack for read-only audit session and lock verification.".to_string(),
            actions: vec!["audit.session_verify".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_session_dedupe".to_string(),
            description:
                "Static native pack for read-only audit-session finding grouping.".to_string(),
            actions: vec!["audit.session_dedupe".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_session_bundle".to_string(),
            description:
                "Static native pack for projecting grouped audit findings into bundles."
                    .to_string(),
            actions: vec!["audit.session_bundle".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_verify_diff".to_string(),
            description:
                "Static native pack for read-only expected-vs-actual diff verification."
                    .to_string(),
            actions: vec!["audit.verify_diff".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_worker_scope_review".to_string(),
            description:
                "Static native pack for reviewing diff scope against bundle targets."
                    .to_string(),
            actions: vec!["audit.worker_scope_review".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_test_gap".to_string(),
            description:
                "Static native pack for caller-supplied changed-symbol test-gap checks."
                    .to_string(),
            actions: vec!["audit.test_gap".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_test_suggestions".to_string(),
            description:
                "Static native pack for deterministic suggestions from explicit test-gap evidence."
                    .to_string(),
            actions: vec!["audit.test_suggestions".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_abi".to_string(),
            description:
                "Static native pack for normalized JSON schema ABI comparisons.".to_string(),
            actions: vec!["audit.abi".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_path_verify".to_string(),
            description:
                "Static native pack for bounded source-window path verification.".to_string(),
            actions: vec!["audit.path_verify".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_pressure".to_string(),
            description:
                "Static native pack for shallow quota and global side-effect pressure scans."
                    .to_string(),
            actions: vec!["audit.pressure".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_concurrency".to_string(),
            description:
                "Static native pack for shallow lock-acquisition and nested-lock hint scans."
                    .to_string(),
            actions: vec!["audit.concurrency".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_simulate_fault".to_string(),
            description:
                "Static native pack for shallow Rust fault-window simulation scans."
                    .to_string(),
            actions: vec!["audit.simulate_fault".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_session_dispatch".to_string(),
            description:
                "Static native pack for read-only prompt-packet rendering from one audit bundle."
                    .to_string(),
            actions: vec!["audit.session_dispatch".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_session_review_worker".to_string(),
            description:
                "Static native pack for reviewing worker changes against one audit bundle."
                    .to_string(),
            actions: vec!["audit.session_review_worker".to_string()],
        },
        DiscoveredPackSummary {
            name: "audit_coverage".to_string(),
            description:
                "Static native pack for explicit source-index test-link coverage.".to_string(),
            actions: vec!["audit.coverage".to_string()],
        },
        DiscoveredPackSummary {
            name: "rust_fsm".to_string(),
            description:
                "Static native pack for bounded Rust state-machine extraction.".to_string(),
            actions: vec!["audit.extract_fsm".to_string()],
        },
        DiscoveredPackSummary {
            name: "rust_error_topology".to_string(),
            description:
                "Static native pack for bounded Rust error-topology scans.".to_string(),
            actions: vec!["audit.error_topology".to_string()],
        },
    ]
}

pub(crate) fn donor_actions_missing() -> usize {
    DONOR_ACTIONS_TOTAL
        - DONOR_ACTIONS_IMPLEMENTED
        - DONOR_ACTIONS_INTENTIONAL_DIVERGENCE
        - DONOR_ACTIONS_WITHDRAWN
}

fn discover_groups_result(
    group: CurrentWorkspaceGroupState,
    primary_note: &str,
    boundary_note: &str,
) -> DiscoverResult {
    DiscoverResult {
        namespace: ONTOGRAPH_NAMESPACE.to_string(),
        groups: vec![DiscoveredGroupSummary {
            name: group.name,
            repo_roots: group.repo_roots,
            artifact_root: group.artifact_root,
        }],
        packs: Vec::new(),
        tools: Vec::new(),
        repos: Vec::new(),
        routes: Vec::new(),
        routes_truncated: false,
        parity_matrix_path: PARITY_MATRIX_PATH.to_string(),
        donor_actions_total: DONOR_ACTIONS_TOTAL,
        donor_actions_implemented: DONOR_ACTIONS_IMPLEMENTED,
        donor_actions_intentional_divergence: DONOR_ACTIONS_INTENTIONAL_DIVERGENCE,
        donor_actions_missing: donor_actions_missing(),
        notes: vec![primary_note.to_string(), boundary_note.to_string()],
    }
}

fn discover_repo_from_manifest(
    root: &OntographArtifactRoot,
) -> Result<DiscoveredRepoSummary, FunctionCallError> {
    let generation_id = read_to_string(&root.current_pointer_path(), 4 * 1024)
        .map_err(|_| {
            FunctionCallError::RespondToModel(
                "ontograph.discover active manifest pointer is missing or unreadable".to_string(),
            )
        })?
        .trim()
        .to_string();
    let manifest_path = root.generation_manifest_path(&generation_id).map_err(|_| {
        FunctionCallError::RespondToModel(
            "ontograph.discover active manifest pointer is invalid".to_string(),
        )
    })?;
    let manifest = serde_json::from_str::<OntographManifest>(
        &read_to_string(&manifest_path, MAX_FILE_BYTES).map_err(|_| {
            FunctionCallError::RespondToModel(
                "ontograph.discover active manifest is missing or unreadable".to_string(),
            )
        })?,
    )
    .map_err(|_| {
        FunctionCallError::RespondToModel(
            "ontograph.discover active manifest is invalid".to_string(),
        )
    })?;
    let generation = manifest
        .completed_generations
        .iter()
        .find(|generation| generation.metadata.generation_id == manifest.active_generation_id)
        .ok_or_else(|| {
            FunctionCallError::RespondToModel(
                "ontograph.discover active manifest has no completed active generation".to_string(),
            )
        })?;
    Ok(DiscoveredRepoSummary {
        root: manifest.repository.root,
        vcs: manifest.repository.vcs,
        remote_url: manifest.repository.remote_url,
        active_generation_id: manifest.active_generation_id,
        target_head: generation.metadata.target_head.clone(),
        files: generation.counts.files,
        symbols: generation.counts.symbols,
        relationships: generation.counts.relationships,
    })
}

fn resolve_workspace_root(cwd: &Path) -> PathBuf {
    let mut current = cwd.to_path_buf();
    if current.is_file()
        && let Some(parent) = current.parent()
    {
        current = parent.to_path_buf();
    }

    let fallback = current.clone();
    let mut cargo_root = None;
    let temp_root = std::env::temp_dir();
    loop {
        if current == temp_root {
            return cargo_root.unwrap_or(fallback);
        }
        if current.join("Cargo.toml").is_file() {
            cargo_root = Some(current.clone());
        }
        let git_path = current.join(".git");
        if git_path.is_dir() || git_path.is_file() {
            return cargo_root.unwrap_or(current);
        }
        if !current.pop() {
            return cargo_root.unwrap_or(fallback);
        }
    }
}

fn manifest_expectation(repo_root: &Path) -> Result<ManifestExpectation, RustIndexError> {
    let committed_head = resolve_target_head(repo_root);
    let target_head = format!(
        "{}-{}",
        committed_head,
        rust_workspace_state_key(repo_root, &committed_head)?
    );
    Ok(ManifestExpectation {
        repository: crate::manifest::RepositoryIdentity {
            root: repo_root.display().to_string(),
            vcs: "git".to_string(),
            remote_url: None,
        },
        version: crate::manifest::ArtifactVersion {
            schema_version: crate::manifest::ONTOGRAPH_SCHEMA_VERSION,
            implementation_version: IMPLEMENTATION_VERSION.to_string(),
        },
        target_head,
    })
}

fn current_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn rust_index_error_message(error: RustIndexError) -> String {
    match error {
        RustIndexError::Walk => "failed to walk the workspace".to_string(),
        RustIndexError::ReadFile(path) => format!("failed to read a Rust source file: {path}"),
        RustIndexError::RebuildInProgress => "another index rebuild is in progress".to_string(),
        RustIndexError::WriteLock => "failed to acquire the index rebuild lock".to_string(),
        RustIndexError::Serialize => "failed to serialize the source index".to_string(),
        RustIndexError::WriteArtifact => "failed to write source-index.json".to_string(),
        RustIndexError::PublishManifest(_) => {
            "failed to publish the source-index manifest".to_string()
        }
        RustIndexError::DocsIndex(_) => "failed to write docs-index.json".to_string(),
    }
}

#[cfg(test)]
mod source_index_error_message_tests {
    use super::*;

    #[test]
    fn source_index_error_message_renders_relative_path_and_preserves_other_text() {
        assert_eq!(
            rust_index_error_message(RustIndexError::ReadFile("src/invalid.rs".to_string())),
            "failed to read a Rust source file: src/invalid.rs"
        );
        assert_eq!(
            rust_index_error_message(RustIndexError::Walk),
            "failed to walk the workspace"
        );
        assert_eq!(
            rust_index_error_message(RustIndexError::RebuildInProgress),
            "another index rebuild is in progress"
        );
        assert_eq!(
            rust_index_error_message(RustIndexError::WriteLock),
            "failed to acquire the index rebuild lock"
        );
        assert_eq!(
            rust_index_error_message(RustIndexError::Serialize),
            "failed to serialize the source index"
        );
        assert_eq!(
            rust_index_error_message(RustIndexError::WriteArtifact),
            "failed to write source-index.json"
        );
        assert_eq!(
            rust_index_error_message(RustIndexError::PublishManifest(
                crate::manifest::ArtifactPublishError::WriteManifest
            )),
            "failed to publish the source-index manifest"
        );
    }
}

fn group_state_error_message(error: GroupStateError) -> String {
    match error {
        GroupStateError::CreateDirectory => {
            "failed to create .ontocode/ontograph/groups".to_string()
        }
        GroupStateError::Read => {
            "failed to read .ontocode/ontograph/groups/current-workspace.json".to_string()
        }
        GroupStateError::Serialize => {
            "failed to serialize current-workspace group state".to_string()
        }
        GroupStateError::Write => {
            "failed to write .ontocode/ontograph/groups/current-workspace.json".to_string()
        }
        GroupStateError::Corrupt(message) => format!("corrupt group state: {message}"),
    }
}

fn normalize_model_path(path: &str, tool_name: &str) -> Result<String, FunctionCallError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(FunctionCallError::RespondToModel(format!(
            "{tool_name} path must not be empty"
        )));
    }
    if trimmed.contains('\0') || trimmed.contains("://") {
        return Err(FunctionCallError::RespondToModel(format!(
            "{tool_name} path must be a local file path"
        )));
    }

    let path = Path::new(trimmed);
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(segment) => normalized.push(segment),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(FunctionCallError::RespondToModel(format!(
                    "{tool_name} path must be relative and stay within the current working directory"
                )));
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err(FunctionCallError::RespondToModel(format!(
            "{tool_name} path must point to a file"
        )));
    }

    Ok(normalized.display().to_string())
}

fn parse_inspect_path_arg(path: Option<&str>) -> Result<&str, FunctionCallError> {
    path.ok_or_else(|| {
        FunctionCallError::RespondToModel(
            "ontograph.inspect path is required for action `context`".to_string(),
        )
    })
}

fn normalize_inspect_targets(
    targets: Option<&[String]>,
    cwd: &Path,
    repo_root: &Path,
) -> Result<Vec<String>, FunctionCallError> {
    let mut normalized = Vec::new();
    for target in targets.unwrap_or_default() {
        let trimmed = target.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((path, line)) = parse_line_target(trimmed) {
            let normalized_path = normalize_model_path(path, "ontograph.inspect")?;
            let indexed_path = resolve_existing_indexed_path(&normalized_path, cwd, repo_root)
                .unwrap_or(normalized_path);
            normalized.push(format!("{indexed_path}:{line}"));
            continue;
        }
        match normalize_model_path(trimmed, "ontograph.inspect") {
            Ok(path) => {
                if let Some(indexed_path) = resolve_existing_indexed_path(&path, cwd, repo_root) {
                    normalized.push(indexed_path);
                } else {
                    normalized.push(trimmed.to_string());
                }
            }
            Err(_) => normalized.push(trimmed.to_string()),
        }
    }
    if normalized.is_empty() {
        return Err(FunctionCallError::RespondToModel(
            "ontograph.inspect targets are required for action `evidence`".to_string(),
        ));
    }
    Ok(normalized)
}

fn parse_line_target(target: &str) -> Option<(&str, usize)> {
    let colon = target.rfind(':')?;
    let path = target[..colon].trim();
    let line = target[colon + 1..].trim().parse::<usize>().ok()?;
    (!path.is_empty() && line > 0).then_some((path, line))
}

fn resolve_existing_indexed_path(path: &str, cwd: &Path, repo_root: &Path) -> Option<String> {
    let cwd_path = cwd.join(path);
    if cwd_path.is_file() {
        return Some(resolve_indexed_module_path(path, cwd, repo_root));
    }
    let repo_path = repo_root.join(path);
    repo_path.is_file().then(|| path.to_string())
}

fn inspect_load_error(error: SourceIndexLoadError) -> FunctionCallError {
    let message = match error {
        SourceIndexLoadError::MissingCurrentPointer => {
            "ontograph.inspect active source index is missing"
        }
        SourceIndexLoadError::InvalidCurrentPointer => {
            "ontograph.inspect active source index pointer is invalid"
        }
        SourceIndexLoadError::MissingSourceIndex => {
            "ontograph.inspect source-index.json is missing"
        }
        SourceIndexLoadError::ReadSourceIndex => {
            "ontograph.inspect source-index.json is unreadable"
        }
        SourceIndexLoadError::InvalidSourceIndex => {
            "ontograph.inspect source-index.json is invalid"
        }
    };
    FunctionCallError::RespondToModel(message.to_string())
}

fn discover_load_error(error: SourceIndexLoadError) -> FunctionCallError {
    let message = match error {
        SourceIndexLoadError::MissingCurrentPointer => {
            "ontograph.discover active source index is missing"
        }
        SourceIndexLoadError::InvalidCurrentPointer => {
            "ontograph.discover active source index pointer is invalid"
        }
        SourceIndexLoadError::MissingSourceIndex => {
            "ontograph.discover source-index.json is missing"
        }
        SourceIndexLoadError::ReadSourceIndex => {
            "ontograph.discover source-index.json is unreadable"
        }
        SourceIndexLoadError::InvalidSourceIndex => {
            "ontograph.discover source-index.json is invalid"
        }
    };
    FunctionCallError::RespondToModel(message.to_string())
}

fn discover_query_error(error: QueryError) -> FunctionCallError {
    let message = match error {
        QueryError::MissingArtifact => {
            "ontograph.discover route is not present in source-index.json"
        }
        QueryError::AmbiguousArtifact => "ontograph.discover source index is ambiguous",
        QueryError::StaleArtifact => "ontograph.discover source index is stale",
        QueryError::CorruptArtifact => "ontograph.discover source index is corrupt",
        QueryError::Timeout => "ontograph.discover query timed out",
        QueryError::OverBudget => "ontograph.discover query exceeded its budget",
        QueryError::ExternalCommand => "ontograph.discover query failed to access local resources",
    };
    FunctionCallError::RespondToModel(message.to_string())
}

fn docs_index_load_error(error: DocsIndexLoadError) -> FunctionCallError {
    let message = match error {
        DocsIndexLoadError::MissingCurrentPointer => {
            "ontograph.docs active source index is missing"
        }
        DocsIndexLoadError::InvalidCurrentPointer => {
            "ontograph.docs active source index pointer is invalid"
        }
        DocsIndexLoadError::MissingSourceManifest => {
            "ontograph.docs active source-index manifest is missing"
        }
        DocsIndexLoadError::ReadSourceManifest => {
            "ontograph.docs active source-index manifest is unreadable"
        }
        DocsIndexLoadError::InvalidSourceManifest => {
            "ontograph.docs active source-index manifest is invalid"
        }
        DocsIndexLoadError::MissingDocsIndex => "ontograph.docs docs-index.json is missing",
        DocsIndexLoadError::ReadDocsIndex => "ontograph.docs docs-index.json is unreadable",
        DocsIndexLoadError::InvalidDocsIndex => "ontograph.docs docs-index.json is invalid",
        DocsIndexLoadError::UnsupportedSchema => {
            "ontograph.docs docs-index.json schema is unsupported"
        }
        DocsIndexLoadError::InProgress => "ontograph.docs docs-index.json is in progress",
        DocsIndexLoadError::SourceIdentityMismatch => {
            "ontograph.docs docs-index.json does not match the active source-index.json identity"
        }
    };
    FunctionCallError::RespondToModel(message.to_string())
}

fn docs_query_error(error: QueryError) -> FunctionCallError {
    let message = match error {
        QueryError::MissingArtifact => "ontograph.docs query has no matches in docs-index.json",
        QueryError::AmbiguousArtifact => "ontograph.docs exactly one context selector is required",
        QueryError::StaleArtifact => "ontograph.docs docs-index.json is stale",
        QueryError::CorruptArtifact => "ontograph.docs docs-index.json is corrupt",
        QueryError::Timeout => "ontograph.docs query timed out",
        QueryError::OverBudget => "ontograph.docs query exceeded its budget",
        QueryError::ExternalCommand => "ontograph.docs query failed to access local resources",
    };
    FunctionCallError::RespondToModel(message.to_string())
}

fn docs_trace_query_error(error: QueryError) -> FunctionCallError {
    let message = match error {
        QueryError::MissingArtifact => "ontograph.docs trace has no matches in docs-index.json",
        QueryError::AmbiguousArtifact => "ontograph.docs trace requires at least one selector",
        QueryError::StaleArtifact => "ontograph.docs docs-index.json is stale",
        QueryError::CorruptArtifact => "ontograph.docs docs-index.json is corrupt",
        QueryError::Timeout => "ontograph.docs trace query timed out",
        QueryError::OverBudget => "ontograph.docs trace query exceeded its budget",
        QueryError::ExternalCommand => {
            "ontograph.docs trace query failed to access local resources"
        }
    };
    FunctionCallError::RespondToModel(message.to_string())
}

fn docs_source_index_load_error(error: SourceIndexLoadError) -> FunctionCallError {
    let message = match error {
        SourceIndexLoadError::MissingCurrentPointer => {
            "ontograph.docs active source index is missing"
        }
        SourceIndexLoadError::InvalidCurrentPointer => {
            "ontograph.docs active source index pointer is invalid"
        }
        SourceIndexLoadError::MissingSourceIndex => "ontograph.docs source-index.json is missing",
        SourceIndexLoadError::ReadSourceIndex => "ontograph.docs source-index.json is unreadable",
        SourceIndexLoadError::InvalidSourceIndex => "ontograph.docs source-index.json is invalid",
    };
    FunctionCallError::RespondToModel(message.to_string())
}

fn manage_load_error(error: SourceIndexLoadError) -> FunctionCallError {
    let message = match error {
        SourceIndexLoadError::MissingCurrentPointer => {
            "ontograph.manage active source index is missing"
        }
        SourceIndexLoadError::InvalidCurrentPointer => {
            "ontograph.manage active source index pointer is invalid"
        }
        SourceIndexLoadError::MissingSourceIndex => "ontograph.manage source-index.json is missing",
        SourceIndexLoadError::ReadSourceIndex => "ontograph.manage source-index.json is unreadable",
        SourceIndexLoadError::InvalidSourceIndex => "ontograph.manage source-index.json is invalid",
    };
    FunctionCallError::RespondToModel(message.to_string())
}

fn manage_query_error(error: QueryError) -> FunctionCallError {
    let message = match error {
        QueryError::MissingArtifact => {
            "ontograph.manage route or path is not present in source-index.json"
        }
        QueryError::AmbiguousArtifact => "ontograph.manage route or path is ambiguous",
        QueryError::StaleArtifact => "ontograph.manage source index is stale",
        QueryError::CorruptArtifact => "ontograph.manage source index is corrupt",
        QueryError::Timeout => "ontograph.manage query timed out",
        QueryError::OverBudget => "ontograph.manage query exceeded its budget",
        QueryError::ExternalCommand => "ontograph.manage query failed to access local resources",
    };
    FunctionCallError::RespondToModel(message.to_string())
}

fn inspect_query_error(error: QueryError) -> FunctionCallError {
    let message = match error {
        QueryError::MissingArtifact => "ontograph.inspect path is not present in source-index.json",
        QueryError::AmbiguousArtifact => "ontograph.inspect source index is ambiguous",
        QueryError::StaleArtifact => "ontograph.inspect source index is stale",
        QueryError::CorruptArtifact => "ontograph.inspect source index is corrupt",
        QueryError::Timeout => "ontograph.inspect query timed out",
        QueryError::OverBudget => "ontograph.inspect query exceeded its budget",
        QueryError::ExternalCommand => "ontograph.inspect query failed to access local resources",
    };
    FunctionCallError::RespondToModel(message.to_string())
}

fn inspect_shape_query_error(error: QueryError) -> FunctionCallError {
    let message = match error {
        QueryError::MissingArtifact => {
            "ontograph.inspect route or path is not present in source-index.json"
        }
        QueryError::AmbiguousArtifact => "ontograph.inspect route or path is ambiguous",
        QueryError::StaleArtifact => "ontograph.inspect source index is stale",
        QueryError::CorruptArtifact => "ontograph.inspect source index is corrupt",
        QueryError::Timeout => "ontograph.inspect query timed out",
        QueryError::OverBudget => "ontograph.inspect query exceeded its budget",
        QueryError::ExternalCommand => "ontograph.inspect query failed to access local resources",
    };
    FunctionCallError::RespondToModel(message.to_string())
}

fn km_context_resolve_error(error: KmResolveError) -> FunctionCallError {
    let message = match error {
        KmResolveError::Store(err) => {
            return FunctionCallError::RespondToModel(format!(
                "ontograph.km_context failed to read the KM cache: {}",
                km_store_error_message(err)
            ));
        }
        KmResolveError::MissingSummary => "active KM is missing summary",
        KmResolveError::MissingSources => "active KM is missing sources",
        KmResolveError::MissingLicenses => "active KM is missing licenses",
        KmResolveError::MissingProvenance => "active KM is missing provenance",
        KmResolveError::Manifest(_) => "active KM manifest is unreadable",
        KmResolveError::Facts(_) => "active KM facts are unreadable",
        KmResolveError::Examples => "active KM examples are unreadable",
    };
    FunctionCallError::RespondToModel(format!("ontograph.km_context {message}"))
}

fn km_store_error_message(error: KmStoreError) -> &'static str {
    match error {
        KmStoreError::HomeDir => "home directory is unavailable",
        KmStoreError::MissingSource => "source KM directory is missing",
        KmStoreError::InvalidKm(_) => "KM validation failed",
        KmStoreError::InvalidIndex => "KM index is invalid",
        KmStoreError::InvalidOrigin => "KM index origin is invalid",
        KmStoreError::SerializeIndex => "failed to serialize the KM index",
        KmStoreError::WriteIndex => "failed to write the KM index",
        KmStoreError::CreateCacheDir => "failed to create the KM cache directory",
        KmStoreError::CopyModule => "failed to copy the KM module into cache",
        KmStoreError::HashModule => "failed to hash the KM module",
        KmStoreError::VersionConflict => {
            "KM version content conflict: changed content needs a new version"
        }
        KmStoreError::ProjectScopeConflict => "KM project scope conflict",
        KmStoreError::PackageOwnershipConflict => "KM package id belongs to another author",
        KmStoreError::ThreadCeiling { .. } => "authoring thread ceiling reached",
    }
}

fn impact_load_error(error: SourceIndexLoadError) -> FunctionCallError {
    let message = match error {
        SourceIndexLoadError::MissingCurrentPointer => {
            "ontograph.impact active source index is missing"
        }
        SourceIndexLoadError::InvalidCurrentPointer => {
            "ontograph.impact active source index pointer is invalid"
        }
        SourceIndexLoadError::MissingSourceIndex => "ontograph.impact source-index.json is missing",
        SourceIndexLoadError::ReadSourceIndex => "ontograph.impact source-index.json is unreadable",
        SourceIndexLoadError::InvalidSourceIndex => "ontograph.impact source-index.json is invalid",
    };
    FunctionCallError::RespondToModel(message.to_string())
}

fn impact_query_error(error: QueryError) -> FunctionCallError {
    FunctionCallError::RespondToModel(impact_query_error_message(error).to_string())
}

fn impact_query_error_message(error: QueryError) -> &'static str {
    match error {
        QueryError::MissingArtifact => {
            "ontograph.impact symbol is not present in source-index.json"
        }
        QueryError::AmbiguousArtifact => {
            "ontograph.impact symbol is ambiguous across multiple indexed definitions"
        }
        QueryError::StaleArtifact => "ontograph.impact source index is stale",
        QueryError::CorruptArtifact => "ontograph.impact source index is corrupt",
        QueryError::Timeout => "ontograph.impact query timed out",
        QueryError::OverBudget => "ontograph.impact query exceeded its budget",
        QueryError::ExternalCommand => {
            "ontograph.impact failed to read git diff from the current workspace"
        }
    }
}

fn impact_batch_query_error(symbol: &str, error: QueryError) -> FunctionCallError {
    FunctionCallError::RespondToModel(format!(
        "{} for batch symbol `{symbol}`",
        impact_query_error_message(error)
    ))
}

fn impact_route_query_error(error: QueryError) -> FunctionCallError {
    let message = match error {
        QueryError::MissingArtifact => {
            "ontograph.impact matched route handler is not present in source-index.json"
        }
        QueryError::AmbiguousArtifact => {
            "ontograph.impact matched route handler is ambiguous within source-index.json"
        }
        QueryError::StaleArtifact => "ontograph.impact source index is stale",
        QueryError::CorruptArtifact => "ontograph.impact source index is corrupt",
        QueryError::Timeout => "ontograph.impact query timed out",
        QueryError::OverBudget => "ontograph.impact query exceeded its budget",
        QueryError::ExternalCommand => "ontograph.impact query failed to access local resources",
    };
    FunctionCallError::RespondToModel(message.to_string())
}

fn search_load_error(error: SourceIndexLoadError) -> FunctionCallError {
    let message = match error {
        SourceIndexLoadError::MissingCurrentPointer => {
            "ontograph.search active source index is missing"
        }
        SourceIndexLoadError::InvalidCurrentPointer => {
            "ontograph.search active source index pointer is invalid"
        }
        SourceIndexLoadError::MissingSourceIndex => "ontograph.search source-index.json is missing",
        SourceIndexLoadError::ReadSourceIndex => "ontograph.search source-index.json is unreadable",
        SourceIndexLoadError::InvalidSourceIndex => "ontograph.search source-index.json is invalid",
    };
    FunctionCallError::RespondToModel(message.to_string())
}

fn search_query_error(error: QueryError) -> FunctionCallError {
    let message = match error {
        QueryError::MissingArtifact => "ontograph.search query has no matches in source-index.json",
        QueryError::AmbiguousArtifact => "ontograph.search source index is ambiguous",
        QueryError::StaleArtifact => "ontograph.search source index is stale",
        QueryError::CorruptArtifact => "ontograph.search source index is corrupt",
        QueryError::Timeout => "ontograph.search query timed out",
        QueryError::OverBudget => "ontograph.search query exceeded its budget",
        QueryError::ExternalCommand => "ontograph.search query failed to access local resources",
    };
    FunctionCallError::RespondToModel(message.to_string())
}

fn resolve_module_path(path: &str, cwd: &Path) -> Result<PathBuf, FunctionCallError> {
    let relative_path = Path::new(path);
    let resolved_path = cwd.join(relative_path);
    let mut scoped_path = cwd.to_path_buf();
    for component in relative_path.components() {
        let Component::Normal(segment) = component else {
            continue;
        };
        scoped_path.push(segment);
        let Ok(metadata) = fs::symlink_metadata(&scoped_path) else {
            break;
        };
        if metadata.file_type().is_symlink() {
            return Err(FunctionCallError::RespondToModel(
                "ontograph.explain_module path must not traverse symlinks".to_string(),
            ));
        }
    }

    let metadata = fs::metadata(&resolved_path).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.explain_module path does not exist: {err}"
        ))
    })?;
    if !metadata.is_file() {
        return Err(FunctionCallError::RespondToModel(
            "ontograph.explain_module path must point to a file".to_string(),
        ));
    }

    Ok(resolved_path)
}

fn resolve_indexed_module_path(path: &str, cwd: &Path, repo_root: &Path) -> String {
    let resolved_path = cwd.join(Path::new(path));
    resolved_path
        .strip_prefix(repo_root)
        .ok()
        .map(relative_slash_path)
        .unwrap_or_else(|| path.to_string())
}

fn read_bounded_utf8(path: &Path) -> Result<String, FunctionCallError> {
    let mut options = File::options();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options.open(path).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.explain_module path could not be opened: {err}"
        ))
    })?;
    let mut bytes = Vec::with_capacity(MAX_FILE_BYTES.min(8192));
    file.by_ref()
        .take((MAX_FILE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "ontograph.explain_module path could not be read: {err}"
            ))
        })?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.explain_module path is too large; maximum supported size is {MAX_FILE_BYTES} bytes"
        )));
    }
    String::from_utf8(bytes).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.explain_module path could not be read as UTF-8 text: {err}"
        ))
    })
}

fn detect_language(path: &str) -> Option<String> {
    let extension = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())?
        .to_ascii_lowercase();

    let language = match extension.as_str() {
        "rs" => "rust",
        "py" => "python",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "json" => "json",
        "md" => "markdown",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "sh" => "shell",
        _ => return None,
    };
    Some(language.to_string())
}

fn line_count(content: &str) -> usize {
    content.lines().count()
}

fn collect_top_level_symbols(
    content: &str,
    language: Option<&str>,
    max_symbols: usize,
) -> Vec<String> {
    if language != Some("rust") {
        return Vec::new();
    }

    let mut symbols = Vec::new();
    for line in content.lines() {
        if line.is_empty() || line.chars().next().is_some_and(char::is_whitespace) {
            continue;
        }
        if let Some(symbol) = top_level_rust_symbol(line) {
            symbols.push(symbol);
            if symbols.len() >= max_symbols {
                break;
            }
        }
    }
    symbols
}

fn top_level_rust_symbol(line: &str) -> Option<String> {
    let line = line.trim_end();
    let line = strip_visibility(line);
    let line = line.strip_prefix("async ").unwrap_or(line);
    let (keyword, rest) = line.split_once(' ')?;
    if !matches!(keyword, "fn" | "struct" | "enum" | "trait" | "mod") {
        return None;
    }

    let name: String = rest
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect();
    if name.is_empty() { None } else { Some(name) }
}

fn strip_visibility(line: &str) -> &str {
    if let Some(rest) = line.strip_prefix("pub ") {
        return rest;
    }
    if let Some(rest) = line.strip_prefix("pub(")
        && let Some((_, suffix)) = rest.split_once(") ")
    {
        return suffix;
    }
    line
}

fn relative_slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn parse_symbol_arg<'a>(
    symbol: Option<&'a str>,
    tool_name: &str,
) -> Result<&'a str, FunctionCallError> {
    let Some(symbol) = symbol.map(str::trim) else {
        return Err(FunctionCallError::RespondToModel(format!(
            "{tool_name} symbol must not be empty"
        )));
    };
    if symbol.is_empty() {
        return Err(FunctionCallError::RespondToModel(format!(
            "{tool_name} symbol must not be empty"
        )));
    }
    Ok(symbol)
}

fn parse_batch_symbol_args(
    symbols: Option<&[String]>,
    tool_name: &str,
) -> Result<Vec<String>, FunctionCallError> {
    let Some(symbols) = symbols else {
        return Err(FunctionCallError::RespondToModel(format!(
            "{tool_name} symbols must not be empty"
        )));
    };
    if symbols.is_empty() {
        return Err(FunctionCallError::RespondToModel(format!(
            "{tool_name} symbols must not be empty"
        )));
    }
    if symbols.len() > MAX_QUERY_ITEMS {
        return Err(FunctionCallError::RespondToModel(format!(
            "{tool_name} symbols list exceeds the {MAX_QUERY_ITEMS} item limit"
        )));
    }
    let mut trimmed = Vec::with_capacity(symbols.len());
    for symbol in symbols {
        let symbol = symbol.trim();
        if symbol.is_empty() {
            return Err(FunctionCallError::RespondToModel(format!(
                "{tool_name} symbols must not contain empty values"
            )));
        }
        trimmed.push(symbol.to_string());
    }
    Ok(trimmed)
}

fn render_symbol_impact_result(
    symbol: String,
    result: crate::query::SymbolImpactResult,
    source_note: &str,
) -> ImpactSymbolResult {
    let upstream_truncated = result.upstream.truncated;
    let downstream_truncated = result.downstream.truncated;
    ImpactSymbolResult {
        symbol,
        upstream: result
            .upstream
            .items
            .into_iter()
            .map(|item| ImpactSymbolSummary {
                symbol: item.symbol,
                path: item.path,
                relationship: item.relationship,
                site_line: item.site_line,
            })
            .collect(),
        downstream: result
            .downstream
            .items
            .into_iter()
            .map(|item| ImpactSymbolSummary {
                symbol: item.symbol,
                path: item.path,
                relationship: item.relationship,
                site_line: item.site_line,
            })
            .collect(),
        upstream_truncated,
        downstream_truncated,
        limits: vec![
            format!("impact items are capped at {MAX_QUERY_ITEMS} per direction"),
            source_note.to_string(),
            "The native Rust index resolves unqualified calls and some path-qualified calls; receiver dispatch, cross-file imports, macros, and non-Rust files remain bounded or unresolved."
                .to_string(),
        ],
    }
}

fn render_batch_impact_result(
    symbol: String,
    result: crate::query::SymbolImpactResult,
) -> ImpactBatchItemResult {
    let upstream_truncated = result.upstream.truncated;
    let downstream_truncated = result.downstream.truncated;
    ImpactBatchItemResult {
        symbol,
        upstream: result
            .upstream
            .items
            .into_iter()
            .map(|item| ImpactSymbolSummary {
                symbol: item.symbol,
                path: item.path,
                relationship: item.relationship,
                site_line: item.site_line,
            })
            .collect(),
        downstream: result
            .downstream
            .items
            .into_iter()
            .map(|item| ImpactSymbolSummary {
                symbol: item.symbol,
                path: item.path,
                relationship: item.relationship,
                site_line: item.site_line,
            })
            .collect(),
        upstream_truncated,
        downstream_truncated,
    }
}

fn render_diff_impact_result(result: crate::query::DiffImpactResult) -> ImpactDiffResult {
    let changed_files_truncated = result.changed_files.truncated;
    let affected_processes_truncated = result.affected_processes.truncated;
    ImpactDiffResult {
        summary: ImpactDiffSummary {
            changed_files: result.summary.changed_files,
            changed_symbols: result.summary.changed_symbols,
            affected_processes: result.summary.affected_processes,
            risk_level: result.summary.risk_level,
        },
        changed_files: result
            .changed_files
            .items
            .into_iter()
            .map(|file| ImpactDiffFileResult {
                path: file.path,
                changed_symbols_truncated: file.changed_symbols.truncated,
                changed_symbols: file
                    .changed_symbols
                    .items
                    .into_iter()
                    .map(|symbol| ImpactDiffSymbolResult {
                        symbol: symbol.symbol,
                        kind: symbol.kind,
                        path: symbol.path,
                        line: symbol.line,
                    })
                    .collect(),
            })
            .collect(),
        affected_processes: result
            .affected_processes
            .items
            .into_iter()
            .map(|process| ImpactDiffProcessResult {
                name: process.name,
                entry_symbol: process.entry_symbol,
                path: process.path,
                changed_steps: process.changed_steps,
            })
            .collect(),
        changed_files_truncated,
        affected_processes_truncated,
        warnings: result.warnings,
        limits: vec![
            format!("changed files are capped at {MAX_QUERY_ITEMS} items"),
            format!("changed symbols are capped at {MAX_QUERY_ITEMS} items per file"),
            format!("affected processes are capped at {MAX_QUERY_ITEMS} items"),
            "impact.diff supports scopes `unstaged`, `staged`, `all`, and `compare`".to_string(),
            "impact.diff reads changed files from git diff and changed symbols from the active native source-index artifact"
                .to_string(),
            "The native Rust index resolves unqualified calls and some path-qualified calls; receiver dispatch, cross-file imports, macros, and non-Rust files remain bounded or unresolved."
                .to_string(),
        ],
    }
}

#[async_trait::async_trait]
impl ToolExecutor<ToolCall> for EvidencePlannerTool {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(ONTOGRAPH_NAMESPACE, EVIDENCE_PLANNER_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        namespace_function_tool::<EvidencePlannerArgs, EvidencePlannerResult>(
            EVIDENCE_PLANNER_TOOL_NAME,
            EVIDENCE_PLANNER_DESCRIPTION,
        )
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::DirectModelOnly
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        true
    }

    async fn handle(&self, call: ToolCall) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let args = parse_tool_args::<EvidencePlannerArgs>(&call, "ontograph.evidence_planner")?;
        let shape = match args.shape.as_str() {
            "direct_lookup" => crate::evidence_planner::RetrievalShape::DirectLookup,
            "neighborhood" => crate::evidence_planner::RetrievalShape::Neighborhood,
            "dependency_path" => crate::evidence_planner::RetrievalShape::DependencyPath,
            "provenance" => crate::evidence_planner::RetrievalShape::Provenance,
            "cross_artifact_join" => crate::evidence_planner::RetrievalShape::CrossArtifactJoin,
            "temporal_version" => crate::evidence_planner::RetrievalShape::TemporalVersion,
            other => {
                return Err(FunctionCallError::RespondToModel(format!(
                    "unsupported retrieval shape `{other}`; use direct_lookup, neighborhood, dependency_path, provenance, cross_artifact_join, or temporal_version"
                )));
            }
        };
        let direction = match args.direction.as_deref() {
            Some("upstream") => crate::evidence_planner::TraversalDirection::Upstream,
            Some("downstream") => crate::evidence_planner::TraversalDirection::Downstream,
            Some("lateral") => crate::evidence_planner::TraversalDirection::Lateral,
            Some("bidirectional") | None => {
                crate::evidence_planner::TraversalDirection::Bidirectional
            }
            Some(other) => {
                return Err(FunctionCallError::RespondToModel(format!(
                    "unsupported traversal direction `{other}`; use upstream, downstream, lateral, or bidirectional"
                )));
            }
        };
        let budget = crate::evidence_planner::PlannerBudget {
            max_hops: args.max_hops.unwrap_or(2),
            max_records: args.max_records.unwrap_or(50),
            max_artifacts: args.max_artifacts.unwrap_or(10),
            max_context_bytes: args.max_context_bytes.unwrap_or(64 * 1024),
            timeout_ms: args.timeout_ms.unwrap_or(10_000),
        };
        let project_scope = self
            .thread_state
            .current_repo_root()
            .map(|root| root.display().to_string());
        let plan = crate::evidence_planner::RetrievalPlan {
            shape,
            anchor: args.anchor,
            direction,
            relation_types: args.relation_types.unwrap_or_default(),
            project_scope,
            budget,
        };
        plan.validate()
            .map_err(|e| FunctionCallError::RespondToModel(e.to_string()))?;

        let provider =
            ThreadStateOperationsProvider::with_timeout(&self.thread_state, plan.budget.timeout_ms);
        let engine = crate::evidence_planner::PlannerEngine::new(&provider);
        let subgraph = engine
            .execute(plan)
            .map_err(|e| FunctionCallError::RespondToModel(e.to_string()))?;
        let value = serde_json::to_value(EvidencePlannerResult {
            subgraph: serde_json::to_value(&subgraph)
                .map_err(|e| FunctionCallError::RespondToModel(e.to_string()))?,
            advisory_note: "Evidence planner output provides advisory graph retrieval records and references for reasoning; it does not replace source verification or execute tool permissions.".to_string(),
        })
        .map_err(|e| FunctionCallError::RespondToModel(e.to_string()))?;
        Ok(Box::new(JsonToolOutput::new(value)))
    }
}

pub(crate) struct ThreadStateOperationsProvider<'a> {
    thread_state: &'a Arc<OntographThreadState>,
    /// Planner deadline for the whole retrieval, when the caller set one.
    ///
    /// Store queries already enforce `QueryBudget::timeout_ms` internally, so
    /// the provider passes the time actually remaining instead of the fixed
    /// per-query maximum. Without this, a caller's `timeout_ms` is discarded
    /// and a multi-query traversal can run many multiples of it before the
    /// planner regains control between operations.
    deadline: Option<(std::time::Instant, std::time::Duration)>,
}

impl<'a> ThreadStateOperationsProvider<'a> {
    #[cfg(test)]
    pub(crate) fn new(thread_state: &'a Arc<OntographThreadState>) -> Self {
        Self {
            thread_state,
            deadline: None,
        }
    }

    /// Binds the provider to the planner's overall timeout budget.
    pub(crate) fn with_timeout(
        thread_state: &'a Arc<OntographThreadState>,
        timeout_ms: u64,
    ) -> Self {
        Self {
            thread_state,
            deadline: Some((
                std::time::Instant::now(),
                std::time::Duration::from_millis(timeout_ms),
            )),
        }
    }

    /// Compatibility timeout value for one store query.
    ///
    /// This helper preserves the existing query-budget contract, but it is not
    /// an admission check. Deadline-bound operations must call `admit_query`
    /// before starting work so an expired planner request does not issue a
    /// query with a clamped 1 ms budget.
    #[cfg(test)]
    pub(crate) fn query_timeout_ms(&self) -> u64 {
        match self.deadline {
            Some((started, timeout)) => timeout
                .saturating_sub(started.elapsed())
                .as_millis()
                .clamp(1, u128::from(MAX_QUERY_TIMEOUT_MS))
                as u64,
            None => MAX_QUERY_TIMEOUT_MS,
        }
    }

    /// Admit one bounded synchronous provider operation.
    ///
    /// The elapsed sample is used both for expiry and for the live remaining
    /// query budget. This rejects expired requests before further work; it
    /// does not preempt work that is already running.
    fn admit_query(&self) -> Result<u64, String> {
        match self.deadline {
            Some((started, timeout)) => {
                let elapsed = started.elapsed();
                if elapsed >= timeout {
                    return Err(format!("{:?}", QueryError::Timeout));
                }
                Ok(timeout
                    .saturating_sub(elapsed)
                    .as_millis()
                    .clamp(1, u128::from(MAX_QUERY_TIMEOUT_MS)) as u64)
            }
            None => Ok(MAX_QUERY_TIMEOUT_MS),
        }
    }

    fn project_scope(&self) -> Option<String> {
        self.thread_state
            .current_repo_root()
            .map(|root| root.display().to_string())
    }

    /// Immutable commit identity of the indexed workspace, when the host owns one.
    ///
    /// `resolve_target_head` reports an unversioned marker for a checkout with
    /// no resolvable `HEAD`. That marker is not an immutable identity, so it is
    /// reported as absent instead of being attached as provenance.
    fn source_commit(&self) -> Option<String> {
        let repo_root = self.thread_state.current_repo_root()?;
        let head = resolve_target_head(&repo_root);
        (head != UNVERSIONED_WORKSPACE_HEAD).then_some(head)
    }
}

impl<'a> crate::evidence_planner::PlannerOperationsProvider for ThreadStateOperationsProvider<'a> {
    fn search_symbols(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<crate::evidence_planner::EvidenceRecord>, String> {
        self.admit_query()?;
        let store = self
            .thread_state
            .load_current_source_index("ontograph.evidence_planner");
        self.admit_query()?;
        let store = store.map_err(|e| e.to_string())?;
        let search_result = store.search_symbols(SymbolSearchRequest {
            query: query.to_string(),
            budget: QueryBudget::bounded(self.admit_query()?, limit, MAX_SERIALIZED_BYTES),
        });
        self.admit_query()?;
        let search_result = search_result.map_err(|e| format!("{e:?}"))?;
        let project_scope = self.project_scope();
        self.admit_query()?;
        let source_commit = self.source_commit();
        self.admit_query()?;
        let mut records = Vec::new();
        for item in search_result.matches.items {
            self.admit_query()?;
            records.push(crate::evidence_planner::EvidenceRecord {
                id: format!("Symbol:{}", item.symbol),
                source_location: Some(crate::evidence_planner::SourceLocation {
                    path: item.path,
                    start_line: None,
                    end_line: None,
                }),
                project_scope: project_scope.clone(),
                version_or_hash: source_commit.clone(),
                relation: Some("symbol_match".to_string()),
                retrieval_reason: format!("textual match for query `{query}`"),
                support_status: crate::evidence_planner::EvidenceSupportStatus::Supported,
                metadata: BTreeMap::new(),
            });
        }
        self.admit_query()?;
        Ok(records)
    }

    fn docs_context(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<crate::evidence_planner::EvidenceRecord>, String> {
        self.admit_query()?;
        let artifact_root = match self.thread_state.current_artifact_root() {
            Some(root) => root,
            None => return Err("workspace context is unavailable for this turn".to_string()),
        };
        self.admit_query()?;
        let store = match DocsIndexStore::load_active_generation(&artifact_root) {
            Ok(s) => s,
            Err(e) => return Err(format!("{e:?}")),
        };
        self.admit_query()?;
        let budget = QueryBudget::bounded(self.admit_query()?, limit, MAX_SERIALIZED_BYTES);
        let docs_result = store
            .context(DocsContextRequest {
                document_id: None,
                path: None,
                heading_id: None,
                requirement_id: None,
                route: None,
                symbol: None,
                text: Some(query.to_string()),
                budget,
            })
            .map_err(|e| format!("{e:?}"))?;
        self.admit_query()?;
        let project_scope = self.project_scope();
        self.admit_query()?;
        let source_commit = self.source_commit();
        self.admit_query()?;
        let mut records = Vec::new();
        for item in docs_result.items.items {
            self.admit_query()?;
            match item {
                crate::query::DocsContextItem::Document {
                    document_id,
                    path,
                    excerpt,
                    ..
                } => {
                    records.push(crate::evidence_planner::EvidenceRecord {
                        id: format!("Doc:{document_id}"),
                        source_location: Some(crate::evidence_planner::SourceLocation {
                            path,
                            start_line: None,
                            end_line: None,
                        }),
                        project_scope: project_scope.clone(),
                        version_or_hash: source_commit.clone(),
                        relation: Some("document".to_string()),
                        retrieval_reason: format!("docs excerpt match: {excerpt}"),
                        support_status: crate::evidence_planner::EvidenceSupportStatus::Supported,
                        metadata: BTreeMap::new(),
                    });
                }
                crate::query::DocsContextItem::Fact {
                    fact_id,
                    document_id,
                    text,
                    line_start,
                    line_end,
                    ..
                } => {
                    records.push(crate::evidence_planner::EvidenceRecord {
                        id: format!("DocFact:{fact_id}"),
                        source_location: Some(crate::evidence_planner::SourceLocation {
                            path: document_id,
                            start_line: Some(line_start),
                            end_line: Some(line_end),
                        }),
                        project_scope: project_scope.clone(),
                        version_or_hash: source_commit.clone(),
                        relation: Some("doc_fact".to_string()),
                        retrieval_reason: format!("doc fact: {text}"),
                        support_status: crate::evidence_planner::EvidenceSupportStatus::Supported,
                        metadata: BTreeMap::new(),
                    });
                }
            }
        }
        self.admit_query()?;
        Ok(records)
    }

    fn inspect_context(
        &self,
        symbol: &str,
    ) -> Result<Option<crate::evidence_planner::EvidenceRecord>, String> {
        self.admit_query()?;
        let store = match self
            .thread_state
            .load_current_source_index("ontograph.evidence_planner")
        {
            Ok(store) => store,
            Err(e) => return Err(e.to_string()),
        };
        self.admit_query()?;
        let budget = QueryBudget::bounded(self.admit_query()?, 5, MAX_SERIALIZED_BYTES);
        let search_result = store
            .search_symbols(SymbolSearchRequest {
                query: symbol.to_string(),
                budget,
            })
            .map_err(|e| format!("{e:?}"))?;
        self.admit_query()?;
        if let Some(first) = search_result.matches.items.into_iter().next() {
            self.admit_query()?;
            let project_scope = self.project_scope();
            self.admit_query()?;
            let source_commit = self.source_commit();
            self.admit_query()?;
            Ok(Some(crate::evidence_planner::EvidenceRecord {
                id: format!("Symbol:{}", first.symbol),
                source_location: Some(crate::evidence_planner::SourceLocation {
                    path: first.path,
                    start_line: None,
                    end_line: None,
                }),
                project_scope,
                version_or_hash: source_commit,
                relation: Some("inspected_symbol".to_string()),
                retrieval_reason: format!("context inspection for `{symbol}`"),
                support_status: crate::evidence_planner::EvidenceSupportStatus::Supported,
                metadata: BTreeMap::new(),
            }))
        } else {
            self.admit_query()?;
            Ok(None)
        }
    }

    fn impact_symbol(
        &self,
        symbol: &str,
        direction: crate::evidence_planner::TraversalDirection,
        max_hops: usize,
    ) -> Result<Vec<crate::evidence_planner::EvidenceRecord>, String> {
        self.impact_symbol_bounded(symbol, direction, max_hops, usize::MAX)
            .map(|result| result.records)
    }

    fn impact_symbol_bounded(
        &self,
        symbol: &str,
        direction: crate::evidence_planner::TraversalDirection,
        max_hops: usize,
        max_records: usize,
    ) -> Result<crate::evidence_planner::TraversalResult, String> {
        self.admit_query()?;
        let store = match self
            .thread_state
            .load_current_source_index("ontograph.evidence_planner")
        {
            Ok(store) => store,
            Err(e) => return Err(e.to_string()),
        };
        self.admit_query()?;
        let project_scope = self.project_scope();
        self.admit_query()?;
        let source_commit = self.source_commit();
        self.admit_query()?;
        // Breadth-first expansion over the impact graph. Depth is counted in
        // hops from the anchor, and each symbol is visited once so a cyclic
        // call graph terminates instead of re-expanding the same node.
        //
        // Bound retained records and frontier nodes before allocation. One
        // additional candidate detects overflow without retaining it.
        let mut records = Vec::new();
        let mut truncated = false;
        let mut visited = BTreeSet::from([symbol.to_string()]);
        let mut frontier = vec![symbol.to_string()];
        for hop in 1..=max_hops {
            self.admit_query()?;
            let mut next_frontier = Vec::new();
            for current in frontier {
                self.admit_query()?;
                // Recomputed per frontier node: this loop issues one store
                // query per node, so a fixed per-query bound would let one
                // traversal run far past the caller's overall timeout.
                let remaining = max_records.saturating_sub(records.len());
                let query_items = if max_records == usize::MAX {
                    20
                } else {
                    remaining.saturating_add(1)
                };
                let budget =
                    QueryBudget::bounded(self.admit_query()?, query_items, MAX_SERIALIZED_BYTES);
                let impact_result = match store.impact_symbol(SymbolImpactRequest {
                    symbol: current.clone(),
                    budget,
                }) {
                    Ok(result) => result,
                    // A neighbor that is missing or ambiguous in the index ends
                    // this branch; it does not invalidate the anchor traversal.
                    Err(QueryError::MissingArtifact | QueryError::AmbiguousArtifact)
                        if current != symbol =>
                    {
                        continue;
                    }
                    Err(e) => return Err(format!("{e:?}")),
                };
                self.admit_query()?;
                let items = match direction {
                    crate::evidence_planner::TraversalDirection::Upstream => {
                        truncated |= impact_result.upstream.truncated;
                        impact_result.upstream.items
                    }
                    crate::evidence_planner::TraversalDirection::Downstream => {
                        truncated |= impact_result.downstream.truncated;
                        impact_result.downstream.items
                    }
                    crate::evidence_planner::TraversalDirection::Lateral
                    | crate::evidence_planner::TraversalDirection::Bidirectional => {
                        truncated |=
                            impact_result.upstream.truncated || impact_result.downstream.truncated;
                        let mut all = impact_result.upstream.items;
                        all.extend(impact_result.downstream.items);
                        all
                    }
                };
                for item in items {
                    self.admit_query()?;
                    if visited.contains(&item.symbol) {
                        continue;
                    }
                    if records.len() == max_records {
                        return Ok(crate::evidence_planner::TraversalResult {
                            records,
                            truncated: true,
                        });
                    }
                    visited.insert(item.symbol.clone());
                    self.admit_query()?;
                    let mut metadata = BTreeMap::new();
                    metadata.insert("hops".to_string(), hop.to_string());
                    records.push(crate::evidence_planner::EvidenceRecord {
                        id: format!("Impact:{}", item.symbol),
                        source_location: Some(crate::evidence_planner::SourceLocation {
                            path: item.path,
                            start_line: None,
                            end_line: None,
                        }),
                        project_scope: project_scope.clone(),
                        version_or_hash: source_commit.clone(),
                        relation: Some(item.relationship),
                        retrieval_reason: format!(
                            "symbol impact neighbor of `{symbol}` at hop {hop}"
                        ),
                        support_status: crate::evidence_planner::EvidenceSupportStatus::Supported,
                        metadata,
                    });
                    next_frontier.push(item.symbol);
                }
            }
            self.admit_query()?;
            if next_frontier.is_empty() {
                break;
            }
            frontier = next_frontier;
        }
        self.admit_query()?;
        Ok(crate::evidence_planner::TraversalResult { records, truncated })
    }

    fn procedural_project(
        &self,
        anchor: &str,
        hops: usize,
    ) -> Result<Vec<crate::evidence_planner::EvidenceRecord>, String> {
        Ok(self
            .procedural_project_bounded(anchor, hops, MAX_PLANNER_PROCEDURAL_RECORDS)?
            .records)
    }

    fn procedural_project_bounded(
        &self,
        anchor: &str,
        hops: usize,
        max_records: usize,
    ) -> Result<crate::evidence_planner::TraversalResult, String> {
        self.admit_query()?;
        let Some(repo_root) = self.thread_state.current_repo_root() else {
            return Err("workspace context is unavailable for this turn".to_string());
        };
        self.admit_query()?;
        let Some(store) = KmStore::for_project_path(&repo_root) else {
            return Err("workspace context is unavailable for this turn".to_string());
        };
        self.admit_query()?;
        let project_scope = self.project_scope();
        self.admit_query()?;
        let retained_cap = max_records.min(MAX_PLANNER_PROCEDURAL_RECORDS);
        let mut records = Vec::new();
        let mut truncated = false;
        let mut remaining = retained_cap;
        // The procedural projection clamps traversal to its own safety cap,
        // which is lower than the planner's public hop budget. Report the
        // reduced depth as evidence so a bounded result is never mistaken for
        // a complete traversal at the requested depth. The hop-limit record
        // itself consumes capacity and proves overflow when capacity is zero.
        if hops > crate::procedural_graph::MAX_REQUESTED_HOPS {
            if remaining == 0 {
                truncated = true;
                self.admit_query()?;
                return Ok(crate::evidence_planner::TraversalResult { records, truncated });
            }
            records.push(crate::evidence_planner::EvidenceRecord {
                id: format!("Blocked:ProceduralHopLimit:{anchor}"),
                source_location: None,
                project_scope: project_scope.clone(),
                version_or_hash: None,
                relation: Some("procedural_hop_limit".to_string()),
                retrieval_reason: format!(
                    "procedural traversal requested {hops} hop(s) from `{anchor}`"
                ),
                support_status: crate::evidence_planner::EvidenceSupportStatus::Blocked {
                    reason: format!(
                        "procedural traversal depth reduced to {} hop(s); evidence beyond that depth was not collected",
                        crate::procedural_graph::MAX_REQUESTED_HOPS
                    ),
                },
                metadata: BTreeMap::from([(
                    "effective_hops".to_string(),
                    crate::procedural_graph::MAX_REQUESTED_HOPS.to_string(),
                )]),
            });
            remaining -= 1;
        }
        // Only active, non-withdrawn records are projectable. Agent-authored
        // records stay owned by their authoring thread, which this read-only
        // surface cannot claim, so they are excluded rather than leaked.
        self.admit_query()?;
        let listed_records = store.list(/*include_obsolete*/ false).map_err(|err| {
            format!(
                "failed to read the KM cache: {}",
                km_store_error_message(err)
            )
        })?;
        self.admit_query()?;
        for record in listed_records {
            self.admit_query()?;
            if record.withdrawal.is_some()
                || matches!(record.origin, Some(KmIndexOrigin::AgentThread { .. }))
            {
                continue;
            }
            if let (Some(expected), Some(found)) = (&project_scope, &record.project_scope)
                && expected != found
            {
                continue;
            }
            // `validated_module_root` recomputes the cache path, refuses an
            // escaping directory, and verifies the stored content hash, so a
            // stale or tampered package is reported instead of projected.
            self.admit_query()?;
            let root = match store.validated_module_root(&record) {
                Ok(root) => root,
                Err(err) => {
                    if remaining == 0 {
                        truncated = true;
                        break;
                    }
                    records.push(crate::evidence_planner::EvidenceRecord {
                        id: format!("Km:{}@{}", record.id, record.version),
                        source_location: None,
                        project_scope: project_scope.clone(),
                        version_or_hash: None,
                        relation: Some("knowledge_module".to_string()),
                        retrieval_reason: format!(
                            "knowledge module `{}` failed cache validation",
                            record.id
                        ),
                        support_status: crate::evidence_planner::EvidenceSupportStatus::Blocked {
                            reason: km_store_error_message(err).to_string(),
                        },
                        metadata: BTreeMap::new(),
                    });
                    remaining -= 1;
                    continue;
                }
            };
            self.admit_query()?;
            // Transitions are optional in a v1 package. `validated_module_root`
            // already rejected a malformed file, so absence projects as an
            // empty graph rather than an error.
            self.admit_query()?;
            let transitions = if root.join("transitions.jsonl").is_file() {
                crate::km::load_transitions(&root).unwrap_or_default()
            } else {
                Vec::new()
            };
            self.admit_query()?;
            let graph =
                crate::procedural_graph::ProceduralGraph::from_validated_transitions(transitions);
            self.admit_query()?;
            // Pass the still-remaining record capacity into the graph so
            // omitted edges are reported as truncation instead of being
            // discovered only after an unbounded projection.
            let (lookup, graph_truncated) = graph.project_bounded(anchor, hops, remaining);
            truncated |= graph_truncated;
            let crate::procedural_graph::ProceduralLookup::Localized(neighborhood) = lookup else {
                if truncated {
                    break;
                }
                continue;
            };
            self.admit_query()?;
            let module_path = root.display().to_string();
            for edge in neighborhood.edges {
                self.admit_query()?;
                if remaining == 0 {
                    truncated = true;
                    break;
                }
                self.admit_query()?;
                let mut metadata = BTreeMap::new();
                metadata.insert("hops".to_string(), edge.hops.to_string());
                metadata.insert("km_id".to_string(), record.id.clone());
                metadata.insert("km_version".to_string(), record.version.clone());
                metadata.insert("source".to_string(), edge.transition.source.clone());
                metadata.insert("target".to_string(), edge.transition.target.clone());
                records.push(crate::evidence_planner::EvidenceRecord {
                    id: format!(
                        "KmTransition:{}@{}:{}->{}",
                        record.id, record.version, edge.transition.source, edge.transition.target
                    ),
                    source_location: Some(crate::evidence_planner::SourceLocation {
                        path: module_path.clone(),
                        start_line: None,
                        end_line: None,
                    }),
                    project_scope: project_scope.clone(),
                    version_or_hash: Some(record.content_hash.clone()),
                    relation: Some(edge.transition.relation.clone()),
                    retrieval_reason: format!(
                        "procedural transition {} hop(s) from `{anchor}` in knowledge module `{}`",
                        edge.hops, record.id
                    ),
                    support_status: crate::evidence_planner::EvidenceSupportStatus::Supported,
                    metadata,
                });
                remaining -= 1;
            }
            if truncated {
                break;
            }
            // At exact capacity, keep scanning later eligible modules with a
            // zero-capacity projection until an omitted edge or blocked record
            // proves truncation. Capacity equality alone is not overflow.
        }
        self.admit_query()?;
        Ok(crate::evidence_planner::TraversalResult { records, truncated })
    }
}

#[cfg(test)]
#[path = "tool_tests.rs"]
mod tests;
