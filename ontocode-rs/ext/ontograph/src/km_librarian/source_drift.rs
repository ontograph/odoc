use std::fs;
use std::path::Component;
use std::path::Path;

use super::KmLibrarianHealthIssue;
use super::issue;
use crate::km_store::KmIndexRecord;
use crate::km_store::KmModuleStatus;
use crate::km_store::KmStoreError;
use crate::km_store::hash_directory;

pub(super) fn issue_for(
    record: &KmIndexRecord,
) -> Result<Option<KmLibrarianHealthIssue>, KmStoreError> {
    if record.status != KmModuleStatus::Active {
        return Ok(None);
    }
    let Some(project_scope) = record.project_scope.as_deref() else {
        return Ok(None);
    };
    let project_root = Path::new(project_scope);
    if !project_root.is_absolute() {
        return Ok(None);
    }
    let source_path = Path::new(&record.source_path);
    // Authored checked-in packages live directly under this project-owned root;
    // underscore-prefixed siblings are generated store and operations state.
    let checked_in_root = project_root.join(".ontocode/ontograph/km");
    let Ok(relative_source) = source_path.strip_prefix(&checked_in_root) else {
        return Ok(None);
    };
    let Some(Component::Normal(package_dir)) = relative_source.components().next() else {
        return Ok(None);
    };
    if package_dir.to_string_lossy().starts_with('_') {
        return Ok(None);
    }
    if !source_path.is_dir() {
        return Ok(Some(issue(
            &record.id,
            &record.version,
            &record.source_path,
            "source-missing",
            "checked-in KM source package is missing; publish and import a new package version",
        )));
    }

    let Ok(canonical_source) = fs::canonicalize(source_path) else {
        return Ok(None);
    };
    if !canonical_source.starts_with(project_root) {
        return Ok(None);
    }
    if hash_directory(&canonical_source)? == record.content_hash {
        return Ok(None);
    }
    Ok(Some(issue(
        &record.id,
        &record.version,
        &record.source_path,
        "source-changed",
        "checked-in KM source package differs from the imported snapshot; publish and import a new package version",
    )))
}
