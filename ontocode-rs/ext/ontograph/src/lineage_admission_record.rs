//! Opt-in local record of one turn's lineage admission outcome.
//!
//! Rollout traces do not carry the admission tuple, so an out-of-process run
//! (for example `ontocode exec`) cannot recover it. When the operator names a
//! directory in [`ADMISSION_RECORD_DIR_ENV`], each turn appends one JSON line
//! holding only the turn id, the three routing tags and the elapsed time.
//! Nothing else is written. Unset or invalid destinations disable the record.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;

/// Environment variable naming the directory that receives `lineage_admission.jsonl`.
pub(crate) const ADMISSION_RECORD_DIR_ENV: &str = "LINEAGE_ADMISSION_RECORD_DIR";
const RECORD_FILE: &str = "lineage_admission.jsonl";
const MAX_RECORD_FILE_BYTES: u64 = 1024 * 1024;
const MAX_TURN_ID_CHARS: usize = 128;

/// One appended line. Field names and order are the append-only on-disk contract.
/// Stage A elapsed time includes induction, gating, retrieval, selection and admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct AdmissionRecord {
    pub(crate) turn_id: String,
    pub(crate) outcome: &'static str,
    pub(crate) admission: &'static str,
    pub(crate) fallback_category: &'static str,
    pub(crate) stage_a_elapsed_ms: u64,
}

/// Validated destination, resolved once per thread so the turn path never reads the environment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AdmissionRecordSink {
    dir: Option<PathBuf>,
}

impl AdmissionRecordSink {
    pub(crate) fn from_env() -> Self {
        Self::from_dir(std::env::var_os(ADMISSION_RECORD_DIR_ENV).map(PathBuf::from))
    }

    /// Accepts only an existing directory that is not itself a symlink.
    pub(crate) fn from_dir(dir: Option<PathBuf>) -> Self {
        let dir = dir.filter(|dir| {
            dir.is_absolute()
                && std::fs::symlink_metadata(dir).is_ok_and(|meta| meta.file_type().is_dir())
        });
        Self { dir }
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.dir.is_some()
    }

    /// Appends one line; every failure is swallowed because telemetry must not affect a turn.
    pub(crate) fn append(&self, record: &AdmissionRecord) {
        let Some(dir) = &self.dir else {
            return;
        };
        let _ = append_line(dir, record);
    }
}

fn safe_turn_id(turn_id: &str) -> bool {
    !turn_id.is_empty()
        && turn_id.chars().count() <= MAX_TURN_ID_CHARS
        && turn_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
}

fn append_line(dir: &Path, record: &AdmissionRecord) -> std::io::Result<()> {
    if !safe_turn_id(&record.turn_id) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "unsafe turn id",
        ));
    }
    let mut line = serde_json::to_string(record).map_err(std::io::Error::other)?;
    line.push('\n');
    let path = dir.join(RECORD_FILE);
    let mut options = OpenOptions::new();
    options.append(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        options
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .mode(0o600);
    }
    let mut file = options.open(&path)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len().saturating_add(line.len() as u64) > MAX_RECORD_FILE_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "record file is not a regular file or is full",
        ));
    }
    file.write_all(line.as_bytes())
}

#[cfg(test)]
#[path = "lineage_admission_record_tests.rs"]
mod tests;
