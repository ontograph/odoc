//! Synthetic eval regressions, independent of the real benchmark.
//!
//! Measurement requires a pinned preparation manifest; unpinned commands refuse
//! before opening inputs. `admitted_protocol_runs_dev_then_one_held_out_attempt`
//! covers the admitted dev and single held-out path end to end on synthetic data.
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
    let pool = ok(fx.pool("pool2.jsonl"));
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
fn unpinned_measurement_fails_closed() {
    let fx = Fixture::new();
    refused(
        fx.run("dev", "blocked-dev", false),
        "measurement requires an admitted preparation",
    );
    assert!(!fx.path("blocked-dev").exists());
    refused(
        fx.run("test", "blocked-test", true),
        "measurement requires an admitted preparation",
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
            "measurement requires an admitted preparation",
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
            refused(
                fx.cli(&args),
                "measurement requires an admitted preparation",
            );
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
            "measurement requires an admitted preparation",
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
        "measurement requires an admitted preparation",
    );
    assert!(!fx.path("bad-hash").exists());
    refused(
        fx.run("dev", "existing", false),
        "measurement requires an admitted preparation",
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

/// End-to-end synthetic protocol through separate CLI processes: pinned
/// preparation admits dev sweep/run/score/misses, freeze and check-frozen;
/// then the single held-out attempt runs, is scored from completed custody
/// and can never be retried. Synthetic data only; not a scientific result.
#[test]
fn admitted_protocol_runs_dev_then_one_held_out_attempt() {
    let fx = Fixture::new();
    let pin = fx.prepare();
    let config = s(&fx.config);
    // Wrong or malformed pins and unpinned commands refuse.
    refused(
        fx.admitted(
            &["eval", "sweep", config, "--split", "dev", "--out", "x"],
            &"0".repeat(64),
        ),
        "preparation refused",
    );
    refused(
        fx.cli(&["eval", "sweep", config, "--split", "dev", "--out", "x"]),
        "measurement requires an admitted preparation",
    );
    // Dev execution never opens held-out, combined, pool or provenance files.
    let hidden = [
        "test-queries.jsonl",
        "test-qrels.jsonl",
        "queries.jsonl",
        "qrels.jsonl",
        "pool.jsonl",
        "query-review.jsonl",
        "annotation.jsonl",
        "annotation.md",
    ];
    for name in hidden {
        fs::rename(fx.path(name), fx.path(&format!("{name}.held"))).unwrap();
        fs::create_dir(fx.path(name)).unwrap();
    }
    let sweep_out = fx.path("sweep.json");
    let sweep = ok(fx.admitted(
        &[
            "eval",
            "sweep",
            config,
            "--split",
            "dev",
            "--out",
            s(&sweep_out),
        ],
        &pin,
    ));
    assert_eq!(sweep["dev_answerable"], 75);
    let dev_run = fx.path("dev-run");
    let report = ok(fx.admitted(
        &[
            "eval",
            "run",
            config,
            "--split",
            "dev",
            "--out",
            s(&dev_run),
        ],
        &pin,
    ));
    assert_eq!(report["meta"]["preparation_sha256"], pin.as_str());
    assert_eq!(report["meta"]["version"], 2);
    assert_eq!(
        report["per_corpus_comparisons"].as_array().unwrap().len(),
        14
    );
    let rescored = ok(fx.admitted(&["eval", "score", config, s(&dev_run)], &pin));
    assert_eq!(rescored, report);
    ok(fx.admitted(&["eval", "misses", config, s(&dev_run)], &pin));
    let frozen_path = fx.path("frozen-config.toml");
    let frozen = ok(fx.admitted(
        &[
            "eval",
            "freeze",
            config,
            "--threshold",
            "0.3",
            "--repo",
            s(&fx.repo),
            "--out",
            s(&frozen_path),
        ],
        &pin,
    ));
    assert_eq!(frozen["preparation_sha256"], pin.as_str());
    assert_eq!(frozen["version"], 2);
    ok(fx.admitted(
        &[
            "eval",
            "check-frozen",
            config,
            s(&frozen_path),
            "--repo",
            s(&fx.repo),
        ],
        &pin,
    ));
    // Restore held-out payloads only for the confirmed attempt.
    for name in hidden {
        fs::remove_dir(fx.path(name)).unwrap();
        fs::rename(fx.path(&format!("{name}.held")), fx.path(name)).unwrap();
    }
    let custody = fx.repo.join(".git/ontodoc-eval");
    let test_run = fx.path("test-run");
    let test = &[
        "eval",
        "run",
        config,
        "--split",
        "test",
        "--out",
        s(&test_run),
    ];
    refused(fx.admitted(test, &pin), "--confirm-test-split");
    assert!(!custody.join("attempt-started.json").exists());
    let confirmed = [&test[..], &["--confirm-test-split"]].concat();
    let held_out = fx.admitted(&confirmed, &pin);
    let started = custody.join("attempt-started.json");
    assert!(started.exists(), "attempt reserved");
    let result: serde_json::Value =
        serde_json::from_slice(&fs::read(custody.join("attempt-result.json")).unwrap()).unwrap();
    // The synthetic fixture passes H1 and judged@10, so the attempt completes.
    let report = ok(held_out);
    assert_eq!(result["state"], "completed");
    assert_eq!(report["meta"]["split"], "test");
    assert_eq!(report["meta"]["preparation_sha256"], pin.as_str());
    assert_eq!(report["comparisons"][0]["decision"], "pass");
    let saved = ok(fx.admitted(
        &[
            "eval",
            "score",
            config,
            s(&test_run),
            "--confirm-test-split",
        ],
        &pin,
    ));
    assert_eq!(saved, report);
    // Unconfirmed saved held-out scoring and dev-pinned scoring both refuse.
    assert!(
        !fx.admitted(&["eval", "score", config, s(&test_run)], &pin)
            .status
            .success()
    );
    // No second attempt, whatever the outcome or output path.
    let retry = fx.path("retry");
    let again = fx.admitted(
        &[
            "eval",
            "run",
            config,
            "--split",
            "test",
            "--out",
            s(&retry),
            "--confirm-test-split",
        ],
        &pin,
    );
    assert!(!again.status.success());
    assert!(!retry.exists());
}

/// A benchmark directory inside a corpus checkout is refused before any
/// payload capture, so freshness walks and `git status` cannot stat it.
#[test]
fn benchmark_inside_corpus_checkout_is_refused() {
    let fx = Fixture::new();
    let pin = fx.prepare();
    let mut config = fx.read("config.json");
    config["corpora"][0]["root"] = json!(".");
    let nested = fx.path("corpus_a/config.json");
    fs::write(&nested, serde_json::to_vec(&config).unwrap()).unwrap();
    let manifest = fx.path("preparation.json");
    let out = fx.cli(&[
        "eval",
        "sweep",
        s(&nested),
        "--split",
        "dev",
        "--out",
        s(&fx.path("nested-sweep.json")),
        "--preparation",
        s(&manifest),
        "--preparation-sha256",
        &pin,
    ]);
    refused(out, "must be outside corpus");
}

/// A benchmark payload placed inside a corpus checkout is refused even when
/// the config itself lives outside it.
#[test]
fn partition_inside_corpus_checkout_is_refused() {
    let fx = Fixture::new();
    let pin = fx.prepare();
    fs::create_dir(fx.path("corpus_a/held")).unwrap();
    fs::copy(
        fx.path("test-qrels.jsonl"),
        fx.path("corpus_a/held/test-qrels.jsonl"),
    )
    .unwrap();
    let mut config = fx.read("config.json");
    config["partitions"]["test"]["qrels"] = json!("corpus_a/held/test-qrels.jsonl");
    fx.write("config.json", &config);
    let out = fx.admitted(
        &[
            "eval",
            "sweep",
            s(&fx.config),
            "--split",
            "dev",
            "--out",
            s(&fx.path("sweep.json")),
        ],
        &pin,
    );
    refused(out, "must be outside corpus");
}

/// A config that swaps dev and test partition paths differs from the admitted
/// manifest and refuses before any partition payload is read: held-out files
/// are replaced by unreadable directories, so a read would fail differently.
#[test]
fn swapped_partitions_refuse_before_reading_payloads() {
    let fx = Fixture::new();
    let pin = fx.prepare();
    let mut config = fx.read("config.json");
    let files = config["partitions"].clone();
    config["partitions"]["dev"] = files["test"].clone();
    config["partitions"]["test"] = files["dev"].clone();
    fx.write("config.json", &config);
    for name in ["test-queries.jsonl", "test-qrels.jsonl"] {
        fs::rename(fx.path(name), fx.path(&format!("{name}.held"))).unwrap();
        fs::create_dir(fx.path(name)).unwrap();
    }
    let out = fx.admitted(
        &[
            "eval",
            "sweep",
            s(&fx.config),
            "--split",
            "dev",
            "--out",
            s(&fx.path("sweep.json")),
        ],
        &pin,
    );
    refused(out, "differs from prevalidation");
}

/// A corpus edited after freezing is detected from metadata before the sole
/// held-out attempt is reserved: refused, nothing consumed, no output.
#[test]
fn stale_corpus_refuses_held_out_before_reservation() {
    let fx = Fixture::new();
    let pin = fx.prepare();
    let config = s(&fx.config);
    let frozen = fx.path("frozen-config.toml");
    ok(fx.admitted(
        &[
            "eval",
            "freeze",
            config,
            "--threshold",
            "0.3",
            "--repo",
            s(&fx.repo),
            "--out",
            s(&frozen),
        ],
        &pin,
    ));
    let custody = fx.repo.join(".git/ontodoc-eval");
    assert!(custody.join("registration.json").exists());
    let doc = fs::read_dir(fx.path("corpus_a"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|x| x == "md"))
        .unwrap();
    fs::write(&doc, "# Edited after freeze\n\nchanged\n").unwrap();
    let out = fx.path("test-run");
    let refusal = fx.admitted(
        &[
            "eval",
            "run",
            config,
            "--split",
            "test",
            "--out",
            s(&out),
            "--confirm-test-split",
        ],
        &pin,
    );
    refused(refusal, "corpus freshness or pin changed");
    assert!(!custody.join("attempt-started.json").exists());
    assert!(!out.exists());
}

/// An output parent that admission accepts but cannot hold the private
/// staging directory (read-only) fails before the sole attempt is reserved.
#[test]
fn unwritable_output_parent_refuses_before_reservation() {
    use std::os::unix::fs::PermissionsExt;
    let fx = Fixture::new();
    let pin = fx.prepare();
    let config = s(&fx.config);
    let frozen = fx.path("frozen-config.toml");
    ok(fx.admitted(
        &[
            "eval",
            "freeze",
            config,
            "--threshold",
            "0.3",
            "--repo",
            s(&fx.repo),
            "--out",
            s(&frozen),
        ],
        &pin,
    ));
    let custody = fx.repo.join(".git/ontodoc-eval");
    let parent = fx.path("read-only");
    fs::create_dir(&parent).unwrap();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o555)).unwrap();
    // A privileged user can write anyway; the refusal is then untestable.
    if fs::write(parent.join("probe"), b"").is_ok() {
        return;
    }
    let out = parent.join("test-run");
    let refusal = fx.admitted(
        &[
            "eval",
            "run",
            config,
            "--split",
            "test",
            "--out",
            s(&out),
            "--confirm-test-split",
        ],
        &pin,
    );
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).unwrap();
    refused(refusal, "read-only");
    assert!(!custody.join("attempt-started.json").exists());
    assert!(!out.exists());
}

/// Trusted preparation inputs placed inside a corpus checkout are refused
/// before corpus freshness or git inspection could read them: the notes file
/// is Markdown, so a freshness pass would otherwise hash it as corpus text.
#[test]
fn preparation_inputs_inside_corpus_checkout_are_refused() {
    let fx = Fixture::new();
    fx.prepare();
    fs::copy(
        fx.path("annotation.md"),
        fx.path("corpus_a/annotation-notes.md"),
    )
    .unwrap();
    let out = fx.cli(&[
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
        s(&fx.path("corpus_a/annotation-notes.md")),
        "--preparation-out",
        s(&fx.path("preparation-2.json")),
        "--operator",
        "op",
        "--operator-evidence",
        "audit/prep",
    ]);
    refused(out, "must be outside corpus");
    assert!(!fx.path("preparation-2.json").exists());
}

/// Resample counts outside 1..=10000 refuse at configuration admission, so
/// dev scoring cannot allocate unbounded bootstrap work or report empty CIs.
#[test]
fn out_of_range_resamples_refuse_at_admission() {
    let fx = Fixture::new();
    let pin = fx.prepare();
    for resamples in [0, 10_001] {
        let mut config = fx.read("config.json");
        config["resamples"] = json!(resamples);
        fx.write("config.json", &config);
        let out = fx.admitted(
            &[
                "eval",
                "sweep",
                s(&fx.config),
                "--split",
                "dev",
                "--out",
                s(&fx.path("sweep.json")),
            ],
            &pin,
        );
        refused(out, "resamples");
    }
}
