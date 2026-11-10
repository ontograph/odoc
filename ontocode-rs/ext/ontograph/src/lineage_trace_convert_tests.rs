use std::path::Path;

use pretty_assertions::assert_eq;
use serde_json::json;
use sha2::Digest;
use sha2::Sha256;

use super::*;

const PROMPT: &str = "SECRET PROMPT TEXT about the owner contract";
const ANSWER: &str = "SECRET ANSWER TEXT";
const BEARER: &str = "Bearer sk-live-ABCDEF1234567890";

fn hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn event(seq: u64, time: i64, payload: serde_json::Value) -> String {
    json!({
        "schema_version": 1, "seq": seq, "wall_time_unix_ms": time,
        "rollout_id": "r1", "thread_id": "t1", "codex_turn_id": "u1",
        "payload": payload,
    })
    .to_string()
}

fn payload_ref(id: u32, kind: &str) -> serde_json::Value {
    json!({"raw_payload_id": format!("raw_payload:{id}"), "kind": {"type": kind},
           "path": format!("payloads/{id}.json")})
}

/// Writes a one-inference bundle whose terminal event is `terminal`.
fn write_bundle(dir: &Path, terminal: &str) {
    std::fs::create_dir_all(dir.join("payloads")).expect("payloads dir");
    let request = json!({"model": "m-1", "input": [{"text": PROMPT}], "auth": BEARER});
    std::fs::write(dir.join("payloads/1.json"), request.to_string()).expect("request");
    std::fs::write(
        dir.join("payloads/2.json"),
        json!({"text": ANSWER}).to_string(),
    )
    .expect("response");
    let mut lines = vec![event(
        1,
        1_000,
        json!({"type": "inference_started", "inference_call_id": "i1", "thread_id": "t1",
               "codex_turn_id": "u1", "model": "m-1", "provider_name": "display name",
               "request_payload": payload_ref(1, "inference_request")}),
    )];
    lines.push(match terminal {
        "completed" => event(
            2,
            1_250,
            json!({"type": "inference_completed", "inference_call_id": "i1",
                   "response_id": "resp", "upstream_request_id": null,
                   "response_payload": payload_ref(2, "inference_response")}),
        ),
        "failed" => event(
            2,
            1_100,
            json!({"type": "inference_failed", "inference_call_id": "i1",
                   "upstream_request_id": null, "error": PROMPT,
                   "partial_response_payload": null}),
        ),
        "cancelled" => event(
            2,
            1_050,
            json!({"type": "inference_cancelled", "inference_call_id": "i1",
                   "upstream_request_id": null, "reason": "cancelled",
                   "partial_response_payload": null}),
        ),
        _ => String::new(),
    });
    std::fs::write(dir.join("trace.jsonl"), lines.join("\n") + "\n").expect("trace");
}

type Mutation = fn(&mut ConvertInput);

fn input() -> ConvertInput {
    ConvertInput {
        run_id: "run-1".into(),
        case_id: "multi-hop".into(),
        variant: Variant::Lineage,
        corpus_digest: hex(b"corpus"),
        corpus_revision: "rev-1".into(),
        prompt_revision: "prompt-1".into(),
        config_revision: "config-1".into(),
        schema_revision: "schema-1".into(),
        order_seed: 0,
        provider_id: "provider-key".into(),
        admission: Some((
            Outcome::Lineage,
            Admission::Accepted,
            FallbackCategory::None,
        )),
    }
}

#[test]
fn completed_trace_becomes_a_live_record_with_hashes_only() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_bundle(dir.path(), "completed");
    let record = convert_bundle(dir.path(), &input()).expect("convert");
    let request_bytes = std::fs::read(dir.path().join("payloads/1.json")).expect("request");
    let response_bytes = std::fs::read(dir.path().join("payloads/2.json")).expect("response");

    assert_eq!(record.evidence_class, EvidenceClass::Live);
    assert_eq!(record.model_id.as_deref(), Some("m-1"));
    assert_eq!(record.request_hash, Some(hex(&request_bytes)));
    assert_eq!(
        record.terminal,
        Terminal::Completed {
            answer_ref: hex(&response_bytes)
        }
    );
    assert_eq!(record.elapsed_ms, Some(250));
    assert_eq!(record.manifest.provider_id.as_deref(), Some("provider-key"));
    assert_eq!(record.manifest.started_at.as_deref(), Some("1000"));

    let line = serde_json::to_string(&record).expect("serialize");
    for leak in [PROMPT, ANSWER, BEARER, "display name", "payloads/"] {
        assert!(!line.contains(leak), "record leaked {leak}");
    }
    let report = crate::lineage_collector::ingest_jsonl(format!("{line}\n").as_bytes());
    assert!(report.is_ok(), "collector rejected the record: {report:?}");
}

#[test]
fn failed_and_cancelled_traces_map_to_missing_categories() {
    for (terminal, expected, elapsed) in [
        ("failed", MissingCategory::ProviderError, Some(100)),
        ("cancelled", MissingCategory::StreamDropped, Some(50)),
        ("none", MissingCategory::UnknownRemoteState, None),
    ] {
        let dir = tempfile::tempdir().expect("tempdir");
        write_bundle(dir.path(), terminal);
        let record = convert_bundle(dir.path(), &input()).expect("convert");
        assert_eq!(record.terminal, Terminal::Missing(expected), "{terminal}");
        assert_eq!(record.elapsed_ms, elapsed, "{terminal}");
        assert_eq!(record.request_hash, None, "{terminal}");
        let line = serde_json::to_string(&record).expect("serialize");
        assert!(!line.contains(PROMPT), "{terminal} leaked the error text");
    }
}

#[test]
fn converter_rejects_free_text_in_every_identifier_field() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_bundle(dir.path(), "completed");
    let cases: [(&str, Mutation); 7] = [
        ("run_id", |i| i.run_id = PROMPT.into()),
        ("case_id", |i| i.case_id = PROMPT.into()),
        ("prompt_revision", |i| i.prompt_revision = PROMPT.into()),
        ("config_revision", |i| i.config_revision = BEARER.into()),
        ("schema_revision", |i| i.schema_revision = String::new()),
        ("corpus_revision", |i| i.corpus_revision = "a".repeat(129)),
        ("provider_id", |i| i.provider_id = "has space".into()),
    ];
    for (field, mutate) in cases {
        let mut bad = input();
        mutate(&mut bad);
        assert_eq!(
            convert_bundle(dir.path(), &bad).err(),
            Some(ConvertError::UnsafeField(field)),
            "{field}"
        );
    }
    let mut bad = input();
    bad.corpus_digest = "not-a-digest".into();
    assert_eq!(
        convert_bundle(dir.path(), &bad).err(),
        Some(ConvertError::UnsafeField("corpus_digest"))
    );
}

#[test]
fn converter_rejects_a_model_id_that_is_not_a_safe_identifier() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_bundle(dir.path(), "completed");
    let trace = std::fs::read_to_string(dir.path().join("trace.jsonl")).expect("trace");
    std::fs::write(
        dir.path().join("trace.jsonl"),
        trace.replace("\"model\":\"m-1\"", &format!("\"model\":\"{PROMPT}\"")),
    )
    .expect("rewrite");
    assert_eq!(
        convert_bundle(dir.path(), &input()).err(),
        Some(ConvertError::UnsafeField("model_id"))
    );
}

#[test]
fn converter_refuses_payload_paths_that_leave_the_bundle() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_bundle(dir.path(), "completed");
    let trace = std::fs::read_to_string(dir.path().join("trace.jsonl")).expect("trace");
    for bad in [
        "../trace.jsonl",
        "/etc/passwd",
        "payloads/../trace.jsonl",
        "trace.jsonl",
    ] {
        std::fs::write(
            dir.path().join("trace.jsonl"),
            trace.replacen("payloads/1.json", bad, 1),
        )
        .expect("rewrite");
        assert_eq!(
            convert_bundle(dir.path(), &input()).err(),
            Some(ConvertError::PayloadUnsafe),
            "{bad}"
        );
    }
}

#[cfg(unix)]
#[test]
fn converter_refuses_a_symlinked_payload() {
    let dir = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside");
    write_bundle(dir.path(), "completed");
    std::fs::write(outside.path().join("req.json"), "outside bytes").expect("outside");
    std::fs::remove_file(dir.path().join("payloads/1.json")).expect("remove");
    std::os::unix::fs::symlink(
        outside.path().join("req.json"),
        dir.path().join("payloads/1.json"),
    )
    .expect("symlink");
    assert_eq!(
        convert_bundle(dir.path(), &input()).err(),
        Some(ConvertError::PayloadUnsafe)
    );
}

#[test]
fn converter_rejects_malformed_empty_and_multi_inference_traces() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_bundle(dir.path(), "completed");
    let trace = std::fs::read_to_string(dir.path().join("trace.jsonl")).expect("trace");

    std::fs::write(dir.path().join("trace.jsonl"), "not json\n").expect("rewrite");
    assert_eq!(
        convert_bundle(dir.path(), &input()).err(),
        Some(ConvertError::MalformedEvent)
    );

    std::fs::write(dir.path().join("trace.jsonl"), "").expect("rewrite");
    assert_eq!(
        convert_bundle(dir.path(), &input()).err(),
        Some(ConvertError::NoInference)
    );

    let second = trace
        .lines()
        .next()
        .expect("started line")
        .replace("\"i1\"", "\"i2\"");
    std::fs::write(dir.path().join("trace.jsonl"), format!("{trace}{second}\n")).expect("rewrite");
    assert_eq!(
        convert_bundle(dir.path(), &input()).err(),
        Some(ConvertError::MultipleInferences)
    );
}

#[test]
fn eval_dir_report_matches_collect_jsonl() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut records = Vec::new();
    for (variant, name) in [
        (Variant::Baseline, "baseline"),
        (Variant::Lineage, "lineage"),
    ] {
        let bundle = temp.path().join(name);
        write_bundle(&bundle, "completed");
        let mut convert_input = input();
        convert_input.run_id = format!("eval-{name}");
        convert_input.variant = variant;
        // A baseline run never resolves an admission; only the lineage arm does.
        convert_input.admission = match variant {
            Variant::Baseline => None,
            Variant::Lineage => Some((
                Outcome::Lineage,
                Admission::Accepted,
                FallbackCategory::None,
            )),
        };
        records.push(convert_bundle(&bundle, &convert_input).expect("convert"));
    }

    let jsonl = records
        .iter()
        .map(|record| serde_json::to_string(record).expect("serialize record"))
        .collect::<Vec<_>>()
        .join("\n");
    let from_jsonl =
        crate::lineage_collector::collect_jsonl(jsonl.as_bytes()).expect("collect JSONL");
    let directly = crate::lineage_collector::collect(&records).expect("collect records");
    assert_eq!(
        crate::lineage_collector::render_report(&from_jsonl).expect("render JSONL"),
        crate::lineage_collector::render_report(&directly).expect("render records")
    );
}

#[test]
#[ignore]
fn lra_eval_from_env() {
    let Some(eval_dir) = std::env::var_os("LRA_EVAL_DIR").map(std::path::PathBuf::from) else {
        return;
    };
    let output = std::env::var_os("LRA_EVAL_OUT")
        .map(std::path::PathBuf::from)
        .expect("LRA_EVAL_OUT is required when LRA_EVAL_DIR is set");
    let manifest_path = eval_dir.join("manifest.json");
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&manifest_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", manifest_path.display())),
    )
    .unwrap_or_else(|error| panic!("invalid {}: {error}", manifest_path.display()));
    let runs = manifest
        .get("runs")
        .and_then(serde_json::Value::as_array)
        .expect("manifest.json must contain a runs array");
    let mut records = Vec::with_capacity(runs.len());

    for run in runs {
        let string_field = |field: &str| {
            run.get(field)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_else(|| panic!("manifest run is missing string field {field}"))
        };
        let case_id = string_field("case_id");
        let variant_name = string_field("variant");
        let thread_id = string_field("thread_id");
        let variant: Variant =
            serde_json::from_value(serde_json::Value::String(variant_name.to_owned()))
                .unwrap_or_else(|error| panic!("unknown variant `{variant_name}`: {error}"));
        let admission = match run.get("admission") {
            None | Some(serde_json::Value::Null) => None,
            Some(value) => {
                let enum_name = |field: &str| {
                    value
                        .get(field)
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_else(|| {
                            panic!("manifest admission is missing string field {field}")
                        })
                };
                let outcome_name = enum_name("outcome");
                let outcome = serde_json::from_value::<Outcome>(serde_json::Value::String(
                    outcome_name.to_owned(),
                ))
                .unwrap_or_else(|error| {
                    panic!("unknown admission outcome `{outcome_name}`: {error}")
                });
                let admission_name = enum_name("admission");
                let admission = serde_json::from_value::<Admission>(serde_json::Value::String(
                    admission_name.to_owned(),
                ))
                .unwrap_or_else(|error| {
                    panic!("unknown admission value `{admission_name}`: {error}")
                });
                let fallback_name = enum_name("fallback_category");
                let fallback = serde_json::from_value::<FallbackCategory>(
                    serde_json::Value::String(fallback_name.to_owned()),
                )
                .unwrap_or_else(|error| {
                    panic!("unknown fallback category `{fallback_name}`: {error}")
                });
                Some((outcome, admission, fallback))
            }
        };
        let variant_dir = eval_dir.join(case_id).join(variant_name);
        let trace_dir = variant_dir.join("trace");
        let entries = std::fs::read_dir(&trace_dir)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", trace_dir.display()))
            .map(|entry| entry.expect("read trace entry").path())
            .collect::<Vec<_>>();
        assert_eq!(
            entries.len(),
            1,
            "expected one bundle directory under {}",
            trace_dir.display()
        );
        let bundle = &entries[0];
        assert!(
            bundle.is_dir(),
            "trace bundle {} is not a directory",
            bundle.display()
        );

        let mut convert_input = input();
        convert_input.run_id = format!(
            "eval-{}",
            hex(format!("{case_id}:{variant_name}:{thread_id}").as_bytes())
        );
        convert_input.case_id = case_id.to_owned();
        convert_input.variant = variant;
        convert_input.admission = admission;
        records.push(
            convert_bundle(bundle, &convert_input).unwrap_or_else(|error| {
                panic!("failed to convert bundle {}: {error}", bundle.display())
            }),
        );
    }

    let report = crate::lineage_collector::collect(&records).expect("collect evaluation records");
    let rendered = crate::lineage_collector::render_report(&report).expect("render report");
    std::fs::write(&output, rendered)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", output.display()));
}
