use crate::audit_session::AuditSessionLock;
use crate::audit_session::AuditSessionRecord;
use crate::audit_session::AuditSessionSourceIdentity;
use crate::audit_session::AuditSessionState;
use crate::audit_session::current_source_identity_for_action;
use crate::manifest::OntographArtifactRoot;
use ontocode_extension_api::FunctionCallError;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
pub(crate) struct ManageSessionResult {
    pub session: Option<AuditSessionRecord>,
    pub lock: Option<AuditSessionLock>,
    pub active_source_identity: Option<AuditSessionSourceIdentity>,
    pub session_path: Option<String>,
    pub lock_path: Option<String>,
    pub notes: Vec<String>,
}

pub(crate) fn run_manage_session(
    root: &OntographArtifactRoot,
) -> Result<ManageSessionResult, FunctionCallError> {
    let state = AuditSessionState::new(root);
    let active_source_identity = current_source_identity_for_action(root, "manage.session").ok();

    let (session, lock) = match state.load_pair() {
        Ok((s, l)) => (Some(s), Some(l)),
        Err(_) => (None, None),
    };

    Ok(ManageSessionResult {
        session,
        lock,
        active_source_identity,
        session_path: Some(state.session_path()),
        lock_path: Some(state.lock_path()),
        notes: vec![
            "Read-only inspection of the native session model without mutating active lock state."
                .to_string(),
        ],
    })
}
