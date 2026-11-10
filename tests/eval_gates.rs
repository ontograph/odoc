//! Strict gates exercised with an independent synthetic benchmark per test.
//!
//! Before enabling measurement, restore successful freeze/check-frozen tests
//! covering stale sweep evidence, exact resamples, dirty code and commit/lock/
//! binary/config mismatches. Identity collection and refusal tests below do not
//! establish those currently unreachable enforcement paths.
mod common;

use std::fs;

use common::{Fixture, git, refused, s};
use ontodoc::corpus::sha256_hex;
use ontodoc::eval::run::{Bench, MIN_EFFECT, SplitName};
use ontodoc::eval::stats::required_n;
use ontodoc::eval::sweep::{
    CodeIdentity, FrozenConfig, MIN_TEST_ANSWERABLE, POWER, POWER_ALPHA, PowerInputs,
    REQUIRED_RESAMPLES, SweepReport, check_frozen, freeze, sweep,
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
        alpha: 0.0,
        beta: 0.0,
        threshold: 0.3,
        seed: 7,
        resamples: REQUIRED_RESAMPLES,
        config_sha256: hash("config.json"),
        queries_sha256: hash("queries.jsonl"),
        qrels_sha256: hash("qrels.jsonl"),
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
        power: power.clone(),
    };
    let report = SweepReport {
        dev_answerable: 75,
        b1_ndcg10: 1.0,
        grid: Vec::new(),
        best_alpha: 0.0,
        best_beta: 0.0,
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
    let message = "evaluation disabled pending sealed inputs";
    for error in [
        sweep(&bench, SplitName::Dev).unwrap_err(),
        freeze(&bench, &report, 0.3, &code).unwrap_err(),
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
        "evaluation disabled pending sealed inputs",
    );
}

#[test]
fn dev_capture_admission_never_opens_heldout_payloads_or_combined_fallback() {
    let fx = Fixture::new();
    fx.partition_inputs();
    let evidence = Bench::prepare(&fx.config).unwrap();
    for name in [
        "test-queries.jsonl",
        "test-qrels.jsonl",
        "queries.jsonl",
        "qrels.jsonl",
    ] {
        fs::remove_file(fx.path(name)).unwrap();
        fs::create_dir(fx.path(name)).unwrap(); // Cannot be read as a payload, even by root.
    }
    let dev = Bench::load_dev(&fx.config, &evidence).unwrap();
    assert_eq!(dev.queries().len(), 90);
    assert_eq!(dev.qrels().unwrap().0.len(), 75);
    fs::remove_file(fx.path("dev-queries.jsonl")).unwrap();
    assert!(Bench::load_dev(&fx.config, &evidence).is_err());
    assert!(
        sweep(&dev, SplitName::Dev)
            .unwrap_err()
            .to_string()
            .contains("evaluation disabled")
    );
}

#[test]
fn in_memory_pool_check_never_touches_caller_scratch_files() {
    let fx = Fixture::new();
    let original = fx.path("pool.regenerated.jsonl");
    fs::write(&original, "keep").unwrap();
    let bench = Bench::load(&fx.config).unwrap();
    assert!(
        ontodoc::eval::gates::check_pool(
            &bench,
            &fx.path("pool.jsonl"),
            &fx.path("absent-scratch")
        )
        .unwrap()
        .is_empty()
    );
    assert_eq!(fs::read(original).unwrap(), b"keep");
    assert!(!fx.path("absent-scratch").exists());
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
    let message = "evaluation disabled pending sealed inputs";
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
        "evaluation disabled pending sealed inputs",
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
        "evaluation disabled pending sealed inputs",
    );
    assert!(!fx.path("incomplete").exists());
}

#[test]
fn freeze_commands_preserve_existing_output_while_disabled() {
    let fx = Fixture::new();
    fs::write(fx.path("frozen-config.toml"), "keep").unwrap();
    fs::write(fx.repo.join("scratch"), "dirty").unwrap();
    for output in [fx.freeze(), fx.check_frozen()] {
        refused(output, "evaluation disabled pending sealed inputs");
    }
    assert_eq!(
        fs::read_to_string(fx.path("frozen-config.toml")).unwrap(),
        "keep"
    );
    // Admission must precede registration: even a failed freeze must not burn
    // the sole machine-local slot while measurement is universally disabled.
    assert!(!fx.repo.join(".git/ontodoc-eval").exists());
}
