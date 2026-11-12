//! Strict gates exercised with an independent synthetic benchmark per test.
//!
//! Unpinned measurement refuses before reading inputs. Admitted freeze and
//! check-frozen success is covered end to end in `tests/eval_run.rs`.
mod common;

use std::fs;

use common::{Fixture, gate_failures, git, refused, review_record, s};
use ontodoc::corpus::sha256_hex;
use ontodoc::eval::gates::check_query_review;
use ontodoc::eval::run::{Bench, MIN_EFFECT, SplitName, TrustedInputs};
use ontodoc::eval::stats::required_n;
use ontodoc::eval::sweep::{
    CodeIdentity, FrozenConfig, MIN_TEST_ANSWERABLE, POWER, POWER_ALPHA, PowerInputs,
    REQUIRED_RESAMPLES, check_frozen, freeze, sweep,
};
use serde_json::json;

#[test]
fn library_measurement_helpers_refuse_before_mutable_input_reads() {
    let fx = Fixture::new();
    let code = CodeIdentity::current(&fx.repo).unwrap();
    let bench = Bench::load(&fx.config).unwrap();
    let hash = |name| sha256_hex(&fs::read(fx.path(name)).unwrap());
    let power = PowerInputs {
        alpha: POWER_ALPHA,
        power: POWER,
        min_effect: MIN_EFFECT,
        sd_diff_h2: 0.0,
        sd_diff_h5: 0.0,
        n_h2: required_n(0.0, MIN_EFFECT, POWER_ALPHA, POWER),
        n_h5: required_n(0.0, MIN_EFFECT, POWER_ALPHA, POWER),
        required_answerable_test: MIN_TEST_ANSWERABLE,
    };
    let frozen = FrozenConfig {
        version: ontodoc::eval::sweep::FROZEN_VERSION,
        preparation_sha256: hash("config.json"),
        dev_queries_sha256: hash("queries.jsonl"),
        dev_qrels_sha256: hash("qrels.jsonl"),
        test_queries_sha256: hash("queries.jsonl"),
        test_qrels_sha256: hash("qrels.jsonl"),
        alpha: 0.0,
        beta: 0.0,
        threshold: 0.3,
        seed: 7,
        resamples: REQUIRED_RESAMPLES,
        config_sha256: hash("config.json"),
        splits_sha256: hash("splits.json"),
        git_commit: code.git_commit.clone(),
        cargo_lock_sha256: code.cargo_lock_sha256.clone(),
        binary_sha256: code.binary_sha256.clone(),
        generation_ids: bench
            .indexes()
            .unwrap()
            .iter()
            .map(|(name, index)| (name.clone(), index.generation.generation_id.clone()))
            .collect(),
        power,
    };
    // Preparation loaded only a synthetic Bench. These unavailable operations
    // must refuse before rereading mutable qrels, splits, indexes or run metadata.
    for path in ["queries.jsonl", "qrels.jsonl", "splits.json", "gates.json"] {
        fs::remove_file(fx.path(path)).unwrap();
    }
    for corpus in ["corpus_a", "corpus_b"] {
        fs::remove_dir_all(fx.path(&format!("{corpus}/.ontodoc"))).unwrap();
    }
    let message = "measurement requires an admitted preparation";
    for error in [
        sweep(&bench, SplitName::Dev).unwrap_err(),
        freeze(&bench, 0.3, &code).unwrap_err(),
        check_frozen(&frozen, &bench, &code).unwrap_err(),
        ontodoc::eval::run::run(&bench, SplitName::Dev, false, &fx.path("run")).unwrap_err(),
        ontodoc::eval::run::score(&bench, &fx.path("absent-run"), true).unwrap_err(),
    ] {
        assert!(error.to_string().contains(message), "{error}");
    }
    assert!(!fx.path("run").exists());
    assert!(!fx.path("frozen-config.toml").exists());
}

#[test]
fn clean_fixture_passes_pools_and_qrels_stages() {
    let fx = Fixture::new();
    assert!(fx.check("pools").is_empty());
    assert!(fx.check("qrels").is_empty());
    refused(
        fx.cli(&["eval", "check-benchmark", s(&fx.config), "--stage", "run"]),
        "measurement requires an admitted preparation",
    );
}

#[test]
fn dev_capture_admission_never_opens_heldout_payloads_or_combined_fallback() {
    let fx = Fixture::new();
    fx.partition_inputs();
    let pin = fx.review_sha256();
    let (review, annotation, notes) = (
        fx.path("query-review.jsonl"),
        fx.path("annotation.jsonl"),
        fx.path("annotation.md"),
    );
    let evidence = Bench::prepare(
        &fx.config,
        &TrustedInputs {
            query_review: &review,
            query_review_sha256: &pin,
            annotation: &annotation,
            annotation_notes: &notes,
            operator: "op",
            operator_evidence: "audit/prep",
        },
    )
    .unwrap();
    let manifest = fx.path("preparation.json");
    let pin_prep = evidence.publish(&manifest).unwrap();
    // Held-out, combined, full-pool and full provenance payloads become
    // unreadable directories: dev admission must never open them.
    for name in [
        "test-queries.jsonl",
        "test-qrels.jsonl",
        "queries.jsonl",
        "qrels.jsonl",
        "pool.jsonl",
        "query-review.jsonl",
        "annotation.jsonl",
        "annotation.md",
    ] {
        fs::remove_file(fx.path(name)).unwrap();
        fs::create_dir(fx.path(name)).unwrap(); // Cannot be read as a payload, even by root.
    }
    let dev = Bench::load_dev_pinned(&fx.config, &manifest, &pin_prep).unwrap();
    assert_eq!(dev.queries().len(), 90);
    assert_eq!(dev.qrels().unwrap().0.len(), 75);
    fs::remove_file(fx.path("dev-queries.jsonl")).unwrap();
    assert!(Bench::load_dev_pinned(&fx.config, &manifest, &pin_prep).is_err());
    // The pinned dev capture is admitted for measurement.
    assert_eq!(sweep(&dev, SplitName::Dev).unwrap().dev_answerable, 75);
}

#[test]
fn pool_extension_spanning_both_splits_admits_preparation_and_dev_capture() {
    let fx = Fixture::new();
    let splits = fx.read("splits.json");
    let answerable: std::collections::BTreeSet<String> = fx
        .read_lines("queries.jsonl")
        .into_iter()
        .filter(|q| q["stratum"] != "off-topic")
        .map(|q| q["id"].as_str().unwrap().to_owned())
        .collect();
    let pick = |name: &str| {
        splits[name]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .find(|id| answerable.contains(id))
            .unwrap()
    };
    let mut ids = vec![pick("dev"), pick("test")];
    ids.sort();
    let mut config = fx.read("config.json");
    config["pool_extension"] = json!({"depth": 30, "queries": ids});
    fx.write("config.json", &config);
    fx.partition_inputs();
    let pin = fx.review_sha256();
    let (review, annotation, notes) = (
        fx.path("query-review.jsonl"),
        fx.path("annotation.jsonl"),
        fx.path("annotation.md"),
    );
    // Full preparation validates the extension against the whole benchmark.
    let evidence = Bench::prepare(
        &fx.config,
        &TrustedInputs {
            query_review: &review,
            query_review_sha256: &pin,
            annotation: &annotation,
            annotation_notes: &notes,
            operator: "op",
            operator_evidence: "audit/prep",
        },
    )
    .unwrap();
    let manifest = fx.path("preparation.json");
    let pin_prep = evidence.publish(&manifest).unwrap();
    // The selected dev capture must not need the test-split extension id.
    for name in ["test-queries.jsonl", "test-qrels.jsonl", "queries.jsonl"] {
        fs::remove_file(fx.path(name)).unwrap();
        fs::create_dir(fx.path(name)).unwrap();
    }
    assert!(Bench::load_dev_pinned(&fx.config, &manifest, &pin_prep).is_ok());
}

#[test]
fn in_memory_pool_check_never_writes_files() {
    let fx = Fixture::new();
    let original = fx.path("pool.regenerated.jsonl");
    fs::write(&original, "keep").unwrap();
    let bench = Bench::load(&fx.config).unwrap();
    let approval = check_query_review(
        bench.queries(),
        &fs::read(fx.path("query-review.jsonl")).unwrap(),
        &fx.review_sha256(),
    )
    .unwrap();
    assert!(
        ontodoc::eval::gates::check_pool(&bench, &fx.path("pool.jsonl"), Some(&approval),)
            .unwrap()
            .is_empty()
    );
    assert_eq!(fs::read(original).unwrap(), b"keep");
}

#[test]
fn judged_section_missing_and_hash_missing_or_mismatched() {
    let fx = Fixture::new();
    let original = fx.read_lines("qrels.jsonl");
    for (field, value, gate) in [
        (
            "section_id",
            json!("gone.md#missing"),
            "judged_section_missing",
        ),
        ("span_sha256", json!("0".repeat(64)), "span_hash_mismatch"),
    ] {
        let mut qrels = original.clone();
        qrels[0][field] = value;
        fx.lines("qrels.jsonl", &qrels);
        let expected = if field == "section_id" {
            vec!["judged_section_missing", "pooled_judgment_missing"]
        } else {
            vec![gate]
        };
        assert_eq!(fx.check("qrels"), expected);
        assert!(fx.check("pools").is_empty());
    }
    let mut qrels = original;
    qrels[0].as_object_mut().unwrap().remove("span_sha256");
    fx.lines("qrels.jsonl", &qrels);
    assert_eq!(fx.check("qrels"), vec!["span_hash_missing"]);
}

#[test]
fn dev_test_near_duplicate_is_a_leak() {
    let fx = Fixture::new();
    let splits = fx.read("splits.json");
    let mut queries = fx.read_lines("queries.jsonl");
    let dev = queries
        .iter()
        .find(|q| q["id"] == splits["dev"][0])
        .unwrap()["text"]
        .clone();
    let test = queries
        .iter_mut()
        .find(|q| q["id"] == splits["test"][0])
        .unwrap();
    test["text"] = dev;
    fx.lines("queries.jsonl", &queries);
    fx.review(); // A re-reviewed final text; review never hides leakage.
    // The changed query may also change its pool; leakage must still be caught.
    assert!(fx.check("pools").contains(&"dev_test_leak".to_string()));
}

#[test]
fn pool_and_split_must_regenerate_exactly() {
    let fx = Fixture::new();
    let pool = fs::read(fx.path("pool.jsonl")).unwrap();
    fs::write(fx.path("pool.jsonl"), &pool[..pool.len() - 1]).unwrap();
    assert_eq!(fx.check("pools"), vec!["pool_not_reproducible"]);
    fs::write(fx.path("pool.jsonl"), pool).unwrap();
    let original = fx.read("splits.json");
    let mut splits = original.clone();
    let dev = splits["dev"][0].clone();
    splits["dev"][0] = splits["test"][0].clone();
    splits["test"][0] = dev;
    fx.write("splits.json", &splits);
    assert!(fx.check("pools").contains(&"split_not_reproducible".into()));
    let mut splits = original;
    splits["dev"][0] = json!("unknown");
    splits["test"][1] = splits["test"][0].clone();
    fx.write("splits.json", &splits);
    let failures = fx.check("pools");
    for gate in [
        "split_query_unknown",
        "split_query_duplicate",
        "split_query_missing",
        "split_not_reproducible",
    ] {
        assert!(failures.contains(&gate.to_string()), "{failures:?}");
    }
}

#[test]
fn corpus_pins_require_complete_clean_checkouts() {
    let fx = Fixture::new();
    let original = fx.read("gates.json");
    for (commit, gate) in [
        ("deadbeef".to_string(), "corpus_pin_invalid"),
        ("0".repeat(40), "corpus_wrong_commit"),
    ] {
        let mut gates = original.clone();
        gates["pins"][0]["commit"] = json!(commit);
        fx.write("gates.json", &gates);
        assert_eq!(fx.check("pools"), vec![gate]);
    }
    let mut gates = original.clone();
    gates["pins"].as_array_mut().unwrap().pop();
    fx.write("gates.json", &gates);
    assert_eq!(fx.check("pools"), vec!["corpus_pin_missing"]);
    fx.write("gates.json", &original);
    fs::write(fx.path("corpus_a/install.md"), "# Changed\n").unwrap();
    assert_eq!(fx.check("pools"), vec!["corpus_dirty"]);
    git(&fx.path("corpus_a"), &["checkout", "--", "install.md"]);
    fs::remove_dir_all(fx.path("corpus_b/.git")).unwrap();
    assert_eq!(fx.check("pools"), vec!["corpus_not_a_checkout"]);
}

#[test]
fn mandatory_settings_and_strict_floors() {
    let fx = Fixture::new();
    let original = fx.read("gates.json");
    for (field, gate) in [("floors", "floors_missing"), ("pool", "pool_missing")] {
        let mut gates = original.clone();
        gates.as_object_mut().unwrap().remove(field);
        fx.write("gates.json", &gates);
        assert_eq!(fx.check("pools"), vec![gate]);
    }
    let mut gates = original.clone();
    gates["floors"] =
        json!({"min_answerable_test":1,"min_off_topic_test":1,"min_per_stratum_test":1});
    fx.write("gates.json", &gates);
    assert_eq!(fx.check("pools"), vec!["floors_below_minimum"]);
    let mut gates = original;
    gates["floors"] =
        json!({"min_answerable_test":151,"min_off_topic_test":31,"min_per_stratum_test":51});
    fx.write("gates.json", &gates);
    assert_eq!(
        fx.check("pools"),
        vec!["answerable_floor", "off_topic_floor", "stratum_floor"]
    );
}

#[test]
fn measurement_and_sweep_refuse_without_loading_missing_gates() {
    let fx = Fixture::new();
    fs::remove_file(fx.path("gates.json")).unwrap();
    let message = "measurement requires an admitted preparation";
    refused(fx.run("dev", "refused-run", false), message);
    assert!(!fx.path("refused-run").exists());
    let out = fx.path("sweep.json");
    refused(
        fx.cli(&[
            "eval",
            "sweep",
            s(&fx.config),
            "--split",
            "dev",
            "--out",
            s(&out),
        ]),
        message,
    );
    assert!(!out.exists());
}

#[test]
fn sweep_rejects_test_split_and_refuses_unsealed_dev_inputs() {
    let fx = Fixture::new();
    let out = fx.path("sweep.json");
    refused(
        fx.cli(&[
            "eval",
            "sweep",
            s(&fx.config),
            "--split",
            "test",
            "--out",
            s(&out),
        ]),
        "dev split only",
    );
    refused(
        fx.cli(&[
            "eval",
            "sweep",
            s(&fx.config),
            "--split",
            "dev",
            "--out",
            s(&out),
        ]),
        "measurement requires an admitted preparation",
    );
    assert!(!out.exists());
}

#[test]
fn code_identity_records_dirty_tree_and_lock_changes() {
    let fx = Fixture::new();
    let original = CodeIdentity::current(&fx.repo).unwrap();
    assert!(!original.dirty);
    assert_eq!(original.git_commit, git(&fx.repo, &["rev-parse", "HEAD"]));
    assert_eq!(
        original.cargo_lock_sha256,
        sha256_hex(&fs::read(fx.repo.join("Cargo.lock")).unwrap())
    );
    assert_eq!(original.binary_sha256.len(), 64);
    fs::write(fx.repo.join("scratch"), "dirty").unwrap();
    let dirty = CodeIdentity::current(&fx.repo).unwrap();
    assert!(dirty.dirty);
    assert_eq!(dirty.git_commit, original.git_commit);
    fs::remove_file(fx.repo.join("scratch")).unwrap();
    fs::write(fx.repo.join("Cargo.lock"), "changed lock\n").unwrap();
    git(&fx.repo, &["commit", "-qam", "changed lock"]);
    let changed = CodeIdentity::current(&fx.repo).unwrap();
    assert!(!changed.dirty);
    assert_ne!(changed.git_commit, original.git_commit);
    assert_ne!(changed.cargo_lock_sha256, original.cargo_lock_sha256);
    assert_eq!(changed.binary_sha256, original.binary_sha256);
}

#[test]
fn incomplete_judgements_refuse_publication() {
    let fx = Fixture::new();
    // Retain one positive per query, dropping zero-grade pooled candidates.
    let qrels: Vec<_> = fx
        .read_lines("qrels.jsonl")
        .into_iter()
        .filter(|q| q["grade"] == 1)
        .collect();
    fx.lines("qrels.jsonl", &qrels);
    assert_eq!(fx.check("qrels"), vec!["pooled_judgment_missing"]);
    refused(
        fx.run("dev", "incomplete", false),
        "measurement requires an admitted preparation",
    );
    assert!(!fx.path("incomplete").exists());
}

#[test]
fn unpinned_freeze_commands_preserve_existing_output() {
    let fx = Fixture::new();
    fs::write(fx.path("frozen-config.toml"), "keep").unwrap();
    fs::write(fx.repo.join("scratch"), "dirty").unwrap();
    for output in [fx.freeze(), fx.check_frozen()] {
        refused(output, "measurement requires an admitted preparation");
    }
    assert_eq!(
        fs::read_to_string(fx.path("frozen-config.toml")).unwrap(),
        "keep"
    );
    // Admission must precede registration: an unpinned freeze must not burn
    // the sole machine-local registration slot.
    assert!(!fx.repo.join(".git/ontodoc-eval").exists());
}

#[test]
fn pool_and_pools_stage_require_pinned_accepted_query_review_before_ranking() {
    let fx = Fixture::new();
    let original = fs::read(fx.path("query-review.jsonl")).unwrap();
    let pin = fx.review_sha256();
    let pool = |pin: &str| {
        fx.cli(&[
            "eval",
            "pool",
            s(&fx.config),
            "--out",
            s(&fx.path("pool-new.jsonl")),
            "--query-review",
            s(&fx.path("query-review.jsonl")),
            "--query-review-sha256",
            pin,
        ])
    };
    // The pin is supplied out of band; neither a different nor an uppercase
    // pin is accepted, and nothing is ranked or written.
    refused(pool(&"0".repeat(64)), "bytes differ from the pinned hash");
    refused(pool(&pin.to_uppercase()), "64 lowercase hex digits");
    let missing = fx.cli(&[
        "eval",
        "pool",
        s(&fx.config),
        "--out",
        s(&fx.path("pool-new.jsonl")),
    ]);
    assert!(!missing.status.success());
    assert!(!fx.path("pool-new.jsonl").exists());
    let records = fx.read_lines("query-review.jsonl");
    let answerable = records
        .iter()
        .position(|r| r["stratum"] != "off-topic")
        .unwrap();
    type Tamper = fn(&mut Vec<serde_json::Value>, usize);
    let cases: [(&str, Tamper, &str); 13] = [
        (
            "drop",
            |r, _| {
                r.pop();
            },
            "missing record",
        ),
        (
            "duplicate",
            |r, i| {
                let d = r[i].clone();
                r.push(d);
            },
            "duplicate record",
        ),
        (
            "unknown",
            |r, i| r[i]["query_id"] = json!("unknown"),
            "unknown query",
        ),
        (
            "digest",
            |r, i| r[i]["final_text_sha256"] = json!("0".repeat(64)),
            "differs from the final query",
        ),
        (
            "stratum",
            |r, i| {
                let other = if r[i]["stratum"] == "lookup" {
                    "conceptual"
                } else {
                    "lookup"
                };
                r[i]["stratum"] = json!(other);
            },
            "differs from the final query",
        ),
        (
            "relabel",
            |r, i| r[i]["model_drafted"] = json!(false),
            "differs from the final query",
        ),
        (
            "decision",
            |r, i| r[i]["decision"] = json!("revise"),
            "not accept",
        ),
        (
            "self-review",
            |r, i| {
                // A GPT-6-Astra author approving itself is not independent review.
                r[i]["author"] = r[i]["reviewer"].clone();
                r[i]["history"][0]["author"] = r[i]["reviewer"].clone();
            },
            "non-independent",
        ),
        (
            "non-astra",
            |r, i| r[i]["reviewer"]["model"] = json!("other-model"),
            "non-independent",
        ),
        (
            "version",
            |r, i| r[i]["version"] = json!(2),
            "unsupported version",
        ),
        (
            "history",
            |r, i| r[i]["history"][0]["predecessor_sha256"] = json!("0".repeat(64)),
            "invalid revision history",
        ),
        (
            "final-revise",
            |r, i| r[i]["history"][0]["decision"] = json!("revise"),
            "does not end at",
        ),
        (
            "origin",
            |r, i| {
                r[i]["history"][0]["author"] =
                    json!({"id":"h","kind":"human","evidence":"draft/h"});
            },
            "drafting origin differs",
        ),
    ];
    for (name, tamper, message) in cases {
        let mut tampered = records.clone();
        tamper(&mut tampered, answerable);
        fx.lines("query-review.jsonl", &tampered);
        refused(pool(&fx.review_sha256()), message);
        assert!(!fx.path("pool-new.jsonl").exists(), "{name}");
    }
    // An unknown field and an oversized rationale also refuse.
    let mut tampered = records.clone();
    tampered[answerable]["approved"] = json!(true);
    fx.lines("query-review.jsonl", &tampered);
    refused(pool(&fx.review_sha256()), "line");
    let mut tampered = records.clone();
    tampered[answerable]["rationale"] = json!("x".repeat(4097));
    fx.lines("query-review.jsonl", &tampered);
    refused(pool(&fx.review_sha256()), "non-independent review evidence");
    let mut tampered = records.clone();
    tampered[answerable]["corpus"] = json!("c".repeat(129));
    fx.lines("query-review.jsonl", &tampered);
    refused(pool(&fx.review_sha256()), "128 bytes");
    // A model draft rejected, then revised by a human and finally accepted by
    // GPT-6-Astra, keeps its model origin and complete linked history.
    let mut revised = records.clone();
    let final_entry = revised[answerable]["history"][0].clone();
    let mut draft = final_entry.clone();
    draft["text_sha256"] = json!("1".repeat(64));
    draft["decision"] = json!("reject");
    let human = json!({"id":"editor-1","kind":"human","evidence":"edit/1"});
    let mut accepted = final_entry;
    accepted["predecessor_sha256"] = json!("1".repeat(64));
    accepted["author"] = human.clone();
    revised[answerable]["author"] = human;
    revised[answerable]["history"] = json!([draft, accepted]);
    fx.lines("query-review.jsonl", &revised);
    ok_pool(pool(&fx.review_sha256()), &fx);
    // A post-review text change needs fresh review, not the old approval.
    fs::write(fx.path("query-review.jsonl"), &original).unwrap();
    let mut queries = fx.read_lines("queries.jsonl");
    queries[answerable]["text"] = json!("ontodoc changed after review");
    fx.lines("queries.jsonl", &queries);
    refused(pool(&pin), "differs from the final query");
    let mut fresh = records;
    fresh[answerable] = review_record(&queries[answerable]);
    fx.lines("query-review.jsonl", &fresh);
    assert_eq!(common::ok(pool(&fx.review_sha256()))["queries"], 225);
}

fn ok_pool(out: std::process::Output, fx: &Fixture) {
    let summary = common::ok(out);
    assert_eq!(summary["queries"], 225);
    assert_eq!(
        fs::read(fx.path("pool-new.jsonl")).unwrap(),
        fs::read(fx.path("pool.jsonl")).unwrap()
    );
    fs::remove_file(fx.path("pool-new.jsonl")).unwrap();
}

#[test]
fn pools_stage_without_query_review_fails_closed() {
    let fx = Fixture::new();
    let out = fx.cli(&[
        "eval",
        "check-benchmark",
        s(&fx.config),
        "--stage",
        "pools",
        "--gates",
        s(&fx.path("gates.json")),
    ]);
    assert_eq!(gate_failures(out), vec!["query_review_missing"]);
    let out = fx.cli(&[
        "eval",
        "check-benchmark",
        s(&fx.config),
        "--stage",
        "qrels",
        "--gates",
        s(&fx.path("gates.json")),
    ]);
    assert_eq!(gate_failures(out), vec!["query_review_missing"]);
}

fn check_qrels_with(fx: &Fixture, annotation: Option<(&str, &str)>) -> Vec<String> {
    let pin = fx.review_sha256();
    let (gates, review, records, notes) = (
        fx.path("gates.json"),
        fx.path("query-review.jsonl"),
        fx.path(annotation.map_or("", |a| a.0)),
        fx.path(annotation.map_or("", |a| a.1)),
    );
    let mut args = vec![
        "eval",
        "check-benchmark",
        s(&fx.config),
        "--stage",
        "qrels",
        "--gates",
        s(&gates),
        "--query-review",
        s(&review),
        "--query-review-sha256",
        &pin,
    ];
    if annotation.is_some() {
        args.extend(["--annotation", s(&records), "--annotation-notes", s(&notes)]);
    }
    gate_failures(fx.cli(&args))
}

#[test]
fn cli_publishes_pinned_preparation_once_and_only_after_gates_pass() {
    let fx = Fixture::new();
    fx.partition_inputs();
    let pin = fx.review_sha256();
    let out = fx.path("preparation.json");
    let (review, annotation, notes) = (
        fx.path("query-review.jsonl"),
        fx.path("annotation.jsonl"),
        fx.path("annotation.md"),
    );
    let args = |out: &str| {
        vec![
            "eval".to_string(),
            "check-benchmark".into(),
            s(&fx.config).into(),
            "--stage".into(),
            "qrels".into(),
            "--query-review".into(),
            s(&review).into(),
            "--query-review-sha256".into(),
            pin.clone(),
            "--annotation".into(),
            s(&annotation).into(),
            "--annotation-notes".into(),
            s(&notes).into(),
            "--preparation-out".into(),
            out.into(),
            "--operator".into(),
            "op".into(),
            "--operator-evidence".into(),
            "audit/prep".into(),
        ]
    };
    let run = |args: Vec<String>| fx.cli(&args.iter().map(String::as_str).collect::<Vec<_>>());
    let published = common::ok(run(args(s(&out))));
    let bytes = fs::read(&out).unwrap();
    assert_eq!(published["preparation_sha256"], sha256_hex(&bytes));
    // Create-new: a second publication refuses and leaves the bytes intact.
    assert!(!run(args(s(&out))).status.success());
    assert_eq!(fs::read(&out).unwrap(), bytes);
    // A failing gate publishes nothing.
    let mut records = fx.read_lines("annotation.jsonl");
    records.pop();
    fx.lines("annotation.jsonl", &records);
    let failed = fx.path("failed.json");
    assert!(!run(args(s(&failed))).status.success());
    assert!(!failed.exists());
    // Operator identity is mandatory for publication.
    let mut missing = args(s(&failed));
    missing.truncate(missing.len() - 4);
    assert!(!run(missing).status.success());
    // Separate process: dev admission from the pinned manifest only.
    let dev = Bench::load_dev_pinned(&fx.config, &out, &sha256_hex(&bytes)).unwrap();
    assert_eq!(dev.queries().len(), 90);
}

#[test]
fn qrels_stage_reports_agreement_and_requires_human_annotation() {
    let fx = Fixture::new();
    let pin = fx.review_sha256();
    let out = common::ok(fx.cli(&[
        "eval",
        "check-benchmark",
        s(&fx.config),
        "--stage",
        "qrels",
        "--gates",
        s(&fx.path("gates.json")),
        "--query-review",
        s(&fx.path("query-review.jsonl")),
        "--query-review-sha256",
        &pin,
        "--annotation",
        s(&fx.path("annotation.jsonl")),
        "--annotation-notes",
        s(&fx.path("annotation.md")),
    ]));
    let agreement = &out["agreement"];
    // 225 answerable queries in 6 (corpus, stratum) groups of 38/38/37 and
    // 37/37/38: ceil(30%) = 12+12+12+12+12+12 = 72 overlap queries.
    assert_eq!(agreement["answerable_queries"], 225);
    assert_eq!(agreement["overlap_queries"], 72);
    assert_eq!(agreement["kappa"], 1.0);
    assert_eq!(agreement["resamples"], 10_000);
    assert_eq!(agreement["policy"], "corpus-stratum-ceil30-v1");
    assert_eq!(check_qrels_with(&fx, None), vec!["annotation_missing"]);
    // Pools stage never needs annotation.
    assert!(fx.check("pools").is_empty());
}

#[test]
fn qrels_refuse_off_topic_labels() {
    let fx = Fixture::new();
    let queries = fx.read_lines("queries.jsonl");
    let off_topic = queries
        .iter()
        .find(|q| q["stratum"] == "off-topic")
        .unwrap();
    let mut qrels = fx.read_lines("qrels.jsonl");
    // Same corpus (ids are corpus-prefixed) so section and hash stay valid.
    let id = off_topic["id"].as_str().unwrap();
    let mut label = qrels
        .iter()
        .find(|j| j["query_id"].as_str().unwrap()[..1] == id[..1])
        .unwrap()
        .clone();
    label["query_id"] = off_topic["id"].clone();
    label["grade"] = json!(0);
    qrels.push(label);
    fx.lines("qrels.jsonl", &qrels);
    fx.annotate();
    let files = Some(("annotation.jsonl", "annotation.md"));
    assert_eq!(check_qrels_with(&fx, files), vec!["off_topic_judged"]);
}

#[test]
fn annotation_refuses_mismatch_missing_model_only_and_low_agreement() {
    let fx = Fixture::new();
    let files = Some(("annotation.jsonl", "annotation.md"));
    let records = fx.read_lines("annotation.jsonl");
    let notes = fs::read_to_string(fx.path("annotation.md")).unwrap();
    let with = |records: &[serde_json::Value]| {
        fx.lines("annotation.jsonl", records);
        check_qrels_with(&fx, files)
    };
    type Edit = fn(&mut Vec<serde_json::Value>);
    let cases: [(Edit, &str); 9] = [
        (
            |r| r[0]["final"]["grade"] = json!(2),
            "annotation_qrel_mismatch",
        ),
        (
            |r| r[0]["span_sha256"] = json!("0".repeat(64)),
            "annotation_qrel_mismatch",
        ),
        (
            |r| {
                r.pop();
            },
            "annotation_pair_missing",
        ),
        (
            |r| {
                let d = r[0].clone();
                r.push(d);
            },
            "annotation_duplicate",
        ),
        (
            |r| r[0]["query_id"] = json!("unknown"),
            "annotation_unknown_pair",
        ),
        (
            |r| r[0]["grades"] = json!([]),
            "annotation_raw_grade_invalid",
        ),
        (
            |r| r[0]["grades"][1]["annotator"] = json!("h1"),
            "annotation_raw_grade_invalid",
        ),
        (
            |r| {
                r[0]["final"]["adjudicator"] =
                    json!({"id":"m","kind":"model","model":"x","evidence":"e"})
            },
            "annotation_adjudication_invalid",
        ),
        (
            |r| r[0]["draft"] = json!({"model":"x","grade":1,"evidence":"d"}),
            "annotation_draft_unreviewed",
        ),
    ];
    for (edit, gate) in cases {
        let mut tampered = records.clone();
        edit(&mut tampered);
        assert!(with(&tampered).contains(&gate.to_string()), "{gate}");
    }
    // A model draft explicitly reviewed by an annotator is admitted.
    let mut drafted = records.clone();
    drafted[0]["draft"] = json!({"model":"gpt-6-astra","grade":1,"evidence":"draft/0"});
    drafted[0]["draft_review"] = json!({"reviewer":"h1","evidence":"review/0"});
    assert!(with(&drafted).is_empty());
    // Raw second-annotator grades removed from every pair: no overlap.
    let single: Vec<_> = records
        .iter()
        .map(|r| {
            let mut r = r.clone();
            r["grades"].as_array_mut().unwrap().pop();
            r
        })
        .collect();
    let failures = with(&single);
    for gate in [
        "annotation_overlap_incomplete",
        "agreement_overlap_too_small",
        "agreement_undefined",
    ] {
        assert!(failures.contains(&gate.to_string()), "{failures:?}");
    }
    // Systematic disagreement (0 vs 2, 2 vs 0) gives negative kappa.
    let opposed: Vec<_> = records
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut r = r.clone();
            let (a, b) = if i % 2 == 0 { (0, 2) } else { (2, 0) };
            r["grades"][0]["grade"] = json!(a);
            r["grades"][1]["grade"] = json!(b);
            r
        })
        .collect();
    assert_eq!(with(&opposed), vec!["agreement_below_floor"]);
    fx.lines("annotation.jsonl", &records);
    for bad in [
        notes.replace("roster/h2", "roster/h1"),
        notes.replace(
            r#""id":"h2","kind":"human""#,
            r#""id":"h2","kind":"model","model":"m""#,
        ),
        notes.replace("```annotation-notes", "```json"),
        format!("{notes}{notes}"),
    ] {
        fs::write(fx.path("annotation.md"), bad).unwrap();
        assert_eq!(
            check_qrels_with(&fx, files),
            vec!["annotation_notes_invalid"]
        );
    }
}

/// ADR revision 13: two distinct model judges, fixed floor((a + b) / 2) final
/// grade, a model adjudicator; preparation records the provenance. The
/// human-only rules stay unchanged when `label_provenance` is absent.
#[test]
fn model_judged_annotation_is_admitted_only_with_its_declared_rules() {
    let fx = Fixture::new();
    let files = Some(("annotation.jsonl", "annotation.md"));
    let notes = fx.model_judge();
    let records = fx.read_lines("annotation.jsonl");
    let write_notes = |n: &serde_json::Value| {
        fs::write(
            fx.path("annotation.md"),
            format!("# Synthetic model-judged notes\n\n```annotation-notes\n{n}\n```\n"),
        )
        .unwrap();
    };
    assert_eq!(check_qrels_with(&fx, files), Vec::<String>::new());

    // Unequal judges floor: raw 2 and 1 admit final 1 (floor(3/2)).
    let mut unequal = records.clone();
    let pair = unequal
        .iter()
        .position(|r| r["final"]["grade"] == 1)
        .unwrap();
    unequal[pair]["grades"][0]["grade"] = json!(2);
    fx.lines("annotation.jsonl", &unequal);
    let failures = check_qrels_with(&fx, files);
    assert!(
        !failures.iter().any(|f| f.starts_with("annotation_")),
        "{failures:?}"
    );
    // Out-of-range raw grades refuse instead of overflowing (255 + 1).
    let mut overflow = records.clone();
    overflow[0]["grades"][0]["grade"] = json!(255);
    overflow[0]["grades"][1]["grade"] = json!(1);
    fx.lines("annotation.jsonl", &overflow);
    let failures = check_qrels_with(&fx, files);
    assert!(
        failures.contains(&"annotation_raw_grade_invalid".to_string()),
        "{failures:?}"
    );
    // A model judge record carrying a draft is not the declared rule.
    let mut drafted = records.clone();
    drafted[0]["draft"] = json!({"model":"x","grade":1,"evidence":"d"});
    drafted[0]["draft_review"] = json!({"reviewer":"j1","evidence":"r"});
    fx.lines("annotation.jsonl", &drafted);
    assert!(check_qrels_with(&fx, files).contains(&"annotation_model_rule_invalid".to_string()));

    // A final grade off the floor rule, or a missing judge grade, refuses.
    let mut off_rule = records.clone();
    let g = off_rule[0]["final"]["grade"].as_u64().unwrap();
    // floor((g + other) / 2) != g: g=0,o=2 -> 1; g=1,o=0 -> 0; g=2,o=0 -> 1.
    off_rule[0]["grades"][1]["grade"] = json!(if g == 0 { 2 } else { 0 });
    fx.lines("annotation.jsonl", &off_rule);
    assert!(check_qrels_with(&fx, files).contains(&"annotation_model_rule_invalid".to_string()));
    let mut single = records.clone();
    single[0]["grades"].as_array_mut().unwrap().pop();
    fx.lines("annotation.jsonl", &single);
    assert!(check_qrels_with(&fx, files).contains(&"annotation_model_rule_invalid".to_string()));
    // A human adjudicator is not the declared model rule.
    let mut human_adj = records.clone();
    human_adj[0]["final"]["adjudicator"] = json!({"id":"adj","kind":"human","evidence":"a"});
    fx.lines("annotation.jsonl", &human_adj);
    assert!(check_qrels_with(&fx, files).contains(&"annotation_adjudication_invalid".to_string()));
    fx.lines("annotation.jsonl", &records);

    // Same model twice, a human claimed as model judge, an unknown
    // provenance, or model judges without the declaration all refuse.
    let mut same = notes.clone();
    same["annotators"][1]["model"] = json!("gpt-6-astra");
    let mut mixed = notes.clone();
    mixed["annotators"][1] = json!({"id":"j2","kind":"human","evidence":"session/j2"});
    let mut unknown = notes.clone();
    unknown["label_provenance"] = json!("crowd");
    let mut undeclared = notes.clone();
    undeclared
        .as_object_mut()
        .unwrap()
        .remove("label_provenance");
    for bad in [same, mixed, unknown, undeclared] {
        write_notes(&bad);
        assert_eq!(
            check_qrels_with(&fx, files),
            vec!["annotation_notes_invalid"]
        );
    }
    write_notes(&notes);

    // Preparation records the provenance in its agreement counts.
    fx.partition_inputs();
    let out = fx.path("preparation.json");
    common::ok(fx.cli(&[
        "eval",
        "check-benchmark",
        s(&fx.config),
        "--stage",
        "qrels",
        "--query-review",
        s(&fx.path("query-review.jsonl")),
        "--query-review-sha256",
        &fx.review_sha256(),
        "--annotation",
        s(&fx.path("annotation.jsonl")),
        "--annotation-notes",
        s(&fx.path("annotation.md")),
        "--preparation-out",
        s(&out),
        "--operator",
        "op",
        "--operator-evidence",
        "audit/prep",
    ]));
    let manifest = fx.read("preparation.json");
    assert_eq!(
        manifest["counts"]["agreement"]["label_provenance"],
        "model-judged"
    );
}

/// End to end (ADR revision 13): a model-judged preparation admits a dev
/// run whose report carries the provenance; H1 keeps its sanity decision
/// and every other contrast is exploratory with null Holm p and decision.
/// The human-label report omits the field.
#[test]
fn model_judged_preparation_reaches_the_scored_report() {
    let report = |model: bool| {
        let fx = Fixture::new();
        if model {
            fx.model_judge();
        }
        let pin = fx.prepare();
        let run = fx.path("dev-run");
        common::ok(fx.admitted(
            &[
                "eval",
                "run",
                s(&fx.config),
                "--split",
                "dev",
                "--out",
                s(&run),
            ],
            &pin,
        ))
    };
    let human = report(false);
    assert!(human.get("label_provenance").is_none());
    let model = report(true);
    assert_eq!(model["label_provenance"], "model-judged");
    let comparisons = model["comparisons"].as_array().unwrap();
    assert_eq!(comparisons[0]["id"], "H1");
    assert_eq!(comparisons[0]["role"], "sanity");
    assert_eq!(
        comparisons[0]["decision"],
        human["comparisons"][0]["decision"]
    );
    for c in &comparisons[1..] {
        assert_eq!(c["role"], "exploratory", "{c}");
        assert!(c["holm_p"].is_null() && c["decision"].is_null(), "{c}");
    }
    for c in model["per_corpus_comparisons"].as_array().unwrap() {
        assert!(c["holm_p"].is_null() && c["decision"].is_null(), "{c}");
    }
    // Effect estimates are unchanged by the downgrade (same synthetic labels).
    assert_eq!(comparisons[1]["mean"], human["comparisons"][1]["mean"]);
}

#[test]
fn pool_extension_is_part_of_the_reproducible_pool_contract() {
    let fx = Fixture::new();
    let original = fx.read("config.json");
    let answerable = fx
        .read_lines("queries.jsonl")
        .into_iter()
        .find(|q| q["stratum"] != "off-topic")
        .unwrap();
    // A valid extension keeps the pool regenerable (tiny corpus: no new pairs).
    let mut config = original.clone();
    config["pool_extension"] = json!({"depth": 30, "queries": [answerable["id"]]});
    fx.write("config.json", &config);
    assert_eq!(fx.check("pools"), Vec::<String>::new());
    // Off-topic or unknown ids are refused before any ranking.
    for id in ["a-off-topic-000", "unknown"] {
        config["pool_extension"]["queries"] = json!([id]);
        fx.write("config.json", &config);
        assert!(!fx.pool("pool.ext.jsonl").status.success(), "{id}");
    }
    fx.write("config.json", &original);
}
