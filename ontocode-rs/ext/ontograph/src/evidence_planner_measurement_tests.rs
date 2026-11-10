use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::Barrier;
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;

use super::*;

const WARMUPS_PER_WORKLOAD: usize = 20;
const SAMPLES_PER_CATEGORY: usize = 50;
const MEASUREMENT_RUNS: usize = 3;
const COLD_SAMPLES_PER_CATEGORY: usize = 30;
const INDEX_UPDATE_SAMPLES: usize = 20;
const WORKLOAD_ORDER_SEED: u64 = 0x4752_4150_4852_4147;
const MAX_REPORT_BYTES: usize = 16 * 1024 * 1024;
const COLD_MARKER: &str = "EVIDENCE_PLANNER_COLD_SAMPLE=";
const COLD_TEST_NAME: &str =
    "evidence_planner::measurement_tests::evidence_planner_measurement_cold_worker";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Category {
    Lexical,
    VocabularyMismatch,
    Traversal,
    KnowledgeModule,
    WrongProject,
}

impl Category {
    const ALL: [Self; 5] = [
        Self::Lexical,
        Self::VocabularyMismatch,
        Self::Traversal,
        Self::KnowledgeModule,
        Self::WrongProject,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Lexical => "lexical",
            Self::VocabularyMismatch => "vocabulary_mismatch",
            Self::Traversal => "traversal",
            Self::KnowledgeModule => "knowledge_module",
            Self::WrongProject => "wrong_project",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Workload {
    id: String,
    category: Category,
    anchor: String,
    shape: RetrievalShape,
    direction: TraversalDirection,
    max_hops: usize,
}

#[derive(Debug, Clone)]
struct RootHandle {
    id: String,
    path: PathBuf,
    thread_state: Arc<crate::tool::OntographThreadState>,
    generation_id: String,
    symbol_count: usize,
    edge_count: usize,
}

#[derive(Debug, Clone)]
struct Job {
    sequence: usize,
    workload: Workload,
    root_index: usize,
    expected_scope: String,
    stream: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RequestSample {
    sequence: usize,
    run: usize,
    concurrency: usize,
    stream: String,
    workload_id: String,
    category: Category,
    root_id: String,
    elapsed_us: u64,
    outcome: String,
    record_count: usize,
    serialized_record_bytes: usize,
    truncated: bool,
    termination: Option<String>,
    digest: String,
    timeout: bool,
    rejection: bool,
    supported_ids: Vec<String>,
    violations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CorpusFile {
    path: String,
    bytes: u64,
    sha256: String,
}

#[derive(Debug, Clone, Serialize)]
struct CorpusIdentity {
    repository_head: String,
    root_a_manifest_sha256: String,
    root_b_manifest_sha256: String,
    file_count: usize,
    bytes: u64,
    files: Vec<CorpusFile>,
    knowledge_module_sha256: String,
    toolchain: String,
    build_profile: String,
    cpu_count: usize,
    available_memory_bytes: Option<u64>,
    root_a_generation: String,
    root_b_generation: String,
    root_a_symbols: usize,
    root_b_symbols: usize,
    root_a_edges: usize,
    root_b_edges: usize,
}

#[derive(Debug, Clone, Serialize)]
struct FrozenIdentity {
    workload_id: String,
    root_id: String,
    expected_scope: String,
    supported_ids: Vec<String>,
    canonical_digest: String,
}

#[derive(Debug, Clone, Serialize)]
struct LatencySummary {
    stream: String,
    concurrency: usize,
    run: usize,
    samples: usize,
    p50_us: u64,
    p95_us: u64,
    p99_us: u64,
    max_us: u64,
    throughput_per_second: f64,
    timeout_count: usize,
    rejection_count: usize,
    deterministic_mismatch_count: usize,
    unexpected_error_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ColdSample {
    category: Category,
    workload_id: String,
    root_id: String,
    process_us: u64,
    process_peak_rss_bytes: Option<u64>,
    request: RequestSample,
}

#[derive(Debug, Clone, Serialize)]
struct ColdSummary {
    category: Category,
    samples: usize,
    p50_us: u64,
    p95_us: u64,
    p99_us: u64,
    max_us: u64,
    timeout_count: usize,
    rejection_count: usize,
    deterministic_mismatch_count: usize,
    unexpected_error_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct IndexUpdateSample {
    iteration: usize,
    old_generation: String,
    new_generation: String,
    current_pointer: String,
    source_version: String,
    indexing_us: u64,
    first_query_us: u64,
    old_reader_results: usize,
    new_reader_results: usize,
    evidence_free_rejections: usize,
    mixed_generation_results: usize,
    violations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct StressObservation {
    name: String,
    outcome: String,
    record_count: usize,
    truncated: bool,
    termination: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct MemoryObservation {
    point: String,
    rss_bytes: Option<u64>,
    peak_rss_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
struct MeasurementReport {
    protocol: String,
    measurement_command: String,
    cold_command_template: String,
    workload_order_seed: u64,
    corpus: CorpusIdentity,
    frozen_workloads: Vec<FrozenIdentity>,
    warm_samples: Vec<RequestSample>,
    warm_summaries: Vec<LatencySummary>,
    cold_samples: Vec<ColdSample>,
    cold_summaries: Vec<ColdSummary>,
    stress: Vec<StressObservation>,
    index_updates: Vec<IndexUpdateSample>,
    index_update_p95_us: u64,
    memory_sampling_interval: String,
    memory: Vec<MemoryObservation>,
    baseline_rss_bytes: Option<u64>,
    peak_rss_bytes: Option<u64>,
    artifact_bytes: u64,
    correctness_violations: Vec<String>,
    hold_reasons: Vec<String>,
    omitted_samples_due_to_cap: usize,
}

#[test]
fn evidence_planner_measurement() {
    // The full protocol issues ~11,590 requests and cannot finish inside an
    // ordinary test slot, so it is opt-in through the same environment gate the
    // cold worker below already uses. Sample counts and thresholds are
    // unchanged; only the trigger differs.
    if std::env::var("ONTOCODE_MEASUREMENT_RUN").as_deref() != Ok("1") {
        return;
    }
    let repository_root = repository_root();
    let measurement_dir = tempfile::tempdir().expect("create measurement directory");
    let root_a_path = measurement_dir.path().join("root-a");
    let root_b_path = measurement_dir.path().join("root-b");
    let km_path = measurement_dir.path().join("measurement-km");

    let baseline_rss = proc_status_bytes("VmRSS");
    let mut memory = vec![memory_observation("baseline")];

    let root_a_files =
        copy_frozen_corpus(&repository_root, &root_a_path).expect("copy root-a corpus");
    let root_b_files =
        copy_frozen_corpus(&repository_root, &root_b_path).expect("copy root-b corpus");
    assert_eq!(
        corpus_manifest_hash(&root_a_files),
        corpus_manifest_hash(&root_b_files),
        "independent corpus roots must contain identical bytes"
    );
    create_knowledge_module_fixture(&km_path).expect("create repository-owned KM fixture");

    let root_a = prepare_root("root-a", root_a_path, &km_path);
    let root_b = prepare_root("root-b", root_b_path, &km_path);
    let roots = [root_a, root_b];
    memory.push(memory_observation("after_setup"));

    let workloads = workloads();
    let (frozen_workloads, expected_digests, mut correctness_violations) =
        freeze_workloads(&roots, &workloads);

    for workload in &workloads {
        for warmup in 0..WARMUPS_PER_WORKLOAD {
            let root_index = root_index_for(workload.category, warmup);
            let expected_scope = expected_scope(&roots, workload.category, root_index);
            let sample = execute_request(
                &roots[root_index],
                workload,
                &expected_scope,
                0,
                0,
                warmup,
                "warmup",
                None,
            );
            correctness_violations.extend(
                sample
                    .violations
                    .iter()
                    .map(|violation| format!("warmup:{}:{violation}", workload.id)),
            );
        }
    }

    let mut warm_samples = Vec::new();
    let mut warm_summaries = Vec::new();
    for concurrency in [1, 2, 4] {
        for run in 1..=MEASUREMENT_RUNS {
            for category in Category::ALL {
                let jobs = category_jobs(&roots, &workloads, category, concurrency, run, false);
                let (mut samples, wall_us) =
                    execute_jobs(&roots, jobs, concurrency, run, &expected_digests);
                let stream = format!("category:{}", category.name());
                warm_summaries.push(summarize_warm(&stream, concurrency, run, wall_us, &samples));
                warm_samples.append(&mut samples);
            }

            let jobs = mixed_jobs(&roots, &workloads, concurrency, run);
            let (mut samples, wall_us) =
                execute_jobs(&roots, jobs, concurrency, run, &expected_digests);
            warm_summaries.push(summarize_warm("mixed", concurrency, run, wall_us, &samples));
            warm_samples.append(&mut samples);
        }
    }
    memory.push(memory_observation("after_warm_measurements"));

    let (cold_samples, cold_summaries) = collect_cold_samples(
        measurement_dir.path(),
        &roots,
        &workloads,
        &expected_digests,
    );
    memory.push(memory_observation("after_cold_measurements"));

    let stress = run_stress_observations(&roots[0]);
    let index_updates = run_index_updates(&roots[0]);
    memory.push(memory_observation("after_index_updates"));

    for sample in &warm_samples {
        correctness_violations.extend(sample.violations.iter().map(|violation| {
            format!(
                "warm:{}:{}:{}:{violation}",
                sample.stream, sample.run, sample.workload_id
            )
        }));
    }
    for sample in &cold_samples {
        correctness_violations.extend(sample.request.violations.iter().map(|violation| {
            format!(
                "cold:{}:{}:{violation}",
                sample.category.name(),
                sample.workload_id
            )
        }));
    }
    for sample in &index_updates {
        correctness_violations.extend(
            sample
                .violations
                .iter()
                .map(|violation| format!("index_update:{}:{violation}", sample.iteration)),
        );
    }
    correctness_violations.sort();
    correctness_violations.dedup();

    let index_update_p95_us = nearest_rank(
        index_updates
            .iter()
            .map(|sample| sample.indexing_us)
            .collect(),
        95,
    );
    let peak_rss = proc_status_bytes("VmHWM");
    let artifact_bytes = roots
        .iter()
        .map(|root| directory_bytes(&root.path.join(".ontocode/ontograph")))
        .sum();

    let corpus = CorpusIdentity {
        repository_head: git_head(&repository_root),
        root_a_manifest_sha256: corpus_manifest_hash(&root_a_files),
        root_b_manifest_sha256: corpus_manifest_hash(&root_b_files),
        file_count: root_a_files.len(),
        bytes: root_a_files.iter().map(|file| file.bytes).sum(),
        files: root_a_files,
        knowledge_module_sha256: directory_manifest_hash(&km_path),
        toolchain: command_output("rustc", &["--version"]),
        build_profile: option_env!("PROFILE").unwrap_or("test").to_string(),
        cpu_count: std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1),
        available_memory_bytes: mem_available_bytes(),
        root_a_generation: roots[0].generation_id.clone(),
        root_b_generation: roots[1].generation_id.clone(),
        root_a_symbols: roots[0].symbol_count,
        root_b_symbols: roots[1].symbol_count,
        root_a_edges: roots[0].edge_count,
        root_b_edges: roots[1].edge_count,
    };

    let mut hold_reasons = threshold_holds(
        &warm_summaries,
        &cold_summaries,
        index_update_p95_us,
        peak_rss,
        &correctness_violations,
    );
    let mut report = MeasurementReport {
        protocol: "Graph-RAG Real-Provider Measurement Protocol".to_string(),
        measurement_command: "cd ontocode-rs && ONTOCODE_MEASUREMENT_RUN=1 CARGO_BUILD_JOBS=4 cargo nextest run -p ontocode-ontograph-extension --lib evidence_planner_measurement --no-capture".to_string(),
        cold_command_template: format!(
            "/usr/bin/time -v <current-test-binary> --exact {COLD_TEST_NAME} --nocapture"
        ),
        workload_order_seed: WORKLOAD_ORDER_SEED,
        corpus,
        frozen_workloads,
        warm_samples,
        warm_summaries,
        cold_samples,
        cold_summaries,
        stress,
        index_updates,
        index_update_p95_us,
        memory_sampling_interval: "native /proc status sampled after setup and each experiment section; child peak RSS sampled by /usr/bin/time -v".to_string(),
        memory,
        baseline_rss_bytes: baseline_rss,
        peak_rss_bytes: peak_rss,
        artifact_bytes,
        correctness_violations,
        hold_reasons: Vec::new(),
        omitted_samples_due_to_cap: 0,
    };
    report.hold_reasons.append(&mut hold_reasons);
    enforce_report_cap(&mut report);

    let serialized = serde_json::to_string(&report).expect("serialize measurement report");
    println!("EVIDENCE_PLANNER_MEASUREMENT={serialized}");
    assert!(
        report.hold_reasons.is_empty(),
        "measurement HOLD: {}",
        report.hold_reasons.join("; ")
    );
}

#[test]
fn evidence_planner_measurement_cold_worker() {
    if std::env::var("ONTOCODE_MEASUREMENT_COLD").as_deref() != Ok("1") {
        return;
    }
    let root_path =
        PathBuf::from(std::env::var("ONTOCODE_MEASUREMENT_ROOT").expect("cold worker root"));
    let root_id = std::env::var("ONTOCODE_MEASUREMENT_ROOT_ID").expect("cold worker root id");
    let expected_scope =
        std::env::var("ONTOCODE_MEASUREMENT_EXPECTED_SCOPE").expect("cold worker scope");
    let workload = serde_json::from_str::<Workload>(
        &std::env::var("ONTOCODE_MEASUREMENT_WORKLOAD").expect("cold worker workload"),
    )
    .expect("deserialize cold workload");

    let thread_state = Arc::new(crate::tool::OntographThreadState::default());
    thread_state.set_current_cwd(root_path.clone());
    let root = RootHandle {
        id: root_id,
        path: root_path,
        thread_state,
        generation_id: String::new(),
        symbol_count: 0,
        edge_count: 0,
    };
    let sample = execute_request(&root, &workload, &expected_scope, 0, 1, 0, "cold", None);
    println!(
        "{COLD_MARKER}{}",
        serde_json::to_string(&sample).expect("serialize cold sample")
    );
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repository root")
        .to_path_buf()
}

fn copy_frozen_corpus(repository_root: &Path, destination: &Path) -> io::Result<Vec<CorpusFile>> {
    fs::create_dir_all(destination.join(".git"))?;
    let roots = [
        (
            repository_root.join("ontocode-rs/ext/ontograph/src"),
            PathBuf::from("ontocode-rs/ext/ontograph/src"),
        ),
        (
            repository_root.join(".memory-bank/knowledge-hub"),
            PathBuf::from(".memory-bank/knowledge-hub"),
        ),
    ];
    let mut files = Vec::new();
    for (source, relative) in roots {
        copy_tree(&source, &destination.join(&relative), &relative, &mut files)?;
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

fn copy_tree(
    source: &Path,
    destination: &Path,
    relative: &Path,
    files: &mut Vec<CorpusFile>,
) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    let mut entries = fs::read_dir(source)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let file_type = entry.file_type()?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let relative_path = relative.join(entry.file_name());
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            copy_tree(&source_path, &destination_path, &relative_path, files)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        fs::copy(&source_path, &destination_path)?;
        let bytes = fs::read(&destination_path)?;
        files.push(CorpusFile {
            path: slash_path(&relative_path),
            bytes: bytes.len() as u64,
            sha256: sha256(&bytes),
        });
    }
    Ok(())
}

fn slash_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn corpus_manifest_hash(files: &[CorpusFile]) -> String {
    let mut manifest = String::new();
    for file in files {
        manifest.push_str(&format!("{}\t{}\t{}\n", file.path, file.bytes, file.sha256));
    }
    sha256(manifest.as_bytes())
}

fn directory_manifest_hash(root: &Path) -> String {
    let mut files = Vec::new();
    collect_directory_files(root, root, &mut files);
    files.sort_by(|left, right| left.path.cmp(&right.path));
    corpus_manifest_hash(&files)
}

fn collect_directory_files(root: &Path, current: &Path, files: &mut Vec<CorpusFile>) {
    let Ok(read_dir) = fs::read_dir(current) else {
        return;
    };
    let mut entries = read_dir.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_directory_files(root, &entry.path(), files);
        } else if file_type.is_file()
            && let Ok(bytes) = fs::read(entry.path())
        {
            files.push(CorpusFile {
                path: slash_path(entry.path().strip_prefix(root).expect("relative file")),
                bytes: bytes.len() as u64,
                sha256: sha256(&bytes),
            });
        }
    }
}

fn create_knowledge_module_fixture(root: &Path) -> io::Result<()> {
    fs::create_dir_all(root.join("examples"))?;
    fs::create_dir_all(root.join("tests"))?;
    fs::write(
        root.join("km.toml"),
        concat!(
            "id = \"codebase-graph/evidence-planner-measurement\"\n",
            "version = \"0.1.0\"\n",
            "kind = \"codebase-graph\"\n",
            "summary = \"Repository-owned measurement fixture for the evidence planner.\"\n",
            "sources = [\"silver:doc\"]\n",
            "licenses = [\"MIT\"]\n",
            "[provenance]\n",
            "created_by = \"evidence-planner-measurement-tests\"\n",
            "generated_at = \"2026-09-16\"\n",
            "source_commit = \"TEST\"\n",
        ),
    )?;
    fs::write(
        root.join("facts.jsonl"),
        "{\"id\":\"measurement-fact\",\"kind\":\"rule\",\"claim\":\"real-provider measurements retain project scope and provenance\",\"source\":\"silver:doc\",\"confidence\":1.0}\n",
    )?;
    let transitions = [
        ("measure_prepare", "measure_query"),
        ("measure_query", "measure_verify"),
        ("measure_verify", "measure_publish"),
        ("measure_publish", "measure_prepare"),
    ]
    .into_iter()
    .map(|(source, target)| {
        serde_json::json!({
            "source": source,
            "target": target,
            "relation": "next",
            "condition": "measurement input is frozen",
            "guidance": "Retain bounded provenance and project scope.",
            "pitfalls": ["Do not pool unrelated percentile populations"]
        })
        .to_string()
    })
    .collect::<Vec<_>>()
    .join("\n");
    fs::write(root.join("transitions.jsonl"), format!("{transitions}\n"))?;
    fs::write(root.join("examples/README.md"), "measurement example\n")?;
    fs::write(root.join("tests/README.md"), "measurement fixture test\n")?;
    Ok(())
}

fn prepare_root(id: &str, path: PathBuf, km_path: &Path) -> RootHandle {
    fs::write(
        path.join("doc"),
        "Repository-owned source for the evidence planner measurement KM.\n",
    )
    .expect("write KM source");
    let store = crate::km_store::KmStore::for_project_path(&path).expect("project KM store");
    store
        .import_for_project(km_path, &path.display().to_string())
        .expect("import measurement KM");

    let (generation_id, artifact) = publish_index(&path);
    let symbol_count = artifact.files.iter().map(|file| file.symbols.len()).sum();
    let edge_count = artifact.relationships.len();
    let thread_state = Arc::new(crate::tool::OntographThreadState::default());
    thread_state.set_current_cwd(path.clone());
    RootHandle {
        id: id.to_string(),
        path,
        thread_state,
        generation_id,
        symbol_count,
        edge_count,
    }
}

fn publish_index(path: &Path) -> (String, crate::indexer::SourceIndexArtifact) {
    let committed_head = crate::indexer::resolve_target_head(path);
    let state_key = crate::indexer::rust_workspace_state_key(path, &committed_head)
        .expect("workspace state key");
    let generation_id = format!("{committed_head}-{state_key}");
    let (artifact, _) = crate::indexer::index_rust_workspace(crate::indexer::RustIndexRequest {
        repo_root: path.to_path_buf(),
        artifact_root: crate::manifest::OntographArtifactRoot::new(path),
        generation_id: generation_id.clone(),
        target_head: generation_id.clone(),
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp_unix_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64,
    })
    .expect("publish measurement index");
    (generation_id, artifact)
}

fn workloads() -> Vec<Workload> {
    let lexical = [
        "PlannerEngine",
        "SourceIndexStore",
        "QueryDeadline",
        "TerminationReason",
    ];
    let mismatched = [
        "quantum scheduler",
        "PlannerEngin",
        "SourceIndexStor",
        "QueryDeadlin",
    ];
    let traversal = [
        ("admit_query", TraversalDirection::Upstream, 1),
        ("search_symbols", TraversalDirection::Downstream, 3),
        (
            "set_timeout_termination",
            TraversalDirection::Bidirectional,
            1,
        ),
        ("project_bounded", TraversalDirection::Bidirectional, 3),
    ];
    let km = [
        "measure_prepare",
        "measure_query",
        "measure_verify",
        "measure_publish",
    ];
    let mut workloads = Vec::new();
    for anchor in lexical {
        workloads.push(workload(
            Category::Lexical,
            anchor,
            RetrievalShape::DirectLookup,
            TraversalDirection::Bidirectional,
            2,
        ));
        workloads.push(workload(
            Category::WrongProject,
            anchor,
            RetrievalShape::DirectLookup,
            TraversalDirection::Bidirectional,
            2,
        ));
    }
    for anchor in mismatched {
        workloads.push(workload(
            Category::VocabularyMismatch,
            anchor,
            RetrievalShape::DirectLookup,
            TraversalDirection::Bidirectional,
            2,
        ));
    }
    for (anchor, direction, max_hops) in traversal {
        workloads.push(workload(
            Category::Traversal,
            anchor,
            RetrievalShape::DependencyPath,
            direction,
            max_hops,
        ));
    }
    for anchor in km {
        workloads.push(workload(
            Category::KnowledgeModule,
            anchor,
            RetrievalShape::CrossArtifactJoin,
            TraversalDirection::Lateral,
            2,
        ));
    }
    workloads.sort_by(|left, right| left.id.cmp(&right.id));
    workloads
}

fn workload(
    category: Category,
    anchor: &str,
    shape: RetrievalShape,
    direction: TraversalDirection,
    max_hops: usize,
) -> Workload {
    Workload {
        id: format!("{}:{anchor}", category.name()),
        category,
        anchor: anchor.to_string(),
        shape,
        direction,
        max_hops,
    }
}

#[allow(clippy::type_complexity)]
fn freeze_workloads(
    roots: &[RootHandle; 2],
    workloads: &[Workload],
) -> (
    Vec<FrozenIdentity>,
    BTreeMap<(String, String), String>,
    Vec<String>,
) {
    let mut frozen = Vec::new();
    let mut digests = BTreeMap::new();
    let mut violations = Vec::new();
    for workload in workloads {
        let root_indexes: &[usize] = if workload.category == Category::WrongProject {
            &[1]
        } else {
            &[0, 1]
        };
        for root_index in root_indexes {
            let expected_scope = expected_scope(roots, workload.category, *root_index);
            let sample = execute_request(
                &roots[*root_index],
                workload,
                &expected_scope,
                0,
                0,
                0,
                "freeze",
                None,
            );
            // A wrong-project request may exhaust its artifact budget before the
            // planner reaches scope rejection. `BudgetExceeded` is the declared
            // bounded-budget signal and sets `truncated`, so for this category it
            // is an expected rejection path rather than a freeze failure. Any
            // other non-ok outcome is still a violation.
            let budget_rejected = workload.category == Category::WrongProject
                && sample.outcome.contains("planner budget exceeded");
            if sample.outcome != "ok" && !budget_rejected {
                violations.push(format!(
                    "failed to freeze {} on {}: {}",
                    workload.id, roots[*root_index].id, sample.outcome
                ));
            }
            if workload.category != Category::VocabularyMismatch
                && workload.category != Category::WrongProject
                && sample.supported_ids.is_empty()
            {
                violations.push(format!(
                    "unresolved frozen anchor {} on {}",
                    workload.id, roots[*root_index].id
                ));
            }
            if workload.category == Category::WrongProject && !sample.supported_ids.is_empty() {
                violations.push(format!(
                    "wrong-project workload {} returned supported evidence",
                    workload.id
                ));
            }
            digests.insert(
                (roots[*root_index].id.clone(), workload.id.clone()),
                sample.digest.clone(),
            );
            frozen.push(FrozenIdentity {
                workload_id: workload.id.clone(),
                root_id: roots[*root_index].id.clone(),
                expected_scope,
                supported_ids: sample.supported_ids,
                canonical_digest: sample.digest,
            });
        }
    }
    (frozen, digests, violations)
}

fn root_index_for(category: Category, sequence: usize) -> usize {
    if category == Category::WrongProject {
        1
    } else {
        sequence % 2
    }
}

fn expected_scope(roots: &[RootHandle; 2], category: Category, root_index: usize) -> String {
    if category == Category::WrongProject {
        roots[0].path.display().to_string()
    } else {
        roots[root_index].path.display().to_string()
    }
}

fn category_jobs(
    roots: &[RootHandle; 2],
    workloads: &[Workload],
    category: Category,
    concurrency: usize,
    run: usize,
    mixed: bool,
) -> Vec<Job> {
    let selected = workloads
        .iter()
        .filter(|workload| workload.category == category)
        .cloned()
        .collect::<Vec<_>>();
    (0..SAMPLES_PER_CATEGORY)
        .map(|sequence| {
            let workload = selected
                [(sequence + run + concurrency + WORKLOAD_ORDER_SEED as usize) % selected.len()]
            .clone();
            let root_index = root_index_for(category, sequence + run);
            Job {
                sequence,
                expected_scope: expected_scope(roots, category, root_index),
                root_index,
                stream: if mixed {
                    "mixed".to_string()
                } else {
                    format!("category:{}", category.name())
                },
                workload,
            }
        })
        .collect()
}

fn mixed_jobs(
    roots: &[RootHandle; 2],
    workloads: &[Workload],
    concurrency: usize,
    run: usize,
) -> Vec<Job> {
    let mut per_category = BTreeMap::<Category, Vec<Workload>>::new();
    for workload in workloads {
        per_category
            .entry(workload.category)
            .or_default()
            .push(workload.clone());
    }
    let mut category_counts = BTreeMap::<Category, usize>::new();
    (0..SAMPLES_PER_CATEGORY)
        .map(|sequence| {
            let slot =
                (sequence * 37 + run * 11 + concurrency * 7 + WORKLOAD_ORDER_SEED as usize) % 100;
            let category = match slot {
                0..=29 => Category::Lexical,
                30..=44 => Category::VocabularyMismatch,
                45..=69 => Category::Traversal,
                70..=89 => Category::KnowledgeModule,
                _ => Category::WrongProject,
            };
            let count = category_counts.entry(category).or_default();
            let candidates = &per_category[&category];
            let workload = candidates[*count % candidates.len()].clone();
            *count += 1;
            let root_index = root_index_for(category, sequence + run);
            Job {
                sequence,
                expected_scope: expected_scope(roots, category, root_index),
                root_index,
                stream: "mixed".to_string(),
                workload,
            }
        })
        .collect()
}

fn execute_jobs(
    roots: &[RootHandle; 2],
    jobs: Vec<Job>,
    concurrency: usize,
    run: usize,
    expected_digests: &BTreeMap<(String, String), String>,
) -> (Vec<RequestSample>, u64) {
    let mut partitions = vec![Vec::new(); concurrency];
    for job in jobs {
        partitions[job.sequence % concurrency].push(job);
    }
    let barrier = Arc::new(Barrier::new(concurrency + 1));
    let (sender, receiver) = mpsc::channel();
    let started = Instant::now();
    std::thread::scope(|scope| {
        for partition in partitions {
            let barrier = Arc::clone(&barrier);
            let sender = sender.clone();
            scope.spawn(move || {
                barrier.wait();
                for job in partition {
                    let expected_digest = expected_digests
                        .get(&(roots[job.root_index].id.clone(), job.workload.id.clone()))
                        .map(String::as_str);
                    let sample = execute_request(
                        &roots[job.root_index],
                        &job.workload,
                        &job.expected_scope,
                        run,
                        concurrency,
                        job.sequence,
                        &job.stream,
                        expected_digest,
                    );
                    sender.send(sample).expect("send measurement sample");
                }
            });
        }
        barrier.wait();
    });
    drop(sender);
    let wall_us = elapsed_us(started);
    let mut samples = receiver.into_iter().collect::<Vec<_>>();
    samples.sort_by_key(|sample| sample.sequence);
    (samples, wall_us)
}

#[allow(clippy::too_many_arguments)]
fn execute_request(
    root: &RootHandle,
    workload: &Workload,
    expected_scope: &str,
    run: usize,
    concurrency: usize,
    sequence: usize,
    stream: &str,
    expected_digest: Option<&str>,
) -> RequestSample {
    let plan = RetrievalPlan {
        shape: workload.shape,
        anchor: workload.anchor.clone(),
        direction: workload.direction,
        relation_types: Vec::new(),
        project_scope: Some(expected_scope.to_string()),
        budget: PlannerBudget {
            max_hops: workload.max_hops,
            max_records: 64,
            max_artifacts: 16,
            max_context_bytes: 32 * 1024,
            timeout_ms: 5_000,
        },
    };
    let provider =
        crate::tool::ThreadStateOperationsProvider::with_timeout(&root.thread_state, 5_000);
    let engine = PlannerEngine::new(&provider);
    let started = Instant::now();
    let result = engine.execute(plan);
    let elapsed_us = elapsed_us(started);
    match result {
        Ok(subgraph) => {
            let serialized = serde_json::to_vec(&subgraph).expect("serialize subgraph");
            let digest = sha256(&serialized);
            let mut violations = validate_subgraph(&subgraph, expected_scope, workload.category);
            if expected_digest.is_some_and(|expected| expected != digest) {
                violations.push(format!(
                    "deterministic result mismatch: expected {expected_digest:?}, found {digest}"
                ));
            }
            let supported_ids = subgraph
                .records
                .iter()
                .filter(|record| matches!(record.support_status, EvidenceSupportStatus::Supported))
                .map(|record| record.id.clone())
                .collect();
            let rejection = subgraph
                .records
                .iter()
                .any(|record| !matches!(record.support_status, EvidenceSupportStatus::Supported));
            RequestSample {
                sequence,
                run,
                concurrency,
                stream: stream.to_string(),
                workload_id: workload.id.clone(),
                category: workload.category,
                root_id: root.id.clone(),
                elapsed_us,
                outcome: "ok".to_string(),
                record_count: subgraph.records.len(),
                serialized_record_bytes: subgraph.total_serialized_bytes(),
                truncated: subgraph.truncated,
                termination: subgraph
                    .termination
                    .as_ref()
                    .map(|termination| serde_json::to_string(termination).expect("termination")),
                digest,
                timeout: subgraph.termination.is_some(),
                rejection,
                supported_ids,
                violations,
            }
        }
        Err(error) => {
            // A wrong-project request can exhaust its artifact budget before the
            // planner reaches scope rejection. That is the declared bounded-budget
            // signal and still returns no foreign evidence, so it counts as a
            // rejection for this category instead of an unexpected error.
            let budget_rejected = workload.category == Category::WrongProject
                && matches!(error, PlannerModelError::BudgetExceeded(_));
            RequestSample {
                sequence,
                run,
                concurrency,
                stream: stream.to_string(),
                workload_id: workload.id.clone(),
                category: workload.category,
                root_id: root.id.clone(),
                elapsed_us,
                outcome: format!("error:{error}"),
                record_count: 0,
                serialized_record_bytes: 0,
                truncated: false,
                termination: None,
                digest: sha256(error.to_string().as_bytes()),
                timeout: matches!(error, PlannerModelError::TimeoutExceeded { .. }),
                rejection: budget_rejected,
                supported_ids: Vec::new(),
                violations: if budget_rejected {
                    Vec::new()
                } else {
                    vec![format!("unexpected planner error: {error}")]
                },
            }
        }
    }
}

fn validate_subgraph(
    subgraph: &EvidenceSubgraph,
    expected_scope: &str,
    category: Category,
) -> Vec<String> {
    let mut violations = Vec::new();
    if subgraph.records.len() > subgraph.plan.budget.max_records {
        violations.push("record budget exceeded".to_string());
    }
    if subgraph.total_serialized_bytes() > subgraph.plan.budget.max_context_bytes {
        violations.push("context byte budget exceeded".to_string());
    }
    let artifacts = subgraph
        .records
        .iter()
        .map(|record| {
            record
                .source_location
                .as_ref()
                .map(|location| location.path.clone())
                .or_else(|| record.version_or_hash.clone())
                .unwrap_or_else(|| record.id.clone())
        })
        .collect::<BTreeSet<_>>();
    if artifacts.len() > subgraph.plan.budget.max_artifacts {
        violations.push("artifact budget exceeded".to_string());
    }
    for record in &subgraph.records {
        if record.project_scope.as_deref() != Some(expected_scope) {
            violations.push(format!(
                "scope violation in {}: expected {}, found {:?}",
                record.id, expected_scope, record.project_scope
            ));
        }
        if matches!(record.support_status, EvidenceSupportStatus::Supported)
            && record.source_location.is_none()
            && record.version_or_hash.is_none()
        {
            violations.push(format!("missing provenance in {}", record.id));
        }
    }
    if category == Category::WrongProject
        && subgraph
            .records
            .iter()
            .any(|record| matches!(record.support_status, EvidenceSupportStatus::Supported))
    {
        violations.push("wrong-project request returned supported foreign evidence".to_string());
    }
    violations
}

fn summarize_warm(
    stream: &str,
    concurrency: usize,
    run: usize,
    wall_us: u64,
    samples: &[RequestSample],
) -> LatencySummary {
    let durations = samples
        .iter()
        .map(|sample| sample.elapsed_us)
        .collect::<Vec<_>>();
    LatencySummary {
        stream: stream.to_string(),
        concurrency,
        run,
        samples: samples.len(),
        p50_us: nearest_rank(durations.clone(), 50),
        p95_us: nearest_rank(durations.clone(), 95),
        p99_us: nearest_rank(durations.clone(), 99),
        max_us: durations.into_iter().max().unwrap_or(0),
        throughput_per_second: if wall_us == 0 {
            0.0
        } else {
            samples.len() as f64 * 1_000_000.0 / wall_us as f64
        },
        timeout_count: samples.iter().filter(|sample| sample.timeout).count(),
        rejection_count: samples.iter().filter(|sample| sample.rejection).count(),
        deterministic_mismatch_count: samples
            .iter()
            .filter(|sample| {
                sample
                    .violations
                    .iter()
                    .any(|violation| violation.contains("deterministic result mismatch"))
            })
            .count(),
        unexpected_error_count: samples
            .iter()
            .filter(|sample| sample.outcome != "ok")
            .count(),
    }
}

fn collect_cold_samples(
    scratch: &Path,
    roots: &[RootHandle; 2],
    workloads: &[Workload],
    expected_digests: &BTreeMap<(String, String), String>,
) -> (Vec<ColdSample>, Vec<ColdSummary>) {
    let current_exe = std::env::current_exe().expect("current test binary");
    let mut samples = Vec::new();
    for category in Category::ALL {
        let candidates = workloads
            .iter()
            .filter(|workload| workload.category == category)
            .collect::<Vec<_>>();
        for sequence in 0..COLD_SAMPLES_PER_CATEGORY {
            let workload = candidates[sequence % candidates.len()];
            let root_index = root_index_for(category, sequence);
            let root = &roots[root_index];
            let expected_scope = expected_scope(roots, category, root_index);
            let time_path = scratch.join(format!("time-{}-{sequence}.txt", category.name()));
            let started = Instant::now();
            let output = Command::new("/usr/bin/time")
                .arg("-v")
                .arg("-o")
                .arg(&time_path)
                .arg(&current_exe)
                .arg("--exact")
                .arg(COLD_TEST_NAME)
                .arg("--nocapture")
                .env("ONTOCODE_MEASUREMENT_COLD", "1")
                .env("ONTOCODE_MEASUREMENT_ROOT", &root.path)
                .env("ONTOCODE_MEASUREMENT_ROOT_ID", &root.id)
                .env("ONTOCODE_MEASUREMENT_EXPECTED_SCOPE", &expected_scope)
                .env(
                    "ONTOCODE_MEASUREMENT_WORKLOAD",
                    serde_json::to_string(workload).expect("serialize cold workload"),
                )
                .output()
                .expect("run cold measurement process");
            let process_us = elapsed_us(started);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut request = stdout
                .lines()
                .find_map(|line| line.strip_prefix(COLD_MARKER))
                .and_then(|json| serde_json::from_str::<RequestSample>(json).ok())
                .unwrap_or_else(|| RequestSample {
                    sequence,
                    run: 0,
                    concurrency: 1,
                    stream: "cold".to_string(),
                    workload_id: workload.id.clone(),
                    category,
                    root_id: root.id.clone(),
                    elapsed_us: 0,
                    outcome: format!(
                        "error:cold worker exit {:?}: {}",
                        output.status.code(),
                        String::from_utf8_lossy(&output.stderr)
                    ),
                    record_count: 0,
                    serialized_record_bytes: 0,
                    truncated: false,
                    termination: None,
                    digest: String::new(),
                    timeout: false,
                    rejection: false,
                    supported_ids: Vec::new(),
                    violations: vec!["cold worker did not emit a sample".to_string()],
                });
            request.sequence = sequence;
            if let Some(expected) = expected_digests.get(&(root.id.clone(), workload.id.clone()))
                && request.digest != *expected
            {
                request.violations.push(format!(
                    "deterministic result mismatch: expected {expected}, found {}",
                    request.digest
                ));
            }
            samples.push(ColdSample {
                category,
                workload_id: workload.id.clone(),
                root_id: root.id.clone(),
                process_us,
                process_peak_rss_bytes: parse_time_peak_rss(&time_path),
                request,
            });
        }
    }
    let summaries = Category::ALL
        .into_iter()
        .map(|category| summarize_cold(category, &samples))
        .collect();
    (samples, summaries)
}

fn summarize_cold(category: Category, samples: &[ColdSample]) -> ColdSummary {
    let selected = samples
        .iter()
        .filter(|sample| sample.category == category)
        .collect::<Vec<_>>();
    let durations = selected
        .iter()
        .map(|sample| sample.process_us)
        .collect::<Vec<_>>();
    ColdSummary {
        category,
        samples: selected.len(),
        p50_us: nearest_rank(durations.clone(), 50),
        p95_us: nearest_rank(durations.clone(), 95),
        p99_us: nearest_rank(durations.clone(), 99),
        max_us: durations.into_iter().max().unwrap_or(0),
        timeout_count: selected
            .iter()
            .filter(|sample| sample.request.timeout)
            .count(),
        rejection_count: selected
            .iter()
            .filter(|sample| sample.request.rejection)
            .count(),
        deterministic_mismatch_count: selected
            .iter()
            .filter(|sample| {
                sample
                    .request
                    .violations
                    .iter()
                    .any(|violation| violation.contains("deterministic result mismatch"))
            })
            .count(),
        unexpected_error_count: selected
            .iter()
            .filter(|sample| sample.request.outcome != "ok")
            .count(),
    }
}

fn run_stress_observations(root: &RootHandle) -> Vec<StressObservation> {
    let provider = crate::tool::ThreadStateOperationsProvider::new(&root.thread_state);
    let zero_impact = provider
        .impact_symbol_bounded(
            // `search_symbols` occurs in several modules, so a bare name is
            // ambiguous across the corpus. Use the same uniquely resolving
            // anchor the one-byte stress workload below already relies on.
            "PlannerEngine",
            TraversalDirection::Bidirectional,
            3,
            0,
        )
        .expect("zero-capacity impact");
    let zero_procedural = provider
        .procedural_project_bounded("measure_prepare", 2, 0)
        .expect("zero-capacity procedural projection");

    let one_byte = Workload {
        id: "stress:one_byte_context".to_string(),
        category: Category::Lexical,
        anchor: "PlannerEngine".to_string(),
        shape: RetrievalShape::DirectLookup,
        direction: TraversalDirection::Bidirectional,
        max_hops: 2,
    };
    let one_byte_plan = RetrievalPlan {
        shape: one_byte.shape,
        anchor: one_byte.anchor,
        direction: one_byte.direction,
        relation_types: Vec::new(),
        project_scope: Some(root.path.display().to_string()),
        budget: PlannerBudget {
            max_hops: 2,
            max_records: 64,
            max_artifacts: 16,
            max_context_bytes: 1,
            timeout_ms: 5_000,
        },
    };
    let one_byte_result = PlannerEngine::new(&provider)
        .execute(one_byte_plan)
        .expect("one-byte stress result");

    let expired_provider =
        crate::tool::ThreadStateOperationsProvider::with_timeout(&root.thread_state, 1);
    std::thread::sleep(Duration::from_millis(2));
    let expired_plan = RetrievalPlan {
        shape: RetrievalShape::DirectLookup,
        anchor: "PlannerEngine".to_string(),
        direction: TraversalDirection::Bidirectional,
        relation_types: Vec::new(),
        project_scope: Some(root.path.display().to_string()),
        budget: PlannerBudget {
            max_hops: 2,
            max_records: 64,
            max_artifacts: 16,
            max_context_bytes: 32 * 1024,
            timeout_ms: 5_000,
        },
    };
    let expired_result = PlannerEngine::new(&expired_provider)
        .execute(expired_plan)
        .expect("expired deadline stress result");

    vec![
        StressObservation {
            name: "zero_remaining_impact_capacity".to_string(),
            outcome: "ok".to_string(),
            record_count: zero_impact.records.len(),
            truncated: zero_impact.truncated,
            termination: None,
        },
        StressObservation {
            name: "zero_remaining_procedural_capacity".to_string(),
            outcome: "ok".to_string(),
            record_count: zero_procedural.records.len(),
            truncated: zero_procedural.truncated,
            termination: None,
        },
        StressObservation {
            name: "one_byte_context".to_string(),
            outcome: "ok".to_string(),
            record_count: one_byte_result.records.len(),
            truncated: one_byte_result.truncated,
            termination: one_byte_result
                .termination
                .map(|termination| serde_json::to_string(&termination).expect("termination")),
        },
        StressObservation {
            name: "exhausted_deadline".to_string(),
            outcome: "ok".to_string(),
            record_count: expired_result.records.len(),
            truncated: expired_result.truncated,
            termination: expired_result
                .termination
                .map(|termination| serde_json::to_string(&termination).expect("termination")),
        },
    ]
}

fn run_index_updates(root: &RootHandle) -> Vec<IndexUpdateSample> {
    let source_path = root
        .path
        .join("ontocode-rs/ext/ontograph/src/evidence_planner.rs");
    let original = fs::read_to_string(&source_path).expect("read update source");
    let mut old_generation = root.generation_id.clone();
    let mut samples = Vec::new();
    for iteration in 1..=INDEX_UPDATE_SAMPLES {
        let source_version = format!("measurement_update_generation_{iteration}");
        fs::write(
            &source_path,
            format!(
                "{original}\n#[allow(dead_code)]\nfn {source_version}() {{}} // {}\n",
                "x".repeat(iteration)
            ),
        )
        .expect("write update source");

        let barrier = Arc::new(Barrier::new(5));
        let (sender, receiver) = mpsc::channel();
        let expected_scope = root.path.display().to_string();
        // Publication must overlap active readers, so the publish call runs
        // inside the scope while the reader threads are still issuing requests.
        let (indexing_us, new_generation) = std::thread::scope(|scope| {
            for _ in 0..4 {
                let barrier = Arc::clone(&barrier);
                let sender = sender.clone();
                let expected_scope = expected_scope.clone();
                scope.spawn(move || {
                    barrier.wait();
                    for sequence in 0..8 {
                        let workload = Workload {
                            id: "index_update_reader".to_string(),
                            category: Category::Lexical,
                            anchor: "measurement_update_generation".to_string(),
                            shape: RetrievalShape::DirectLookup,
                            direction: TraversalDirection::Bidirectional,
                            max_hops: 2,
                        };
                        let sample = execute_request(
                            root,
                            &workload,
                            &expected_scope,
                            0,
                            4,
                            sequence,
                            "index_update_reader",
                            None,
                        );
                        sender.send(sample).expect("send index reader result");
                    }
                });
            }
            barrier.wait();
            let indexing_started = Instant::now();
            let (new_generation, _) = publish_index(&root.path);
            (elapsed_us(indexing_started), new_generation)
        });
        drop(sender);
        let readers = receiver.into_iter().collect::<Vec<_>>();

        let current_pointer = fs::read_to_string(
            crate::manifest::OntographArtifactRoot::new(&root.path).current_pointer_path(),
        )
        .expect("read current generation")
        .trim()
        .to_string();

        let workload = Workload {
            id: "index_update_first_query".to_string(),
            category: Category::Lexical,
            anchor: source_version.clone(),
            shape: RetrievalShape::DirectLookup,
            direction: TraversalDirection::Bidirectional,
            max_hops: 2,
        };
        let first_started = Instant::now();
        let first = execute_request(
            root,
            &workload,
            &expected_scope,
            0,
            1,
            iteration,
            "index_update_first_query",
            None,
        );
        let first_query_us = elapsed_us(first_started);

        let mut old_reader_results = 0;
        let mut new_reader_results = 0;
        let mut evidence_free_rejections = 0;
        let mut mixed_generation_results = 0;
        for reader in &readers {
            let generations = reader
                .supported_ids
                .iter()
                .filter(|id| id.contains("measurement_update_generation_"))
                .cloned()
                .collect::<BTreeSet<_>>();
            if generations.is_empty() {
                evidence_free_rejections += 1;
            } else if generations.iter().all(|id| id.contains(&source_version)) {
                new_reader_results += 1;
            } else if generations.len() == 1 {
                old_reader_results += 1;
            } else {
                mixed_generation_results += 1;
            }
        }
        let mut violations = Vec::new();
        if current_pointer != new_generation {
            violations.push(format!(
                "current pointer {current_pointer} does not match {new_generation}"
            ));
        }
        if !first
            .supported_ids
            .iter()
            .any(|id| id.contains(&source_version))
        {
            violations.push(format!(
                "first query did not return source version {source_version}"
            ));
        }
        if mixed_generation_results != 0 {
            violations.push(format!(
                "{mixed_generation_results} reader results mixed generations"
            ));
        }
        violations.extend(first.violations);
        samples.push(IndexUpdateSample {
            iteration,
            old_generation: old_generation.clone(),
            new_generation: new_generation.clone(),
            current_pointer,
            source_version,
            indexing_us,
            first_query_us,
            old_reader_results,
            new_reader_results,
            evidence_free_rejections,
            mixed_generation_results,
            violations,
        });
        old_generation = new_generation;
    }
    samples
}

fn threshold_holds(
    warm: &[LatencySummary],
    cold: &[ColdSummary],
    index_update_p95_us: u64,
    peak_rss: Option<u64>,
    correctness_violations: &[String],
) -> Vec<String> {
    let mut holds = Vec::new();
    for summary in warm {
        if summary.p95_us > 500_000 {
            holds.push(format!(
                "{} concurrency {} run {} warm p95 {} us exceeds 500000 us",
                summary.stream, summary.concurrency, summary.run, summary.p95_us
            ));
        }
        if summary.p99_us > 2_000_000 {
            holds.push(format!(
                "{} concurrency {} run {} warm p99 {} us exceeds 2000000 us",
                summary.stream, summary.concurrency, summary.run, summary.p99_us
            ));
        }
    }
    for summary in cold {
        if summary.p95_us > 5_000_000 {
            holds.push(format!(
                "{} cold p95 {} us exceeds 5000000 us",
                summary.category.name(),
                summary.p95_us
            ));
        }
    }
    if index_update_p95_us > 10_000_000 {
        holds.push(format!(
            "index update p95 {index_update_p95_us} us exceeds 10000000 us"
        ));
    }
    if peak_rss.is_some_and(|bytes| bytes > 1024 * 1024 * 1024) {
        holds.push(format!(
            "peak RSS {} bytes exceeds 1073741824 bytes",
            peak_rss.unwrap_or_default()
        ));
    }
    if !correctness_violations.is_empty() {
        holds.push(format!(
            "{} correctness violation(s); see correctness_violations",
            correctness_violations.len()
        ));
    }
    holds
}

fn enforce_report_cap(report: &mut MeasurementReport) {
    let mut serialized = serde_json::to_vec(report).expect("serialize report for cap");
    if serialized.len() <= MAX_REPORT_BYTES {
        return;
    }
    report.hold_reasons.push(format!(
        "structured report exceeded the {MAX_REPORT_BYTES} byte cap; samples were explicitly omitted"
    ));
    while serialized.len() > MAX_REPORT_BYTES && !report.warm_samples.is_empty() {
        report.warm_samples.pop();
        report.omitted_samples_due_to_cap += 1;
        serialized = serde_json::to_vec(report).expect("serialize capped report");
    }
    while serialized.len() > MAX_REPORT_BYTES && !report.cold_samples.is_empty() {
        report.cold_samples.pop();
        report.omitted_samples_due_to_cap += 1;
        serialized = serde_json::to_vec(report).expect("serialize capped report");
    }
    assert!(
        serialized.len() <= MAX_REPORT_BYTES,
        "bounded report metadata exceeds the protocol cap"
    );
}

fn nearest_rank(mut values: Vec<u64>, percentile: usize) -> u64 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    let rank = (values.len() * percentile).div_ceil(100).max(1);
    values[rank - 1]
}

fn elapsed_us(started: Instant) -> u64 {
    started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn git_head(repository_root: &Path) -> String {
    Command::new("git")
        .args(["rev-parse", "--verify", "HEAD"])
        .current_dir(repository_root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|head| head.trim().to_string())
        .unwrap_or_else(|| "unavailable".to_string())
}

fn command_output(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|output| output.trim().to_string())
        .unwrap_or_else(|| "unavailable".to_string())
}

fn proc_status_bytes(field: &str) -> Option<u64> {
    fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find(|line| line.starts_with(field))?
        .split_whitespace()
        .nth(1)?
        .parse::<u64>()
        .ok()
        .map(|kib| kib * 1024)
}

fn mem_available_bytes() -> Option<u64> {
    fs::read_to_string("/proc/meminfo")
        .ok()?
        .lines()
        .find(|line| line.starts_with("MemAvailable:"))?
        .split_whitespace()
        .nth(1)?
        .parse::<u64>()
        .ok()
        .map(|kib| kib * 1024)
}

fn memory_observation(point: &str) -> MemoryObservation {
    MemoryObservation {
        point: point.to_string(),
        rss_bytes: proc_status_bytes("VmRSS"),
        peak_rss_bytes: proc_status_bytes("VmHWM"),
    }
}

fn parse_time_peak_rss(path: &Path) -> Option<u64> {
    fs::read_to_string(path)
        .ok()?
        .lines()
        .find(|line| line.trim_start().starts_with("Maximum resident set size"))?
        .split(':')
        .nth(1)?
        .trim()
        .parse::<u64>()
        .ok()
        .map(|kib| kib * 1024)
}

fn directory_bytes(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| {
            entry
                .file_type()
                .ok()
                .map(|file_type| {
                    if file_type.is_dir() {
                        directory_bytes(&entry.path())
                    } else if file_type.is_file() {
                        entry.metadata().map(|metadata| metadata.len()).unwrap_or(0)
                    } else {
                        0
                    }
                })
                .unwrap_or(0)
        })
        .sum()
}
