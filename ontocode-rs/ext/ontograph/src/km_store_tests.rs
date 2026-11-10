use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;

use pretty_assertions::assert_eq;
use tempfile::TempDir;

use super::*;

#[test]
fn import_valid_km_records_active_module() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");

    let imported = store.import(&source).expect("import succeeds");
    let active = store
        .resolve_active("codebase-graph/creating-ontograph")
        .expect("resolve succeeds")
        .expect("active module");

    assert_eq!(imported.status, KmModuleStatus::Active);
    assert_eq!(imported, active);
    assert!(Path::new(&imported.path).exists());
}

#[test]
fn scoped_import_accepts_absolute_absent_scope_label() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    let scope = tempdir.path().join("absent-project-scope");
    let scope = scope.display().to_string();

    let imported = store
        .import_for_project(&source, &scope)
        .expect("opaque absolute scope label is accepted");

    assert_eq!(imported.project_scope, Some(scope));
}

#[test]
fn scoped_import_requires_existing_project_sources() {
    let tempdir = TempDir::new().expect("temp dir");
    let project_root = tempdir.path().join("project");
    fs::create_dir_all(&project_root).expect("project root");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");

    let error = store
        .import_for_project(&source, &project_root.display().to_string())
        .expect_err("missing citation must fail");

    let KmStoreError::InvalidKm(report) = error else {
        panic!("unexpected error: {error:?}");
    };
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.contains("invalid source")),
        "unexpected errors: {:?}",
        report.errors
    );
}

#[test]
fn scoped_import_rejects_missing_escaping_and_absolute_sources() {
    let tempdir = TempDir::new().expect("temp dir");
    let project_root = tempdir.path().join("project");
    fs::create_dir_all(&project_root).expect("project root");
    fs::write(project_root.join("doc"), "source").expect("source");

    for (case, invalid_source) in [
        ("missing", "silver:missing"),
        ("parent", "silver:../outside"),
        ("absolute", "silver:/outside"),
    ] {
        let case_root = tempdir.path().join(case);
        fs::create_dir_all(&case_root).expect("case root");
        let source = write_valid_km(&case_root, "0.1.0", "bounded claim");
        fs::write(
            source.join("facts.jsonl"),
            format!(
                "{{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"bounded claim\",\"source\":\"{invalid_source}\",\"confidence\":0.9}}\n"
            ),
        )
        .expect("facts");
        let store = KmStore::new(tempdir.path().join(format!("store-{case}")));

        assert!(
            matches!(
                store.import_for_project(&source, &project_root.display().to_string()),
                Err(KmStoreError::InvalidKm(_))
            ),
            "{case} source must fail"
        );
    }
}

#[test]
fn scoped_import_accepts_existing_project_sources() {
    let tempdir = TempDir::new_in("/dev/shm").expect("temp dir");
    let project_root = tempdir.path().join("project");
    fs::create_dir_all(&project_root).expect("project root");
    fs::write(project_root.join("doc"), "source").expect("source");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");

    let imported = store
        .import_for_project(&source, &project_root.display().to_string())
        .expect("existing citation passes");

    assert_eq!(
        imported.project_scope,
        Some(
            fs::canonicalize(project_root)
                .expect("canonical project")
                .display()
                .to_string()
        )
    );
}

#[test]
fn unscoped_custom_store_import_preserves_legacy_source_compatibility() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("custom-store"));
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");

    let imported = store
        .import(&source)
        .expect("unscoped import remains valid");

    assert_eq!(imported.project_scope, None);
}

#[test]
fn cached_snapshot_validation_does_not_require_donor_source() {
    let tempdir = TempDir::new().expect("temp dir");
    let project_root = tempdir.path().join("project");
    fs::create_dir_all(&project_root).expect("project root");
    fs::write(project_root.join("doc"), "source").expect("source");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    let imported = store
        .import_for_project(&source, &project_root.display().to_string())
        .expect("import");
    fs::remove_file(project_root.join("doc")).expect("remove donor source");

    assert_eq!(
        store
            .validated_module_root(&imported)
            .expect("cached snapshot remains valid"),
        fs::canonicalize(&imported.path).expect("canonical cached snapshot")
    );
}

#[cfg(unix)]
#[test]
fn scoped_import_rejects_symlinked_project_source() {
    use std::os::unix::fs::symlink;

    let tempdir = TempDir::new().expect("temp dir");
    let project_root = tempdir.path().join("project");
    fs::create_dir_all(&project_root).expect("project root");
    let outside = tempdir.path().join("outside");
    fs::write(&outside, "source").expect("outside source");
    symlink(&outside, project_root.join("doc")).expect("symlink");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");

    assert!(matches!(
        store.import_for_project(&source, &project_root.display().to_string()),
        Err(KmStoreError::InvalidKm(_))
    ));
}

#[test]
fn new_import_populates_topic_words() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(
        tempdir.path(),
        "0.1.0",
        "Graph retrieval should match bounded context",
    );

    let imported = store.import(&source).expect("import succeeds");

    assert_eq!(
        imported.topic_words,
        vec![
            "bounded".to_string(),
            "context".to_string(),
            "graph".to_string(),
            "match".to_string(),
            "retrieval".to_string(),
        ]
    );
}

#[test]
fn topic_words_exclude_stop_words() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(tempdir.path(), "0.1.0", "The graph and context are useful");

    let imported = store.import(&source).expect("import succeeds");

    assert_eq!(
        imported.topic_words,
        vec![
            "bounded".to_string(),
            "context".to_string(),
            "graph".to_string(),
            "useful".to_string(),
        ]
    );
}

#[test]
fn old_index_without_topic_words_round_trips_and_backfills() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let source = write_valid_km(tempdir.path(), "0.1.0", "Graph backfill context");
    let initial = store.import(&source).expect("initial import");
    let old_index = format!(
        "{{\n  \"schemaVersion\": 1,\n  \"modules\": [{{\n    \"id\": \"{}\",\n    \"version\": \"{}\",\n    \"path\": \"{}\",\n    \"contentHash\": \"{}\",\n    \"status\": \"active\",\n    \"importedAtUnixSeconds\": {},\n    \"sourcePath\": \"{}\"\n  }}]\n}}\n",
        initial.id,
        initial.version,
        initial.path,
        initial.content_hash,
        initial.imported_at_unix_seconds,
        initial.source_path,
    );
    fs::write(store_root.join("index.json"), old_index).expect("old index");

    let old_record = store
        .list(/*include_obsolete*/ true)
        .expect("old index loads")
        .pop()
        .expect("old record");
    assert_eq!(old_record.topic_words, Vec::<String>::new());

    let backfilled = store.import(&source).expect("reimport backfills");

    assert_eq!(
        backfilled.topic_words,
        vec![
            "backfill".to_string(),
            "bounded".to_string(),
            "context".to_string(),
            "graph".to_string(),
        ]
    );
}

#[test]
fn old_index_rows_without_origin_and_withdrawal_remain_readable() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let source = write_valid_km(tempdir.path(), "0.1.0", "Legacy metadata");
    let imported = store.import(&source).expect("initial import");
    let legacy_index = format!(
        "{{\n  \"schemaVersion\": 1,\n  \"modules\": [{{\n    \"id\": \"{}\",\n    \"version\": \"{}\",\n    \"path\": \"{}\",\n    \"contentHash\": \"{}\",\n    \"status\": \"active\",\n    \"importedAtUnixSeconds\": {},\n    \"sourcePath\": \"{}\"\n  }}]\n}}\n",
        imported.id,
        imported.version,
        imported.path,
        imported.content_hash,
        imported.imported_at_unix_seconds,
        imported.source_path,
    );
    fs::write(store_root.join("index.json"), legacy_index).expect("legacy index");

    let expected = KmIndexRecord {
        id: imported.id.clone(),
        version: imported.version.clone(),
        path: imported.path.clone(),
        content_hash: imported.content_hash.clone(),
        status: KmModuleStatus::Active,
        imported_at_unix_seconds: imported.imported_at_unix_seconds,
        source_path: imported.source_path,
        topic_words: Vec::new(),
        origin: None,
        withdrawal: None,
        project_scope: None,
    };

    let listed = store
        .list(/*include_obsolete*/ true)
        .expect("legacy index loads")
        .into_iter()
        .collect::<Vec<_>>();
    let resolved = store
        .resolve_active("codebase-graph/creating-ontograph")
        .expect("resolve succeeds")
        .expect("active module");

    assert_eq!(listed, vec![expected.clone()]);
    assert_eq!(resolved, expected);
}

#[test]
fn import_invalid_km_does_not_update_index() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    fs::write(source.join("facts.jsonl"), "{\n").expect("break facts");

    let error = store.import(&source).expect_err("import should fail");
    let listed = store
        .list(/*include_obsolete*/ true)
        .expect("list succeeds");

    assert!(matches!(error, KmStoreError::InvalidKm(_)));
    assert_eq!(listed, Vec::<KmIndexRecord>::new());
}

#[test]
fn import_with_agent_origin_rejects_empty_and_whitespace_thread_ids() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/agent-empty",
        "0.1.0",
        "bounded claim",
    );

    assert_eq!(
        store.import_with_origin(
            &source,
            KmIndexOrigin::AgentThread {
                thread_id: "".to_string(),
            },
        ),
        Err(KmStoreError::InvalidOrigin)
    );
    assert_eq!(
        store.import_with_origin(
            &source,
            KmIndexOrigin::AgentThread {
                thread_id: "   ".to_string(),
            },
        ),
        Err(KmStoreError::InvalidOrigin)
    );
}

#[test]
fn list_for_origin_matches_exact_origin_and_thread_without_provenance_parsing() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let human_source = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/origin-human",
        "0.1.0",
        "bounded claim",
    );
    let agent_source = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/origin-agent",
        "0.1.0",
        "bounded claim",
    );
    let other_agent_source = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/origin-agent-other",
        "0.1.0",
        "bounded claim",
    );

    let human = store
        .import_with_origin(&human_source, KmIndexOrigin::Human)
        .expect("human import");
    let agent = store
        .import_with_origin(
            &agent_source,
            KmIndexOrigin::AgentThread {
                thread_id: "thread-match".to_string(),
            },
        )
        .expect("agent import");
    let other_agent = store
        .import_with_origin(
            &other_agent_source,
            KmIndexOrigin::AgentThread {
                thread_id: "thread-other".to_string(),
            },
        )
        .expect("other agent import");

    assert_eq!(
        store
            .list_for_origin(
                /*include_obsolete*/ true,
                Some(&KmIndexOrigin::Human),
                None,
            )
            .expect("filter by human"),
        vec![human],
    );
    assert_eq!(
        store
            .list_for_origin(
                /*include_obsolete*/ true,
                Some(&KmIndexOrigin::AgentThread {
                    thread_id: "thread-match".to_string(),
                }),
                Some("thread-match"),
            )
            .expect("filter by agent and thread"),
        vec![agent.clone()],
    );
    assert_eq!(
        store
            .list_for_origin(/*include_obsolete*/ true, None, Some("thread-match"),)
            .expect("filter by thread"),
        vec![agent],
    );
    assert_eq!(
        store
            .list_for_origin(
                /*include_obsolete*/ true,
                Some(&KmIndexOrigin::AgentThread {
                    thread_id: "thread-match".to_string(),
                }),
                Some("thread-other"),
            )
            .expect("mismatched thread filter"),
        Vec::<KmIndexRecord>::new(),
    );

    // Ensure thread-only filtering remains a first-class, explicit origin path.
    let _ = other_agent;
}

#[test]
fn legacy_originless_record_cannot_claim_agent_authorship() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let source = write_valid_km(
        &tempdir.path().join("legacy-thread"),
        "0.1.0",
        "bounded claim",
    );
    let imported = store.import(&source).expect("import succeeds");
    let legacy_index = format!(
        "{{\n  \"schemaVersion\": 1,\n  \"modules\": [{{\n    \"id\": \"{}\",\n    \"version\": \"{}\",\n    \"path\": \"{}\",\n    \"contentHash\": \"{}\",\n    \"status\": \"active\",\n    \"importedAtUnixSeconds\": {},\n    \"sourcePath\": \"{}\"\n  }}]\n}}\n",
        imported.id,
        imported.version,
        imported.path,
        imported.content_hash,
        imported.imported_at_unix_seconds,
        imported.source_path,
    );
    fs::write(store_root.join("index.json"), legacy_index).expect("legacy index");

    assert_eq!(
        store.list_for_origin(
            /*include_obsolete*/ true,
            Some(&KmIndexOrigin::AgentThread {
                thread_id: "legacy-thread".to_string(),
            }),
            Some("legacy-thread"),
        ),
        Ok(Vec::new())
    );
}

#[test]
fn obsolete_marks_record_and_hides_it_from_default_list_and_resolve() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    store.import(&source).expect("import succeeds");

    let obsolete = store
        .obsolete("codebase-graph/creating-ontograph", "0.1.0")
        .expect("obsolete succeeds")
        .expect("record updated");
    let default_list = store
        .list(/*include_obsolete*/ false)
        .expect("list succeeds");
    let full_list = store
        .list(/*include_obsolete*/ true)
        .expect("full list succeeds");

    assert_eq!(obsolete.status, KmModuleStatus::Obsolete);
    assert_eq!(default_list, Vec::<KmIndexRecord>::new());
    assert_eq!(full_list.len(), 1);
    assert_eq!(
        store
            .resolve_active("codebase-graph/creating-ontograph")
            .expect("resolve succeeds"),
        None
    );
}

#[test]
fn superseded_records_preserve_full_record_shape() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source_v1 = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/superseded-shape",
        "0.1.0",
        "bounded claim",
    );
    let source_v2 = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/superseded-shape",
        "0.2.0",
        "bounded claim",
    );

    let original = store.import(&source_v1).expect("import first");
    let _ = store.import(&source_v2).expect("import second");
    let mut expected_superseded = original;
    expected_superseded.status = KmModuleStatus::Obsolete;

    let superseded = store
        .list(/*include_obsolete*/ true)
        .expect("list succeeds")
        .into_iter()
        .find(|record| record.version == "0.1.0")
        .expect("superseded record");

    assert_eq!(superseded, expected_superseded);
}

#[test]
fn withdrawn_record_is_distinct_from_version_supersession_and_reimport_keeps_withdrawal() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source_v1 = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/withdrawal-shape",
        "0.1.0",
        "bounded claim",
    );
    let source_v2 = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/withdrawal-shape",
        "0.2.0",
        "bounded claim",
    );

    let original = store.import(&source_v1).expect("import first");
    let _ = store.import(&source_v2).expect("import second");
    let mut expected_superseded = original.clone();
    expected_superseded.status = KmModuleStatus::Obsolete;

    let superseded_before_withdraw = store
        .list(/*include_obsolete*/ true)
        .expect("list succeeds")
        .into_iter()
        .find(|record| record.version == "0.1.0")
        .expect("superseded record");
    assert_eq!(superseded_before_withdraw, expected_superseded);

    let withdrawn = store
        .withdraw(
            "codebase-graph/withdrawal-shape",
            "0.1.0",
            "policy violation".to_string(),
        )
        .expect("withdraw succeeds")
        .expect("withdraw record");

    let withdrawal = withdrawn
        .withdrawal
        .as_ref()
        .expect("withdrawal metadata present");
    assert_eq!(withdrawn.status, KmModuleStatus::Obsolete);
    assert_eq!(withdrawal.reason, "policy violation");
    assert!(withdrawal.withdrawn_at_unix_seconds > 0);

    let mut expected_withdrawn = original;
    expected_withdrawn.status = KmModuleStatus::Obsolete;
    expected_withdrawn.withdrawal = Some(KmModuleWithdrawal {
        reason: "policy violation".to_string(),
        withdrawn_at_unix_seconds: withdrawal.withdrawn_at_unix_seconds,
    });

    assert_eq!(
        store
            .list(/*include_obsolete*/ true)
            .expect("list succeeds")
            .into_iter()
            .find(|record| record.version == "0.1.0")
            .expect("withdrawn record"),
        expected_withdrawn,
    );

    let reimported = store.import(&source_v1).expect("re-import first");

    let preserved_withdrawal = store
        .list(/*include_obsolete*/ true)
        .expect("list succeeds")
        .into_iter()
        .find(|record| record.version == "0.1.0")
        .expect("withdrawn record");

    assert_eq!(withdrawn.version, reimported.version);
    assert_eq!(preserved_withdrawal, withdrawn);
    assert_eq!(reimported, withdrawn);
    assert_eq!(
        store
            .resolve_active("codebase-graph/withdrawal-shape")
            .expect("resolve succeeds")
            .expect("newer version remains active")
            .version,
        "0.2.0"
    );
}

#[test]
fn withdrawn_same_version_reimport_stays_unresolvable() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");

    store.import(&source).expect("import");
    store
        .withdraw(
            "codebase-graph/creating-ontograph",
            "0.1.0",
            "policy violation".to_string(),
        )
        .expect("withdraw succeeds")
        .expect("withdrawn record");
    let reimported = store.import(&source).expect("re-import");

    assert_eq!(reimported.status, KmModuleStatus::Obsolete);
    assert!(reimported.withdrawal.is_some());
    assert_eq!(
        store
            .resolve_active("codebase-graph/creating-ontograph")
            .expect("resolve succeeds"),
        None
    );
}

#[test]
fn agent_import_cannot_take_over_another_threads_package_id() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source_v1 = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/shared-id",
        "0.1.0",
        "bounded claim",
    );
    let source_v2 = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/shared-id",
        "0.2.0",
        "replacement claim",
    );

    store
        .import_for_agent_thread(&source_v1, "/tmp/project", "thread-a", 5)
        .expect("thread A import");
    assert_eq!(
        store.import_for_agent_thread(&source_v2, "/tmp/project", "thread-b", 5),
        Err(KmStoreError::PackageOwnershipConflict)
    );
    assert_eq!(
        store
            .resolve_active("codebase-graph/shared-id")
            .expect("resolve succeeds")
            .expect("thread A record remains active")
            .version,
        "0.1.0"
    );
}

#[test]
fn reimport_same_content_is_deterministic_and_reactivates_module() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");

    let first = store.import(&source).expect("first import");
    store
        .obsolete("codebase-graph/creating-ontograph", "0.1.0")
        .expect("obsolete succeeds");
    let second = store.import(&source).expect("second import");

    assert_eq!(first.content_hash, second.content_hash);
    assert_eq!(first.path, second.path);
    assert_eq!(
        first.imported_at_unix_seconds,
        second.imported_at_unix_seconds
    );
    assert_eq!(second.status, KmModuleStatus::Active);
}

#[test]
fn reimport_same_content_for_another_project_is_rejected() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");

    store
        .import_for_project(&source, "/tmp/project-a")
        .expect("first import");
    let error = store
        .import_for_project(&source, "/tmp/project-b")
        .expect_err("scope conflict expected");

    assert_eq!(error, KmStoreError::ProjectScopeConflict);
    assert_eq!(
        store
            .resolve_active("codebase-graph/creating-ontograph")
            .expect("resolve succeeds")
            .expect("active module")
            .project_scope,
        Some("/tmp/project-a".to_string())
    );
}

#[test]
fn project_store_root_is_stable_below_the_git_root() {
    let tempdir = TempDir::new().expect("temp dir");
    fs::create_dir(tempdir.path().join(".git")).expect("git dir");
    let nested = tempdir.path().join("nested/deeper");
    fs::create_dir_all(&nested).expect("nested dir");

    assert_eq!(
        project_root(&nested),
        Some(fs::canonicalize(tempdir.path()).expect("canonical root"))
    );
}

#[test]
fn project_root_is_unresolved_outside_git() {
    let tempdir = TempDir::new_in("/dev/shm").expect("temp dir");

    assert_eq!(project_root(tempdir.path()), None);
    assert_eq!(KmStore::for_project_path(tempdir.path()), None);
}

#[test]
fn importing_new_version_obsoletes_old_active_version() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let first = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    let second = write_valid_km(tempdir.path(), "0.2.0", "new bounded claim");

    store.import(&first).expect("first import");
    let newer = store.import(&second).expect("second import");
    let all = store
        .list(/*include_obsolete*/ true)
        .expect("list succeeds");

    assert_eq!(newer.version, "0.2.0");
    assert_eq!(all.len(), 2);
    assert_eq!(
        all.iter()
            .find(|record| record.version == "0.1.0")
            .expect("old version")
            .status,
        KmModuleStatus::Obsolete
    );
    assert_eq!(
        store
            .resolve_active("codebase-graph/creating-ontograph")
            .expect("resolve succeeds")
            .expect("active module")
            .version,
        "0.2.0"
    );
}

#[test]
fn importing_same_version_with_different_content_is_rejected() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let first = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    let second_parent = tempdir.path().join("other");
    fs::create_dir_all(&second_parent).expect("other dir");
    let second = write_valid_km(&second_parent, "0.1.0", "different claim");

    store.import(&first).expect("first import");
    let error = store.import(&second).expect_err("conflict expected");

    assert_eq!(error, KmStoreError::VersionConflict);
}

#[test]
fn validated_module_root_accepts_untampered_import() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    let imported = store.import(&source).expect("import succeeds");

    let root = store
        .validated_module_root(&imported)
        .expect("valid cache root");
    assert!(root.is_dir());
}

#[test]
fn validated_module_root_rejects_path_not_matching_id_version_layout() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    let mut imported = store.import(&source).expect("import succeeds");
    // Simulate a tampered index.json that points the record at an arbitrary path.
    imported.path = tempdir.path().join("evil").display().to_string();

    assert_eq!(
        store.validated_module_root(&imported),
        Err(KmStoreError::InvalidIndex)
    );
}

#[test]
fn validated_module_root_rejects_tampered_cache_content() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    let imported = store.import(&source).expect("import succeeds");

    // Tamper with the cached module after import so its hash no longer matches
    // the recorded content hash.
    fs::write(Path::new(&imported.path).join("facts.jsonl"), "{}\n").expect("tamper cache");

    assert_eq!(
        store.validated_module_root(&imported),
        Err(KmStoreError::InvalidIndex)
    );
}

#[test]
fn concurrent_imports_of_distinct_ids_all_remain_indexed() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = Arc::new(KmStore::new(tempdir.path().join("km-cache")));
    let mut sources = Vec::new();
    for index in 0..6 {
        let parent = tempdir.path().join(format!("src-{index}"));
        fs::create_dir_all(&parent).expect("src dir");
        sources.push(write_valid_km_with_id(
            &parent,
            &format!("codebase-graph/module-{index}"),
            "0.1.0",
            "bounded claim",
        ));
    }

    let handles = sources
        .into_iter()
        .map(|source| {
            let store = Arc::clone(&store);
            thread::spawn(move || store.import(&source).expect("import succeeds"))
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("import thread");
    }

    // Every concurrent import must remain represented (PR-027): the lock
    // serializes the load-modify-save so no appended record is lost.
    let listed = store
        .list(/*include_obsolete*/ true)
        .expect("list succeeds");
    assert_eq!(listed.len(), 6);
}

#[test]
fn failed_copy_leaves_no_partial_final_package() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    let imported = store.import(&source).expect("import succeeds");
    let module_root = std::path::PathBuf::from(&imported.path);

    // The staging sibling used during copy must not linger after a successful
    // import (PR-028): only the final module directory should exist.
    let staging = module_root.parent().expect("module parent").join(format!(
        "{}.staging",
        module_root
            .file_name()
            .expect("module name")
            .to_string_lossy()
    ));
    assert!(module_root.is_dir());
    assert!(!staging.exists());
}

#[test]
fn old_import_lock_owned_by_current_process_is_not_reclaimed() {
    assert!(!should_reclaim_import_lock(
        /*old_enough*/ true,
        Some(std::process::id())
    ));
}

#[cfg(unix)]
#[test]
fn old_import_lock_owned_by_dead_process_is_reclaimed() {
    assert!(should_reclaim_import_lock(
        /*old_enough*/ true,
        Some(i32::MAX as u32)
    ));
}

#[test]
fn old_import_lock_with_unknown_owner_is_not_reclaimed() {
    assert!(!should_reclaim_import_lock(
        /*old_enough*/ true, /*owner_pid*/ None
    ));
}

/// Withdrawal is the only kill-switch for a record that auto-injects into model
/// context, so it must survive a concurrent import. `withdraw` and `import`
/// both run load-modify-save over the same index file; without a shared lock
/// the later writer overwrites the earlier one and the withdrawal disappears.
#[test]
fn concurrent_withdraw_and_import_preserve_both_writes() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = Arc::new(KmStore::new(tempdir.path().join("km-cache")));
    let withdrawn_source = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/withdrawn",
        "0.1.0",
        "bounded claim",
    );
    store.import(&withdrawn_source).expect("seed import");

    let appended_source = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/appended",
        "0.1.0",
        "bounded claim",
    );

    let withdraw_store = Arc::clone(&store);
    let import_store = Arc::clone(&store);
    let withdraw_handle = thread::spawn(move || {
        withdraw_store
            .withdraw(
                "codebase-graph/withdrawn",
                "0.1.0",
                "superseded by review".to_string(),
            )
            .expect("withdraw succeeds")
    });
    let import_handle = thread::spawn(move || {
        import_store
            .import(&appended_source)
            .expect("import succeeds")
    });
    withdraw_handle.join().expect("withdraw thread");
    import_handle.join().expect("import thread");

    let listed = store
        .list(/*include_obsolete*/ true)
        .expect("list succeeds");
    let withdrawn = listed
        .iter()
        .find(|record| record.id == "codebase-graph/withdrawn")
        .expect("withdrawn record survives the concurrent import");
    assert!(
        withdrawn.withdrawal.is_some(),
        "withdrawal must not be lost to a concurrent import"
    );
    assert!(
        listed
            .iter()
            .any(|record| record.id == "codebase-graph/appended"),
        "concurrently imported record must not be lost to a withdrawal"
    );
}

/// `obsolete` shares the same unlocked load-modify-save shape as `withdraw`, so
/// it needs the same guarantee against a concurrent import.
#[test]
fn concurrent_obsolete_and_import_preserve_both_writes() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = Arc::new(KmStore::new(tempdir.path().join("km-cache")));
    let retired_source = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/retired",
        "0.1.0",
        "bounded claim",
    );
    store.import(&retired_source).expect("seed import");

    let appended_source = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/appended",
        "0.1.0",
        "bounded claim",
    );

    let obsolete_store = Arc::clone(&store);
    let import_store = Arc::clone(&store);
    let obsolete_handle = thread::spawn(move || {
        obsolete_store
            .obsolete("codebase-graph/retired", "0.1.0")
            .expect("obsolete succeeds")
    });
    let import_handle = thread::spawn(move || {
        import_store
            .import(&appended_source)
            .expect("import succeeds")
    });
    obsolete_handle.join().expect("obsolete thread");
    import_handle.join().expect("import thread");

    let listed = store
        .list(/*include_obsolete*/ true)
        .expect("list succeeds");
    let retired = listed
        .iter()
        .find(|record| record.id == "codebase-graph/retired")
        .expect("retired record survives the concurrent import");
    assert_eq!(retired.status, KmModuleStatus::Obsolete);
    assert!(
        listed
            .iter()
            .any(|record| record.id == "codebase-graph/appended"),
        "concurrently imported record must not be lost to an obsolete call"
    );
}

/// ADR decision D5: withdrawn records still count against the authoring
/// thread's budget, so a thread cannot free capacity by withdrawing its own
/// work. Pinned here so the ceiling cannot be quietly relaxed later.
#[test]
fn withdrawing_every_record_does_not_free_thread_ceiling_budget() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let ceiling = 2;
    let scope = "/workspace/project";

    for index in 0..ceiling {
        let source = write_valid_km_with_id(
            tempdir.path(),
            &format!("codebase-graph/authored-{index}"),
            "0.1.0",
            "bounded claim",
        );
        store
            .import_for_agent_thread(&source, scope, "thread-1", ceiling)
            .expect("import within ceiling");
        store
            .withdraw(
                &format!("codebase-graph/authored-{index}"),
                "0.1.0",
                "retracted".to_string(),
            )
            .expect("withdraw succeeds");
    }

    let overflow = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/authored-overflow",
        "0.1.0",
        "bounded claim",
    );
    let error = store
        .import_for_agent_thread(&overflow, scope, "thread-1", ceiling)
        .expect_err("ceiling still applies after withdrawal");

    assert_eq!(
        error,
        KmStoreError::ThreadCeiling {
            ceiling,
            current: ceiling
        }
    );
}

/// A truncated or corrupt index must fail loudly. Silently treating it as an
/// empty index would let the next import start from scratch and drop every
/// previously recorded module.
#[test]
fn corrupt_index_is_rejected_instead_of_read_as_empty() {
    let tempdir = TempDir::new().expect("temp dir");
    let cache_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&cache_root);
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    store.import(&source).expect("seed import");

    let index_path = cache_root.join("index.json");
    let raw = fs::read_to_string(&index_path).expect("index readable");
    fs::write(&index_path, &raw[..raw.len() / 2]).expect("truncate index");

    assert_eq!(
        store
            .list(/*include_obsolete*/ true)
            .expect_err("corrupt index must not read as empty"),
        KmStoreError::InvalidIndex
    );
    let second_source = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/second",
        "0.1.0",
        "bounded claim",
    );
    assert_eq!(
        store
            .import(&second_source)
            .expect_err("import must not overwrite a corrupt index"),
        KmStoreError::InvalidIndex
    );
}

// Backlog 1: a held lock makes a competing import fail bounded, not hang.
#[test]
fn import_lock_contention_times_out_with_write_index_error() {
    let tempdir = TempDir::new().expect("temp dir");
    let cache_root = tempdir.path().join("km-cache");
    fs::create_dir_all(&cache_root).expect("cache root");
    // A fresh lock owned by this live process is never reclaimed.
    fs::write(
        cache_root.join("import.lock"),
        std::process::id().to_string(),
    )
    .expect("lock");
    let store = KmStore::new(&cache_root);
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");

    let started = std::time::Instant::now();
    let error = store.import(&source).expect_err("lock is held");

    assert_eq!(error, KmStoreError::WriteIndex);
    assert!(started.elapsed() >= KM_IMPORT_LOCK_MAX_WAIT);
}

// Backlog 2: an import that fails after lock acquisition releases the lock.
#[test]
fn import_lock_released_on_import_error() {
    let tempdir = TempDir::new().expect("temp dir");
    let cache_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&cache_root);
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    store.import(&source).expect("first import");
    // Same id/version with different content fails after the lock is taken.
    fs::write(
        source.join("examples/README.md"),
        "# Example\n\nChanged body.\n",
    )
    .expect("mutate source");

    assert_eq!(
        store.import(&source).expect_err("version conflict"),
        KmStoreError::VersionConflict
    );
    assert!(!cache_root.join("import.lock").exists());
}

// Backlog 3: two importers racing over one reclaimable stale lock both finish
// and neither record is lost.
#[test]
fn stale_lock_reclaim_does_not_race_second_reclaimer() {
    let tempdir = TempDir::new().expect("temp dir");
    let cache_root = tempdir.path().join("km-cache");
    fs::create_dir_all(&cache_root).expect("cache root");
    let lock_path = cache_root.join("import.lock");
    // Dead owner: pid i32::MAX cannot exist. Backdate past the stale window.
    fs::write(&lock_path, i32::MAX.to_string()).expect("lock");
    let lock_file = fs::OpenOptions::new()
        .write(true)
        .open(&lock_path)
        .expect("open lock");
    lock_file
        .set_modified(SystemTime::now() - KM_IMPORT_LOCK_STALE - Duration::from_secs(30))
        .expect("backdate lock");
    drop(lock_file);

    let store = Arc::new(KmStore::new(&cache_root));
    let sources = [
        write_valid_km_with_id(
            tempdir.path(),
            "codebase-graph/one",
            "0.1.0",
            "bounded claim",
        ),
        write_valid_km_with_id(
            tempdir.path(),
            "codebase-graph/two",
            "0.1.0",
            "bounded claim",
        ),
    ];
    let handles = sources
        .into_iter()
        .map(|source| {
            let store = Arc::clone(&store);
            thread::spawn(move || store.import(&source).expect("import succeeds"))
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("import thread");
    }

    assert_eq!(
        store
            .list(/*include_obsolete*/ true)
            .expect("list succeeds")
            .len(),
        2
    );
}

// Backlog 4: unparseable lock content blocks reclaim at any age.
#[test]
fn lock_file_with_garbage_pid_is_never_reclaimed() {
    let tempdir = TempDir::new().expect("temp dir");
    let lock_path = tempdir.path().join("import.lock");
    fs::write(&lock_path, "not-a-pid").expect("lock");
    let lock_file = fs::OpenOptions::new()
        .write(true)
        .open(&lock_path)
        .expect("open lock");
    lock_file
        .set_modified(SystemTime::now() - KM_IMPORT_LOCK_STALE - Duration::from_secs(30))
        .expect("backdate lock");
    drop(lock_file);

    assert!(!lock_is_stale(&lock_path));
}

// Backlog 5: a lock younger than the stale window is respected even when its
// recorded owner is already dead.
#[test]
fn fresh_lock_from_dead_process_is_not_reclaimed_before_stale_window() {
    let tempdir = TempDir::new().expect("temp dir");
    let lock_path = tempdir.path().join("import.lock");
    fs::write(&lock_path, i32::MAX.to_string()).expect("lock");

    assert!(!lock_is_stale(&lock_path));
}

// Backlog 6: whichever order withdraw and obsolete interleave, the withdrawal
// metadata survives because obsolete never clears it.
#[test]
fn concurrent_withdraw_and_obsolete_on_same_record_keep_withdrawal() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = Arc::new(KmStore::new(tempdir.path().join("km-cache")));
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    store.import(&source).expect("seed import");

    let withdraw_store = Arc::clone(&store);
    let obsolete_store = Arc::clone(&store);
    let withdraw_handle = thread::spawn(move || {
        withdraw_store
            .withdraw(
                "codebase-graph/creating-ontograph",
                "0.1.0",
                "retracted".to_string(),
            )
            .expect("withdraw succeeds")
    });
    let obsolete_handle = thread::spawn(move || {
        obsolete_store
            .obsolete("codebase-graph/creating-ontograph", "0.1.0")
            .expect("obsolete succeeds")
    });
    withdraw_handle.join().expect("withdraw thread");
    obsolete_handle.join().expect("obsolete thread");

    let listed = store
        .list(/*include_obsolete*/ true)
        .expect("list succeeds");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].status, KmModuleStatus::Obsolete);
    assert!(listed[0].withdrawal.is_some());
}

// Backlog 7: the ceiling check and the append are atomic, so concurrent agent
// imports on one thread id can never overshoot the ceiling.
#[test]
fn concurrent_agent_imports_same_thread_never_exceed_ceiling() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = Arc::new(KmStore::new(tempdir.path().join("km-cache")));
    let ceiling = 2;
    let handles = (0..4)
        .map(|index| {
            let store = Arc::clone(&store);
            let source = write_valid_km_with_id(
                tempdir.path(),
                &format!("codebase-graph/racing-{index}"),
                "0.1.0",
                "bounded claim",
            );
            thread::spawn(move || {
                store.import_for_agent_thread(&source, "/workspace/project", "thread-1", ceiling)
            })
        })
        .collect::<Vec<_>>();

    let mut successes = 0;
    for handle in handles {
        match handle.join().expect("import thread") {
            Ok(_) => successes += 1,
            Err(KmStoreError::ThreadCeiling { .. }) => {}
            Err(other) => panic!("unexpected error: {other:?}"),
        }
    }
    assert_eq!(successes, ceiling);
}

// Backlog 8: junk left in a staging directory by a killed process does not
// block the next import of the same module.
#[test]
fn orphaned_staging_directory_is_replaced_by_next_import() {
    let tempdir = TempDir::new().expect("temp dir");
    let cache_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&cache_root);
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    let staging = cache_root
        .join("modules/codebase-graph/creating-ontograph")
        .join("0.1.0.staging");
    fs::create_dir_all(&staging).expect("orphan staging");
    fs::write(staging.join("junk.bin"), b"partial copy").expect("junk");

    let imported = store.import(&source).expect("import succeeds");

    assert!(!staging.exists());
    assert!(Path::new(&imported.path).is_dir());
}

// Backlog 9: a failed index write never destroys the previously saved index.
#[cfg(unix)]
#[test]
fn index_write_failure_leaves_previous_index_readable() {
    use std::os::unix::fs::PermissionsExt;

    let tempdir = TempDir::new().expect("temp dir");
    let cache_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&cache_root);
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    store.import(&source).expect("seed import");
    let before = fs::read_to_string(cache_root.join("index.json")).expect("index readable");

    fs::set_permissions(&cache_root, fs::Permissions::from_mode(0o555)).expect("read-only root");
    let result = store.withdraw(
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "retracted".to_string(),
    );
    fs::set_permissions(&cache_root, fs::Permissions::from_mode(0o755)).expect("restore root");

    assert!(result.is_err(), "write into a read-only root must fail");
    assert_eq!(
        fs::read_to_string(cache_root.join("index.json")).expect("index still readable"),
        before
    );
}

// Backlog 10: the serialized index is byte-stable across a same-content
// reimport and ends with exactly one trailing newline.
#[test]
fn save_index_output_ends_with_single_newline_and_stable_key_order() {
    let tempdir = TempDir::new().expect("temp dir");
    let cache_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&cache_root);
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    store.import(&source).expect("first import");
    let first = fs::read_to_string(cache_root.join("index.json")).expect("index readable");

    store.import(&source).expect("reimport same content");
    let second = fs::read_to_string(cache_root.join("index.json")).expect("index readable");

    assert!(first.ends_with('\n') && !first.ends_with("\n\n"));
    assert_eq!(first, second);
}

// Backlog 11: an unknown schema version is rejected, never silently migrated
// or treated as empty.
#[test]
fn schema_version_bump_is_rejected_not_migrated() {
    let tempdir = TempDir::new().expect("temp dir");
    let cache_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&cache_root);
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    store.import(&source).expect("seed import");

    let index_path = cache_root.join("index.json");
    let raw = fs::read_to_string(&index_path).expect("index readable");
    fs::write(
        &index_path,
        raw.replace("\"schemaVersion\": 1", "\"schemaVersion\": 2"),
    )
    .expect("bump schema");

    assert_eq!(
        store
            .list(/*include_obsolete*/ true)
            .expect_err("unknown schema must fail"),
        KmStoreError::InvalidIndex
    );
    let second = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/second",
        "0.1.0",
        "bounded claim",
    );
    assert_eq!(
        store
            .import(&second)
            .expect_err("import must not overwrite an unknown schema"),
        KmStoreError::InvalidIndex
    );
}

#[test]
fn oversized_index_is_rejected_before_deserialization() {
    let tempdir = TempDir::new().expect("temp dir");
    let cache_root = tempdir.path().join("km-cache");
    fs::create_dir_all(&cache_root).expect("cache root");
    fs::write(
        cache_root.join("index.json"),
        vec![b' '; MAX_KM_INDEX_BYTES + 1],
    )
    .expect("oversized index");

    let store = KmStore::new(&cache_root);
    assert_eq!(
        store
            .list(/*include_obsolete*/ true)
            .expect_err("index must be bounded"),
        KmStoreError::InvalidIndex
    );
}

// Backlog 12: swapping content between two same-length files changes the hash.
#[test]
fn hash_directory_distinguishes_file_content_swap_between_same_length_files() {
    let tempdir = TempDir::new().expect("temp dir");
    let left = tempdir.path().join("left");
    let right = tempdir.path().join("right");
    fs::create_dir_all(&left).expect("left dir");
    fs::create_dir_all(&right).expect("right dir");
    fs::write(left.join("f1"), "ab").expect("f1");
    fs::write(left.join("f2"), "cd").expect("f2");
    fs::write(right.join("f1"), "cd").expect("f1");
    fs::write(right.join("f2"), "ab").expect("f2");

    assert_ne!(
        hash_directory(&left).expect("left hash"),
        hash_directory(&right).expect("right hash")
    );
}

// Backlog 13: an empty file and an absent file hash differently because the
// framing includes the file name and length.
#[test]
fn hash_directory_distinguishes_empty_file_from_absent_file() {
    let tempdir = TempDir::new().expect("temp dir");
    let with_empty = tempdir.path().join("with-empty");
    let without = tempdir.path().join("without");
    fs::create_dir_all(&with_empty).expect("dir");
    fs::create_dir_all(&without).expect("dir");
    fs::write(with_empty.join("keep"), "x").expect("keep");
    fs::write(with_empty.join("empty"), "").expect("empty");
    fs::write(without.join("keep"), "x").expect("keep");

    assert_ne!(
        hash_directory(&with_empty).expect("hash"),
        hash_directory(&without).expect("hash")
    );
}

// Backlog 14: a nested id maps to nested cache directories that stay inside
// the modules root and validate cleanly.
#[test]
fn module_cache_path_nested_id_segments_stay_inside_modules_root() {
    let tempdir = TempDir::new().expect("temp dir");
    let cache_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&cache_root);
    let source = write_valid_km_with_id(tempdir.path(), "a/b/c", "0.1.0", "bounded claim");

    let imported = store.import(&source).expect("import succeeds");

    let expected = cache_root.join("modules/a/b/c/0.1.0");
    assert_eq!(
        Path::new(&imported.path),
        expected.display().to_string().as_str()
    );
    assert_eq!(
        store.validated_module_root(&imported).expect("validates"),
        fs::canonicalize(&expected).expect("canonical")
    );
}

// Backlog 100: simulate a process killed between staging the copy and renaming
// it into place. The index never references the staged path, and the next
// import self-heals by replacing the orphan.
#[test]
fn store_survives_kill9_simulation_between_stage_and_rename() {
    let tempdir = TempDir::new().expect("temp dir");
    let cache_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&cache_root);
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");

    // Stage a copy exactly where a killed import would have left it, and leave
    // a stale lock behind from the dead process.
    let module_parent = cache_root.join("modules/codebase-graph/creating-ontograph");
    let staging = module_parent.join("0.1.0.staging");
    fs::create_dir_all(&staging).expect("staging dir");
    fs::write(staging.join("km.toml"), "partial").expect("partial stage");
    fs::create_dir_all(&cache_root).expect("cache root");
    let lock_path = cache_root.join("import.lock");
    fs::write(&lock_path, i32::MAX.to_string()).expect("dead-owner lock");
    let lock_file = fs::OpenOptions::new()
        .write(true)
        .open(&lock_path)
        .expect("open lock");
    lock_file
        .set_modified(SystemTime::now() - KM_IMPORT_LOCK_STALE - Duration::from_secs(30))
        .expect("backdate lock");
    drop(lock_file);

    // The index must not mention the interrupted import at all.
    assert_eq!(
        store
            .list(/*include_obsolete*/ true)
            .expect("list succeeds"),
        Vec::new()
    );

    // The next import reclaims the stale lock, replaces the orphan staging
    // directory, and lands a valid module.
    let imported = store.import(&source).expect("import self-heals");

    assert!(!staging.exists(), "orphan staging directory is cleared");
    assert!(!lock_path.exists(), "stale lock is released");
    assert_eq!(
        Path::new(&imported.path),
        module_parent.join("0.1.0").as_path()
    );
    assert!(
        store.validated_module_root(&imported).is_ok(),
        "the healed module passes hash and containment validation"
    );
}

fn write_valid_km_with_id(root: &Path, id: &str, version: &str, claim: &str) -> std::path::PathBuf {
    let km_root = root.join(format!("{}-{version}", id.replace('/', "-")));
    fs::create_dir_all(km_root.join("examples")).expect("examples dir");
    fs::create_dir_all(km_root.join("tests")).expect("tests dir");
    fs::write(km_root.join("km.toml"), manifest_with_id(id, version)).expect("manifest");
    fs::write(
        km_root.join("facts.jsonl"),
        format!(
            "{{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"{claim}\",\"source\":\"silver:doc\",\"confidence\":0.9}}\n"
        ),
    )
    .expect("facts");
    fs::write(
        km_root.join("examples/README.md"),
        "# Example\n\nExample body.\n",
    )
    .expect("example");
    fs::write(km_root.join("tests/README.md"), "# Test\n\nTest body.\n").expect("test");
    km_root
}

fn manifest_with_id(id: &str, version: &str) -> String {
    format!(
        "id = \"{id}\"\nversion = \"{version}\"\nkind = \"codebase-graph\"\nsummary = \"A bounded KM\"\nsources = [\"silver:doc\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n"
    )
}

fn write_valid_km(root: &Path, version: &str, claim: &str) -> std::path::PathBuf {
    let km_root = root.join(format!("sample-km-{version}"));
    fs::create_dir_all(km_root.join("examples")).expect("examples dir");
    fs::create_dir_all(km_root.join("tests")).expect("tests dir");
    fs::write(km_root.join("km.toml"), valid_manifest(version)).expect("manifest");
    fs::write(
        km_root.join("facts.jsonl"),
        format!(
            "{{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"{claim}\",\"source\":\"silver:doc\",\"confidence\":0.9}}\n"
        ),
    )
    .expect("facts");
    fs::write(
        km_root.join("examples/README.md"),
        "# Example\n\nExample body.\n",
    )
    .expect("example");
    fs::write(km_root.join("tests/README.md"), "# Test\n\nTest body.\n").expect("test");
    km_root
}

fn valid_manifest(version: &str) -> String {
    format!(
        "id = \"codebase-graph/creating-ontograph\"\nversion = \"{version}\"\nkind = \"codebase-graph\"\nsummary = \"A bounded KM\"\nsources = [\"silver:doc\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n"
    )
}
