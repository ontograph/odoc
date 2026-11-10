//! Synthetic data only: never read the project's real benchmark.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ontodoc::store::Store;
use serde_json::{Value, json};

pub fn s(path: &Path) -> &str {
    path.to_str().unwrap()
}

pub fn ok(out: Output) -> Value {
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

pub fn refused(out: Output, message: &str) {
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty(), "refusals must not expose results");
    let error: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert!(
        error["message"].as_str().unwrap().contains(message),
        "{error}"
    );
}

pub fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn checkout(dir: &Path) -> String {
    git(dir, &["init", "-q"]);
    git(dir, &["add", "."]);
    git(dir, &["commit", "-q", "-m", "synthetic fixture"]);
    let head = git(dir, &["rev-parse", "HEAD"]);
    assert_eq!(head.len(), 40);
    head
}

pub struct Fixture {
    pub dir: tempfile::TempDir,
    pub config: PathBuf,
    pub repo: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("code");
        fs::create_dir(&repo).unwrap();
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock"),
            repo.join("Cargo.lock"),
        )
        .unwrap();
        checkout(&repo);
        let fx = Self {
            config: dir.path().join("config.json"),
            dir,
            repo,
        };
        let mut pins = Vec::new();
        for corpus in ["a", "b"] {
            let rel = format!("corpus_{corpus}");
            let root = fx.path(&rel);
            fs::create_dir(&root).unwrap();
            let source = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/eval")
                .join(&rel);
            for entry in fs::read_dir(source).unwrap() {
                let entry = entry.unwrap();
                fs::copy(entry.path(), root.join(entry.file_name())).unwrap();
            }
            pins.push(json!({"name": corpus, "commit": checkout(&root)}));
            ok(fx.cli(&["--root", s(&root), "index"]));
        }
        fx.write(
            "config.json",
            &json!({
                "corpora": [{"name":"a", "root":"corpus_a"}, {"name":"b", "root":"corpus_b"}],
                "queries":"queries.jsonl", "qrels":"qrels.jsonl", "splits":"splits.json",
                "seed":7, "resamples":10_000, "alpha_grid":[0.0], "beta_grid":[0.0],
                "alpha":0.0, "beta":0.0, "threshold":0.3
            }),
        );
        let mut queries = Vec::new();
        for (corpus, sizes) in [("a", [38, 38, 37, 23]), ("b", [37, 37, 38, 22])] {
            for (stratum, n) in ["lookup", "conceptual", "multi-hop", "off-topic"]
                .into_iter()
                .zip(sizes)
            {
                for i in 0..n {
                    let id = format!("{corpus}-{stratum}-{i:03}");
                    // One unique token and one indexed term give distinct queries
                    // with Jaccard 1/3, while every answerable query retrieves.
                    let unique = format!("synthetic{corpus}{}x{i}", queries.len());
                    let text = if stratum == "off-topic" {
                        unique
                    } else {
                        format!(
                            "{} {unique}",
                            if corpus == "a" { "ontodoc" } else { "search" }
                        )
                    };
                    queries.push(json!({"id":id,"text":text,"stratum":stratum,"corpus":corpus,"model_drafted":true}));
                }
            }
        }
        fx.lines("queries.jsonl", &queries);
        fs::write(fx.path("qrels.jsonl"), "").unwrap();
        ok(fx.cli(&[
            "eval",
            "split",
            s(&fx.config),
            "--seed",
            "7",
            "--out",
            s(&fx.path("splits.json")),
        ]));
        let splits = fx.read("splits.json");
        let test: BTreeSet<&str> = splits["test"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap())
            .collect();
        let selected: Vec<&Value> = queries
            .iter()
            .filter(|q| test.contains(q["id"].as_str().unwrap()))
            .collect();
        assert_eq!(
            selected
                .iter()
                .filter(|q| q["stratum"] != "off-topic")
                .count(),
            150
        );
        assert_eq!(
            selected
                .iter()
                .filter(|q| q["stratum"] == "off-topic")
                .count(),
            30
        );
        for stratum in ["lookup", "conceptual", "multi-hop"] {
            assert_eq!(
                selected.iter().filter(|q| q["stratum"] == stratum).count(),
                50
            );
        }
        ok(fx.cli(&[
            "eval",
            "pool",
            s(&fx.config),
            "--out",
            s(&fx.path("pool.jsonl")),
        ]));
        let hashes: BTreeMap<_, _> = ["a", "b"]
            .into_iter()
            .map(|corpus| {
                let generation = Store::new(fx.path(&format!("corpus_{corpus}/.ontodoc")))
                    .load()
                    .unwrap();
                (
                    corpus,
                    generation
                        .sections
                        .into_iter()
                        .map(|section| (section.id, section.sha256))
                        .collect::<BTreeMap<_, _>>(),
                )
            })
            .collect();
        let mut positive = BTreeSet::new();
        let qrels: Vec<Value> = fx.read_lines("pool.jsonl").into_iter().map(|pair| {
            let id = pair["query_id"].as_str().unwrap();
            let corpus = &id[..1];
            let section = pair["section_id"].as_str().unwrap();
            let grade = if positive.insert(id.to_string()) { 1 } else { 0 };
            json!({"query_id":id,"section_id":section,"grade":grade,"span_sha256":hashes[corpus][section]})
        }).collect();
        assert_eq!(
            positive.len(),
            225,
            "every answerable query has a positive pooled judgement"
        );
        fx.lines("qrels.jsonl", &qrels);
        fx.write("gates.json", &json!({"floors":{"min_answerable_test":150,"min_off_topic_test":30,"min_per_stratum_test":15},"pool":"pool.jsonl","pins":pins}));
        fx
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }

    /// Preparation fixture only; synthetic labels do not establish provenance.
    pub fn partition_inputs(&self) {
        let splits = self.read("splits.json");
        for name in ["dev", "test"] {
            let ids: BTreeSet<_> = splits[name]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            for (payload, field) in [("queries", "id"), ("qrels", "query_id")] {
                let selected: Vec<_> = self
                    .read_lines(&format!("{payload}.jsonl"))
                    .into_iter()
                    .filter(|v| ids.contains(v[field].as_str().unwrap()))
                    .collect();
                self.lines(&format!("{name}-{payload}.jsonl"), &selected);
            }
        }
        let mut config = self.read("config.json");
        config["partitions"] = json!({"dev":{"queries":"dev-queries.jsonl","qrels":"dev-qrels.jsonl"},
            "test":{"queries":"test-queries.jsonl","qrels":"test-qrels.jsonl"}});
        self.write("config.json", &config);
    }
    pub fn cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ontodoc"))
            .current_dir(&self.repo)
            .args(args)
            .output()
            .unwrap()
    }
    pub fn read(&self, rel: &str) -> Value {
        serde_json::from_slice(&fs::read(self.path(rel)).unwrap()).unwrap()
    }
    pub fn write(&self, rel: &str, value: &Value) {
        fs::write(self.path(rel), serde_json::to_vec_pretty(value).unwrap()).unwrap();
    }
    pub fn read_lines(&self, rel: &str) -> Vec<Value> {
        fs::read_to_string(self.path(rel))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
    pub fn lines(&self, rel: &str, values: &[Value]) {
        let text: String = values.iter().map(|value| format!("{value}\n")).collect();
        fs::write(self.path(rel), text).unwrap();
    }
    pub fn check(&self, stage: &str) -> Vec<String> {
        let out = self.cli(&[
            "eval",
            "check-benchmark",
            s(&self.config),
            "--stage",
            stage,
            "--gates",
            s(&self.path("gates.json")),
        ]);
        if out.status.success() {
            assert_eq!(ok(out)["passed"], true);
            return Vec::new();
        }
        assert!(out.stdout.is_empty());
        let err: Value = serde_json::from_slice(&out.stderr).unwrap();
        assert_eq!(err["error"], "gates_failed", "{err}");
        let report: Value = serde_json::from_str(
            err["message"]
                .as_str()
                .unwrap()
                .trim_start_matches("integrity gates failed: "),
        )
        .unwrap();
        report["failures"]
            .as_array()
            .unwrap()
            .iter()
            .map(|failure| failure["gate"].as_str().unwrap().to_string())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    pub fn run(&self, split: &str, rel: &str, confirm: bool) -> Output {
        let output = self.path(rel);
        let mut args = vec![
            "eval",
            "run",
            s(&self.config),
            "--split",
            split,
            "--out",
            s(&output),
        ];
        if confirm {
            args.push("--confirm-test-split");
        }
        self.cli(&args)
    }
    pub fn freeze(&self) -> Output {
        self.cli(&[
            "eval",
            "freeze",
            s(&self.config),
            "--threshold",
            "0.3",
            "--repo",
            s(&self.repo),
            "--out",
            s(&self.path("frozen-config.toml")),
        ])
    }
    pub fn check_frozen(&self) -> Output {
        self.cli(&[
            "eval",
            "check-frozen",
            s(&self.config),
            s(&self.path("frozen-config.toml")),
            "--repo",
            s(&self.repo),
        ])
    }
}
