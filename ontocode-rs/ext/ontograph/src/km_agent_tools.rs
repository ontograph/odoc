//! Bounded agent-authored knowledge verbs.
//!
//! Exactly three verbs live here: `ontograph.km_author`, `ontograph.km_list`,
//! and `ontograph.km_withdraw`. The read-only `ontograph.km_context` verb stays
//! in `tool.rs` and is unchanged.
//!
//! Thread identity and project scope are always taken from host-owned state, so
//! no schema in this module exposes a `thread_id` or `project_scope` field and a
//! model cannot spoof either one.
//!
//! These verbs are exercised by this module's tests and are registered on the
//! accepted embedded host by KAS-3.1, which is the first production caller.
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use ontocode_extension_api::FunctionCallError;
use ontocode_extension_api::JsonToolOutput;
use ontocode_extension_api::ToolCall;
use ontocode_extension_api::ToolExecutor;
use ontocode_extension_api::ToolName;
use ontocode_extension_api::ToolOutput;
use ontocode_extension_api::ToolSpec;
use ontocode_tools::ToolExposure;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::km_agent_authoring::ActivationRejection;
use crate::km_agent_authoring::AgentCandidate;
use crate::km_agent_authoring::activate_agent_candidate;
use crate::km_agent_authoring::resolve_inside_project;
use crate::km_store::KmIndexOrigin;
use crate::km_store::KmStore;
use crate::tool::ONTOGRAPH_NAMESPACE;
use crate::tool::OntographThreadState;
use crate::tool::namespace_function_tool;
use crate::tool::parse_tool_args;

#[cfg(test)]
#[path = "km_agent_tools_tests.rs"]
mod tests;

pub(crate) const KM_AUTHOR_TOOL_NAME: &str = "km_author";
pub(crate) const KM_LIST_TOOL_NAME: &str = "km_list";
pub(crate) const KM_WITHDRAW_TOOL_NAME: &str = "km_withdraw";

/// Maximum records returned by one `ontograph.km_list` call.
const MAX_LISTED_RECORDS: usize = 50;

/// Host-owned identity for one authoring turn. Built by the extension from
/// thread and environment state, never from model arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentToolIdentity {
    pub thread_id: String,
    pub project_root: PathBuf,
}

impl AgentToolIdentity {
    /// Fails closed when either half of the identity is unavailable.
    pub(crate) fn new(thread_id: &str, project_root: PathBuf) -> Option<Self> {
        if thread_id.trim().is_empty() {
            return None;
        }
        Some(Self {
            thread_id: thread_id.to_string(),
            project_root,
        })
    }

    fn origin(&self) -> KmIndexOrigin {
        KmIndexOrigin::AgentThread {
            thread_id: self.thread_id.clone(),
        }
    }

    fn store(&self) -> Result<(KmStore, PathBuf), FunctionCallError> {
        let store_root = self.project_root.join(".ontocode/ontograph/km/_store");
        let store = KmStore::new(&store_root);
        Ok((store, store_root))
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct KmAuthorArgs {
    /// Project-relative directory of the candidate package.
    pub package_path: String,
    /// Project-relative path of the committed evidence backing the claim.
    pub citation_path: String,
    /// Commit the citation bytes must match.
    pub source_commit: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub(crate) struct KmAuthorResult {
    pub id: String,
    pub version: String,
    pub status: String,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct KmListArgs {
    /// Include withdrawn and superseded records. Defaults to active only.
    #[serde(default)]
    pub include_withdrawn: bool,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct KmListEntry {
    pub id: String,
    pub version: String,
    pub status: String,
    pub withdrawn_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub(crate) struct KmListResult {
    pub records: Vec<KmListEntry>,
    pub truncated: bool,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct KmWithdrawArgs {
    /// Package id to withdraw.
    pub id: String,
    /// Exact version to withdraw.
    pub version: String,
    /// Why the record is being withdrawn.
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub(crate) struct KmWithdrawResult {
    pub id: String,
    pub version: String,
    pub withdrawn: bool,
}

/// Authors one candidate under the calling thread's identity.
pub(crate) fn run_km_author(
    identity: &AgentToolIdentity,
    args: &KmAuthorArgs,
) -> Result<KmAuthorResult, FunctionCallError> {
    let (_, store_root) = identity.store()?;
    // A relative, traversal-free path is not sufficient on its own: a symlink
    // inside the project can still resolve outside it, so the package is
    // resolved and proven to stay under the host-owned project root.
    let package =
        resolve_inside_project(&identity.project_root, &args.package_path).ok_or_else(|| {
            FunctionCallError::RespondToModel(
                "ontograph.km_author package_path must resolve to a directory inside the project"
                    .to_string(),
            )
        })?;
    // Topic quality is judged on the package's own summary, not on its
    // directory name, so a caller cannot pass the floor by naming a folder well.
    let topic_text = package_summary_text(&package).ok_or_else(|| {
        FunctionCallError::RespondToModel(
            "ontograph.km_author could not read the candidate package summary".to_string(),
        )
    })?;
    let candidate = AgentCandidate {
        source_path: &package,
        citation_path: args.citation_path.trim(),
        source_commit: args.source_commit.trim(),
        topic_text: &topic_text,
    };
    let imported = activate_agent_candidate(
        &store_root,
        &identity.project_root,
        &identity.thread_id,
        &candidate,
    )
    .map_err(author_rejection_message)?;
    Ok(KmAuthorResult {
        id: imported.id,
        version: imported.version,
        status: imported.status,
        limits: vec![
            "citations must be committed project-local files".to_string(),
            "candidates are evaluated in a temporary store before activation".to_string(),
        ],
    })
}

/// Lists only this thread's own agent-authored records for this project.
pub(crate) fn run_km_list(
    identity: &AgentToolIdentity,
    args: &KmListArgs,
) -> Result<KmListResult, FunctionCallError> {
    let (store, _) = identity.store()?;
    let origin = identity.origin();
    let mut records = store
        .list_for_origin(
            args.include_withdrawn,
            Some(&origin),
            Some(&identity.thread_id),
        )
        .map_err(|_| {
            FunctionCallError::RespondToModel(
                "ontograph.km_list could not read the KM index".to_string(),
            )
        })?;
    let truncated = records.len() > MAX_LISTED_RECORDS;
    records.truncate(MAX_LISTED_RECORDS);
    Ok(KmListResult {
        records: records
            .into_iter()
            .map(|record| KmListEntry {
                id: record.id,
                version: record.version,
                status: format!("{:?}", record.status).to_lowercase(),
                withdrawn_reason: record.withdrawal.map(|withdrawal| withdrawal.reason),
            })
            .collect(),
        truncated,
        limits: vec![format!("at most {MAX_LISTED_RECORDS} records are returned")],
    })
}

/// Withdraws one record this thread authored.
pub(crate) fn run_km_withdraw(
    identity: &AgentToolIdentity,
    args: &KmWithdrawArgs,
) -> Result<KmWithdrawResult, FunctionCallError> {
    let (store, _) = identity.store()?;
    let id = args.id.trim();
    let version = args.version.trim();
    let reason = args.reason.trim();
    if id.is_empty() || version.is_empty() || reason.is_empty() {
        return Err(FunctionCallError::RespondToModel(
            "ontograph.km_withdraw requires id, version, and reason".to_string(),
        ));
    }

    // A thread may only withdraw what it authored.
    let origin = identity.origin();
    let owned = store
        .list_for_origin(
            /*include_obsolete*/ true,
            Some(&origin),
            Some(&identity.thread_id),
        )
        .map_err(|_| {
            FunctionCallError::RespondToModel(
                "ontograph.km_withdraw could not read the KM index".to_string(),
            )
        })?;
    if !owned
        .iter()
        .any(|record| record.id == id && record.version == version)
    {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.km_withdraw has no record `{id}` at version `{version}` for this thread"
        )));
    }

    let withdrawn = store
        .withdraw(id, version, reason.to_string())
        .map_err(|_| {
            FunctionCallError::RespondToModel(
                "ontograph.km_withdraw could not update the KM index".to_string(),
            )
        })?;
    Ok(KmWithdrawResult {
        id: id.to_string(),
        version: version.to_string(),
        withdrawn: withdrawn.is_some(),
    })
}

/// Surfaces one bounded rejection message without leaking store internals.
fn author_rejection_message(rejection: ActivationRejection) -> FunctionCallError {
    let detail = match rejection {
        ActivationRejection::Citation(_) => {
            "citation must be a committed, unmodified project-local file at the declared commit"
        }
        ActivationRejection::Topic(_) => "package topic words are too generic to retrieve reliably",
        ActivationRejection::ThreadCeiling { ceiling, current } => {
            return FunctionCallError::RespondToModel(format!(
                "ontograph.km_author thread ceiling reached: {current} of {ceiling} records already authored"
            ));
        }
        ActivationRejection::Staging(_) => "candidate did not pass isolated evaluation",
    };
    FunctionCallError::RespondToModel(format!("ontograph.km_author rejected: {detail}"))
}

/// Names of the three verbs this module owns, in registration order.
#[cfg(test)]
pub(crate) fn agent_tool_names() -> [&'static str; 3] {
    [
        KM_AUTHOR_TOOL_NAME,
        KM_LIST_TOOL_NAME,
        KM_WITHDRAW_TOOL_NAME,
    ]
}

/// Builds the three agent knowledge tool executors for one thread.
pub(crate) fn agent_knowledge_tools(
    thread_state: Arc<OntographThreadState>,
    thread_id: String,
) -> Vec<Arc<dyn ToolExecutor<ToolCall>>> {
    vec![
        Arc::new(KmAuthorTool {
            thread_state: thread_state.clone(),
            thread_id: thread_id.clone(),
        }),
        Arc::new(KmListTool {
            thread_state: thread_state.clone(),
            thread_id: thread_id.clone(),
        }),
        Arc::new(KmWithdrawTool {
            thread_state,
            thread_id,
        }),
    ]
}

/// Resolves host-owned identity for one turn. The project root comes from the
/// thread's current working directory, never from model arguments.
fn resolve_identity(
    thread_state: &OntographThreadState,
    thread_id: &str,
    verb: &str,
) -> Result<AgentToolIdentity, FunctionCallError> {
    let unavailable = || {
        FunctionCallError::RespondToModel(format!(
            "ontograph.{verb} workspace context is unavailable for this turn"
        ))
    };
    let cwd = thread_state.current_cwd().ok_or_else(unavailable)?;
    let project_root = crate::km_store::project_root(&cwd).ok_or_else(unavailable)?;
    AgentToolIdentity::new(thread_id, project_root).ok_or_else(unavailable)
}

struct KmAuthorTool {
    thread_state: Arc<OntographThreadState>,
    thread_id: String,
}

#[async_trait::async_trait]
impl ToolExecutor<ToolCall> for KmAuthorTool {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(ONTOGRAPH_NAMESPACE, KM_AUTHOR_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        namespace_function_tool::<KmAuthorArgs, KmAuthorResult>(
            KM_AUTHOR_TOOL_NAME,
            "Store durable project knowledge backed by a committed citation.",
        )
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::DirectModelOnly
    }

    async fn handle(&self, call: ToolCall) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let args = parse_tool_args::<KmAuthorArgs>(&call, "ontograph.km_author")?;
        let identity = resolve_identity(&self.thread_state, &self.thread_id, "km_author")?;
        json_output(run_km_author(&identity, &args)?, "km_author")
    }
}

struct KmListTool {
    thread_state: Arc<OntographThreadState>,
    thread_id: String,
}

#[async_trait::async_trait]
impl ToolExecutor<ToolCall> for KmListTool {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(ONTOGRAPH_NAMESPACE, KM_LIST_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        namespace_function_tool::<KmListArgs, KmListResult>(
            KM_LIST_TOOL_NAME,
            "List durable project knowledge authored by this thread.",
        )
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::DirectModelOnly
    }

    async fn handle(&self, call: ToolCall) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let args = parse_tool_args::<KmListArgs>(&call, "ontograph.km_list")?;
        let identity = resolve_identity(&self.thread_state, &self.thread_id, "km_list")?;
        json_output(run_km_list(&identity, &args)?, "km_list")
    }
}

struct KmWithdrawTool {
    thread_state: Arc<OntographThreadState>,
    thread_id: String,
}

#[async_trait::async_trait]
impl ToolExecutor<ToolCall> for KmWithdrawTool {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(ONTOGRAPH_NAMESPACE, KM_WITHDRAW_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        namespace_function_tool::<KmWithdrawArgs, KmWithdrawResult>(
            KM_WITHDRAW_TOOL_NAME,
            "Withdraw durable project knowledge authored by this thread.",
        )
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::DirectModelOnly
    }

    async fn handle(&self, call: ToolCall) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let args = parse_tool_args::<KmWithdrawArgs>(&call, "ontograph.km_withdraw")?;
        let identity = resolve_identity(&self.thread_state, &self.thread_id, "km_withdraw")?;
        json_output(run_km_withdraw(&identity, &args)?, "km_withdraw")
    }
}

fn json_output<T: Serialize>(
    value: T,
    verb: &str,
) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
    let json = serde_json::to_value(value).map_err(|error| {
        FunctionCallError::RespondToModel(format!(
            "failed to serialize ontograph.{verb} result: {error}"
        ))
    })?;
    Ok(Box::new(JsonToolOutput::new(json)))
}

/// Reads the candidate manifest's `summary` line, which is the text the topic
/// gate scores. Returns `None` when the manifest is missing or has no summary.
fn package_summary_text(package: &Path) -> Option<String> {
    let manifest = std::fs::read_to_string(package.join("km.toml")).ok()?;
    manifest.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("summary")?.trim_start();
        let value = rest.strip_prefix('=')?.trim();
        Some(value.trim_matches('"').to_string())
    })
}

/// Verbs advertised for a turn. The feature gate is the only switch: when it is
/// disabled the agent verbs do not exist for the model at all.
#[cfg(test)]
pub(crate) fn advertised_agent_tool_names(feature_enabled: bool) -> Vec<&'static str> {
    if feature_enabled {
        agent_tool_names().to_vec()
    } else {
        Vec::new()
    }
}
