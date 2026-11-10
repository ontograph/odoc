use ontocode_extension_api::FunctionCallError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::audit_build::AuditAction;
use crate::audit_session::AuditSessionBundleStatus;
use crate::audit_session::run_audit_session_bundle;
use crate::manifest::OntographArtifactRoot;

/// Hard cap on the rendered prompt-packet body length. Read-only render only;
/// this is not a model-context fragment, it is returned as tool output.
const MAX_PROMPT_CHARS: usize = 4000;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditSessionDispatchStatus {
    Ready,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditSessionDispatchResult {
    pub action: AuditAction,
    pub status: AuditSessionDispatchStatus,
    pub bundle_id: String,
    pub prompt_packet: String,
    pub prompt_truncated: bool,
    pub notes: Vec<String>,
}

fn inconclusive(bundle_id: String, note: String) -> AuditSessionDispatchResult {
    AuditSessionDispatchResult {
        action: AuditAction::SessionDispatch,
        status: AuditSessionDispatchStatus::Inconclusive,
        bundle_id,
        prompt_packet: String::new(),
        prompt_truncated: false,
        notes: vec![note],
    }
}

pub(crate) fn run_audit_session_dispatch(
    root: &OntographArtifactRoot,
    bundle_id: String,
) -> Result<AuditSessionDispatchResult, FunctionCallError> {
    let bundle_result = run_audit_session_bundle(root)?;
    if bundle_result.status != AuditSessionBundleStatus::Pass {
        return Ok(inconclusive(
            bundle_id,
            "Could not load valid bundle state for session dispatch.".to_string(),
        ));
    }

    let mut matches = bundle_result
        .bundles
        .into_iter()
        .filter(|b| b.bundle_id == bundle_id);
    let bundle = match matches.next() {
        Some(b) => b,
        None => {
            return Ok(inconclusive(
                bundle_id,
                "Specified bundle_id not found in active audit session.".to_string(),
            ));
        }
    };
    // A well-formed bundle set has unique ids; reject duplicate matches closed.
    if matches.next().is_some() {
        return Ok(inconclusive(
            bundle_id,
            "Specified bundle_id is ambiguous across multiple bundles.".to_string(),
        ));
    }

    let full = render_prompt_packet(&bundle);
    let prompt_truncated = full.chars().count() > MAX_PROMPT_CHARS;
    let prompt_packet = full.chars().take(MAX_PROMPT_CHARS).collect::<String>();

    Ok(AuditSessionDispatchResult {
        action: AuditAction::SessionDispatch,
        status: AuditSessionDispatchStatus::Ready,
        bundle_id: bundle.bundle_id,
        prompt_packet,
        prompt_truncated,
        notes: vec![
            "audit.session_dispatch renders a read-only prompt packet from one existing bundle."
                .to_string(),
            "No worker, scheduler, planner, router, or session mutation was created.".to_string(),
        ],
    })
}

fn render_prompt_packet(bundle: &crate::audit_session::AuditSessionBundleItem) -> String {
    let mut out = String::new();
    out.push_str(&format!("bundle_id: {}\n", bundle.bundle_id));
    out.push_str(&format!("finding_ids: {}\n", bundle.finding_ids.join(", ")));
    out.push_str(&format!("owner_files: {}\n", bundle.owner_files.join(", ")));
    out.push_str(&format!(
        "target_symbols: {}\n",
        bundle.target_symbols.join(", ")
    ));
    out.push_str(&format!(
        "required_tests: {}\n",
        bundle.required_tests.join(", ")
    ));
    out.push_str(&format!(
        "stop_conditions: {}\n",
        bundle.stop_conditions.join("; ")
    ));
    out.push_str(&format!(
        "files_count: {} loc_estimate: {} cap_exceeded: {}\n",
        bundle.files_count, bundle.loc_estimate, bundle.cap_exceeded
    ));
    out
}
