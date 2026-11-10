use std::path::Path;

use tempfile::TempDir;

use super::*;
use crate::km_store::KmModuleStatus;
use crate::km_store::KmStoreError;

#[test]
fn feature_off_advertises_none_of_the_agent_verbs() {
    assert_eq!(
        advertised_agent_tool_names(/*feature_enabled*/ false),
        Vec::<&str>::new()
    );
    assert_eq!(
        advertised_agent_tool_names(/*feature_enabled*/ true),
        vec!["km_author", "km_list", "km_withdraw"]
    );
}

#[test]
fn exactly_three_agent_verbs_exist_and_km_context_is_not_one_of_them() {
    let names = agent_tool_names();
    assert_eq!(names.len(), 3);
    assert_eq!(names, ["km_author", "km_list", "km_withdraw"]);
    // km_context stays read-only in tool.rs and is never re-declared here.
    assert!(!names.contains(&"km_context"));
}

#[test]
fn no_agent_tool_schema_accepts_caller_supplied_identity() {
    // A model must never be able to name the thread or project it writes to.
    for schema in [
        serde_json::to_string(&schemars::schema_for!(KmAuthorArgs)).expect("author schema"),
        serde_json::to_string(&schemars::schema_for!(KmListArgs)).expect("list schema"),
        serde_json::to_string(&schemars::schema_for!(KmWithdrawArgs)).expect("withdraw schema"),
    ] {
        assert!(!schema.contains("thread_id"), "schema leaks thread_id");
        assert!(
            !schema.contains("project_scope"),
            "schema leaks project_scope"
        );
        assert!(
            !schema.contains("project_root"),
            "schema leaks project_root"
        );
    }
}

#[test]
fn identity_fails_closed_on_a_blank_thread_id() {
    assert!(AgentToolIdentity::new("", PathBuf::from("/tmp/project")).is_none());
    assert!(AgentToolIdentity::new("   ", PathBuf::from("/tmp/project")).is_none());
    assert!(AgentToolIdentity::new("thread-1", PathBuf::from("/tmp/project")).is_some());
}

#[test]
fn list_returns_only_this_threads_records_and_hides_withdrawn_by_default() {
    let project = TempDir::new().expect("temp dir");
    let identity = identity_for(project.path(), "thread-1");
    let (store, _) = identity.store().expect("store");

    seed(&store, project.path(), "codebase-graph/mine-a", "thread-1");
    seed(&store, project.path(), "codebase-graph/mine-b", "thread-1");
    seed(&store, project.path(), "codebase-graph/theirs", "thread-2");
    store
        .withdraw(
            "codebase-graph/mine-b",
            "0.1.0",
            "policy violation".to_string(),
        )
        .expect("withdraw succeeds")
        .expect("withdrawn record");

    let active = run_km_list(
        &identity,
        &KmListArgs {
            include_withdrawn: false,
        },
    )
    .expect("list succeeds");
    assert_eq!(
        active.records,
        vec![KmListEntry {
            id: "codebase-graph/mine-a".to_string(),
            version: "0.1.0".to_string(),
            status: "active".to_string(),
            withdrawn_reason: None,
        }]
    );

    let all = run_km_list(
        &identity,
        &KmListArgs {
            include_withdrawn: true,
        },
    )
    .expect("list succeeds");
    let ids = all
        .records
        .iter()
        .map(|record| record.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["codebase-graph/mine-a", "codebase-graph/mine-b"]);
    assert_eq!(
        all.records[1].withdrawn_reason.as_deref(),
        Some("policy violation")
    );
}

#[test]
fn withdraw_refuses_a_record_authored_by_another_thread() {
    let project = TempDir::new().expect("temp dir");
    let mine = identity_for(project.path(), "thread-1");
    let (store, _) = mine.store().expect("store");
    seed(&store, project.path(), "codebase-graph/theirs", "thread-2");

    let error = run_km_withdraw(
        &mine,
        &KmWithdrawArgs {
            id: "codebase-graph/theirs".to_string(),
            version: "0.1.0".to_string(),
            reason: "policy violation".to_string(),
        },
    )
    .expect_err("cross-thread withdrawal must be refused");
    assert!(format!("{error:?}").contains("has no record"));

    // The other thread's record is untouched.
    let records = store
        .list_for_origin(/*include_obsolete*/ true, None, None)
        .expect("list succeeds");
    assert_eq!(records[0].status, KmModuleStatus::Active);
    assert!(records[0].withdrawal.is_none());
}

#[test]
fn withdraw_marks_this_threads_own_record() {
    let project = TempDir::new().expect("temp dir");
    let identity = identity_for(project.path(), "thread-1");
    let (store, _) = identity.store().expect("store");
    seed(&store, project.path(), "codebase-graph/mine", "thread-1");

    let result = run_km_withdraw(
        &identity,
        &KmWithdrawArgs {
            id: "codebase-graph/mine".to_string(),
            version: "0.1.0".to_string(),
            reason: "policy violation".to_string(),
        },
    )
    .expect("withdraw succeeds");
    assert!(result.withdrawn);

    let records = store
        .list_for_origin(/*include_obsolete*/ true, None, None)
        .expect("list succeeds");
    assert_eq!(
        records[0].withdrawal.as_ref().map(|w| w.reason.as_str()),
        Some("policy violation")
    );
}

#[test]
fn withdraw_rejects_blank_arguments_without_touching_the_store() {
    let project = TempDir::new().expect("temp dir");
    let identity = identity_for(project.path(), "thread-1");
    let (store, _) = identity.store().expect("store");
    seed(&store, project.path(), "codebase-graph/mine", "thread-1");
    let before = store_bytes(project.path());

    for args in [
        KmWithdrawArgs {
            id: "  ".to_string(),
            version: "0.1.0".to_string(),
            reason: "policy".to_string(),
        },
        KmWithdrawArgs {
            id: "codebase-graph/mine".to_string(),
            version: String::new(),
            reason: "policy".to_string(),
        },
        KmWithdrawArgs {
            id: "codebase-graph/mine".to_string(),
            version: "0.1.0".to_string(),
            reason: "   ".to_string(),
        },
    ] {
        let error = run_km_withdraw(&identity, &args).expect_err("blank arguments are refused");
        assert!(format!("{error:?}").contains("requires id, version, and reason"));
    }

    assert_eq!(store_bytes(project.path()), before);
}

#[test]
fn author_refuses_a_package_path_that_escapes_the_project_root() {
    let project = TempDir::new().expect("temp dir");
    let identity = identity_for(project.path(), "thread-1");

    // `PathBuf::join` would discard project_root for an absolute path, so these
    // must be refused before any filesystem or store access.
    for package_path in ["/etc", "../outside", "docs/../../escape", "  "] {
        let error = run_km_author(
            &identity,
            &KmAuthorArgs {
                package_path: package_path.to_string(),
                citation_path: "docs/evidence.md".to_string(),
                source_commit: "HEAD".to_string(),
            },
        )
        .expect_err("escaping package path must be refused");
        assert!(
            format!("{error:?}").contains("must resolve to a directory inside the project"),
            "path must be refused as non-project-local: {package_path}"
        );
    }
}

#[test]
fn list_is_bounded_and_reports_truncation() {
    let project = TempDir::new().expect("temp dir");
    let identity = identity_for(project.path(), "thread-1");
    let (store, _) = identity.store().expect("store");
    for index in 0..(MAX_LISTED_RECORDS + 1) {
        seed(
            &store,
            project.path(),
            &format!("codebase-graph/bulk-{index:03}"),
            "thread-1",
        );
    }

    let listed = run_km_list(
        &identity,
        &KmListArgs {
            include_withdrawn: false,
        },
    )
    .expect("list succeeds");

    assert_eq!(listed.records.len(), MAX_LISTED_RECORDS);
    assert!(listed.truncated);
}

/// Recursive byte snapshot of the live store, used to prove a rejection wrote
/// nothing.
fn store_bytes(project_root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut entries = Vec::new();
    collect_store_files(
        &project_root.join(".ontocode/ontograph/km/_store"),
        &mut entries,
    );
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    entries
}

fn collect_store_files(dir: &Path, entries: &mut Vec<(PathBuf, Vec<u8>)>) {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_store_files(&path, entries);
        } else if let Ok(bytes) = std::fs::read(&path) {
            entries.push((path, bytes));
        }
    }
}

#[test]
fn author_surfaces_one_bounded_rejection_for_an_uncitable_candidate() {
    let project = TempDir::new().expect("temp dir");
    let identity = identity_for(project.path(), "thread-1");
    // A real package must exist, or authoring stops at the manifest read before
    // it ever evaluates the citation this test is about.
    write_candidate_package_at(&project.path().join("candidate"));

    let error = run_km_author(
        &identity,
        &KmAuthorArgs {
            package_path: "candidate".to_string(),
            citation_path: "docs/never-committed.md".to_string(),
            source_commit: "0000000000000000000000000000000000000000".to_string(),
        },
    )
    .expect_err("uncitable candidate must be refused");

    let message = format!("{error:?}");
    assert!(message.contains("ontograph.km_author rejected"));
    // The rejection is bounded: no store paths or internal errors leak.
    assert!(!message.contains(".ontocode/ontograph"));
}

fn identity_for(project_root: &Path, thread_id: &str) -> AgentToolIdentity {
    AgentToolIdentity::new(thread_id, project_root.to_path_buf()).expect("identity")
}

// Backlog 63: a held store lock makes authoring fail bounded rather than hang.
#[test]
fn author_rejects_candidate_while_import_lock_is_held_with_bounded_error() {
    let repo = git_repo_with_committed_evidence();
    let identity = identity_for(repo.path(), "thread-1");
    write_eval_suite(repo.path());
    let package_dir = "candidate-locked";
    write_candidate_package_at(&repo.path().join(package_dir));

    // Take the store lock with a live owner so it is never reclaimed.
    let store_root = repo.path().join(".ontocode/ontograph/km/_store");
    std::fs::create_dir_all(&store_root).expect("store root");
    std::fs::write(
        store_root.join("import.lock"),
        std::process::id().to_string(),
    )
    .expect("hold lock");

    let started = std::time::Instant::now();
    let error = run_km_author(
        &identity,
        &KmAuthorArgs {
            package_path: package_dir.to_string(),
            citation_path: "docs/evidence.md".to_string(),
            source_commit: repo.head(),
        },
    )
    .expect_err("author cannot take the held lock");

    assert!(started.elapsed() < std::time::Duration::from_secs(60));
    let message = format!("{error:?}");
    assert!(message.contains("ontograph.km_author"));
    assert!(!message.contains(".ontocode/ontograph"));
}

// Backlog 64 (decision: pin current behavior): withdrawing twice overwrites
// the stored reason and timestamp with the latest call.
#[test]
fn withdraw_of_already_withdrawn_record_overwrites_reason() {
    let project = TempDir::new().expect("temp dir");
    let identity = identity_for(project.path(), "thread-1");
    let (store, _) = identity.store().expect("store");
    seed(
        &store,
        project.path(),
        "codebase-graph/twice-withdrawn",
        "thread-1",
    );

    let args = |reason: &str| KmWithdrawArgs {
        id: "codebase-graph/twice-withdrawn".to_string(),
        version: "0.1.0".to_string(),
        reason: reason.to_string(),
    };
    let first = run_km_withdraw(&identity, &args("first reason")).expect("first withdraw");
    let second = run_km_withdraw(&identity, &args("second reason")).expect("second withdraw");

    assert!(first.withdrawn);
    assert!(second.withdrawn, "repeat withdrawal stays successful");
    let listed = store
        .list(/*include_obsolete*/ true)
        .expect("list succeeds");
    assert_eq!(
        listed[0]
            .withdrawal
            .as_ref()
            .expect("withdrawal present")
            .reason,
        "second reason"
    );
}

// Backlog 65: listing with withdrawn records included exposes the reason.
#[test]
fn list_include_withdrawn_shows_reason_and_timestamp() {
    let project = TempDir::new().expect("temp dir");
    let identity = identity_for(project.path(), "thread-1");
    let (store, _) = identity.store().expect("store");
    seed(
        &store,
        project.path(),
        "codebase-graph/listed-withdrawn",
        "thread-1",
    );
    run_km_withdraw(
        &identity,
        &KmWithdrawArgs {
            id: "codebase-graph/listed-withdrawn".to_string(),
            version: "0.1.0".to_string(),
            reason: "documented reason".to_string(),
        },
    )
    .expect("withdraw succeeds");

    let listed = run_km_list(
        &identity,
        &KmListArgs {
            include_withdrawn: true,
        },
    )
    .expect("list succeeds");

    assert_eq!(listed.records.len(), 1);
    assert_eq!(
        listed.records[0].withdrawn_reason.as_deref(),
        Some("documented reason")
    );
}

// Backlog 66: a freshly authored record is immediately resolvable through the
// same store the author wrote to.
#[test]
fn author_then_immediate_context_lookup_returns_the_new_record() {
    let repo = git_repo_with_committed_evidence();
    let identity = identity_for(repo.path(), "thread-1");
    write_eval_suite(repo.path());
    let package_dir = "candidate-readback";
    write_candidate_package_at(&repo.path().join(package_dir));

    run_km_author(
        &identity,
        &KmAuthorArgs {
            package_path: package_dir.to_string(),
            citation_path: "docs/evidence.md".to_string(),
            source_commit: repo.head(),
        },
    )
    .expect("author succeeds");

    let (store, _) = identity.store().expect("store");
    let resolved = crate::km_resolve::resolve_active_km(&store, "codebase-graph/public-workflow")
        .expect("resolve succeeds")
        .expect("record is immediately readable");
    assert_eq!(resolved.id, "codebase-graph/public-workflow");
}

// Backlog 67: two threads authoring the same id/version leave exactly one
// record. Ownership is checked before content, so the second thread is refused
// as a package-ownership conflict rather than a version conflict; the
// same-thread case is what surfaces VersionConflict.
#[test]
fn two_threads_author_same_id_version_different_content_one_gets_version_conflict() {
    let project = TempDir::new().expect("temp dir");
    let store = KmStore::new(project.path().join("km-cache"));
    let first = project.path().join("first");
    let second = project.path().join("second");
    write_candidate_package_at(&first);
    write_candidate_package_at(&second);
    // Same id and version, different bytes.
    std::fs::write(second.join("examples/README.md"), "different example").expect("diverge second");

    store
        .import_for_agent_thread(&first, "/workspace/project", "thread-a", 5)
        .expect("first import wins");
    let cross_thread = store
        .import_for_agent_thread(&second, "/workspace/project", "thread-b", 5)
        .expect_err("another thread cannot claim an owned package");
    assert_eq!(cross_thread, KmStoreError::PackageOwnershipConflict);

    // The owning thread reaches the content check and is refused there.
    let same_thread = store
        .import_for_agent_thread(&second, "/workspace/project", "thread-a", 5)
        .expect_err("same version with different content is refused");
    assert_eq!(same_thread, KmStoreError::VersionConflict);

    assert_eq!(
        store
            .list(/*include_obsolete*/ true)
            .expect("list succeeds")
            .len(),
        1
    );
}

// Backlog 70: a thread id containing path separators never escapes the module
// cache layout, because the id is stored as opaque metadata.
#[test]
fn agent_origin_thread_id_with_path_separators_is_stored_inert() {
    let project = TempDir::new().expect("temp dir");
    let cache_root = project.path().join("km-cache");
    let store = KmStore::new(&cache_root);
    let package = project.path().join("candidate");
    write_candidate_package_at(&package);

    let imported = store
        .import_for_agent_thread(&package, "/workspace/project", "../../escape", 5)
        .expect("import succeeds");

    assert_eq!(
        imported.origin,
        Some(KmIndexOrigin::AgentThread {
            thread_id: "../../escape".to_string()
        })
    );
    // The cache path is derived from id and version only.
    assert_eq!(
        Path::new(&imported.path),
        cache_root
            .join("modules/codebase-graph/public-workflow/0.1.0")
            .as_path()
    );
    assert!(
        store.validated_module_root(&imported).is_ok(),
        "cache layout stays inside the modules root"
    );
}

// Backlog 72: the ceiling rejection names the exact numbers so the model knows
// it is out of budget rather than malformed.
#[test]
fn ceiling_error_message_reports_exact_current_and_ceiling_numbers() {
    let project = TempDir::new().expect("temp dir");
    let store = KmStore::new(project.path().join("km-cache"));
    let first = project.path().join("first");
    write_candidate_package_at(&first);
    store
        .import_for_agent_thread(&first, "/workspace/project", "thread-1", 1)
        .expect("first import");

    let second = project.path().join("second");
    write_candidate_package_at(&second);
    std::fs::write(
        second.join("km.toml"),
        "id = \"codebase-graph/second\"\nversion = \"0.1.0\"\nkind = \"codebase-graph\"\nsummary = \"ontograph km_store bounded retrieval summary\"\nsources = [\"silver:doc\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n",
    )
    .expect("second manifest");

    let error = store
        .import_for_agent_thread(&second, "/workspace/project", "thread-1", 1)
        .expect_err("ceiling reached");

    assert_eq!(
        error,
        KmStoreError::ThreadCeiling {
            ceiling: 1,
            current: 1
        }
    );
}

/// Drives all four public stages through the real verbs: author, retrieve from
/// the store the author wrote, list, then withdraw. Seeding the store directly
/// would let `run_km_author` regress to always-reject, or write to a different
/// store than listing reads, without any test noticing.
#[test]
fn the_public_author_retrieve_list_withdraw_workflow_succeeds_end_to_end() {
    let repo = git_repo_with_committed_evidence();
    let identity = identity_for(repo.path(), "thread-1");
    write_eval_suite(repo.path());
    let package_dir = "candidate-workflow";
    write_candidate_package_at(&repo.path().join(package_dir));

    // 1. Author through the public verb.
    let authored = run_km_author(
        &identity,
        &KmAuthorArgs {
            package_path: package_dir.to_string(),
            citation_path: "docs/evidence.md".to_string(),
            source_commit: repo.head(),
        },
    )
    .expect("author succeeds");
    assert_eq!(authored.id, "codebase-graph/public-workflow");

    // 2. Retrieve from the store the author actually wrote, with typed origin.
    let (store, _) = identity.store().expect("store");
    let stored = store
        .list_for_origin(
            /*include_obsolete*/ true,
            Some(&KmIndexOrigin::AgentThread {
                thread_id: "thread-1".to_string(),
            }),
            Some("thread-1"),
        )
        .expect("list succeeds");
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].id, "codebase-graph/public-workflow");

    // 3. List through the public verb.
    assert_eq!(
        run_km_list(
            &identity,
            &KmListArgs {
                include_withdrawn: false,
            },
        )
        .expect("list succeeds")
        .records,
        vec![KmListEntry {
            id: "codebase-graph/public-workflow".to_string(),
            version: "0.1.0".to_string(),
            status: "active".to_string(),
            withdrawn_reason: None,
        }]
    );

    // 4. Withdraw through the public verb; default listing stops showing it.
    assert!(
        run_km_withdraw(
            &identity,
            &KmWithdrawArgs {
                id: "codebase-graph/public-workflow".to_string(),
                version: "0.1.0".to_string(),
                reason: "superseded".to_string(),
            },
        )
        .expect("withdraw succeeds")
        .withdrawn
    );
    assert_eq!(
        run_km_list(
            &identity,
            &KmListArgs {
                include_withdrawn: false,
            },
        )
        .expect("list succeeds")
        .records,
        Vec::<KmListEntry>::new()
    );
}

struct WorkflowRepo {
    dir: TempDir,
}

impl WorkflowRepo {
    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn head(&self) -> String {
        let output = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(self.path())
            .output()
            .expect("run git");
        String::from_utf8(output.stdout)
            .expect("utf8 head")
            .trim()
            .to_string()
    }
}

fn git_repo_with_committed_evidence() -> WorkflowRepo {
    let repo = WorkflowRepo {
        dir: TempDir::new().expect("temp dir"),
    };
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(repo.path())
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .output()
            .expect("run git");
        assert!(output.status.success(), "git {args:?} failed");
    };
    git(&["init", "--quiet"]);
    std::fs::create_dir_all(repo.path().join("docs")).expect("docs dir");
    std::fs::write(repo.path().join("docs/evidence.md"), "committed evidence\n")
        .expect("write evidence");
    git(&["add", "docs/evidence.md"]);
    git(&["commit", "-m", "add evidence"]);
    repo
}

fn write_eval_suite(project_root: &Path) {
    let suite = project_root.join(".ontocode/ontograph/km/_eval/suite");
    std::fs::create_dir_all(&suite).expect("eval suite dir");
    std::fs::write(
        suite.join("case-1.json"),
        "{\"id\":\"case-1\",\"prompt\":\"ontograph km_store retrieval\",\"requiredSubstrings\":[\"ontograph\"]}",
    )
    .expect("eval cases");
}

fn write_candidate_package_at(package: &Path) {
    std::fs::create_dir_all(package.join("examples")).expect("examples");
    std::fs::create_dir_all(package.join("tests")).expect("tests");
    std::fs::write(
        package.join("km.toml"),
        "id = \"codebase-graph/public-workflow\"\nversion = \"0.1.0\"\nkind = \"codebase-graph\"\nsummary = \"ontograph km_store resolve_active_km import_with_scope km_context retrieval\"\nsources = [\"silver:docs/evidence.md\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n",
    )
    .expect("manifest");
    std::fs::write(
        package.join("facts.jsonl"),
        "{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"ontograph km_store bounded retrieval\",\"source\":\"silver:docs/evidence.md\",\"confidence\":0.9}\n",
    )
    .expect("facts");
    std::fs::write(package.join("examples/README.md"), "example").expect("example");
    std::fs::write(package.join("tests/README.md"), "test").expect("test");
}

#[test]
fn the_complete_list_then_withdraw_then_list_workflow_round_trips() {
    let project = TempDir::new().expect("temp dir");
    let identity = identity_for(project.path(), "thread-1");
    let (store, _) = identity.store().expect("store");
    seed(
        &store,
        project.path(),
        "codebase-graph/round-trip",
        "thread-1",
    );

    // List sees the active record.
    let listed = run_km_list(
        &identity,
        &KmListArgs {
            include_withdrawn: false,
        },
    )
    .expect("list succeeds");
    assert_eq!(
        listed.records,
        vec![KmListEntry {
            id: "codebase-graph/round-trip".to_string(),
            version: "0.1.0".to_string(),
            status: "active".to_string(),
            withdrawn_reason: None,
        }]
    );

    // Withdraw it.
    let withdrawn = run_km_withdraw(
        &identity,
        &KmWithdrawArgs {
            id: "codebase-graph/round-trip".to_string(),
            version: "0.1.0".to_string(),
            reason: "superseded by newer guidance".to_string(),
        },
    )
    .expect("withdraw succeeds");
    assert!(withdrawn.withdrawn);

    // Default listing no longer shows it; explicit opt-in does, with its reason.
    assert_eq!(
        run_km_list(
            &identity,
            &KmListArgs {
                include_withdrawn: false,
            },
        )
        .expect("list succeeds")
        .records,
        Vec::<KmListEntry>::new()
    );
    assert_eq!(
        run_km_list(
            &identity,
            &KmListArgs {
                include_withdrawn: true,
            },
        )
        .expect("list succeeds")
        .records,
        vec![KmListEntry {
            id: "codebase-graph/round-trip".to_string(),
            version: "0.1.0".to_string(),
            status: "obsolete".to_string(),
            withdrawn_reason: Some("superseded by newer guidance".to_string()),
        }]
    );
}

/// Imports one typed agent-authored record directly, bypassing the authoring
/// gates that `km_agent_authoring_tests.rs` already covers.
fn seed(store: &KmStore, root: &Path, id: &str, thread_id: &str) {
    let package = root.join(format!("seed-{}", id.replace('/', "-")));
    std::fs::create_dir_all(package.join("examples")).expect("examples");
    std::fs::create_dir_all(package.join("tests")).expect("tests");
    std::fs::write(
        package.join("km.toml"),
        format!(
            "id = \"{id}\"\nversion = \"0.1.0\"\nkind = \"codebase-graph\"\nsummary = \"ontograph km_store bounded retrieval context\"\nsources = [\"silver:doc\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n"
        ),
    )
    .expect("manifest");
    std::fs::write(
        package.join("facts.jsonl"),
        "{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"ontograph km_store bounded retrieval\",\"source\":\"silver:doc\",\"confidence\":0.9}\n",
    )
    .expect("facts");
    std::fs::write(package.join("examples/README.md"), "example").expect("example");
    std::fs::write(package.join("tests/README.md"), "test").expect("test");
    // `silver:doc` resolves against the project root, so the cited file must exist.
    std::fs::write(root.join("doc"), "doc").expect("cited source");
    store
        .import_for_project_with_origin(
            &package,
            &root.display().to_string(),
            KmIndexOrigin::AgentThread {
                thread_id: thread_id.to_string(),
            },
        )
        .expect("seed record");
}
