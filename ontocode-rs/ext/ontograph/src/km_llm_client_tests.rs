use std::cell::Cell;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;

use tempfile::tempdir;

use super::AUTHORING_CALLS_LOG;
use super::DraftFact;
use super::DraftMapping;
use super::KmLlmCallKind;
use super::KmLlmCallOutcome;
use super::KmLlmCallRecord;
use super::KmLlmClientConfig;
use super::KmLlmClientError;
use super::KmLlmExtractDraft;
use super::KmLlmTextModel;
use super::KmLlmTokenSource;
use super::MAX_MODEL_DRAFT_ROWS;
use super::MAX_MODEL_FIELD_CHARS;
use super::MAX_MODEL_OUTPUT_BYTES;
use super::MAX_MODEL_PROMPT_BYTES;
use super::accounting_line;
use super::canonical_row_source;
use super::estimate_tokens;
use super::metadata_log_line;
use super::run_model_extract;
use super::run_model_text;
use super::sanitize_accounting_label;
use super::validate_draft;
use super::validate_draft_fact;
use super::validate_draft_mapping;
use super::validate_row_source;

struct StubModel {
    result: Result<String, KmLlmClientError>,
    calls: Cell<usize>,
}

impl StubModel {
    fn ok(draft: KmLlmExtractDraft) -> Self {
        Self {
            result: Ok(serde_json::to_string(&draft).expect("json")),
            calls: Cell::new(0),
        }
    }

    fn ok_text(text: &str) -> Self {
        Self {
            result: Ok(text.to_string()),
            calls: Cell::new(0),
        }
    }

    fn err() -> Self {
        Self {
            result: Err(KmLlmClientError::ModelCall("boom".to_string())),
            calls: Cell::new(0),
        }
    }
}

impl KmLlmTextModel for StubModel {
    fn complete(&self, _prompt_text: &str) -> Result<String, KmLlmClientError> {
        self.calls.set(self.calls.get() + 1);
        self.result.clone()
    }
}

fn sample_draft() -> KmLlmExtractDraft {
    KmLlmExtractDraft {
        facts: vec![DraftFact {
            id: "f1".to_string(),
            kind: "draft_source_reference".to_string(),
            claim: "example".to_string(),
            source: "silver:docs/source.md".to_string(),
            confidence: 0.5,
        }],
        mappings: vec![DraftMapping {
            id: "m1".to_string(),
            kind: "draft_source_mapping".to_string(),
            source_concept: "docs/source.md".to_string(),
            target_concept: "ontograph_km_fact".to_string(),
            relation: "candidate_source_for".to_string(),
            source: "silver:docs/source.md".to_string(),
            confidence: 0.5,
        }],
    }
}

fn config() -> KmLlmClientConfig {
    KmLlmClientConfig {
        enabled: true,
        offline_only: true,
        provider_is_local: true,
        max_input_tokens: None,
        model_slug: "test-model".to_string(),
        provider_id: "local".to_string(),
        project_root: None,
    }
}

fn config_with_root(root: &Path) -> KmLlmClientConfig {
    let mut cfg = config();
    cfg.project_root = Some(root.to_path_buf());
    cfg
}

fn calls_log_path(root: &Path) -> PathBuf {
    root.join(AUTHORING_CALLS_LOG)
}

fn read_call_lines(root: &Path) -> Vec<String> {
    let path = calls_log_path(root);
    if !path.exists() {
        return Vec::new();
    }
    fs::read_to_string(path)
        .expect("read calls log")
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

#[test]
fn enabled_local_call_returns_model_draft() {
    let model = StubModel::ok(sample_draft());
    let out = run_model_extract(&config(), "prompt", &model);
    assert_eq!(out, Some(sample_draft()));
    assert_eq!(model.calls.get(), 1);
}

#[test]
fn disabled_does_not_call_model() {
    let mut cfg = config();
    cfg.enabled = false;
    let model = StubModel::ok(sample_draft());
    assert_eq!(run_model_extract(&cfg, "prompt", &model), None);
    assert_eq!(model.calls.get(), 0);
}

#[test]
fn offline_only_refuses_non_local_provider() {
    let mut cfg = config();
    cfg.provider_is_local = false;
    let model = StubModel::ok(sample_draft());
    assert_eq!(run_model_extract(&cfg, "prompt", &model), None);
    assert_eq!(model.calls.get(), 0);
}

#[test]
fn non_offline_allows_non_local_provider() {
    let mut cfg = config();
    cfg.offline_only = false;
    cfg.provider_is_local = false;
    let model = StubModel::ok(sample_draft());
    assert_eq!(
        run_model_extract(&cfg, "prompt", &model),
        Some(sample_draft())
    );
}

#[test]
fn input_cap_exceeded_falls_back() {
    let mut cfg = config();
    cfg.max_input_tokens = Some(1);
    let model = StubModel::ok(sample_draft());
    assert_eq!(
        run_model_extract(&cfg, "a very long prompt body", &model),
        None
    );
    assert_eq!(model.calls.get(), 0);
}

#[test]
fn hard_prompt_byte_cap_falls_back_without_calling_model() {
    let model = StubModel::ok_text("unused");
    assert_eq!(
        run_model_text(&config(), &"x".repeat(MAX_MODEL_PROMPT_BYTES + 1), &model),
        None
    );
    assert_eq!(model.calls.get(), 0);
}

#[test]
fn extract_prompt_reserves_instruction_within_hard_byte_cap() {
    struct PromptLengthModel {
        prompt_len: Cell<usize>,
    }

    impl KmLlmTextModel for PromptLengthModel {
        fn complete(&self, prompt_text: &str) -> Result<String, KmLlmClientError> {
            self.prompt_len.set(prompt_text.len());
            serde_json::to_string(&sample_draft())
                .map_err(|error| KmLlmClientError::Parse(error.to_string()))
        }
    }

    let model = PromptLengthModel {
        prompt_len: Cell::new(0),
    };

    assert_eq!(
        run_model_extract(&config(), &"x".repeat(MAX_MODEL_PROMPT_BYTES), &model),
        Some(sample_draft())
    );
    assert_eq!(model.prompt_len.get(), MAX_MODEL_PROMPT_BYTES);
}

#[test]
fn hard_output_byte_cap_falls_back() {
    let model = StubModel::ok_text(&"x".repeat(MAX_MODEL_OUTPUT_BYTES + 1));
    assert_eq!(run_model_text(&config(), "prompt", &model), None);
    assert_eq!(model.calls.get(), 1);
}

#[test]
fn model_error_falls_back() {
    let model = StubModel::err();
    assert_eq!(run_model_extract(&config(), "prompt", &model), None);
    assert_eq!(model.calls.get(), 1);
}

#[test]
fn enabled_local_call_returns_raw_text() {
    let model = StubModel::ok_text("plain text");
    let out = run_model_text(&config(), "prompt", &model);
    assert_eq!(out, Some("plain text".to_string()));
    assert_eq!(model.calls.get(), 1);
}

#[test]
fn validate_row_source_checks_prefix_and_existence() {
    let root = tempdir().expect("tempdir");
    std::fs::create_dir_all(root.path().join("docs")).expect("docs dir");
    std::fs::write(root.path().join("docs/source.md"), "# Source\n").expect("source");

    assert!(validate_row_source("silver:docs/source.md", root.path()));
    assert!(!validate_row_source("silver:docs/missing.md", root.path()));
    assert!(!validate_row_source("gold:docs/source.md", root.path()));
    assert!(!validate_row_source("no-prefix", root.path()));
    assert!(!validate_row_source("silver:/abs/path.md", root.path()));
    assert!(!validate_row_source(
        "silver:C:\\\\abs\\\\path.md",
        root.path()
    ));
    assert!(!validate_row_source(
        "silver:\\\\server\\\\share.md",
        root.path()
    ));
    assert!(!validate_row_source("silver:../secret.md", root.path()));
    assert_eq!(
        canonical_row_source("silver:docs/./source.md", root.path()),
        Some("silver:docs/source.md".to_string())
    );
}

#[cfg(unix)]
#[test]
fn validate_row_source_rejects_symlink_escape() {
    use std::os::unix::fs::symlink;

    let root = tempdir().expect("tempdir");
    let outside = tempdir().expect("outside");
    std::fs::write(outside.path().join("secret.md"), "secret").expect("secret");
    symlink(outside.path(), root.path().join("linked")).expect("symlink");

    assert!(!validate_row_source("silver:linked/secret.md", root.path()));
}

#[test]
fn draft_validation_rejects_empty_oversized_nan_and_excessive_rows() {
    let mut fact = sample_draft().facts.remove(0);
    let mut mapping = sample_draft().mappings.remove(0);

    fact.id.clear();
    assert!(!validate_draft_fact(&fact));
    fact.id = "f1".to_string();
    fact.claim = "x".repeat(501);
    assert!(!validate_draft_fact(&fact));
    fact.claim = "claim".to_string();
    fact.confidence = f64::NAN;
    assert!(!validate_draft_fact(&fact));

    mapping.relation.clear();
    assert!(!validate_draft_mapping(&mapping));
    mapping.relation = "maps_to".to_string();
    mapping.source = "x".repeat(301);
    assert!(!validate_draft_mapping(&mapping));

    let fact = sample_draft().facts.remove(0);
    let excessive = KmLlmExtractDraft {
        facts: vec![fact; MAX_MODEL_DRAFT_ROWS + 1],
        mappings: Vec::new(),
    };
    assert!(validate_draft(excessive).is_none());
}

#[test]
fn metadata_log_line_excludes_prompt_and_output_bodies() {
    let secret = "sk-super-secret-token";
    let mut cfg = config();
    cfg.model_slug = "test-model".to_string();
    cfg.provider_id = "local".to_string();
    let line = metadata_log_line(&cfg, KmLlmCallKind::Extract, 3, 4);
    assert!(!line.contains(secret));
    assert!(!line.contains("example"));
    assert_eq!(
        line,
        "km_llm_extract call_kind=extract provider=local model=test-model facts=3 mappings=4"
    );
}

// Backlog 90: the Authoring Call Accounting Contract allows stage, provider,
// model, counts, duration, and a bounded outcome. Prompt bodies, response
// bodies, and source contents are forbidden. This drives real prompt and
// response text through the call boundary and asserts none of it reaches the
// accounting line.
#[test]
fn client_call_accounting_never_records_prompt_or_response_bodies() {
    struct EchoModel {
        response: String,
    }
    impl KmLlmTextModel for EchoModel {
        fn complete(&self, _prompt: &str) -> Result<String, KmLlmClientError> {
            Ok(self.response.clone())
        }
    }

    let prompt = "SOURCE-BODY bronze:secret/design.md contents of the source file";
    let response = "RESPONSE-BODY model produced this draft text";
    let root = tempdir().expect("tempdir");
    let cfg = config_with_root(root.path());
    let model = EchoModel {
        response: response.to_string(),
    };

    let output = run_model_text(&cfg, prompt, &model).expect("model call succeeds");
    assert_eq!(output, response, "the caller still receives the body");

    let lines = read_call_lines(root.path());
    assert_eq!(lines.len(), 1);
    let line = &lines[0];
    for forbidden in [
        "SOURCE-BODY",
        "RESPONSE-BODY",
        "bronze:secret/design.md",
        "contents of the source file",
    ] {
        assert!(
            !line.contains(forbidden),
            "accounting line leaked {forbidden}: {line}"
        );
    }
    assert!(line.contains("\"provider\":\"local\""));
    assert!(line.contains("\"model\":\"test-model\""));
}

// GDK-4.0, ADR Accounting Amendment II: call kind is derived from the invoked
// entry point, so it must report `text` for run_model_text and `extract` for
// run_model_extract without any caller supplying it. The pre-existing
// hardcoded `stage=extract` was wrong for the recipe, judge, and lint paths.
#[test]
fn call_kind_is_derived_from_the_invoked_entry_point() {
    assert_eq!(
        accounting_line(&record_for(KmLlmCallKind::Text), 7),
        "km_llm_call call_kind=text outcome=completed provider=local model=test-model input_tokens=11 output_tokens=3 token_source=estimated duration_ms=7"
    );
    assert_eq!(
        accounting_line(&record_for(KmLlmCallKind::Extract), 7),
        "km_llm_call call_kind=extract outcome=completed provider=local model=test-model input_tokens=11 output_tokens=3 token_source=estimated duration_ms=7"
    );
}

// The Authoring Call Accounting Contract permits stage, provider, model, token
// counts labelled returned or estimated, duration, and a bounded outcome. This
// pins every outcome string and the estimated label so neither can drift into
// free-form text.
#[test]
fn accounting_outcomes_are_bounded_and_labelled_estimated() {
    let outcomes = [
        (KmLlmCallOutcome::Disabled, "disabled"),
        (KmLlmCallOutcome::OfflineOnlyRefused, "offline-only-refused"),
        (KmLlmCallOutcome::PromptTooLarge, "prompt-too-large"),
        (KmLlmCallOutcome::TokenCapExceeded, "token-cap-exceeded"),
        (KmLlmCallOutcome::ModelError, "model-error"),
        (KmLlmCallOutcome::OutputTooLarge, "output-too-large"),
        (KmLlmCallOutcome::Completed, "completed"),
    ];
    for (outcome, expected) in outcomes {
        let mut record = record_for(KmLlmCallKind::Text);
        record.outcome = outcome;
        let line = accounting_line(&record, 0);
        assert!(
            line.contains(&format!("outcome={expected}")),
            "unexpected outcome rendering: {line}"
        );
        assert!(line.contains("token_source=estimated"), "{line}");
    }
}

// A record has no field that can hold a prompt body, response body, source
// content, endpoint, or credential value. This drives real secret-shaped text
// through the boundary and asserts none of it can reach the accounting line.
#[test]
fn accounting_line_rejects_secret_and_body_markers() {
    struct SecretModel;
    impl KmLlmTextModel for SecretModel {
        fn complete(&self, _prompt: &str) -> Result<String, KmLlmClientError> {
            Ok(
                "RESPONSE-BODY Authorization: Bearer [REDACTED:Authorization header] token]"
                    .to_string(),
            )
        }
    }

    let root = tempdir().expect("tempdir");
    let cfg = config_with_root(root.path());
    let prompt = "SOURCE-BODY https://api.example.com/v1 Cookie: session=abc";
    let output = run_model_text(&cfg, prompt, &SecretModel).expect("model call succeeds");
    assert!(
        output.contains("RESPONSE-BODY"),
        "caller still gets the body"
    );

    let lines = read_call_lines(root.path());
    assert_eq!(lines.len(), 1);
    let line = &lines[0];
    for forbidden in [
        "SOURCE-BODY",
        "RESPONSE-BODY",
        "Authorization",
        "Bearer",
        "sk-live-secret",
        "Cookie",
        "https://",
        "api.example.com",
        "keychain",
    ] {
        assert!(
            !line.contains(forbidden),
            "accounting line leaked {forbidden}: {line}"
        );
    }
}

// Exactly one record per authoring call. `run_model_extract` must not emit a
// second record for the nested text path, and a disabled stage must stay
// non-fatal: the caller still receives None rather than an error.
#[test]
fn disabled_and_fallback_paths_stay_non_fatal_with_bounded_outcomes() {
    let mut cfg = config();
    cfg.enabled = false;
    let model = StubModel::ok(sample_draft());
    assert_eq!(run_model_extract(&cfg, "prompt", &model), None);
    assert_eq!(model.calls.get(), 0, "disabled stage never calls the model");

    let cfg = config();
    let model = StubModel::err();
    assert_eq!(run_model_text(&cfg, "prompt", &model), None);
    assert_eq!(
        model.calls.get(),
        1,
        "a model error is one call, not a retry"
    );

    let model = StubModel::ok(sample_draft());
    assert!(run_model_extract(&cfg, "prompt", &model).is_some());
    assert_eq!(
        model.calls.get(),
        1,
        "run_model_extract performs exactly one model call, so it accounts for exactly one"
    );
}

// ADR Accounting Amendment III D1: one authoring call writes exactly one JSON
// line under the project-local operations area.
#[test]
fn accounting_sink_writes_one_json_line_under_ops_authoring() {
    let root = tempdir().expect("tempdir");
    let cfg = config_with_root(root.path());
    let model = StubModel::ok_text("plain text");
    assert_eq!(
        run_model_text(&cfg, "prompt", &model),
        Some("plain text".to_string())
    );

    let path = calls_log_path(root.path());
    assert!(path.is_file(), "expected sink at {}", path.display());
    let lines = read_call_lines(root.path());
    assert_eq!(lines.len(), 1);
    let value: serde_json::Value = serde_json::from_str(&lines[0]).expect("json line");
    assert_eq!(value["call_kind"], "text");
    assert_eq!(value["outcome"], "completed");
    assert_eq!(value["provider"], "local");
    assert_eq!(value["model"], "test-model");
    assert_eq!(value["token_source"], "estimated");
    assert!(value["duration_ms"].as_u64().is_some());
}

// ADR Accounting Amendment III D1: N extract calls append exactly N lines.
#[test]
fn accounting_cardinality_is_one_record_per_extract_call() {
    let root = tempdir().expect("tempdir");
    let cfg = config_with_root(root.path());
    let n = 3;
    for _ in 0..n {
        let model = StubModel::ok(sample_draft());
        assert!(run_model_extract(&cfg, "prompt", &model).is_some());
        assert_eq!(model.calls.get(), 1);
    }
    let lines = read_call_lines(root.path());
    assert_eq!(lines.len(), n);
    for line in &lines {
        let value: serde_json::Value = serde_json::from_str(line).expect("json line");
        assert_eq!(value["call_kind"], "extract");
    }
}

// ADR Accounting Amendment IV F2: accounting vocabulary stays module-private.
// The child test module can still name the items through `super`, which is the
// only intended consumer outside the owner body.
#[test]
fn accounting_surface_stays_module_private() {
    let _kind = KmLlmCallKind::Text;
    let _outcome = KmLlmCallOutcome::Completed;
    let _source = KmLlmTokenSource::Estimated;
    let _record = record_for(KmLlmCallKind::Text);
    let _line = accounting_line(&_record, 0);
    let _meta = metadata_log_line(&config(), KmLlmCallKind::Text, 0, 0);
}

// ADR Accounting Amendment IV F3: shape-only label guarantee.
#[test]
fn accounting_label_shape_truncates_empty_blocks_injection_and_paths() {
    let long = "a".repeat(200);
    let truncated = sanitize_accounting_label(&long);
    assert_eq!(truncated.chars().count(), 64);
    assert_eq!(truncated, "a".repeat(64));

    assert_eq!(sanitize_accounting_label(""), "unknown");

    let forged = "openai\nkm_llm_call forged=1";
    let sanitized_forged = sanitize_accounting_label(forged);
    assert_eq!(sanitized_forged, "openai_km_llm_call_forged_1");
    assert!(!sanitized_forged.contains('\n'));

    let root = tempdir().expect("tempdir");
    let mut cfg = config_with_root(root.path());
    cfg.provider_id = forged.to_string();
    let model = StubModel::ok_text("ok");
    assert_eq!(
        run_model_text(&cfg, "prompt", &model),
        Some("ok".to_string())
    );
    let raw = fs::read_to_string(calls_log_path(root.path())).expect("raw log");
    let lines = read_call_lines(root.path());
    assert_eq!(
        lines.len(),
        1,
        "newline injection must not forge records: {raw:?}"
    );
    assert_eq!(
        raw.lines().count(),
        1,
        "raw sink must stay one physical line: {raw:?}"
    );

    let keychain = "/Users/me/Library/Keychains/login.keychain-db";
    let sanitized_path = sanitize_accounting_label(keychain);
    assert!(!sanitized_path.contains('/'));
    assert!(!sanitized_path.contains('\\'));
    assert_eq!(
        sanitized_path,
        "_Users_me_Library_Keychains_login.keychain-db"
    );
}

// ADR Accounting Amendment IV F3: allowlist-valid secret-shaped values pass
// through unchanged. Provenance (config.model_provider_id from the merged
// model_providers map key), not shape filtering, is what bounds this field.
#[test]
fn accounting_label_provenance_passes_allowlist_valid_secret_shaped_values() {
    for value in ["AWS_SECRET_ACCESS_KEY", "sk-live-secret", "api.example.com"] {
        assert_eq!(
            sanitize_accounting_label(value),
            value,
            "shape must not pretend to filter allowlist-valid secret-shaped labels"
        );
    }
}

// ADR Accounting Amendment IV F3: provider/model labels are shape-bounded and
// JSON-escaped so newline injection cannot forge extra records.
#[test]
fn accounting_sanitizes_provider_and_model_labels() {
    let root = tempdir().expect("tempdir");
    let mut cfg = config_with_root(root.path());
    cfg.provider_id =
        "https://api.example.com/v1\nHOME=$HOME sk-live-secret Authorization: Bearer [REDACTED:Authorization header]"
            .to_string();
    cfg.model_slug = "gpt\nkeychain:/Users/me/Library/Keychains/login.keychain-db".to_string();
    let model = StubModel::ok_text("ok");
    assert_eq!(
        run_model_text(&cfg, "prompt", &model),
        Some("ok".to_string())
    );

    let raw = fs::read_to_string(calls_log_path(root.path())).expect("raw log");
    let lines = read_call_lines(root.path());
    assert_eq!(
        lines.len(),
        1,
        "newline injection must not forge records: {raw:?}"
    );
    let line = &lines[0];
    let value: serde_json::Value = serde_json::from_str(line).expect("json line");
    let provider = value["provider"].as_str().expect("provider");
    let model_slug = value["model"].as_str().expect("model");

    // Intact forbidden forms cannot survive: scheme separators, env expansion,
    // path separators, and raw newlines.
    for forbidden in ["https://", "$HOME", "/Users/", "Authorization: ", "Bearer "] {
        assert!(
            !line.contains(forbidden),
            "accounting line leaked {forbidden}: {line}"
        );
        assert!(!provider.contains(forbidden), "{provider}");
        assert!(!model_slug.contains(forbidden), "{model_slug}");
    }
    assert!(!provider.contains('/'), "{provider}");
    assert!(!model_slug.contains('/'), "{model_slug}");
    assert!(!provider.chars().any(char::is_whitespace), "{provider}");
    assert!(!model_slug.chars().any(char::is_whitespace), "{model_slug}");
    assert!(
        provider
            .chars()
            .all(|ch| matches!(ch, 'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '_' | ':' | '-')),
        "{provider}"
    );
    assert!(
        model_slug
            .chars()
            .all(|ch| matches!(ch, 'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '_' | ':' | '-')),
        "{model_slug}"
    );
    assert!(provider.chars().count() <= 64, "{provider}");
    assert!(model_slug.chars().count() <= 64, "{model_slug}");
    assert_ne!(provider, "");
    assert_ne!(model_slug, "");
}

// ADR Accounting Amendment III D4: oversize responses still estimate tokens.
#[test]
fn oversize_output_records_nonzero_estimated_tokens() {
    let root = tempdir().expect("tempdir");
    let cfg = config_with_root(root.path());
    let body = "x".repeat(MAX_MODEL_OUTPUT_BYTES + 1);
    let expected_tokens = estimate_tokens(&body);
    assert!(expected_tokens > 0);
    let model = StubModel::ok_text(&body);
    assert_eq!(run_model_text(&cfg, "prompt", &model), None);
    assert_eq!(model.calls.get(), 1);

    let lines = read_call_lines(root.path());
    assert_eq!(lines.len(), 1);
    let value: serde_json::Value = serde_json::from_str(&lines[0]).expect("json line");
    assert_eq!(value["outcome"], "output-too-large");
    assert_eq!(value["token_source"], "estimated");
    assert_eq!(value["output_tokens"], expected_tokens);
    assert!(value["output_tokens"].as_u64().unwrap() > 0);
}

// ADR Accounting Amendment III D1: missing or unwritable roots stay non-fatal.
#[test]
fn accounting_write_failure_is_non_fatal() {
    let model = StubModel::ok_text("plain text");
    let none_cfg = config();
    assert_eq!(
        run_model_text(&none_cfg, "prompt", &model),
        Some("plain text".to_string())
    );

    let root = tempdir().expect("tempdir");
    let blocker = root.path().join(".ontocode/ontograph/km/_ops/authoring");
    fs::create_dir_all(blocker.parent().expect("parent")).expect("parent dirs");
    fs::write(&blocker, b"not-a-dir").expect("file blocker");
    let blocked_cfg = config_with_root(root.path());
    let model = StubModel::ok_text("plain text");
    assert_eq!(
        run_model_text(&blocked_cfg, "prompt", &model),
        Some("plain text".to_string())
    );
    assert!(!calls_log_path(root.path()).exists());

    #[cfg(unix)]
    {
        let ro = tempdir().expect("tempdir");
        let ops = ro.path().join(".ontocode/ontograph/km/_ops");
        fs::create_dir_all(&ops).expect("ops");
        let mut perms = fs::metadata(&ops).expect("meta").permissions();
        perms.set_mode(0o555);
        fs::set_permissions(&ops, perms).expect("chmod");
        let ro_cfg = config_with_root(ro.path());
        let model = StubModel::ok_text("plain text");
        assert_eq!(
            run_model_text(&ro_cfg, "prompt", &model),
            Some("plain text".to_string())
        );
        let mut perms = fs::metadata(&ops).expect("meta").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&ops, perms).expect("chmod restore");
    }
}

fn record_for(call_kind: KmLlmCallKind) -> KmLlmCallRecord {
    KmLlmCallRecord {
        call_kind,
        outcome: KmLlmCallOutcome::Completed,
        provider_id: "local".to_string(),
        model_slug: "test-model".to_string(),
        input_tokens: 11,
        output_tokens: 3,
        token_source: KmLlmTokenSource::Estimated,
    }
}

// OGRC-03: every client error exposes a stable reason code that does not vary
// with the provider- or parser-supplied message.
#[test]
fn client_error_reason_codes_are_stable_and_distinct() {
    let model_call = KmLlmClientError::ModelCall("connection reset by peer".to_string());
    let parse = KmLlmClientError::Parse("expected value at line 1".to_string());

    assert_eq!(model_call.reason_code(), "km_llm.model_call_failed");
    assert_eq!(parse.reason_code(), "km_llm.model_output_unparseable");
    assert_ne!(model_call.reason_code(), parse.reason_code());

    // The code is a property of the variant, not of the message text.
    let other_message = KmLlmClientError::ModelCall("504 gateway timeout".to_string());
    assert_eq!(model_call.reason_code(), other_message.reason_code());
}

// OGRC-03: the redacted diagnostic never echoes the underlying message, which
// can carry prompt text, model output, or a credential embedded in a URL.
#[test]
fn redacted_diagnostic_does_not_leak_sensitive_message_content() {
    let secret = "https://user:hunter2@provider.example/v1 responded with prompt echo";
    let error = KmLlmClientError::ModelCall(secret.to_string());

    let diagnostic = error.redacted_diagnostic();

    // No fragment of the sensitive message survives.
    assert!(!diagnostic.contains("hunter2"));
    assert!(!diagnostic.contains("provider.example"));
    assert!(!diagnostic.contains("prompt echo"));
    assert!(!diagnostic.contains(secret));

    // What remains is the stable code plus a non-identifying length.
    let expected = format!(
        "km_llm.model_call_failed (detail redacted, {} bytes)",
        secret.len()
    );
    assert_eq!(diagnostic, expected);

    // Display still carries the detail, so the redacted form is the one that is
    // safe to log and the two must not be confused.
    assert!(error.to_string().contains("hunter2"));
}

#[test]
fn extract_keeps_valid_rows_and_drops_invalid_ones() {
    let mut draft = sample_draft();
    let mut bad = draft.facts[0].clone();
    bad.id.clear();
    draft.facts.push(bad);
    let model = StubModel::ok(draft);
    assert_eq!(
        run_model_extract(&config(), "prompt", &model),
        Some(sample_draft())
    );
}

#[test]
fn extract_returns_none_when_no_row_is_valid() {
    let mut draft = sample_draft();
    draft.facts[0].id.clear();
    draft.mappings[0].relation.clear();
    let model = StubModel::ok(draft);
    assert_eq!(run_model_extract(&config(), "prompt", &model), None);
    assert_eq!(model.calls.get(), 1);
}

#[test]
fn draft_validation_accepts_the_row_cap_and_rejects_any_invalid_mapping() {
    let fact = sample_draft().facts.remove(0);
    let at_cap = KmLlmExtractDraft {
        facts: vec![fact; MAX_MODEL_DRAFT_ROWS],
        mappings: Vec::new(),
    };
    assert_eq!(
        validate_draft(at_cap).map(|draft| draft.facts.len()),
        Some(MAX_MODEL_DRAFT_ROWS)
    );

    let mut draft = sample_draft();
    let mut bad = draft.mappings[0].clone();
    bad.relation.clear();
    draft.mappings.push(bad);
    assert!(validate_draft(draft).is_none());

    let mut mappings_only = sample_draft();
    mappings_only.facts.clear();
    let kept = validate_draft(mappings_only).expect("valid mappings survive");
    assert_eq!(kept.mappings.len(), 1);
    assert!(kept.facts.is_empty());
}

#[test]
fn draft_validation_enforces_confidence_and_length_bounds_and_keeps_fact_order() {
    let base_fact = sample_draft().facts.remove(0);
    let base_mapping = sample_draft().mappings.remove(0);

    for confidence in [0.0, 1.0] {
        let mut fact = base_fact.clone();
        fact.confidence = confidence;
        assert!(validate_draft_fact(&fact), "{confidence} is in range");
        let mut mapping = base_mapping.clone();
        mapping.confidence = confidence;
        assert!(validate_draft_mapping(&mapping), "{confidence} is in range");
    }
    for confidence in [-0.01, 1.01, f64::NAN, f64::INFINITY] {
        let mut fact = base_fact.clone();
        fact.confidence = confidence;
        assert!(!validate_draft_fact(&fact), "{confidence} is out of range");
        let mut mapping = base_mapping.clone();
        mapping.confidence = confidence;
        assert!(
            !validate_draft_mapping(&mapping),
            "{confidence} is out of range"
        );
    }

    let mut at_limit = base_fact.clone();
    at_limit.claim = "x".repeat(MAX_MODEL_FIELD_CHARS);
    assert!(validate_draft_fact(&at_limit));
    let mut over_limit = base_fact.clone();
    over_limit.claim = "x".repeat(MAX_MODEL_FIELD_CHARS + 1);
    assert!(!validate_draft_fact(&over_limit));
    let mut blank = base_fact.clone();
    blank.claim = "   ".to_string();
    assert!(!validate_draft_fact(&blank));

    let ids = ["a", "b", "c"];
    let mut facts = Vec::new();
    for id in ids {
        let mut fact = base_fact.clone();
        fact.id = id.to_string();
        facts.push(fact);
    }
    facts[1].confidence = 2.0;
    let kept = validate_draft(KmLlmExtractDraft {
        facts,
        mappings: vec![base_mapping],
    })
    .expect("valid rows survive");
    let kept_ids: Vec<&str> = kept.facts.iter().map(|fact| fact.id.as_str()).collect();
    assert_eq!(kept_ids, ["a", "c"]);
    assert_eq!(kept.mappings.len(), 1);
}

#[test]
fn draft_validation_bounds_mapping_count_and_rejects_all_invalid_facts() {
    let mapping = sample_draft().mappings.remove(0);
    let at_cap = KmLlmExtractDraft {
        facts: Vec::new(),
        mappings: vec![mapping.clone(); MAX_MODEL_DRAFT_ROWS],
    };
    assert_eq!(
        validate_draft(at_cap).map(|draft| draft.mappings.len()),
        Some(MAX_MODEL_DRAFT_ROWS)
    );
    let over_cap = KmLlmExtractDraft {
        facts: Vec::new(),
        mappings: vec![mapping; MAX_MODEL_DRAFT_ROWS + 1],
    };
    assert!(validate_draft(over_cap).is_none());

    let mut bad_fact = sample_draft().facts.remove(0);
    bad_fact.id.clear();
    let only_bad_facts = KmLlmExtractDraft {
        facts: vec![bad_fact],
        mappings: Vec::new(),
    };
    assert!(validate_draft(only_bad_facts).is_none());
}
