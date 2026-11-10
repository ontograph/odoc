//! Synthetic eval regressions, independent of the real benchmark.
//!
//! Refusal tests do not exercise the disabled scoring internals. Before enabling
//! measurement, restore end-to-end coverage for input/run digests, seed/parameter
//! tampering, system ordering, missing gates, judged@10, reproducible dev reports,
//! held-out identity and single-attempt receipts. Do not bypass readiness in tests.
mod common;

use std::collections::BTreeMap;
use std::fs;

use common::{Fixture, ok, refused, s};
use ontodoc::eval::run::{PerQuery, compare};
use ontodoc::eval::stats::SplitMix64;
use serde_json::json;

#[test]
fn split_and_pool_regenerate_byte_for_byte() {
    let fx = Fixture::new();
    let split2 = fx.path("split2.json");
    let summary = ok(fx.cli(&[
        "eval",
        "split",
        s(&fx.config),
        "--seed",
        "7",
        "--out",
        s(&split2),
    ]));
    assert_eq!(summary["dev"], 90);
    assert_eq!(summary["test"], 180);
    assert_eq!(
        fs::read(fx.path("splits.json")).unwrap(),
        fs::read(&split2).unwrap()
    );
    let split3 = fx.path("split3.json");
    ok(fx.cli(&[
        "eval",
        "split",
        s(&fx.config),
        "--seed",
        "8",
        "--out",
        s(&split3),
    ]));
    assert_ne!(fs::read(&split2).unwrap(), fs::read(&split3).unwrap());
    let pool2 = fx.path("pool2.jsonl");
    let pool = ok(fx.cli(&["eval", "pool", s(&fx.config), "--out", s(&pool2)]));
    assert_eq!(pool["queries"], 225);
    assert_eq!(
        fs::read(fx.path("pool.jsonl")).unwrap(),
        fs::read(&pool2).unwrap()
    );
    let text = fs::read_to_string(&pool2).unwrap();
    let lines: Vec<_> = text.lines().collect();
    let mut sorted = lines.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(lines, sorted);
    assert!(!text.contains("off-topic"));
    assert_eq!(fx.read_lines("qrels.jsonl").len(), lines.len());
}

#[test]
fn measurement_fails_closed_without_sealed_inputs() {
    let fx = Fixture::new();
    refused(
        fx.run("dev", "blocked-dev", false),
        "evaluation disabled pending sealed inputs",
    );
    assert!(!fx.path("blocked-dev").exists());
    refused(
        fx.run("test", "blocked-test", true),
        "evaluation disabled pending sealed inputs",
    );
    assert!(!fx.path("blocked-test").exists());

    // Metadata claiming either split cannot bypass the score guard.
    fs::create_dir(fx.path("saved-run")).unwrap();
    for split in ["dev", "test"] {
        let meta = json!({
            "split": split, "seed": 7, "resamples": 10_000,
            "alpha": 0.0, "beta": 0.0, "threshold": 0.3,
            "config_sha256": "", "queries_sha256": "", "qrels_sha256": "",
            "splits_sha256": "", "corpora": [], "git_commit": null,
            "git_dirty": null, "binary_sha256": null, "systems": [], "run_sha256": {}
        });
        fx.write("saved-run/meta.json", &meta);
        refused(
            fx.cli(&[
                "eval",
                "score",
                s(&fx.config),
                s(&fx.path("saved-run")),
                "--confirm-test-split",
            ]),
            "evaluation disabled pending sealed inputs",
        );
    }
    assert!(!fx.repo.join(".git/ontodoc-eval").exists());
}

#[test]
fn unavailable_commands_refuse_before_opening_benchmark_or_results() {
    let fx = Fixture::new();
    let out = fx.path("protected-output");
    fs::create_dir(&out).unwrap();
    fs::write(out.join("sentinel"), "keep").unwrap();
    let frozen = fx.path("frozen-config.toml");
    fs::write(&frozen, "not valid TOML").unwrap();
    // Missing and malformed files would fail loading if the readiness check
    // were late. Error precedence is evidence of early refusal, not syscall tracing.
    for missing in [true, false] {
        if missing {
            for name in ["queries.jsonl", "qrels.jsonl"] {
                fs::remove_file(fx.path(name)).unwrap();
            }
        } else {
            for name in ["queries.jsonl", "qrels.jsonl"] {
                fs::write(fx.path(name), "not valid JSON").unwrap();
            }
        }
        let config = s(&fx.config);
        let output = s(&out);
        for args in [
            vec!["eval", "run", config, "--split", "dev", "--out", output],
            vec![
                "eval",
                "run",
                config,
                "--split",
                "test",
                "--out",
                output,
                "--confirm-test-split",
            ],
            vec!["eval", "score", config, output],
            vec!["eval", "score", config, output, "--confirm-test-split"],
            vec!["eval", "sweep", config, "--split", "dev", "--out", output],
            vec![
                "eval",
                "freeze",
                config,
                "--threshold",
                "0.3",
                "--out",
                output,
            ],
            vec!["eval", "check-frozen", config, s(&frozen)],
            vec!["eval", "misses", config, output],
            vec!["eval", "check-benchmark", config, "--stage", "run"],
            vec![
                "eval",
                "check-benchmark",
                config,
                "--stage",
                "pools",
                "--run",
                output,
            ],
            vec![
                "eval",
                "check-benchmark",
                config,
                "--stage",
                "qrels",
                "--run",
                output,
            ],
        ] {
            refused(fx.cli(&args), "evaluation disabled pending sealed inputs");
        }
        refused(fx.run("test", "unconfirmed", false), "--confirm-test-split");
        refused(
            fx.cli(&["eval", "sweep", config, "--split", "test", "--out", output]),
            "dev split only",
        );
        assert_eq!(fs::read_to_string(out.join("sentinel")).unwrap(), "keep");
        assert_eq!(fs::read_dir(&out).unwrap().count(), 1);
        assert_eq!(fs::read_to_string(&frozen).unwrap(), "not valid TOML");
        assert!(!fx.path("unconfirmed").exists());
    }
}

#[test]
fn score_refuses_before_parsing_saved_metadata() {
    let fx = Fixture::new();
    let run = fx.path("run");
    fs::create_dir(&run).unwrap();
    for metadata in ["not valid JSON", "{}"] {
        fs::write(run.join("meta.json"), metadata).unwrap();
        refused(
            fx.cli(&["eval", "score", s(&fx.config), s(&run)]),
            "evaluation disabled pending sealed inputs",
        );
        assert_eq!(fs::read_to_string(run.join("meta.json")).unwrap(), metadata);
        assert!(!run.join("report.json").exists());
    }
}

#[test]
fn measurement_refuses_bad_hash_and_preserves_existing_output() {
    let fx = Fixture::new();
    let mut qrels = fx.read_lines("qrels.jsonl");
    qrels[0]["span_sha256"] = json!("0".repeat(64));
    fx.lines("qrels.jsonl", &qrels);
    fs::create_dir(fx.path("existing")).unwrap();
    fs::write(fx.path("existing/sentinel"), "keep").unwrap();
    assert_eq!(fx.check("qrels"), vec!["span_hash_mismatch"]);
    refused(
        fx.run("dev", "bad-hash", false),
        "evaluation disabled pending sealed inputs",
    );
    assert!(!fx.path("bad-hash").exists());
    refused(
        fx.run("dev", "existing", false),
        "evaluation disabled pending sealed inputs",
    );
    assert_eq!(
        fs::read_to_string(fx.path("existing/sentinel")).unwrap(),
        "keep"
    );
    assert!(!fx.path("existing/report.json").exists());
}

#[test]
fn discrimination_check_on_synthetic_scores() {
    let mut rng = SplitMix64::new(2026);
    let mut oracle = PerQuery::new();
    let mut shuffled = PerQuery::new();
    let mut corpus_of = BTreeMap::new();
    for i in 0..80 {
        let q = format!("q{i:02}");
        oracle.insert(q.clone(), 0.8 + rng.below(200) as f64 / 1000.0);
        shuffled.insert(q.clone(), rng.below(400) as f64 / 1000.0);
        corpus_of.insert(q, if i % 2 == 0 { "a".into() } else { "b".into() });
    }
    let (n, ci, p) = compare(&oracle, &shuffled, &corpus_of, 10_000, 1);
    assert_eq!(n, 80);
    assert!(p < 0.01, "{p}");
    assert!(ci.lo > 0.0);
    let (_, ci, p) = compare(&oracle, &oracle, &corpus_of, 10_000, 1);
    assert_eq!(p, 1.0);
    assert!(ci.lo <= 0.0 && ci.hi >= 0.0);
}
