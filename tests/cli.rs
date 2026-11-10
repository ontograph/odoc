//! End-to-end tests of the `ontodoc` binary against a copy of the graph fixture.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let to = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to);
        } else {
            fs::copy(entry.path(), to).unwrap();
        }
    }
}

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graph");
    copy_dir(&src, dir.path());
    dir
}

fn run(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ontodoc"))
        .arg("--root")
        .arg(root)
        .args(args)
        .output()
        .unwrap()
}

fn ok(out: &Output) -> Value {
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn err(out: &Output) -> Value {
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    serde_json::from_slice(&out.stderr).unwrap()
}

#[test]
fn missing_index_is_a_typed_error() {
    let dir = fixture();
    for args in [
        &["status"][..],
        &["search", "alpha"],
        &["read", "a.md#alpha"],
        &["related", "a.md#alpha~0"],
    ] {
        let e = err(&run(dir.path(), args));
        assert_eq!(e["error"], "no_index", "{args:?}");
        assert!(e["message"].as_str().unwrap().contains("ontodoc index"));
    }
}

#[test]
fn committed_publication_reports_maintenance_failure_without_rollback() {
    let dir = fixture();
    let complete = ok(&run(dir.path(), &["index"]));
    assert_eq!(complete["committed"], true);
    assert_eq!(complete["durability"], "synced");
    assert_eq!(complete["maintenance"], "complete");
    assert_eq!(complete["failure_phase"], Value::Null);
    assert_eq!(complete["warnings"], serde_json::json!([]));
    let store = dir.path().join(".ontodoc");
    let original = fs::read(store.join("CURRENT")).unwrap();
    let damaged_history = b"not a canonical generation record\n";
    fs::write(store.join("HISTORY"), damaged_history).unwrap();
    fs::write(dir.path().join("new.md"), "# New\n\nNew document.\n").unwrap();
    let output = run(dir.path(), &["index"]);
    let committed = ok(&output);
    assert!(output.stderr.is_empty());
    assert_eq!(committed["committed"], true);
    assert_eq!(committed["durability"], "synced");
    assert_eq!(committed["maintenance"], "failed");
    assert_eq!(committed["failure_phase"], "history_read");
    assert_eq!(committed["warnings"], serde_json::json!(["history_read"]));
    let active = fs::read(store.join("CURRENT")).unwrap();
    assert_ne!(active, original);
    assert_eq!(
        std::str::from_utf8(&active).unwrap(),
        committed["generation_id"].as_str().unwrap()
    );
    assert_eq!(fs::read(store.join("HISTORY")).unwrap(), damaged_history);
    assert_eq!(
        ok(&run(dir.path(), &["status"]))["generation_id"],
        committed["generation_id"]
    );
    // Once the trusted administrator removes the damaged history, ordinary
    // republication can finish maintenance without changing the active bytes.
    fs::remove_file(store.join("HISTORY")).unwrap();
    assert_eq!(ok(&run(dir.path(), &["index"]))["maintenance"], "complete");
    assert_eq!(fs::read(store.join("CURRENT")).unwrap(), active);
}

#[test]
fn collection_admission_refuses_before_search_and_keeps_current_pointer() {
    let root = fixture();
    let report = ok(&run(root.path(), &["index"]));
    let id = report["generation_id"].as_str().unwrap();
    let store = root.path().join(".ontodoc");
    let pointer = fs::read(store.join("CURRENT")).unwrap();
    let dir = store.join("generations").join(id);
    let mut artifact: Value =
        serde_json::from_slice(&fs::read(dir.join("index.json")).unwrap()).unwrap();
    artifact["config"]["max_files"] = serde_json::json!(1);
    let bytes = serde_json::to_vec(&artifact).unwrap();
    fs::write(dir.join("index.json"), &bytes).unwrap();
    let mut meta: Value =
        serde_json::from_slice(&fs::read(dir.join("meta.json")).unwrap()).unwrap();
    meta["artifact_bytes"] = serde_json::json!(bytes.len());
    meta["artifact_sha256"] = serde_json::json!(ontodoc::corpus::sha256_hex(&bytes));
    fs::write(dir.join("meta.json"), serde_json::to_vec(&meta).unwrap()).unwrap();
    for args in [&["status"][..], &["search", "alpha"]] {
        let error = err(&run(root.path(), args));
        assert_eq!(error["error"], "cap_exceeded");
        assert!(
            !error
                .to_string()
                .contains(&root.path().display().to_string())
        );
    }
    assert_eq!(fs::read(store.join("CURRENT")).unwrap(), pointer);
}

#[test]
fn index_search_read_related_verify_status() {
    let dir = fixture();
    let root = dir.path();

    let index = ok(&run(root, &["index"]));
    assert_eq!(index["documents"], 4);
    assert_eq!(index["chunks"], 8);
    assert_eq!(index["link_diagnostics"], 3);
    let id = index["generation_id"].as_str().unwrap().to_string();
    assert_eq!(id.len(), 64);
    // Same content elsewhere gives the same generation id.
    let other = fixture();
    assert_eq!(
        ok(&run(other.path(), &["index"]))["generation_id"],
        id.as_str()
    );

    let status = ok(&run(root, &["status"]));
    assert_eq!(status["generation_id"], id.as_str());
    assert_eq!(status["fresh"], true);
    assert_eq!(status["chunks"], 8);
    assert_eq!(status["link_diagnostic_count"], 3);

    let args = ["search", "cfg", "--mode", "graph", "--explain"];
    let first = run(root, &args);
    assert_eq!(
        first.stdout,
        run(root, &args).stdout,
        "deterministic output"
    );
    let search = ok(&first);
    assert_eq!(search["fresh"], true);
    assert_eq!(search["mode"], "graph");
    assert_eq!(search["abstained"], Value::Null);
    let top = &search["results"][0];
    assert_eq!(top["chunk_id"], "b.md#~0");
    assert_eq!(top["tier"], "l2");
    assert_eq!(top["text"], "Preamble with `cfg`.");
    assert_eq!(top["citation"]["path"], "b.md");
    assert_eq!(top["citation"]["generation_id"], id.as_str());
    assert_eq!(top["explain"]["bm25_rank"], 1);

    let abstain = ok(&run(root, &["search", "zebra", "--threshold", "0.1"]));
    assert_eq!(abstain["abstained"], "no_lexical_match");
    assert_eq!(abstain["results"], Value::Array(vec![]));

    let budget = ok(&run(
        root,
        &["search", "alpha gamma", "--budget-bytes", "40"],
    ));
    let used: usize = budget["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["text"].as_str().unwrap().len())
        .sum();
    assert!(used <= 40, "{used}");

    let section = ok(&run(root, &["read", "a.md#details"]));
    assert_eq!(section["kind"], "section");
    assert_eq!(section["id"], "a.md#alpha/details");
    assert!(section["text"].as_str().unwrap().starts_with("## Details"));
    let chunk = ok(&run(root, &["read", "b.md#beta/setup~0"]));
    assert_eq!(chunk["kind"], "chunk");
    assert_eq!(chunk["citation"]["line_start"], 5);
    assert_eq!(
        err(&run(root, &["read", "a.md#nope"]))["error"],
        "not_found"
    );

    let related = ok(&run(root, &["related", "a.md#alpha~0"]));
    let links: Vec<String> = related["neighbours"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["kind"] == "LINKS_TO" && n["direction"] == "out")
        .map(|n| n["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(links, vec!["b.md#beta/setup~0", "b.md#~0"]);

    let citation = top["citation"].to_string();
    let verify = ok(&run(root, &["verify", &citation]));
    assert_eq!(verify["results"][0]["state"], "valid");

    // Shift b.md: the cited span moves, the index is stale.
    let b = root.join("b.md");
    let text = fs::read_to_string(&b).unwrap();
    fs::write(&b, format!("New first line.\n\n{text}")).unwrap();
    let verify = ok(&run(root, &["verify", &format!("[{citation}]")]));
    assert_eq!(verify["results"][0]["state"], "moved");
    assert_eq!(verify["results"][0]["span"]["line_start"], 3);
    let status = ok(&run(root, &["status"]));
    assert_eq!(status["fresh"], false);
    assert_eq!(status["modified"], serde_json::json!(["b.md"]));
    let stale = ok(&run(root, &["search", "cfg"]));
    assert_eq!(stale["fresh"], false);
    assert_eq!(stale["results"][0]["stale"], true);

    assert_eq!(
        err(&run(root, &["search", "x", "--top-k", "0"]))["error"],
        "invalid_argument"
    );
    assert_eq!(
        err(&run(
            root,
            &["search", "x", "--mode", "graph", "--alpha", "2"]
        ))["error"],
        "invalid_argument"
    );
    assert_eq!(
        err(&run(root, &["verify", "{"]))["error"],
        "invalid_argument"
    );
}

#[test]
fn escaped_read_is_bounded_in_serialized_json() {
    let dir = fixture();
    fs::write(
        dir.path().join("escape.md"),
        format!("# Escape\n\n{}", "\"\\\u{1}é".repeat(20000)),
    )
    .unwrap();
    ok(&run(dir.path(), &["index"]));
    let output = run(dir.path(), &["read", "escape.md#escape"]);
    let result = ok(&output);
    assert!(output.stdout.len() <= ontodoc::cli::MAX_RESPONSE_BYTES + 1);
    assert_eq!(result["truncated"], true);
    assert!(!result["text"].as_str().unwrap().is_empty());
    // Truncating returned text does not truncate or invalidate its whole-section citation.
    let verified = ok(&run(
        dir.path(),
        &["verify", &result["citation"].to_string()],
    ));
    assert_eq!(verified["results"][0]["state"], "valid");
}

#[test]
fn verify_rejects_fabrication_and_inactive_generations_but_preserves_hidden_members() {
    let dir = fixture();
    fs::write(dir.path().join(".ontodocignore"), "!.hidden.md\n").unwrap();
    fs::write(dir.path().join(".hidden.md"), "# Hidden\n\noriginal\n").unwrap();
    ok(&run(dir.path(), &["index"]));
    let hidden = ok(&run(dir.path(), &["read", ".hidden.md#hidden"]));
    let citation = hidden["citation"].clone();
    fs::write(dir.path().join(".ontodocignore"), "*.md\n").unwrap();
    assert_eq!(
        ok(&run(dir.path(), &["verify", &citation.to_string()]))["results"][0]["state"],
        "valid"
    );
    let mut forged = citation.clone();
    forged["path"] = Value::from("unindexed.md");
    assert_eq!(
        err(&run(dir.path(), &["verify", &forged.to_string()]))["error"],
        "invalid_citation"
    );
    let batch = serde_json::json!(vec![citation.clone(); 101]);
    assert_eq!(
        err(&run(dir.path(), &["verify", &batch.to_string()]))["error"],
        "cap_exceeded"
    );
    fs::write(dir.path().join(".ontodocignore"), "!.hidden.md\n").unwrap();
    fs::write(dir.path().join(".hidden.md"), "# Hidden\n\nnew\n").unwrap();
    ok(&run(dir.path(), &["index"]));
    assert_eq!(
        err(&run(dir.path(), &["verify", &citation.to_string()]))["error"],
        "generation_unavailable"
    );
}

#[test]
fn internal_store_errors_redact_private_paths() {
    let dir = tempfile::Builder::new()
        .prefix("private-cli-sentinel")
        .tempdir()
        .unwrap();
    let missing = err(&run(dir.path(), &["status"]));
    assert_eq!(missing["error"], "no_index");
    assert!(!missing.to_string().contains("private-cli-sentinel"));
    fs::create_dir_all(dir.path().join(".ontodoc/CURRENT")).unwrap();
    let io = err(&run(dir.path(), &["status"]));
    assert_eq!(
        io,
        serde_json::json!({"error":"io","message":"Internal operation failed"})
    );
}

#[test]
fn schema_two_exact_ids_precede_unchanged_anchor_aliases() {
    let root = tempfile::tempdir().unwrap();
    let paths = ["a.md", "a#part~0.md", "plain~0.md"];
    for path in paths {
        fs::write(root.path().join(path), "intro\n\n# Parent\n\nparent\n\n## !!!\n\nempty\n\n### Child\n\nchild\n\n# Child\n\nduplicate\n").unwrap();
    }
    fs::write(root.path().join("no.md"), "# !!!\n\nbody\n").unwrap();
    ok(&run(root.path(), &["index"]));
    assert_eq!(ok(&run(root.path(), &["status"]))["schema_version"], 2);
    for path in paths {
        for suffix in [
            "",
            "parent",
            "parent/@empty",
            "parent/@empty/child",
            "child-1",
        ] {
            let id = format!("{path}#{suffix}");
            let section = ok(&run(root.path(), &["read", &id]));
            assert_eq!(section["id"], id);
            assert_eq!(section["kind"], "section");
            assert_eq!(section["citation"]["path"], path);
            let chunk_id = format!("{id}~0");
            let chunk = ok(&run(root.path(), &["read", &chunk_id]));
            assert_eq!(chunk["id"], chunk_id);
            assert_eq!(chunk["kind"], "chunk");
        }
    }
    let preamble = ok(&run(root.path(), &["read", "a.md#"]));
    assert_eq!(preamble["text"], "intro");
    let empty_alias = ok(&run(root.path(), &["read", "no.md#"]));
    assert_eq!(empty_alias["id"], "no.md#@empty");
    assert_eq!(
        ok(&run(root.path(), &["read", "a.md#child"]))["id"],
        "a.md#parent/@empty/child"
    );
    assert_eq!(
        err(&run(root.path(), &["read", "a.md#@empty"]))["error"],
        "not_found"
    );
    // This filename's anchor alias is intentionally not extended; its exact IDs above work.
    assert_eq!(
        err(&run(root.path(), &["read", "a#part~0.md#child"]))["error"],
        "not_found"
    );
    let generation = ontodoc::store::Store::new(root.path().join(".ontodoc"))
        .load()
        .unwrap();
    assert_eq!(generation.config.schema_version, 2);
}
