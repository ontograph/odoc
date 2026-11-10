use std::collections::BTreeMap;
use std::fs;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use ontocode_extension_api::FunctionCallError;
use ontocode_utils_path::write_atomically;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::audit_build::AuditAction;
use crate::manifest::OntographArtifactRoot;
use crate::manifest::OntographManifest;

const AUDIT_SESSION_FILE: &str = "audit-session.json";
const AUDIT_SESSION_LOCK_FILE: &str = "audit-session.lock.json";
const MAX_SESSION_ID_LENGTH: usize = 120;
const MAX_FINDINGS: usize = 200;
const OPEN_FINDING_STATUS: &str = "open";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditSessionFindingInput {
    #[serde(default)]
    pub fingerprint: Option<String>,
    #[serde(default)]
    pub finding_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub write_set: Vec<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub line: Option<usize>,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditSessionFindingSummary {
    #[serde(default)]
    pub fingerprint: Option<String>,
    pub finding_id: String,
    pub title: String,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub write_set: Vec<String>,
    pub path: Option<String>,
    pub line: Option<usize>,
    pub severity: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditSessionSourceIdentity {
    pub generation_id: String,
    pub source_index_content_hash: String,
    pub target_head: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditSessionRecordStatus {
    Open,
    StaleSession,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditSessionRecord {
    pub session_id: String,
    pub created_at_unix_seconds: i64,
    pub status: AuditSessionRecordStatus,
    pub source_identity: AuditSessionSourceIdentity,
    pub findings: Vec<AuditSessionFindingSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditSessionLock {
    pub session_id: String,
    pub source_identity: AuditSessionSourceIdentity,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditSessionStartStatus {
    Created,
    StaleSession,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditSessionStartResult {
    pub action: AuditAction,
    pub status: AuditSessionStartStatus,
    pub session: AuditSessionRecord,
    pub lock: AuditSessionLock,
    pub session_path: String,
    pub lock_path: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditSessionVerifyStatus {
    Current,
    StaleSession,
    Missing,
    Incomplete,
    Corrupt,
    Mismatched,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditSessionVerifyResult {
    pub action: AuditAction,
    pub status: AuditSessionVerifyStatus,
    pub session: Option<AuditSessionRecord>,
    pub lock: Option<AuditSessionLock>,
    pub active_source_identity: AuditSessionSourceIdentity,
    pub session_path: String,
    pub lock_path: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditSessionDedupeStatus {
    Pass,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditSessionDedupeResult {
    pub action: AuditAction,
    pub status: AuditSessionDedupeStatus,
    pub session_id: Option<String>,
    pub groups: Vec<AuditSessionDedupeGroup>,
    pub ungrouped_limitations: Vec<AuditSessionDedupeLimitation>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditSessionDedupeGroup {
    pub key_kind: AuditSessionDedupeKeyKind,
    pub key: String,
    pub finding_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditSessionDedupeLimitation {
    pub finding_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditSessionDedupeKeyKind {
    Fingerprint,
    Symbol,
    WriteSet,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditSessionBundleStatus {
    Pass,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditSessionBundleResult {
    pub action: AuditAction,
    pub status: AuditSessionBundleStatus,
    pub session_id: Option<String>,
    pub bundles: Vec<AuditSessionBundleItem>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditSessionBundleItem {
    pub bundle_id: String,
    pub finding_ids: Vec<String>,
    pub owner_files: Vec<String>,
    pub target_symbols: Vec<String>,
    pub required_tests: Vec<String>,
    pub stop_conditions: Vec<String>,
    pub files_count: usize,
    pub loc_estimate: usize,
    pub cap_exceeded: bool,
}

pub(crate) fn run_audit_session_start(
    root: &OntographArtifactRoot,
    session_id: Option<String>,
    findings: Vec<AuditSessionFindingInput>,
    created_at_unix_seconds: Option<i64>,
) -> Result<AuditSessionStartResult, FunctionCallError> {
    let state = AuditSessionState::new(root);
    let source_identity = current_source_identity(root)?;
    match state.load_existing()? {
        Some((session, lock)) if lock.source_identity != source_identity => {
            return Ok(AuditSessionStartResult {
                action: AuditAction::SessionStart,
                status: AuditSessionStartStatus::StaleSession,
                session: AuditSessionRecord {
                    status: AuditSessionRecordStatus::StaleSession,
                    ..session
                },
                lock,
                session_path: state.session_path(),
                lock_path: state.lock_path(),
                notes: vec![
                    "Existing audit session lock does not match the active source-index identity."
                        .to_string(),
                    "No new session was created; remove or resolve the stale local session state before continuing."
                        .to_string(),
                ],
            });
        }
        Some(_) | None => {}
    }

    let created_at = created_at_unix_seconds.unwrap_or_else(current_unix_seconds);
    let session_id = normalize_session_id(session_id, created_at)?;
    let findings = normalize_findings(findings)?;
    let session = AuditSessionRecord {
        session_id: session_id.clone(),
        created_at_unix_seconds: created_at,
        status: AuditSessionRecordStatus::Open,
        source_identity: source_identity.clone(),
        findings,
    };
    let lock = AuditSessionLock {
        session_id,
        source_identity,
    };
    state.write(&session, &lock)?;
    let (session, lock) = state.load_pair()?;
    Ok(AuditSessionStartResult {
        action: AuditAction::SessionStart,
        status: AuditSessionStartStatus::Created,
        session,
        lock,
        session_path: state.session_path(),
        lock_path: state.lock_path(),
        notes: vec![
            "Created one local Ontograph audit session and lock under the existing artifact root."
                .to_string(),
            "This slice does not dispatch workers, schedule bundles, plan prompts, or expose host/alpha surfaces."
                .to_string(),
        ],
    })
}

pub(crate) fn run_audit_session_dedupe(
    root: &OntographArtifactRoot,
) -> Result<AuditSessionDedupeResult, FunctionCallError> {
    let state = AuditSessionState::new(root);
    let active_source_identity = current_source_identity_for_action(root, "session_dedupe")?;
    let (session, lock) = match state.load_for_verify() {
        VerifyLoadState::Loaded(session, lock)
            if session.session_id == lock.session_id
                && session.source_identity == lock.source_identity
                && lock.source_identity == active_source_identity =>
        {
            (session, lock)
        }
        VerifyLoadState::Missing => {
            return Ok(inconclusive_dedupe_result(
                "No local audit session or lock exists under the active artifact root.".to_string(),
            ));
        }
        VerifyLoadState::Incomplete => {
            return Ok(inconclusive_dedupe_result(
                "Only one of audit-session.json or audit-session.lock.json exists.".to_string(),
            ));
        }
        VerifyLoadState::Corrupt(note) => return Ok(inconclusive_dedupe_result(note)),
        VerifyLoadState::Loaded(session, lock) if session.session_id != lock.session_id => {
            return Ok(inconclusive_dedupe_result(
                "Audit session and lock have different session ids.".to_string(),
            ));
        }
        VerifyLoadState::Loaded(session, lock)
            if session.source_identity != lock.source_identity =>
        {
            return Ok(inconclusive_dedupe_result(
                "Audit session and lock have different source identities.".to_string(),
            ));
        }
        VerifyLoadState::Loaded(_, _) => {
            return Ok(inconclusive_dedupe_result(
                "Audit session lock does not match the active source-index identity.".to_string(),
            ));
        }
    };
    let mut grouped: BTreeMap<(AuditSessionDedupeKeyKind, String), Vec<String>> = BTreeMap::new();
    let mut ungrouped_limitations = Vec::new();
    for finding in &session.findings {
        if !is_open_finding_status(&finding.status) {
            ungrouped_limitations.push(AuditSessionDedupeLimitation {
                finding_id: finding.finding_id.clone(),
                reason: format!("finding status is not open: {}", finding.status),
            });
            continue;
        }

        match dedupe_key(finding) {
            Some((kind, key)) => {
                grouped
                    .entry((kind, key))
                    .or_default()
                    .push(finding.finding_id.clone());
            }
            None => ungrouped_limitations.push(AuditSessionDedupeLimitation {
                finding_id: finding.finding_id.clone(),
                reason: "missing fingerprint, symbol, and write_set grouping fields".to_string(),
            }),
        }
    }
    let groups = grouped
        .into_iter()
        .filter_map(|((key_kind, key), finding_ids)| {
            (!finding_ids.is_empty()).then_some(AuditSessionDedupeGroup {
                key_kind,
                key,
                finding_ids,
            })
        })
        .collect::<Vec<_>>();
    Ok(AuditSessionDedupeResult {
        action: AuditAction::SessionDedupe,
        status: AuditSessionDedupeStatus::Pass,
        session_id: Some(lock.session_id),
        groups,
        ungrouped_limitations,
        notes: vec![
            "Grouped existing local audit-session findings by fingerprint, then symbol, then write_set."
                .to_string(),
            "No session state was modified; no bundle, dispatch, scheduler, planner, or worker runtime was created."
                .to_string(),
        ],
    })
}

pub(crate) fn run_audit_session_verify(
    root: &OntographArtifactRoot,
) -> Result<AuditSessionVerifyResult, FunctionCallError> {
    let state = AuditSessionState::new(root);
    let active_source_identity = current_source_identity_for_action(root, "session_verify")?;
    let session_path = state.session_path();
    let lock_path = state.lock_path();
    let (status, session, lock, notes) = match state.load_for_verify() {
        VerifyLoadState::Missing => (
            AuditSessionVerifyStatus::Missing,
            None,
            None,
            vec!["No local audit session or lock exists under the active artifact root.".to_string()],
        ),
        VerifyLoadState::Incomplete => (
            AuditSessionVerifyStatus::Incomplete,
            None,
            None,
            vec![
                "Only one of audit-session.json or audit-session.lock.json exists.".to_string(),
                "No session state was modified by session_verify.".to_string(),
            ],
        ),
        VerifyLoadState::Corrupt(note) => (
            AuditSessionVerifyStatus::Corrupt,
            None,
            None,
            vec![note, "No session state was modified by session_verify.".to_string()],
        ),
        VerifyLoadState::Loaded(session, lock) if session.session_id != lock.session_id => (
            AuditSessionVerifyStatus::Mismatched,
            Some(*session),
            Some(lock),
            vec![
                "Audit session and lock have different session ids.".to_string(),
                "No semantic finding verification, dedupe, bundle, dispatch, or worker review was performed.".to_string(),
            ],
        ),
        VerifyLoadState::Loaded(session, lock)
            if session.source_identity != lock.source_identity =>
        {
            (
                AuditSessionVerifyStatus::Mismatched,
                Some(*session),
                Some(lock),
                vec![
                    "Audit session and lock have different source identities.".to_string(),
                    "No semantic finding verification, dedupe, bundle, dispatch, or worker review was performed.".to_string(),
                ],
            )
        }
        VerifyLoadState::Loaded(session, lock) if lock.source_identity != active_source_identity => (
            AuditSessionVerifyStatus::StaleSession,
            Some(*session),
            Some(lock),
            vec![
                "Audit session lock does not match the active source-index identity.".to_string(),
                "No semantic finding verification, dedupe, bundle, dispatch, or worker review was performed.".to_string(),
            ],
        ),
        VerifyLoadState::Loaded(session, lock) => (
            AuditSessionVerifyStatus::Current,
            Some(*session),
            Some(lock),
            vec![
                "Audit session and lock match the active source-index identity.".to_string(),
                "No semantic finding verification, dedupe, bundle, dispatch, or worker review was performed.".to_string(),
            ],
        ),
    };

    Ok(AuditSessionVerifyResult {
        action: AuditAction::SessionVerify,
        status,
        session,
        lock,
        active_source_identity,
        session_path,
        lock_path,
        notes,
    })
}

pub(crate) fn run_audit_session_bundle(
    root: &OntographArtifactRoot,
) -> Result<AuditSessionBundleResult, FunctionCallError> {
    let dedupe_result = run_audit_session_dedupe(root)?;
    if dedupe_result.status != AuditSessionDedupeStatus::Pass {
        return Ok(AuditSessionBundleResult {
            action: AuditAction::SessionBundle,
            status: AuditSessionBundleStatus::Inconclusive,
            session_id: dedupe_result.session_id,
            bundles: Vec::new(),
            notes: vec![
                "audit.session_bundle requires a successful dedupe pass.".to_string(),
                "No session state was modified; no bundle, dispatch, scheduler, planner, or worker runtime was created.".to_string(),
            ],
        });
    }

    let state = AuditSessionState::new(root);
    let session = match state.load_pair() {
        Ok((session, _)) => session,
        Err(_) => {
            return Ok(AuditSessionBundleResult {
                action: AuditAction::SessionBundle,
                status: AuditSessionBundleStatus::Inconclusive,
                session_id: dedupe_result.session_id,
                bundles: Vec::new(),
                notes: vec![
                    "Could not load local session state for bundle projection.".to_string(),
                    "No session state was modified; no bundle, dispatch, scheduler, planner, or worker runtime was created.".to_string(),
                ],
            });
        }
    };

    let mut bundles = Vec::new();
    for (index, group) in dedupe_result.groups.into_iter().enumerate() {
        let mut owner_files = std::collections::BTreeSet::new();
        let mut target_symbols = std::collections::BTreeSet::new();
        let mut required_tests = std::collections::BTreeSet::new();

        for finding_id in &group.finding_ids {
            if let Some(finding) = session
                .findings
                .iter()
                .find(|f| &f.finding_id == finding_id)
            {
                if !is_open_finding_status(&finding.status) {
                    continue;
                }

                for file in &finding.write_set {
                    owner_files.insert(file.clone());
                }
                if let Some(path) = &finding.path {
                    owner_files.insert(path.clone());
                    required_tests.insert(format!(
                        "cargo test -p {}",
                        path.split('/').next().unwrap_or("crate")
                    ));
                }
                if let Some(symbol) = &finding.symbol {
                    target_symbols.insert(symbol.clone());
                }
            }
        }

        let files_count = owner_files.len();
        // Arbitrary estimate: 100 LOC per file
        let loc_estimate = files_count * 100;
        let cap_exceeded = files_count > 5 || loc_estimate > 500;

        let mut stop_conditions = Vec::new();
        if cap_exceeded {
            stop_conditions.push("Max files/LOC cap exceeded.".to_string());
        }

        bundles.push(AuditSessionBundleItem {
            bundle_id: format!("bundle-{}", index + 1),
            finding_ids: group.finding_ids,
            owner_files: owner_files.into_iter().collect(),
            target_symbols: target_symbols.into_iter().collect(),
            required_tests: required_tests.into_iter().collect(),
            stop_conditions,
            files_count,
            loc_estimate,
            cap_exceeded,
        });
    }

    Ok(AuditSessionBundleResult {
        action: AuditAction::SessionBundle,
        status: AuditSessionBundleStatus::Pass,
        session_id: dedupe_result.session_id,
        bundles,
        notes: vec![
            "Projected deduped findings into read-only bundles.".to_string(),
            "No session state was modified; no dispatch, scheduler, planner, or worker runtime was created.".to_string(),
        ],
    })
}

fn inconclusive_dedupe_result(note: String) -> AuditSessionDedupeResult {
    AuditSessionDedupeResult {
        action: AuditAction::SessionDedupe,
        status: AuditSessionDedupeStatus::Inconclusive,
        session_id: None,
        groups: Vec::new(),
        ungrouped_limitations: Vec::new(),
        notes: vec![
            note,
            "No session state was modified by session_dedupe.".to_string(),
        ],
    }
}

fn dedupe_key(finding: &AuditSessionFindingSummary) -> Option<(AuditSessionDedupeKeyKind, String)> {
    finding
        .fingerprint
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| (AuditSessionDedupeKeyKind::Fingerprint, value.to_string()))
        .or_else(|| {
            finding
                .symbol
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| (AuditSessionDedupeKeyKind::Symbol, value.to_string()))
        })
        .or_else(|| {
            let write_set = normalized_write_set(&finding.write_set);
            (!write_set.is_empty())
                .then(|| (AuditSessionDedupeKeyKind::WriteSet, write_set.join("\n")))
        })
}

fn is_open_finding_status(status: &str) -> bool {
    status.trim().eq_ignore_ascii_case(OPEN_FINDING_STATUS)
}

pub(crate) struct AuditSessionState<'a> {
    root: &'a OntographArtifactRoot,
}

impl<'a> AuditSessionState<'a> {
    pub(crate) fn new(root: &'a OntographArtifactRoot) -> Self {
        Self { root }
    }

    pub(crate) fn session_path(&self) -> String {
        self.root
            .root_path()
            .join(AUDIT_SESSION_FILE)
            .display()
            .to_string()
    }

    pub(crate) fn lock_path(&self) -> String {
        self.root
            .root_path()
            .join(AUDIT_SESSION_LOCK_FILE)
            .display()
            .to_string()
    }

    fn load_existing(
        &self,
    ) -> Result<Option<(AuditSessionRecord, AuditSessionLock)>, FunctionCallError> {
        let session_exists = self.root.root_path().join(AUDIT_SESSION_FILE).exists();
        let lock_exists = self.root.root_path().join(AUDIT_SESSION_LOCK_FILE).exists();
        match (session_exists, lock_exists) {
            (false, false) => Ok(None),
            (true, true) => self.load_pair().map(Some),
            _ => Err(FunctionCallError::RespondToModel(
                "ontograph.audit session_start found incomplete local audit session state"
                    .to_string(),
            )),
        }
    }

    pub(crate) fn load_pair(
        &self,
    ) -> Result<(AuditSessionRecord, AuditSessionLock), FunctionCallError> {
        let session_path = self.root.root_path().join(AUDIT_SESSION_FILE);
        let lock_path = self.root.root_path().join(AUDIT_SESSION_LOCK_FILE);
        let session = serde_json::from_str::<AuditSessionRecord>(
            &fs::read_to_string(session_path).map_err(|_| {
                FunctionCallError::RespondToModel(
                    "ontograph.audit session_start could not read audit-session.json".to_string(),
                )
            })?,
        )
        .map_err(|_| {
            FunctionCallError::RespondToModel(
                "ontograph.audit session_start found corrupt audit-session.json".to_string(),
            )
        })?;
        let lock = serde_json::from_str::<AuditSessionLock>(
            &fs::read_to_string(lock_path).map_err(|_| {
                FunctionCallError::RespondToModel(
                    "ontograph.audit session_start could not read audit-session.lock.json"
                        .to_string(),
                )
            })?,
        )
        .map_err(|_| {
            FunctionCallError::RespondToModel(
                "ontograph.audit session_start found corrupt audit-session.lock.json".to_string(),
            )
        })?;
        if session.session_id != lock.session_id {
            return Err(FunctionCallError::RespondToModel(
                "ontograph.audit session_start found mismatched audit session and lock".to_string(),
            ));
        }
        if session.source_identity != lock.source_identity {
            return Err(FunctionCallError::RespondToModel(
                "ontograph.audit session_start found mismatched audit session source identity"
                    .to_string(),
            ));
        }
        Ok((session, lock))
    }

    fn load_for_verify(&self) -> VerifyLoadState {
        let session_path = self.root.root_path().join(AUDIT_SESSION_FILE);
        let lock_path = self.root.root_path().join(AUDIT_SESSION_LOCK_FILE);
        let session_exists = session_path.exists();
        let lock_exists = lock_path.exists();
        match (session_exists, lock_exists) {
            (false, false) => VerifyLoadState::Missing,
            (true, false) | (false, true) => VerifyLoadState::Incomplete,
            (true, true) => {
                let session = match fs::read_to_string(&session_path)
                    .ok()
                    .and_then(|contents| serde_json::from_str::<AuditSessionRecord>(&contents).ok())
                {
                    Some(session) => session,
                    None => {
                        return VerifyLoadState::Corrupt(
                            "Could not read or parse audit-session.json.".to_string(),
                        );
                    }
                };
                let lock = match fs::read_to_string(&lock_path)
                    .ok()
                    .and_then(|contents| serde_json::from_str::<AuditSessionLock>(&contents).ok())
                {
                    Some(lock) => lock,
                    None => {
                        return VerifyLoadState::Corrupt(
                            "Could not read or parse audit-session.lock.json.".to_string(),
                        );
                    }
                };
                VerifyLoadState::Loaded(Box::new(session), lock)
            }
        }
    }

    fn write(
        &self,
        session: &AuditSessionRecord,
        lock: &AuditSessionLock,
    ) -> Result<(), FunctionCallError> {
        let root = self.root.root_path();
        fs::create_dir_all(&root).map_err(|_| {
            FunctionCallError::RespondToModel(
                "ontograph.audit session_start could not create local audit session directory"
                    .to_string(),
            )
        })?;
        write_json(&root.join(AUDIT_SESSION_FILE), session)?;
        write_json(&root.join(AUDIT_SESSION_LOCK_FILE), lock)?;
        Ok(())
    }
}

enum VerifyLoadState {
    Missing,
    Incomplete,
    Corrupt(String),
    Loaded(Box<AuditSessionRecord>, AuditSessionLock),
}

fn current_source_identity(
    root: &OntographArtifactRoot,
) -> Result<AuditSessionSourceIdentity, FunctionCallError> {
    current_source_identity_for_action(root, "session_start")
}

pub(crate) fn current_source_identity_for_action(
    root: &OntographArtifactRoot,
    action: &str,
) -> Result<AuditSessionSourceIdentity, FunctionCallError> {
    let generation_id = fs::read_to_string(root.current_pointer_path())
        .map_err(|_| {
            FunctionCallError::RespondToModel(format!(
                "ontograph.audit {action} requires an active source-index generation"
            ))
        })?
        .trim()
        .to_string();
    let manifest_path = root.generation_manifest_path(&generation_id).map_err(|_| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.audit {action} found an invalid source-index pointer"
        ))
    })?;
    let manifest = serde_json::from_str::<OntographManifest>(
        &fs::read_to_string(manifest_path).map_err(|_| {
            FunctionCallError::RespondToModel(format!(
                "ontograph.audit {action} could not read active source-index manifest"
            ))
        })?,
    )
    .map_err(|_| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.audit {action} found corrupt active source-index manifest"
        ))
    })?;
    let generation = manifest
        .completed_generations
        .iter()
        .find(|generation| generation.metadata.generation_id == manifest.active_generation_id)
        .ok_or_else(|| {
            FunctionCallError::RespondToModel(format!(
                "ontograph.audit {action} active source-index generation is missing"
            ))
        })?;
    Ok(AuditSessionSourceIdentity {
        generation_id: generation.metadata.generation_id.clone(),
        source_index_content_hash: generation.metadata.manifest_hash.clone(),
        target_head: generation.metadata.target_head.clone(),
    })
}

fn normalize_session_id(
    session_id: Option<String>,
    created_at_unix_seconds: i64,
) -> Result<String, FunctionCallError> {
    let session_id = session_id
        .map(|session_id| session_id.trim().to_string())
        .filter(|session_id| !session_id.is_empty())
        .unwrap_or_else(|| format!("audit-session-{created_at_unix_seconds}"));
    if session_id.len() > MAX_SESSION_ID_LENGTH
        || !session_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(FunctionCallError::RespondToModel(
            "ontograph.audit session_start session_id must be ASCII alphanumeric, dash, underscore, or dot"
                .to_string(),
        ));
    }
    Ok(session_id)
}

fn normalize_findings(
    findings: Vec<AuditSessionFindingInput>,
) -> Result<Vec<AuditSessionFindingSummary>, FunctionCallError> {
    if findings.len() > MAX_FINDINGS {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit session_start accepts at most {MAX_FINDINGS} findings"
        )));
    }
    Ok(findings
        .into_iter()
        .enumerate()
        .map(|(index, finding)| AuditSessionFindingSummary {
            fingerprint: finding
                .fingerprint
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            finding_id: finding
                .finding_id
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| format!("finding-{}", index + 1)),
            title: finding
                .title
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "Untitled finding".to_string()),
            symbol: finding
                .symbol
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            write_set: normalized_write_set(&finding.write_set),
            path: finding.path.filter(|value| !value.trim().is_empty()),
            line: finding.line,
            severity: finding.severity.filter(|value| !value.trim().is_empty()),
            status: finding
                .status
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "open".to_string()),
        })
        .collect())
}

fn normalized_write_set(write_set: &[String]) -> Vec<String> {
    let mut values = write_set
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn write_json<T: Serialize>(path: &std::path::Path, value: &T) -> Result<(), FunctionCallError> {
    let contents = serde_json::to_string_pretty(value).map_err(|_| {
        FunctionCallError::RespondToModel(
            "ontograph.audit session_start could not serialize local audit session state"
                .to_string(),
        )
    })?;
    write_atomically(path, &format!("{contents}\n")).map_err(|_| {
        FunctionCallError::RespondToModel(
            "ontograph.audit session_start could not write local audit session state".to_string(),
        )
    })
}

fn current_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}
