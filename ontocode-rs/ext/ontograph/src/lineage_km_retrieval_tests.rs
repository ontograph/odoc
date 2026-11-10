use std::path::Path;

use pretty_assertions::assert_eq;
use tempfile::tempdir;

use super::*;
use crate::km_store::KmStore;
use crate::lineage_grounding::ContainmentStatus;
use crate::lineage_grounding::DemandSupport;
use crate::lineage_grounding::GroundingClaim;
use crate::lineage_grounding::GroundingError;
use crate::lineage_grounding::ground_claims;

/// Example text whose second line carries the term the tests search for.
const EXAMPLE: &str = "intro line\nthe dispatcher validates every lease\ntrailing line";

/// Seeds one importable KM whose example file contains `EXAMPLE`.
fn seed_store(dir: &Path) -> KmStore {
    let store = KmStore::new(dir.join("km-cache"));
    import_example(
        dir,
        &store,
        "codebase-graph/lineage",
        "Dispatcher lease rules",
        EXAMPLE,
    );
    store
}

fn import_example(dir: &Path, store: &KmStore, id: &str, summary: &str, example: &str) {
    let km_root = dir.join(format!("auto-km-{}-0.1.0", id.replace('/', "-")));
    std::fs::create_dir_all(km_root.join("examples")).expect("create examples");
    std::fs::create_dir_all(km_root.join("tests")).expect("create tests");
    std::fs::write(
        km_root.join("km.toml"),
        format!("id = \"{id}\"\nversion = \"0.1.0\"\nkind = \"codebase-graph\"\nsummary = \"{summary}\"\nsources = [\"silver:doc\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n"),
    )
    .expect("write manifest");
    std::fs::write(
        km_root.join("facts.jsonl"),
        "{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"the dispatcher validates every lease\",\"source\":\"silver:doc\",\"confidence\":0.9}\n",
    )
    .expect("write facts");
    std::fs::write(km_root.join("examples/README.md"), example).expect("write example");
    std::fs::write(km_root.join("tests/README.md"), "test").expect("write test");
    std::fs::write(dir.join("doc"), "doc").expect("write cited source");
    store
        .import_for_project(&km_root, &dir.display().to_string())
        .expect("import seeded km");
}

#[test]
fn source_keys_round_trip_without_colliding_at_id_or_version_boundaries() {
    let cases = [
        ("a/b", "0.1.0", "c"),
        ("a", "0.1.0", "b/c"),
        ("a@0.1.0/b:é", "0.1.0-rc.1+build.2", "examples/é.md"),
        ("a/b", "0.2.0", "c"),
    ];
    let mut keys = std::collections::BTreeSet::new();
    for (id, version, path) in cases {
        let key = km_source_key(id, version, path);
        assert!(keys.insert(key.clone()));
        assert_eq!(km_source_relative_path(&key), path);
    }
    assert_eq!(km_source_key("a/b", "0.1.0", "c"), "3:a/b@0.1.0/c");
    for path in [
        "examples/README.md",
        "src/a.rs",
        "999:a@0.1.0/examples/README.md",
        "1:é@0.1.0/examples/README.md",
        "1:a@invalid/examples/README.md",
    ] {
        assert_eq!(km_source_relative_path(path), path);
    }
}

#[test]
fn two_modules_with_the_same_example_path_refresh_and_render_independently() {
    use crate::lineage_admission::AdmissionRejection;
    use crate::lineage_admission::admit_lineage;

    let dir = tempdir().expect("tempdir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let examples = [
        ("codebase-graph/alpha", "dispatcher validates leases"),
        ("codebase-graph/beta", "allocator reserves memory"),
    ];
    for (id, text) in examples {
        import_example(dir.path(), &store, id, text, text);
    }
    let LineageAttempt::Selected {
        ledger,
        mut selection,
        generation,
        ..
    } = km_lineage_attempt(&store, "dispatcher allocator")
    else {
        panic!("both matching modules must produce a selection");
    };
    assert_eq!(selection.selected.len(), 2);
    let expected_files = examples
        .iter()
        .map(|(id, text)| SourceGenerationFile {
            path: km_source_key(id, "0.1.0", "examples/README.md"),
            generation_id: content_hash(text),
            lines: vec![text.to_string()],
        })
        .collect();
    assert_eq!(generation, SourceGeneration::from_files(expected_files));
    // Stable first-seen order must survive duplicates and repeated headers.
    selection
        .selected
        .sort_by(|left, right| left.passage_id.cmp(&right.passage_id));
    let duplicate = selection.selected[0].clone();
    selection.selected.push(duplicate);
    let fresh = refresh_selected(
        &store,
        &dir.path().display().to_string(),
        &selection,
        &ledger,
        |_| true,
    )
    .expect("two owners of a relative path are valid");
    assert_eq!(fresh, generation);
    let rendered = admit_lineage(
        &selection,
        &ledger,
        &generation,
        &fresh,
        /*scope_valid*/ true,
    )
    .expect("both spans must be admitted")
    .body;
    assert_eq!(
        rendered.split_once("\n\n").expect("framing separator").1,
        "examples/README.md:1-1\ndispatcher validates leases\n---\nexamples/README.md:1-1\nallocator reserves memory"
    );

    let beta_key = km_source_key("codebase-graph/beta", "0.1.0", "examples/README.md");
    let mut stale = fresh;
    stale
        .files
        .get_mut(&beta_key)
        .expect("beta file")
        .generation_id = "sha256:changed".to_string();
    assert_eq!(
        admit_lineage(
            &selection,
            &ledger,
            &generation,
            &stale,
            /*scope_valid*/ true
        ),
        Err(AdmissionRejection::SourceStale)
    );
    let mut beta_selection = selection.clone();
    beta_selection
        .selected
        .retain(|passage| passage.passage_id == "codebase-graph/beta:examples/README.md:1");
    assert_eq!(beta_selection.selected.len(), 1);
    assert_eq!(
        admit_lineage(
            &beta_selection,
            &ledger,
            &generation,
            &stale,
            /*scope_valid*/ true
        ),
        Err(AdmissionRejection::SourceStale)
    );
    selection.selected.truncate(/*len*/ 1);
    assert!(
        admit_lineage(
            &selection,
            &ledger,
            &generation,
            &stale,
            /*scope_valid*/ true
        )
        .is_ok()
    );
    let mut wrong_owner = ledger;
    wrong_owner
        .passages
        .iter_mut()
        .find(|passage| passage.passage_id == selection.selected[0].passage_id)
        .expect("alpha lineage")
        .source_location
        .path = beta_key;
    assert_eq!(
        refresh_selected(
            &store,
            &dir.path().display().to_string(),
            &selection,
            &wrong_owner,
            |_| true,
        ),
        Err(AdmissionRejection::SchemaInvalid)
    );
}

#[test]
fn a_matched_example_line_becomes_a_line_addressed_candidate() {
    let dir = tempdir().expect("tempdir");
    let store = seed_store(dir.path());

    let attempt = km_lineage_attempt(&store, "who validates the lease?");
    let LineageAttempt::Selected {
        ledger,
        selection,
        generation,
        ..
    } = attempt
    else {
        panic!("a matching example must produce a selection, got {attempt:?}");
    };

    let passage = ledger.passages.first().expect("one merged passage");
    // The term is on line 2, padded by one context line on each side.
    assert_eq!(passage.source_location.start_line, Some(1));
    assert_eq!(passage.source_location.end_line, Some(3));
    let key = km_source_key("codebase-graph/lineage", "0.1.0", "examples/README.md");
    assert_eq!(passage.source_location.path, key);
    assert!(!selection.selected.is_empty());

    // The generation carries the real file lines.
    let file = generation.files.get(&key).expect("generation file");
    assert_eq!(file.lines.len(), 3);
    assert_eq!(file.lines[1], "the dispatcher validates every lease");
}

#[test]
fn grounding_accepts_an_answer_quoting_the_retrieved_span() {
    let dir = tempdir().expect("tempdir");
    let store = seed_store(dir.path());

    let LineageAttempt::Selected {
        ledger,
        selection,
        generation,
        ..
    } = km_lineage_attempt(&store, "who validates the lease?")
    else {
        panic!("a matching example must produce a selection");
    };

    let selected = selection.selected.first().expect("one selected passage");
    let lineage = ledger
        .lineage_for(&selected.passage_id)
        .expect("selected passage has lineage");
    let observed_generation_id = generation
        .files
        .get(&lineage.source_location.path)
        .expect("generation file")
        .generation_id
        .clone();

    let report = ground_claims(
        &ledger,
        &selection,
        &generation,
        &[GroundingClaim {
            passage_id: selected.passage_id.clone(),
            demand_id: lineage.first_exposure.demand_id.clone(),
            observed_generation_id,
            quoted_text: "the dispatcher validates every lease".to_string(),
        }],
    )
    .expect("grounding resolves a retrieved span");

    let claim = report.claims.first().expect("one grounded claim");
    assert_eq!(claim.containment, ContainmentStatus::Contained);
    assert_eq!(claim.demand_support, DemandSupport::Retrieved);
    assert_eq!(report.usable_claims().len(), 1);
}

#[test]
fn a_source_edited_after_retrieval_is_reported_as_stale() {
    let dir = tempdir().expect("tempdir");
    let store = seed_store(dir.path());

    let LineageAttempt::Selected {
        ledger,
        selection,
        generation,
        ..
    } = km_lineage_attempt(&store, "who validates the lease?")
    else {
        panic!("a matching example must produce a selection");
    };
    let selected = selection.selected.first().expect("one selected passage");
    let lineage = ledger
        .lineage_for(&selected.passage_id)
        .expect("selected passage has lineage");

    // A generation id observed before an edit must not ground against current
    // content; otherwise a moved span would be silently accepted.
    let error = ground_claims(
        &ledger,
        &selection,
        &generation,
        &[GroundingClaim {
            passage_id: selected.passage_id.clone(),
            demand_id: lineage.first_exposure.demand_id.clone(),
            observed_generation_id: "sha256:stale".to_string(),
            quoted_text: "the dispatcher validates every lease".to_string(),
        }],
    )
    .expect_err("a stale generation must fail closed");
    assert!(matches!(error, GroundingError::StaleSource { .. }));
}

#[test]
fn an_empty_store_and_an_unmatched_question_both_fail_closed() {
    let empty_dir = tempdir().expect("tempdir");
    let empty = KmStore::new(empty_dir.path().join("km-cache"));
    assert!(matches!(
        km_lineage_attempt(&empty, "who validates the lease?"),
        LineageAttempt::RetrievalUnavailable(_)
    ));

    let seeded_dir = tempdir().expect("tempdir");
    let seeded = seed_store(seeded_dir.path());
    assert!(matches!(
        km_lineage_attempt(&seeded, "zzz nonmatching terminology"),
        LineageAttempt::RetrievalUnavailable(_)
    ));
}

#[test]
fn a_question_without_searchable_terms_fails_closed() {
    let dir = tempdir().expect("tempdir");
    let store = seed_store(dir.path());
    assert!(matches!(
        km_lineage_attempt(&store, "a b c"),
        LineageAttempt::InductionFailed(_)
    ));
}

#[test]
fn matches_on_the_first_two_lines_of_one_example_do_not_conflict() {
    let dir = tempdir().expect("tempdir");
    let store = KmStore::new(dir.path().join("km-cache"));
    // Line 1 matches "dispatcher" and line 2 matches "validates". Both spans start at
    // line 1, so they must end together instead of failing as conflicting
    // provenance.
    import_example(
        dir.path(),
        &store,
        "codebase-graph/lineage",
        "Adjacent lines",
        "dispatcher rule line\nvalidates rule line\nplain trailing line\nsecond plain line",
    );
    // Both words appear in the fixture's fact, so the KM is retrieved; "dispatcher"
    // matches line 1 and "validates" matches line 2.
    let attempt = km_lineage_attempt(&store, "dispatcher validates");
    let LineageAttempt::Selected { ledger, .. } = attempt else {
        panic!("first-two-line matches must select, got {attempt:?}");
    };
    for passage in &ledger.passages {
        assert_eq!(passage.source_location.start_line, Some(1));
        assert_eq!(passage.source_location.end_line, Some(3));
    }
}

#[test]
fn a_match_inside_a_markdown_section_spans_the_section_within_the_byte_cap() {
    // The path line sits under the heading, several lines above the match, so
    // a three-line window around the match would drop it.
    let filler = "filler sentence about unrelated details. ".repeat(5);
    let long = "x".repeat(MAX_SECTION_SPAN_BYTES);
    let text = format!(
        "# Notes\n\n## Leases\nImplementation: `src/lease.rs`\n{filler}\n{filler}\nthe dispatcher validates every lease\n## Next\nnext text\n\n## Huge\nImplementation: `src/huge.rs`\n{long}\nthe allocator reserves memory\ntail\nplain"
    );
    let lines = text.lines().map(str::to_string).collect::<Vec<_>>();

    // Lines 3..=7: from the heading to the line before the next heading.
    assert_eq!(span_bounds(&lines, 6), (3, 7));
    // Every match in a section yields the same span, so ids never conflict.
    assert_eq!(span_bounds(&lines, 3), (3, 7));
    // The match before the first `##` heading uses the `#` section.
    assert_eq!(span_bounds(&lines, 1), (1, 2));
    // The cut lands before the match, so the window around it is used.
    assert_eq!(span_bounds(&lines, 13), (13, 15));
    // Text without headings keeps the three-line window.
    let plain = ["a", "b", "c", "d"].map(str::to_string);
    assert_eq!(span_bounds(&plain, 2), (2, 4));
    // A section where only the heading fits never shares a start line with
    // the window around a match on the next line.
    let tight = [
        "## H".to_string(),
        "y".repeat(MAX_SECTION_SPAN_BYTES),
        "z".to_string(),
    ];
    assert_eq!(span_bounds(&tight, 0), (1, 3));
    assert_eq!(span_bounds(&tight, 1), (1, 3));
    // `#hashtag` is not a heading.
    let tagged = ["#tag", "match"].map(str::to_string);
    assert_eq!(span_bounds(&tagged, 1), (1, 2));

    for index in 0..lines.len() {
        let (start, end) = span_bounds(&lines, index);
        assert!(start <= index + 1 && index < end, "{index}: {start}-{end}");
        if end - start > 2 * SPAN_CONTEXT_LINES {
            let bytes: usize = lines[start - 1..end]
                .iter()
                .map(|line| line.len() + 1)
                .sum();
            assert!(bytes <= MAX_SECTION_SPAN_BYTES, "{index}: {bytes}");
        }
    }
}

#[test]
fn the_line_covering_most_question_terms_beats_an_earlier_longer_line() {
    let dir = tempdir().expect("tempdir");
    let store = KmStore::new(dir.path().join("km-cache"));
    // Line 1 is long and mentions only "dispatcher". Line 4 is short and
    // mentions all three question terms. Taking the first match per file would
    // pick line 1 for "dispatcher", and the length score would favour it.
    import_example(
        dir.path(),
        &store,
        "codebase-graph/lineage",
        "Dispatcher lease rules",
        "the dispatcher notes are long and verbose on this first line for ranking purposes only\nfiller line one\nfiller line two\ndispatcher validates lease\nfiller line three\nfiller line four",
    );
    let attempt = km_lineage_attempt(&store, "dispatcher validates lease");
    let LineageAttempt::Selected {
        ledger, selection, ..
    } = attempt
    else {
        panic!("a matching example must select, got {attempt:?}");
    };
    // One passage covers all three demands, so selection needs only that one.
    assert_eq!(selection.selected.len(), 1, "{selection:?}");
    let passage = ledger
        .passages
        .iter()
        .find(|passage| passage.passage_id == selection.selected[0].passage_id)
        .expect("selected passage in ledger");
    // Line 4 of the file, padded by one line each side.
    assert_eq!(passage.source_location.start_line, Some(3));
    assert_eq!(passage.source_location.end_line, Some(5));
    assert_eq!(passage.demand_membership.len(), 3);
}

#[test]
fn a_repeated_term_does_not_retrieve_the_same_passage_twice() {
    let dir = tempdir().expect("tempdir");
    let store = seed_store(dir.path());

    // A KM surfacing in both the local and global result sets previously
    // produced two candidates for one demand, which merging rejects as a
    // duplicate and turned a good retrieval into RetrievalFailed.
    let attempt = km_lineage_attempt(&store, "validates validates the lease");
    let LineageAttempt::Selected { ledger, .. } = attempt else {
        panic!("a repeated term must still select, got {attempt:?}");
    };
    for passage in &ledger.passages {
        for demand_id in passage.exposures.keys() {
            let occurrences = ledger
                .passages
                .iter()
                .filter(|other| {
                    other.passage_id == passage.passage_id
                        && other.exposures.contains_key(demand_id)
                })
                .count();
            assert_eq!(occurrences, 1, "one exposure per demand and passage");
        }
    }
}

#[cfg(unix)]
#[test]
fn refresh_selected_rejects_a_selected_example_replaced_by_a_symlink() {
    use std::os::unix::fs::symlink;

    use crate::lineage_admission::AdmissionRejection;

    let dir = tempdir().expect("tempdir");
    let store = seed_store(dir.path());
    let LineageAttempt::Selected {
        ledger, selection, ..
    } = km_lineage_attempt(&store, "dispatcher")
    else {
        panic!("a matching example must produce a selection");
    };
    let record = store
        .list(/*include_obsolete*/ false)
        .expect("list modules")
        .into_iter()
        .next()
        .expect("seeded module");
    let cached_example = Path::new(&record.path).join("examples/README.md");
    let target = dir.path().join("other-example.md");
    std::fs::write(&target, "another file").expect("write symlink target");
    std::fs::remove_file(&cached_example).expect("remove cached example");
    symlink(&target, &cached_example).expect("replace cached example with symlink");

    assert_eq!(
        refresh_selected(
            &store,
            &dir.path().display().to_string(),
            &selection,
            &ledger,
            |_| true,
        ),
        // `open_regular` refuses the link at open time, before any containment check.
        Err(AdmissionRejection::SourceUnavailable)
    );
}

#[cfg(unix)]
#[test]
fn refresh_selected_rejects_a_selected_example_fifo_without_hanging() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::sync::mpsc;
    use std::time::Duration;

    use crate::lineage_admission::AdmissionRejection;

    let dir = tempdir().expect("tempdir");
    let store = seed_store(dir.path());
    let LineageAttempt::Selected {
        ledger, selection, ..
    } = km_lineage_attempt(&store, "dispatcher")
    else {
        panic!("a matching example must produce a selection");
    };
    let record = store
        .list(/*include_obsolete*/ false)
        .expect("list modules")
        .into_iter()
        .next()
        .expect("seeded module");
    let cached_example = Path::new(&record.path).join("examples/README.md");
    std::fs::remove_file(&cached_example).expect("remove cached example");
    let fifo_c = CString::new(cached_example.as_os_str().as_bytes()).expect("FIFO path has no NUL");
    // mkfifo's mode argument.
    assert_eq!(
        unsafe {
            libc::mkfifo(fifo_c.as_ptr(), /*mode*/ 0o600)
        },
        0
    );

    let worker_store = store;
    let worker_selection = selection;
    let worker_ledger = ledger;
    let scope = dir.path().display().to_string();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = refresh_selected(
            &worker_store,
            &scope,
            &worker_selection,
            &worker_ledger,
            |_| true,
        );
        sender.send(result).expect("send refresh result");
    });

    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(/*secs*/ 5))
            .expect("refreshing a FIFO must not block"),
        Err(AdmissionRejection::SourceUnavailable)
    );
}

#[cfg(unix)]
#[test]
fn refresh_selected_preserves_the_retrieval_generation_for_an_unchanged_package() {
    let dir = tempdir().expect("tempdir");
    let store = seed_store(dir.path());
    let LineageAttempt::Selected {
        ledger,
        selection,
        generation,
        ..
    } = km_lineage_attempt(&store, "dispatcher")
    else {
        panic!("a matching example must produce a selection");
    };

    assert_eq!(
        refresh_selected(
            &store,
            &dir.path().display().to_string(),
            &selection,
            &ledger,
            |_| true,
        ),
        Ok(generation)
    );
}
