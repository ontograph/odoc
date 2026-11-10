use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;

use super::*;
use crate::km_eval::KmEvalResult;
use crate::km_eval::candidate_eval_passed;
use crate::km_store::KmIndexOrigin;
use crate::km_store::KmStore;

/// Frozen acceptance corpus from ADR decision D1. Changing these prompts or the
/// 6-word floor requires a new fixture version and an ADR amendment.
struct KmTopicFixtureV1 {
    negatives: [&'static str; 10],
    positives: [&'static str; 5],
    package_summary: &'static str,
}

const KM_TOPIC_FIXTURE_V1: KmTopicFixtureV1 = KmTopicFixtureV1 {
    // Generic engineering prose with no repository-specific nouns.
    negatives: [
        "please build everything and report any failures you see",
        "run the test suite again and tell me what broke this time",
        "refactor this function so it reads a little more clearly",
        "review my recent changes and leave comments where needed",
        "fix the failing check before we merge this branch today",
        "update the documentation so it matches the current code",
        "add a comment explaining what this block actually does",
        "clean up unused imports and remove any dead code paths",
        "bump the version and prepare the next release candidate",
        "investigate why the pipeline is slower than it was before",
    ],
    // Domain prose carrying repository-specific identifiers.
    positives: [
        "how does ontograph resolve an active km package for retrieval",
        "explain km_store import_with_scope and its project scope handling",
        "what does resolve_active_km return when the km record is obsolete",
        "describe the ontograph km_context contributor and its topic matching",
        "trace km index origin through ontograph resolve and context rendering",
    ],
    package_summary: "ontograph km_store resolve_active_km import_with_scope km_context retrieval",
};

#[test]
fn topic_fixture_v1_negatives_never_reach_the_match_threshold() {
    let package_words = candidate_topic_words(KM_TOPIC_FIXTURE_V1.package_summary);

    for prompt in KM_TOPIC_FIXTURE_V1.negatives {
        let matches = overlap(&package_words, prompt);
        assert_eq!(
            matches, 0,
            "negative prompt must not match the authored package: {prompt}"
        );
    }
}

#[test]
fn topic_fixture_v1_positives_all_reach_the_match_threshold() {
    let package_words = candidate_topic_words(KM_TOPIC_FIXTURE_V1.package_summary);

    for prompt in KM_TOPIC_FIXTURE_V1.positives {
        let matches = overlap(&package_words, prompt);
        assert!(
            matches >= 2,
            "positive prompt must match at least two topic words: {prompt} (got {matches})"
        );
    }
}

#[test]
fn candidates_below_the_frozen_word_floor_are_rejected() {
    // Pins the frozen boundary independently: exactly five surviving words are
    // rejected and exactly six are accepted. Expected counts are literals, not
    // values read back from the implementation, so a tokenizer or threshold
    // regression cannot validate itself.
    let five_words = "alpha beta gamma delta epsilon";
    assert_eq!(
        candidate_topic_words(five_words),
        ["alpha", "beta", "delta", "epsilon", "gamma"]
    );
    assert_eq!(
        validate_topic_discrimination(five_words),
        Err(TopicRejection::TooGeneric { distinct_words: 5 })
    );

    let six_words = "alpha beta gamma delta epsilon zeta";
    assert_eq!(
        candidate_topic_words(six_words),
        ["alpha", "beta", "delta", "epsilon", "gamma", "zeta"]
    );
    assert_eq!(validate_topic_discrimination(six_words), Ok(()));

    assert_eq!(
        validate_topic_discrimination(KM_TOPIC_FIXTURE_V1.package_summary),
        Ok(())
    );
}

#[test]
fn stop_words_and_short_tokens_do_not_count_toward_the_floor() {
    // Only `alpha` survives: the rest are stop words or shorter than three chars.
    assert_eq!(candidate_topic_words("the and with a an alpha"), ["alpha"]);
}

#[test]
fn committed_unmodified_in_project_bytes_are_accepted() {
    let repo = git_repo_with_file("docs/notes.md", "committed evidence\n");

    assert_eq!(
        validate_citation(repo.path(), "docs/notes.md", &repo.head()),
        Ok(())
    );
}

#[test]
fn dirty_working_tree_bytes_are_rejected() {
    let repo = git_repo_with_file("docs/notes.md", "committed evidence\n");
    std::fs::write(repo.path().join("docs/notes.md"), "edited after commit\n")
        .expect("dirty the file");

    assert_eq!(
        validate_citation(repo.path(), "docs/notes.md", &repo.head()),
        Err(CitationRejection::DirtyBytes)
    );
}

#[test]
fn untracked_and_missing_paths_are_rejected() {
    let repo = git_repo_with_file("docs/notes.md", "committed evidence\n");
    std::fs::write(repo.path().join("docs/scratch.md"), "never committed\n")
        .expect("write untracked");

    assert_eq!(
        validate_citation(repo.path(), "docs/scratch.md", &repo.head()),
        Err(CitationRejection::NotCommitted)
    );
    assert_eq!(
        validate_citation(repo.path(), "docs/absent.md", &repo.head()),
        Err(CitationRejection::Missing)
    );
}

#[test]
fn external_and_traversing_paths_are_rejected_before_any_git_call() {
    let repo = git_repo_with_file("docs/notes.md", "committed evidence\n");
    let head = repo.head();

    for path in ["/etc/hostname", "../outside.md", "docs/../../escape.md", ""] {
        assert_eq!(
            validate_citation(repo.path(), path, &head),
            Err(CitationRejection::OutsideProject),
            "path must be refused as non-project-local: {path}"
        );
    }
}

#[test]
fn fabricated_and_mismatched_commits_are_rejected() {
    let repo = git_repo_with_file("docs/notes.md", "committed evidence\n");

    // A well-formed but nonexistent object id.
    assert_eq!(
        validate_citation(
            repo.path(),
            "docs/notes.md",
            "0000000000000000000000000000000000000000"
        ),
        Err(CitationRejection::UnknownCommit)
    );
    assert_eq!(
        validate_citation(repo.path(), "docs/notes.md", "not-a-commit"),
        Err(CitationRejection::UnknownCommit)
    );
    assert_eq!(
        validate_citation(repo.path(), "docs/notes.md", ""),
        Err(CitationRejection::UnknownCommit)
    );
}

#[test]
fn a_file_added_after_the_declared_commit_is_not_citable_at_that_commit() {
    let repo = git_repo_with_file("docs/notes.md", "committed evidence\n");
    let first_commit = repo.head();
    repo.commit_file("docs/later.md", "added in a later commit\n");

    // Present in the working tree and committed, but not at the declared commit.
    assert_eq!(
        validate_citation(repo.path(), "docs/later.md", &first_commit),
        Err(CitationRejection::NotCommitted)
    );
    assert_eq!(
        validate_citation(repo.path(), "docs/later.md", &repo.head()),
        Ok(())
    );
}

struct TestRepo {
    dir: TempDir,
}

impl TestRepo {
    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn head(&self) -> String {
        let output = self.git(&["rev-parse", "HEAD"]);
        String::from_utf8(output)
            .expect("utf8 head")
            .trim()
            .to_string()
    }

    fn commit_file(&self, relative_path: &str, contents: &str) {
        let target: PathBuf = self.path().join(relative_path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        std::fs::write(&target, contents).expect("write file");
        self.git(&["add", relative_path]);
        self.git(&["commit", "-m", "add evidence"]);
    }

    fn git(&self, args: &[&str]) -> Vec<u8> {
        let output = Command::new("git")
            .args(args)
            .current_dir(self.path())
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .output()
            .expect("run git");
        assert!(output.status.success(), "git {args:?} failed");
        output.stdout
    }
}

fn git_repo_with_file(relative_path: &str, contents: &str) -> TestRepo {
    let repo = TestRepo {
        dir: TempDir::new().expect("temp dir"),
    };
    repo.git(&["init", "--quiet"]);
    repo.commit_file(relative_path, contents);
    repo
}

/// Counts distinct package topic words present in the prompt, mirroring how
/// `score_record` intersects package words with user words.
fn overlap(package_words: &[String], prompt: &str) -> usize {
    let prompt_words = candidate_topic_words(prompt);
    package_words
        .iter()
        .filter(|word| prompt_words.contains(word))
        .count()
}

// Backlog 68: a symlink that lives inside the project but resolves outside it
// is refused, because both sides are canonicalized before the prefix check.
#[cfg(unix)]
#[test]
fn citation_of_symlink_inside_project_pointing_outside_is_rejected() {
    let outside = TempDir::new().expect("outside dir");
    std::fs::write(outside.path().join("secret.md"), "outside evidence\n").expect("outside file");
    let repo = git_repo_with_file("docs/evidence.md", "committed evidence\n");
    std::os::unix::fs::symlink(
        outside.path().join("secret.md"),
        repo.path().join("docs/linked.md"),
    )
    .expect("symlink");
    repo.git(&["add", "--all"]);
    repo.git(&["commit", "-m", "add symlink"]);

    assert_eq!(
        resolve_inside_project(repo.path(), "docs/linked.md"),
        None,
        "a symlink escaping the project root is not project-local evidence"
    );
}

// Backlog 69: prose made only of stop words and short tokens never reaches the
// distinct-word floor, no matter how long it is.
#[test]
fn topic_discrimination_rejects_all_stop_word_prose_at_any_length() {
    let stop_prose = "the and for from with that this into must should there their then \
                      the and for from with that this into must should there their then \
                      the and for from with that this into must should there their then";

    assert_eq!(candidate_topic_words(stop_prose), Vec::<String>::new());
    assert_eq!(
        validate_topic_discrimination(stop_prose),
        Err(TopicRejection::TooGeneric { distinct_words: 0 })
    );
}

// Backlog 71: a rejected activation leaves no module directory behind in the
// live store's module cache.
#[test]
fn activation_failure_after_lock_leaves_no_orphan_module_dir() {
    let fixture = CandidateFixture::new("codebase-graph/orphan-check");
    let mut candidate = fixture.candidate();
    candidate.citation_path = "docs/never-committed.md";

    activate_agent_candidate(
        &fixture.store_root(),
        fixture.repo.path(),
        "thread-1",
        &candidate,
    )
    .expect_err("uncitable candidate is refused");

    let modules_root = fixture.store_root().join("modules");
    let mut leftovers = Vec::new();
    if modules_root.exists() {
        collect_files(&modules_root, &mut leftovers);
    }
    assert!(
        leftovers.is_empty(),
        "a refused activation must leave no cached module files: {leftovers:?}"
    );
}

/// Builds a project repo whose store lives at `<root>/.ontocode/ontograph/km/_store`
/// and whose candidate package plus committed citation are ready to activate.
struct CandidateFixture {
    repo: TestRepo,
    package: PathBuf,
    commit: String,
}

impl CandidateFixture {
    fn new(package_id: &str) -> Self {
        let repo = git_repo_with_file("docs/evidence.md", "committed evidence\n");
        let package = write_candidate_package(repo.path(), package_id);
        let commit = repo.head();
        Self {
            repo,
            package,
            commit,
        }
    }

    /// Writes an eval suite into the project tree.
    ///
    /// The staging gate no longer reads this directory; it remains here so the
    /// tests can prove an agent-written suite grants no authority.
    fn with_agent_written_eval_suite(self) -> Self {
        let suite = self.repo.path().join(".ontocode/ontograph/km/_eval/suite");
        std::fs::create_dir_all(&suite).expect("eval suite dir");
        std::fs::write(
            suite.join("case-1.json"),
            "{\"id\":\"case-1\",\"prompt\":\"ontograph km_store retrieval\",\"requiredSubstrings\":[\"ontograph\"]}",
        )
        .expect("eval cases");
        self
    }

    fn store_root(&self) -> PathBuf {
        self.repo.path().join(".ontocode/ontograph/km/_store")
    }

    fn store(&self) -> KmStore {
        KmStore::new(self.store_root())
    }

    fn candidate(&self) -> AgentCandidate<'_> {
        AgentCandidate {
            source_path: &self.package,
            citation_path: "docs/evidence.md",
            source_commit: &self.commit,
            topic_text: KM_TOPIC_FIXTURE_V1.package_summary,
        }
    }

    fn activate(&self, thread_id: &str) -> Result<KmImportResult, ActivationRejection> {
        activate_agent_candidate(
            &self.store_root(),
            self.repo.path(),
            thread_id,
            &self.candidate(),
        )
    }

    /// Live-store bytes, used to prove a rejection changed nothing.
    fn store_bytes(&self) -> Vec<(PathBuf, Vec<u8>)> {
        let mut entries = Vec::new();
        collect_files(&self.store_root(), &mut entries);
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        entries
    }
}

fn collect_files(dir: &Path, entries: &mut Vec<(PathBuf, Vec<u8>)>) {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, entries);
        } else if let Ok(bytes) = std::fs::read(&path) {
            entries.push((path, bytes));
        }
    }
}

fn write_candidate_package(root: &Path, id: &str) -> PathBuf {
    let package = root.join(format!("candidate-{}", id.replace('/', "-")));
    std::fs::create_dir_all(package.join("examples")).expect("examples dir");
    std::fs::create_dir_all(package.join("tests")).expect("tests dir");
    std::fs::write(
        package.join("km.toml"),
        format!(
            "id = \"{id}\"\nversion = \"0.1.0\"\nkind = \"codebase-graph\"\nsummary = \"{}\"\nsources = [\"silver:doc\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n",
            KM_TOPIC_FIXTURE_V1.package_summary
        ),
    )
    .expect("manifest");
    std::fs::write(
        package.join("facts.jsonl"),
        "{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"ontograph km_store resolve_active_km bounded retrieval\",\"source\":\"silver:doc\",\"confidence\":0.9}\n",
    )
    .expect("facts");
    std::fs::write(package.join("examples/README.md"), "example").expect("example");
    std::fs::write(package.join("tests/README.md"), "test").expect("test");
    // `silver:doc` resolves against the project root, so the cited file must exist.
    std::fs::write(root.join("doc"), "doc").expect("cited source");
    package
}

#[test]
fn a_rejected_citation_leaves_the_live_store_byte_for_byte_unchanged() {
    let fixture = CandidateFixture::new("codebase-graph/citation");
    let before = fixture.store_bytes();

    let mut candidate = fixture.candidate();
    candidate.citation_path = "docs/never-committed.md";
    let rejection = activate_agent_candidate(
        &fixture.store_root(),
        fixture.repo.path(),
        "thread-1",
        &candidate,
    )
    .expect_err("citation must be refused");

    assert_eq!(
        rejection,
        ActivationRejection::Citation(CitationRejection::Missing)
    );
    assert_eq!(fixture.store_bytes(), before);
}

#[test]
fn a_rejected_topic_leaves_the_live_store_byte_for_byte_unchanged() {
    let fixture = CandidateFixture::new("codebase-graph/topic");
    let before = fixture.store_bytes();

    let mut candidate = fixture.candidate();
    candidate.topic_text = "alpha beta gamma delta epsilon";
    let rejection = activate_agent_candidate(
        &fixture.store_root(),
        fixture.repo.path(),
        "thread-1",
        &candidate,
    )
    .expect_err("generic topic must be refused");

    assert_eq!(
        rejection,
        ActivationRejection::Topic(TopicRejection::TooGeneric { distinct_words: 5 })
    );
    assert_eq!(fixture.store_bytes(), before);
}

#[test]
fn activation_does_not_depend_on_an_agent_writable_eval_suite() {
    // The project's `_eval/suite` directory is writable by the authoring agent,
    // so it must not decide whether that agent's own candidate activates. A
    // sound candidate activates without one.
    let fixture = CandidateFixture::new("codebase-graph/no-eval");

    fixture
        .activate("thread-1")
        .expect("a sound candidate activates without a project eval suite");
}

#[test]
fn an_agent_written_eval_suite_cannot_admit_an_unsound_candidate() {
    // A candidate carrying no facts renders no retrievable knowledge. Supplying
    // a trivially passing suite must not change that outcome.
    let fixture = CandidateFixture::new("codebase-graph/no-facts").with_agent_written_eval_suite();
    std::fs::write(fixture.package.join("facts.jsonl"), "").expect("empty facts");
    let before = fixture.store_bytes();

    let rejection = fixture
        .activate("thread-1")
        .expect_err("a factless candidate must be refused");

    assert!(matches!(rejection, ActivationRejection::Staging(_)));
    assert_eq!(fixture.store_bytes(), before);
}

#[test]
fn the_thread_ceiling_rejects_before_any_live_store_mutation() {
    let fixture = CandidateFixture::new("codebase-graph/ceiling");
    let store = fixture.store();

    // Seed exactly the frozen budget for this thread, including one withdrawn
    // record, proving withdrawal does not free budget.
    for index in 0..MAX_RECORDS_PER_AUTHORING_THREAD {
        let package = write_candidate_package(
            fixture.repo.path(),
            &format!("codebase-graph/seeded-{index}"),
        );
        store
            .import_for_project_with_origin(
                &package,
                &fixture.repo.path().display().to_string(),
                KmIndexOrigin::AgentThread {
                    thread_id: "thread-1".to_string(),
                },
            )
            .expect("seed record");
    }
    store
        .withdraw(
            "codebase-graph/seeded-0",
            "0.1.0",
            "policy violation".to_string(),
        )
        .expect("withdraw succeeds")
        .expect("withdrawn record");

    let before = fixture.store_bytes();
    let rejection = fixture
        .activate("thread-1")
        .expect_err("ceiling must reject");

    assert_eq!(
        rejection,
        ActivationRejection::ThreadCeiling {
            ceiling: MAX_RECORDS_PER_AUTHORING_THREAD,
            current: MAX_RECORDS_PER_AUTHORING_THREAD,
        }
    );
    assert_eq!(fixture.store_bytes(), before);

    // A different thread still has its own budget.
    assert!(!matches!(
        fixture.activate("thread-2"),
        Err(ActivationRejection::ThreadCeiling { .. })
    ));
}

#[test]
fn an_empty_eval_suite_never_counts_as_a_pass() {
    let empty = KmEvalResult {
        total_cases: 0,
        passed_cases: 0,
        failed_cases: 0,
        ..sample_eval_result()
    };
    assert!(!candidate_eval_passed(&empty));

    let all_passed = KmEvalResult {
        total_cases: 2,
        passed_cases: 2,
        failed_cases: 0,
        ..sample_eval_result()
    };
    assert!(candidate_eval_passed(&all_passed));

    let one_failed = KmEvalResult {
        total_cases: 2,
        passed_cases: 1,
        failed_cases: 1,
        ..sample_eval_result()
    };
    assert!(!candidate_eval_passed(&one_failed));
}

#[test]
fn a_successful_activation_persists_typed_agent_origin_and_consumes_budget() {
    let fixture = CandidateFixture::new("codebase-graph/success-0").with_agent_written_eval_suite();

    let activated = fixture.activate("thread-1").expect("activation succeeds");
    assert_eq!(activated.id, "codebase-graph/success-0");

    // The production writer must persist the typed origin, or the ceiling that
    // filters agent_thread records could never see this activation.
    let stored = fixture
        .store()
        .list_for_origin(
            /*include_obsolete*/ true,
            Some(&KmIndexOrigin::AgentThread {
                thread_id: "thread-1".to_string(),
            }),
            Some("thread-1"),
        )
        .expect("list succeeds");
    assert_eq!(stored.len(), 1);
    assert_eq!(
        stored[0].origin,
        Some(KmIndexOrigin::AgentThread {
            thread_id: "thread-1".to_string()
        })
    );
}

#[test]
fn five_real_activations_exhaust_the_budget_and_the_sixth_is_rejected() {
    let fixture = CandidateFixture::new("codebase-graph/budget-0").with_agent_written_eval_suite();

    // Drive the real production path, not direct typed seeding, so a writer
    // that loses the origin would leave budget free and fail this test.
    for index in 0..MAX_RECORDS_PER_AUTHORING_THREAD {
        let package = write_candidate_package(
            fixture.repo.path(),
            &format!("codebase-graph/budget-{index}"),
        );
        let candidate = AgentCandidate {
            source_path: &package,
            citation_path: "docs/evidence.md",
            source_commit: &fixture.commit,
            topic_text: KM_TOPIC_FIXTURE_V1.package_summary,
        };
        activate_agent_candidate(
            &fixture.store_root(),
            fixture.repo.path(),
            "thread-1",
            &candidate,
        )
        .unwrap_or_else(|error| panic!("activation {index} must succeed: {error:?}"));
    }

    let before = fixture.store_bytes();
    let sixth = write_candidate_package(fixture.repo.path(), "codebase-graph/budget-sixth");
    let candidate = AgentCandidate {
        source_path: &sixth,
        citation_path: "docs/evidence.md",
        source_commit: &fixture.commit,
        topic_text: KM_TOPIC_FIXTURE_V1.package_summary,
    };
    let rejection = activate_agent_candidate(
        &fixture.store_root(),
        fixture.repo.path(),
        "thread-1",
        &candidate,
    )
    .expect_err("sixth activation must be refused");

    assert_eq!(
        rejection,
        ActivationRejection::ThreadCeiling {
            ceiling: MAX_RECORDS_PER_AUTHORING_THREAD,
            current: MAX_RECORDS_PER_AUTHORING_THREAD,
        }
    );
    assert_eq!(fixture.store_bytes(), before);
}

fn sample_eval_result() -> KmEvalResult {
    build_sample_eval_result()
}

#[test]
fn a_symlinked_package_outside_the_project_is_refused() {
    // A relative, traversal-free path can still resolve outside the project via
    // a symlink, which would import bytes the project never committed.
    let outside = TempDir::new().expect("outside dir");
    let external = write_candidate_package(outside.path(), "codebase-graph/external");

    let fixture = CandidateFixture::new("codebase-graph/symlink");
    std::os::unix::fs::symlink(&external, fixture.repo.path().join("escape"))
        .expect("symlink into the project");

    assert_eq!(
        resolve_inside_project(fixture.repo.path(), "escape"),
        None,
        "a symlink resolving outside the project must not be accepted"
    );
}

#[test]
fn concurrent_activations_cannot_exceed_the_thread_ceiling() {
    // Counting outside the store lock lets two authorings both observe a count
    // below the ceiling and both commit, so the ceiling must hold under races.
    let repo = git_repo_with_file("docs/evidence.md", "committed evidence\n");
    let commit = repo.head();
    let store_root = repo.path().join(".ontocode/ontograph/km/_store");

    let packages = (0..MAX_RECORDS_PER_AUTHORING_THREAD + 2)
        .map(|index| write_candidate_package(repo.path(), &format!("codebase-graph/race-{index}")))
        .collect::<Vec<_>>();

    let barrier = std::sync::Arc::new(std::sync::Barrier::new(packages.len()));
    // Spawn every thread before the barrier releases them.
    #[allow(clippy::needless_collect)]
    let handles = packages
        .into_iter()
        .map(|package| {
            let barrier = barrier.clone();
            let store_root = store_root.clone();
            let project_root = repo.path().to_path_buf();
            let commit = commit.clone();
            std::thread::spawn(move || {
                barrier.wait();
                activate_agent_candidate(
                    &store_root,
                    &project_root,
                    "thread-1",
                    &AgentCandidate {
                        source_path: &package,
                        citation_path: "docs/evidence.md",
                        source_commit: &commit,
                        topic_text: KM_TOPIC_FIXTURE_V1.package_summary,
                    },
                )
                .is_ok()
            })
        })
        .collect::<Vec<_>>();
    let activated = handles
        .into_iter()
        .map(|handle| handle.join().expect("authoring thread joins"))
        .filter(|activated| *activated)
        .count();

    let persisted = KmStore::new(&store_root)
        .list_for_origin(
            /*include_obsolete*/ true,
            Some(&KmIndexOrigin::AgentThread {
                thread_id: "thread-1".to_string(),
            }),
            Some("thread-1"),
        )
        .expect("index reads")
        .len();

    assert_eq!(activated, MAX_RECORDS_PER_AUTHORING_THREAD);
    assert_eq!(persisted, MAX_RECORDS_PER_AUTHORING_THREAD);
}

fn build_sample_eval_result() -> KmEvalResult {
    KmEvalResult {
        id: "codebase-graph/sample".to_string(),
        total_cases: 0,
        passed_cases: 0,
        failed_cases: 0,
        pass_rate: 0.0,
        report_path: String::new(),
    }
}

#[test]
fn a_failed_candidate_leaves_a_previously_active_version_untouched() {
    let fixture =
        CandidateFixture::new("codebase-graph/prior-active").with_agent_written_eval_suite();
    fixture.activate("thread-1").expect("first activation");

    let before = fixture.store_bytes();
    let active_before = fixture
        .store()
        .list_for_origin(/*include_obsolete*/ false, None, None)
        .expect("list succeeds");
    assert_eq!(active_before.len(), 1);

    // A second candidate that cannot be cited must not disturb the active row.
    let mut candidate = fixture.candidate();
    candidate.citation_path = "docs/never-committed.md";
    let rejection = activate_agent_candidate(
        &fixture.store_root(),
        fixture.repo.path(),
        "thread-1",
        &candidate,
    )
    .expect_err("uncitable candidate must be refused");
    assert_eq!(
        rejection,
        ActivationRejection::Citation(CitationRejection::Missing)
    );

    assert_eq!(fixture.store_bytes(), before);
    assert_eq!(
        fixture
            .store()
            .list_for_origin(/*include_obsolete*/ false, None, None)
            .expect("list succeeds"),
        active_before
    );
}
