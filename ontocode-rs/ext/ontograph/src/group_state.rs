use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use ontocode_utils_path::write_atomically;
use serde::Deserialize;
use serde::Serialize;

use crate::manifest::repo_ontograph_root;

pub(crate) const CURRENT_WORKSPACE_GROUP: &str = "current-workspace";

const GROUPS_DIR: &str = "groups";
const CURRENT_WORKSPACE_GROUP_FILE: &str = "current-workspace.json";
const GROUP_STATE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CurrentWorkspaceGroupState {
    pub schema_version: u32,
    pub name: String,
    pub repo_roots: Vec<String>,
    pub artifact_root: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GroupStateError {
    CreateDirectory,
    Read,
    Serialize,
    Write,
    Corrupt(String),
}

impl CurrentWorkspaceGroupState {
    pub(crate) fn new(repo_roots: Vec<String>, artifact_root: String) -> Self {
        Self {
            schema_version: GROUP_STATE_SCHEMA_VERSION,
            name: CURRENT_WORKSPACE_GROUP.to_string(),
            repo_roots,
            artifact_root,
        }
    }
}

pub(crate) fn group_state_dir(repo_root: &Path) -> PathBuf {
    repo_ontograph_root(repo_root).join(GROUPS_DIR)
}

pub(crate) fn read_current_workspace_group_state(
    repo_root: &Path,
) -> Result<Option<CurrentWorkspaceGroupState>, GroupStateError> {
    let groups_dir = group_state_dir(repo_root);
    if !groups_dir.exists() {
        return Ok(None);
    }
    if !groups_dir.is_dir() {
        return Err(GroupStateError::Corrupt(
            ".ontocode/ontograph/groups exists but is not a directory".to_string(),
        ));
    }

    let state_path = current_workspace_group_path(repo_root);
    let contents = fs::read_to_string(&state_path).map_err(|err| match err.kind() {
        io::ErrorKind::NotFound => GroupStateError::Corrupt(
            ".ontocode/ontograph/groups/current-workspace.json is missing".to_string(),
        ),
        _ => GroupStateError::Read,
    })?;
    let state = serde_json::from_str::<CurrentWorkspaceGroupState>(&contents)
        .map_err(|err| GroupStateError::Corrupt(format!("invalid JSON: {err}")))?;
    validate_current_workspace_group(&state)?;
    Ok(Some(state))
}

pub(crate) fn write_current_workspace_group_state(
    repo_root: &Path,
    state: &CurrentWorkspaceGroupState,
) -> Result<(), GroupStateError> {
    write_current_workspace_group_state_with(repo_root, state, write_atomically)
}

pub(crate) fn write_current_workspace_group_state_with(
    repo_root: &Path,
    state: &CurrentWorkspaceGroupState,
    writer: impl FnOnce(&Path, &str) -> io::Result<()>,
) -> Result<(), GroupStateError> {
    validate_current_workspace_group(state)?;
    let groups_dir = group_state_dir(repo_root);
    let created_groups_dir = !groups_dir.exists();
    fs::create_dir_all(&groups_dir).map_err(|_| GroupStateError::CreateDirectory)?;
    let contents = serde_json::to_string_pretty(state).map_err(|_| GroupStateError::Serialize)?;
    writer(
        &current_workspace_group_path(repo_root),
        &format!("{contents}\n"),
    )
    .map_err(|_| {
        if created_groups_dir {
            let _ = fs::remove_dir(&groups_dir);
        }
        GroupStateError::Write
    })
}

fn current_workspace_group_path(repo_root: &Path) -> PathBuf {
    group_state_dir(repo_root).join(CURRENT_WORKSPACE_GROUP_FILE)
}

fn validate_current_workspace_group(
    state: &CurrentWorkspaceGroupState,
) -> Result<(), GroupStateError> {
    if state.schema_version != GROUP_STATE_SCHEMA_VERSION {
        return Err(GroupStateError::Corrupt(format!(
            "unsupported schema_version {}",
            state.schema_version
        )));
    }
    if state.name != CURRENT_WORKSPACE_GROUP {
        return Err(GroupStateError::Corrupt(format!(
            "unsupported group name `{}`",
            state.name
        )));
    }
    if state.repo_roots.is_empty() {
        return Err(GroupStateError::Corrupt(
            "current-workspace repo_roots must not be empty".to_string(),
        ));
    }
    if state.repo_roots.iter().any(|root| root.trim().is_empty()) {
        return Err(GroupStateError::Corrupt(
            "current-workspace repo_roots entries must not be empty".to_string(),
        ));
    }
    if state.artifact_root.trim().is_empty() {
        return Err(GroupStateError::Corrupt(
            "current-workspace artifact_root must not be empty".to_string(),
        ));
    }
    Ok(())
}
