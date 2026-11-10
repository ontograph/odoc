use pretty_assertions::assert_eq;

use super::*;

fn record(turn_id: &str) -> AdmissionRecord {
    AdmissionRecord {
        turn_id: turn_id.to_string(),
        outcome: "lineage",
        admission: "accepted",
        fallback_category: "none",
        stage_a_elapsed_ms: 7,
    }
}

#[test]
fn unset_or_unusable_destination_disables_the_sink() {
    assert!(!AdmissionRecordSink::from_dir(None).is_enabled());
    assert!(!AdmissionRecordSink::from_dir(Some(PathBuf::from("relative/dir"))).is_enabled());
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(
        !AdmissionRecordSink::from_dir(Some(dir.path().join("missing"))).is_enabled(),
        "a missing directory must not be created"
    );
    assert!(!dir.path().join("missing").exists());
    let file = dir.path().join("a-file");
    std::fs::write(&file, "x").expect("file");
    assert!(!AdmissionRecordSink::from_dir(Some(file)).is_enabled());
}

#[test]
fn appends_one_exact_line_per_turn_across_turns() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sink = AdmissionRecordSink::from_dir(Some(dir.path().to_path_buf()));
    assert!(sink.is_enabled());
    sink.append(&record("turn-1"));
    sink.append(&AdmissionRecord {
        outcome: "baseline",
        admission: "rejected",
        fallback_category: "source_stale",
        ..record("turn-2")
    });
    let text = std::fs::read_to_string(dir.path().join("lineage_admission.jsonl")).expect("read");
    assert_eq!(
        text,
        concat!(
            "{\"turn_id\":\"turn-1\",\"outcome\":\"lineage\",\"admission\":\"accepted\",\"fallback_category\":\"none\",\"stage_a_elapsed_ms\":7}\n",
            "{\"turn_id\":\"turn-2\",\"outcome\":\"baseline\",\"admission\":\"rejected\",\"fallback_category\":\"source_stale\",\"stage_a_elapsed_ms\":7}\n",
        )
    );
}

#[test]
fn unsafe_turn_ids_are_not_written() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sink = AdmissionRecordSink::from_dir(Some(dir.path().to_path_buf()));
    for bad in ["", "has space", "line\nbreak", &"a".repeat(129), "quote\"d"] {
        sink.append(&record(bad));
    }
    assert!(!dir.path().join("lineage_admission.jsonl").exists());
}

#[cfg(unix)]
#[test]
fn a_symlinked_record_file_is_not_followed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside");
    let target = outside.path().join("victim.txt");
    std::fs::write(&target, "untouched").expect("victim");
    std::os::unix::fs::symlink(&target, dir.path().join("lineage_admission.jsonl"))
        .expect("symlink");
    let sink = AdmissionRecordSink::from_dir(Some(dir.path().to_path_buf()));
    sink.append(&record("turn-1"));
    assert_eq!(std::fs::read_to_string(&target).expect("read"), "untouched");
}

#[cfg(unix)]
#[test]
fn a_symlinked_directory_is_rejected() {
    let real = tempfile::tempdir().expect("real");
    let holder = tempfile::tempdir().expect("holder");
    let link = holder.path().join("link");
    std::os::unix::fs::symlink(real.path(), &link).expect("symlink");
    assert!(!AdmissionRecordSink::from_dir(Some(link)).is_enabled());
}

#[test]
fn the_record_file_is_capped() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("lineage_admission.jsonl");
    std::fs::write(&path, vec![b'x'; MAX_RECORD_FILE_BYTES as usize]).expect("fill");
    let sink = AdmissionRecordSink::from_dir(Some(dir.path().to_path_buf()));
    sink.append(&record("turn-1"));
    assert_eq!(
        std::fs::metadata(&path).expect("meta").len(),
        MAX_RECORD_FILE_BYTES
    );
}
