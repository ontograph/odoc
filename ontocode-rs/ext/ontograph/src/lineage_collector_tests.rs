use super::*;
use pretty_assertions::assert_eq;

fn record(run_id: &str, case_id: &str, variant: Variant) -> RunRecord {
    RunRecord {
        run_id: run_id.to_string(),
        case_id: case_id.to_string(),
        variant,
        evidence_class: EvidenceClass::Offline,
        corpus_digest: "digest".to_string(),
        prompt_revision: "prompt".to_string(),
        config_revision: "config".to_string(),
        schema_revision: "schema".to_string(),
        model_id: None,
        request_hash: None,
        admission: None,
        terminal: Terminal::Completed {
            answer_ref: format!("answer-{run_id}"),
        },
        elapsed_ms: None,
        manifest: RunManifest {
            corpus_revision: "corpus".to_string(),
            order_seed: 7,
            provider_id: None,
            started_at: None,
        },
        demand_coverage: None,
        provenance_validity: None,
        answer_quality: None,
    }
}

fn pair(case_id: &str) -> [RunRecord; 2] {
    [
        record(&format!("{case_id}-baseline"), case_id, Variant::Baseline),
        record(&format!("{case_id}-lineage"), case_id, Variant::Lineage),
    ]
}

fn invalid(case_id: Option<&str>, run_ids: &[&str], reason: InvalidReason) -> Invalid {
    Invalid {
        case_id: case_id.map(str::to_owned),
        run_ids: run_ids.iter().map(|id| (*id).to_string()).collect(),
        reason,
    }
}

fn live_pair(case_id: &str) -> [RunRecord; 2] {
    let mut runs = pair(case_id);
    for run in &mut runs {
        run.evidence_class = EvidenceClass::Live;
        run.model_id = Some("model".to_string());
        run.request_hash =
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string());
        run.manifest.provider_id = Some("provider".to_string());
        run.manifest.started_at = Some("timestamp".to_string());
    }
    runs
}

#[test]
fn pair_completeness() {
    assert_eq!(
        collect(&[record("orphan", "orphan", Variant::Baseline)]),
        Err(CollectError::Invalid(vec![invalid(
            Some("orphan"),
            &["orphan"],
            InvalidReason::IncompletePair,
        )]))
    );
    let mut duplicate = pair("duplicate").to_vec();
    duplicate.push(record("extra", "duplicate", Variant::Baseline));
    assert_eq!(
        collect(&duplicate),
        Err(CollectError::Invalid(vec![invalid(
            Some("duplicate"),
            &["extra"],
            InvalidReason::DuplicateVariant,
        )]))
    );

    let mut missing = pair("missing");
    missing[1].terminal = Terminal::Missing(MissingCategory::ProviderTimeout);
    assert_eq!(
        collect(&missing),
        Ok(PairedReport {
            tables: BTreeMap::from([(
                EvidenceClass::Offline,
                BTreeMap::from([
                    (
                        Variant::Baseline,
                        VariantTable {
                            completed_runs: 1,
                            ..VariantTable::default()
                        }
                    ),
                    (
                        Variant::Lineage,
                        VariantTable {
                            missing: BTreeMap::from([(MissingCategory::ProviderTimeout, 1)]),
                            ..VariantTable::default()
                        }
                    ),
                ])
            )]),
            pairs: vec![PairRow {
                baseline: missing[0].clone(),
                lineage: missing[1].clone(),
                delta: PairedDelta::default(),
            }],
        })
    );
}

type Mutation = fn(&mut RunRecord);

#[test]
fn manifest_validation() {
    let mutations: [(&str, Mutation); 6] = [
        ("corpus_digest", |r| r.corpus_digest.push('x')),
        ("prompt_revision", |r| r.prompt_revision.push('x')),
        ("config_revision", |r| r.config_revision.push('x')),
        ("schema_revision", |r| r.schema_revision.push('x')),
        ("manifest.corpus_revision", |r| {
            r.manifest.corpus_revision.push('x')
        }),
        ("manifest.order_seed", |r| r.manifest.order_seed += 1),
    ];
    for (field, mutate) in mutations {
        let mut runs = pair("revision");
        mutate(&mut runs[1]);
        assert_eq!(
            collect(&runs),
            Err(CollectError::Invalid(vec![invalid(
                /*case_id*/ None,
                &["revision-baseline", "revision-lineage"],
                InvalidReason::RevisionMismatch {
                    field: field.to_string()
                },
            )]))
        );
    }

    let mut mock = pair("model");
    for run in &mut mock {
        run.evidence_class = EvidenceClass::Mock;
        run.model_id = Some("model-a".to_string());
        run.request_hash =
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string());
    }
    mock[1].model_id = Some("model-b".to_string());
    assert_eq!(
        collect(&mock),
        Err(CollectError::Invalid(vec![invalid(
            Some("model"),
            &["model-baseline", "model-lineage"],
            InvalidReason::RevisionMismatch {
                field: "model_id".to_string()
            },
        )]))
    );

    let mut live = live_pair("live");
    live[0].manifest.provider_id = None;
    live[0].manifest.started_at = None;
    assert_eq!(
        collect(&live),
        Err(CollectError::Invalid(vec![
            invalid(
                Some("live"),
                &["live-baseline"],
                InvalidReason::MissingManifestInput {
                    field: "manifest.provider_id".to_string()
                }
            ),
            invalid(
                Some("live"),
                &["live-baseline"],
                InvalidReason::MissingManifestInput {
                    field: "manifest.started_at".to_string()
                }
            ),
        ]))
    );

    let mut mock = pair("mock");
    for run in &mut mock {
        run.evidence_class = EvidenceClass::Mock;
        run.model_id = Some("model".to_string());
        run.request_hash =
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string());
    }
    mock[0].request_hash = None;
    assert_eq!(
        collect(&mock),
        Err(CollectError::Invalid(vec![invalid(
            Some("mock"),
            &["mock-baseline"],
            InvalidReason::MissingManifestInput {
                field: "request_hash".to_string()
            },
        )]))
    );
}

#[test]
fn duplicate_run_id_across_cases_and_classes() {
    let mut runs = pair("offline").to_vec();
    let mut mock = pair("mock");
    for run in &mut mock {
        run.evidence_class = EvidenceClass::Mock;
        run.model_id = Some("model".to_string());
        run.request_hash =
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string());
    }
    mock[0].run_id = runs[0].run_id.clone();
    runs.extend(mock);
    assert_eq!(
        collect(&runs),
        Err(CollectError::Invalid(vec![invalid(
            /*case_id*/ None,
            &["offline-baseline", "offline-baseline"],
            InvalidReason::DuplicateRunId,
        )]))
    );
}

#[test]
fn missing_never_passes() {
    let categories = [
        MissingCategory::NoAdmissionEvent,
        MissingCategory::StreamDropped,
        MissingCategory::DeadlineBudgetExhausted,
        MissingCategory::ProviderTimeout,
        MissingCategory::ProviderError,
        MissingCategory::UnknownRemoteState,
    ];
    let mut runs = Vec::new();
    for (i, category) in categories.into_iter().enumerate() {
        let mut current = pair(&format!("missing-{i}"));
        current[0].terminal = Terminal::Missing(category);
        if category != MissingCategory::NoAdmissionEvent {
            current[0].admission = Some((
                Outcome::Baseline,
                Admission::Rejected,
                FallbackCategory::RetrievalFailed,
            ));
        }
        runs.extend(current);
    }
    let report = collect(&runs).unwrap();
    assert_eq!(
        report.tables[&EvidenceClass::Offline][&Variant::Baseline],
        VariantTable {
            missing: categories
                .into_iter()
                .map(|category| (category, 1))
                .collect(),
            ..VariantTable::default()
        }
    );
    assert_eq!(
        report
            .pairs
            .iter()
            .map(|p| p.delta.clone())
            .collect::<Vec<_>>(),
        vec![PairedDelta::default(); /*missing_categories*/6]
    );
}

#[test]
fn classes_never_pooled() {
    let mut runs = Vec::new();
    for (case, class) in [
        ("offline", EvidenceClass::Offline),
        ("mock", EvidenceClass::Mock),
        ("live", EvidenceClass::Live),
    ] {
        let mut current = pair(case);
        for run in &mut current {
            run.evidence_class = class;
            run.demand_coverage = Some(Counts {
                numerator: 1,
                denominator: 2,
            });
            match class {
                EvidenceClass::Offline => {}
                EvidenceClass::Mock => {
                    run.model_id = Some("model".to_string());
                    run.request_hash = Some(
                        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                            .to_string(),
                    );
                }
                EvidenceClass::Live => {
                    run.model_id = Some("model".to_string());
                    run.request_hash = Some(
                        "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                            .to_string(),
                    );
                    run.manifest.provider_id = Some("provider".to_string());
                    run.manifest.started_at = Some("timestamp".to_string());
                }
            }
        }
        runs.extend(current);
    }
    let report = collect(&runs).unwrap();
    let expected_variant = VariantTable {
        completed_runs: 1,
        demand_coverage: Metric {
            counts: Counts {
                numerator: 1,
                denominator: 2,
            },
            measured_runs: 1,
            basis_points: Some(5000),
        },
        ..VariantTable::default()
    };
    let expected_tables = [
        EvidenceClass::Offline,
        EvidenceClass::Mock,
        EvidenceClass::Live,
    ]
    .into_iter()
    .map(|class| {
        (
            class,
            BTreeMap::from([
                (Variant::Baseline, expected_variant.clone()),
                (Variant::Lineage, expected_variant.clone()),
            ]),
        )
    })
    .collect();
    assert_eq!(report.tables, expected_tables);
    assert_eq!(
        report
            .pairs
            .iter()
            .map(|pair| pair.baseline.case_id.as_str())
            .collect::<Vec<_>>(),
        vec!["offline", "mock", "live"]
    );
    assert_eq!(report.pairs.len(), 3);
    let offline = report.tables[&EvidenceClass::Offline][&Variant::Baseline].clone();
    let mut changed = runs.clone();
    changed
        .iter_mut()
        .find(|r| r.evidence_class == EvidenceClass::Mock && r.variant == Variant::Baseline)
        .unwrap()
        .demand_coverage = Some(Counts {
        numerator: 0,
        denominator: 1,
    });
    assert_eq!(
        collect(&changed).unwrap().tables[&EvidenceClass::Offline][&Variant::Baseline],
        offline
    );
}

#[test]
fn explicit_denominators() {
    let mut a = pair("denom-a");
    a[0].demand_coverage = Some(Counts {
        numerator: 1,
        denominator: 2,
    });
    a[0].admission = Some((
        Outcome::Baseline,
        Admission::Disabled,
        FallbackCategory::Disabled,
    ));
    a[1].demand_coverage = Some(Counts {
        numerator: 0,
        denominator: 0,
    });
    let mut b = pair("denom-b");
    b[0].demand_coverage = Some(Counts {
        numerator: 1,
        denominator: 3,
    });
    b[1].demand_coverage = Some(Counts {
        numerator: 0,
        denominator: 0,
    });
    let mut runs = a.to_vec();
    runs.extend(b);
    let report = collect(&runs).unwrap();
    assert_eq!(
        report.tables[&EvidenceClass::Offline][&Variant::Baseline],
        VariantTable {
            completed_runs: 2,
            demand_coverage: Metric {
                counts: Counts {
                    numerator: 2,
                    denominator: 5
                },
                measured_runs: 2,
                basis_points: Some(4000)
            },
            fallback_rate: Metric {
                counts: Counts {
                    numerator: 1,
                    denominator: 1
                },
                measured_runs: 1,
                basis_points: Some(10000)
            },
            fallback_categories: BTreeMap::from([(FallbackCategory::Disabled, 1)]),
            ..VariantTable::default()
        }
    );
    assert_eq!(
        report.tables[&EvidenceClass::Offline][&Variant::Lineage],
        VariantTable {
            completed_runs: 2,
            demand_coverage: Metric {
                counts: Counts {
                    numerator: 0,
                    denominator: 0
                },
                measured_runs: 2,
                basis_points: None
            },
            ..VariantTable::default()
        }
    );
}

#[test]
fn latency_measurements() {
    assert_eq!(
        collect(&pair("unmeasured")).unwrap().tables[&EvidenceClass::Offline][&Variant::Baseline]
            .latency,
        Latency::default()
    );
    for (n, p50, p95) in [(1_u64, 1_u64, 1_u64), (2, 1, 2), (5, 3, 5), (20, 10, 19)] {
        let mut runs = Vec::new();
        for ms in 1..=n {
            let mut current = pair(&format!("latency-{n}-{ms}"));
            current[0].elapsed_ms = Some(ms);
            runs.extend(current);
        }
        assert_eq!(
            collect(&runs).unwrap().tables[&EvidenceClass::Offline][&Variant::Baseline].latency,
            Latency {
                measured_runs: n,
                p50_ms: Some(p50),
                p95_ms: Some(p95)
            }
        );
    }
}

#[test]
fn deterministic_ordering() {
    let mut runs = pair("b").to_vec();
    runs.extend(pair("a"));
    let first = collect(&runs).unwrap();
    runs.reverse();
    let second = collect(&runs).unwrap();
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );

    let mut bad = pair("duplicate").to_vec();
    bad.push(record("extra", "duplicate", Variant::Baseline));
    bad.push(record("orphan", "orphan", Variant::Lineage));
    let first = collect(&bad).unwrap_err();
    bad.reverse();
    let second = collect(&bad).unwrap_err();
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );

    let mut r = record("quality", "quality", Variant::Baseline);
    r.answer_quality = Some(AnswerQuality {
        rubric_id: "rubric".to_string(),
        score: None,
    });
    let round_trip: RunRecord = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert_eq!(round_trip, r);
    assert_eq!(round_trip.answer_quality, r.answer_quality);
}

#[test]
fn invalid_admission() {
    let mut tuple = pair("tuple");
    tuple[0].admission = Some((
        Outcome::Lineage,
        Admission::Rejected,
        FallbackCategory::None,
    ));
    let mut no_event = pair("no-event");
    no_event[0].terminal = Terminal::Missing(MissingCategory::NoAdmissionEvent);
    no_event[0].admission = Some((
        Outcome::Baseline,
        Admission::Disabled,
        FallbackCategory::Disabled,
    ));
    let mut counts = pair("counts");
    counts[0].demand_coverage = Some(Counts {
        numerator: 2,
        denominator: 1,
    });
    let mut runs = tuple.to_vec();
    runs.extend(no_event);
    runs.extend(counts);
    assert_eq!(
        collect(&runs),
        Err(CollectError::Invalid(vec![
            invalid(
                Some("counts"),
                &["counts-baseline"],
                InvalidReason::InvalidObservation
            ),
            invalid(
                Some("no-event"),
                &["no-event-baseline"],
                InvalidReason::InvalidAdmission
            ),
            invalid(
                Some("tuple"),
                &["tuple-baseline"],
                InvalidReason::InvalidAdmission
            ),
        ]))
    );
}

#[test]
fn overflow() {
    let mut a = pair("overflow-a");
    a[0].demand_coverage = Some(Counts {
        numerator: 0,
        denominator: u64::MAX,
    });
    let mut b = pair("overflow-b");
    b[0].demand_coverage = Some(Counts {
        numerator: 0,
        denominator: 1,
    });
    let mut runs = a.to_vec();
    runs.extend(b);
    assert_eq!(collect(&runs), Err(CollectError::Overflow));
}

const MOCK_RECORDS: &str = include_str!("fixtures/lineage_rollout/mock_records.jsonl");

#[test]
fn fixture_ingests_and_collects() {
    let report = collect_jsonl(MOCK_RECORDS.as_bytes()).expect("fixture collects");
    let mock = &report.tables[&EvidenceClass::Mock];
    assert_eq!(report.tables.len(), 1);
    let lineage = &mock[&Variant::Lineage];
    let baseline = &mock[&Variant::Baseline];
    assert_eq!(
        lineage.fallback_categories,
        BTreeMap::from([
            (FallbackCategory::None, 1),
            (FallbackCategory::SourceStale, 1)
        ])
    );
    assert_eq!(
        baseline.fallback_categories,
        BTreeMap::from([(FallbackCategory::Disabled, 3)])
    );
    assert_eq!(
        lineage.missing,
        BTreeMap::from([(MissingCategory::ProviderTimeout, 1)])
    );
    assert!(baseline.missing.is_empty());
    assert_eq!(
        report
            .pairs
            .iter()
            .map(|pair| pair.baseline.case_id.as_str())
            .collect::<Vec<_>>(),
        vec!["case-a", "case-b", "case-c"]
    );
}

#[test]
fn render_report_is_deterministic() {
    let forward = collect_jsonl(MOCK_RECORDS.as_bytes()).expect("collects");
    let reversed_text = MOCK_RECORDS.lines().rev().collect::<Vec<_>>().join("\n");
    let reversed = collect_jsonl(reversed_text.as_bytes()).expect("collects");
    let first = render_report(&forward).expect("renders");
    assert_eq!(first, render_report(&reversed).expect("renders"));
    assert!(first.ends_with('\n'));
}

/// The Python decision rule (`lra_decision.py`) reads these exact key paths from
/// the rendered report. This pins them so a rename here fails loudly.
#[test]
fn rendered_report_keeps_the_key_paths_the_decision_rule_reads() {
    let report = collect_jsonl(MOCK_RECORDS.as_bytes()).expect("collects");
    let value: serde_json::Value =
        serde_json::from_str(&render_report(&report).expect("renders")).expect("valid JSON");
    for variant in ["baseline", "lineage"] {
        let table = &value["tables"]["mock"][variant];
        assert!(table["demand_coverage"].get("basis_points").is_some());
        assert!(table["provenance_validity"].get("basis_points").is_some());
        assert!(table["fallback_rate"].get("basis_points").is_some());
        assert!(table["latency"].get("p95_ms").is_some());
    }
}

#[test]
fn ingest_skips_blank_lines_and_reports_line_numbers() {
    let first = MOCK_RECORDS.lines().next().expect("first record");
    let text = format!("{first}\n\n   \n{{\"not\":\"a record\"}}\n");
    let error = ingest_jsonl(text.as_bytes()).expect_err("bad fourth line");
    assert_eq!(error.line, 4);
    assert!(matches!(error.reason, IngestReason::Parse(_)));
}

#[test]
fn ingest_rejects_unknown_fields() {
    let first = MOCK_RECORDS.lines().next().expect("first record");
    let text = first.replacen('{', "{\"extra\":1,", 1);
    let error = ingest_jsonl(text.as_bytes()).expect_err("unknown field");
    assert!(matches!(error.reason, IngestReason::Parse(_)));
}

struct CountingReader {
    remaining: usize,
    consumed: usize,
}

impl std::io::Read for CountingReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = buf.len().min(self.remaining).min(4096);
        buf[..n].fill(b'x');
        self.remaining -= n;
        self.consumed += n;
        Ok(n)
    }
}

#[test]
fn ingest_rejects_overlong_line_without_buffering_it() {
    let mut source = CountingReader {
        remaining: usize::MAX / 2,
        consumed: 0,
    };
    let error = ingest_jsonl(std::io::BufReader::with_capacity(4096, &mut source))
        .expect_err("endless line");
    assert_eq!(error.reason, IngestReason::LineTooLong);
    assert!(source.consumed <= MAX_JSONL_LINE_BYTES + 2 * 4096);
}

#[test]
fn ingest_rejects_too_many_records() {
    struct Endless(Vec<u8>, usize);
    impl std::io::Read for Endless {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let n = buf.len().min(self.0.len() - self.1);
            buf[..n].copy_from_slice(&self.0[self.1..self.1 + n]);
            self.1 = (self.1 + n) % self.0.len();
            Ok(n)
        }
    }
    let first = MOCK_RECORDS.lines().next().expect("first record");
    let mut line = first.as_bytes().to_vec();
    line.push(b'\n');
    let error =
        ingest_jsonl(std::io::BufReader::new(Endless(line, 0))).expect_err("endless records");
    assert_eq!(error.reason, IngestReason::TooManyRecords);
}

#[test]
fn ingest_rejects_unbounded_and_control_identifiers() {
    let first = MOCK_RECORDS.lines().next().expect("first record");
    let long = first.replacen("a-base", &"r".repeat(MAX_IDENTIFIER_CHARS + 1), 1);
    assert_eq!(
        ingest_jsonl(long.as_bytes()).expect_err("long id").reason,
        IngestReason::UnboundedIdentifier { field: "run_id" }
    );
    let control = first.replacen("case-a", "case\\u0007a", 1);
    assert_eq!(
        ingest_jsonl(control.as_bytes())
            .expect_err("control")
            .reason,
        IngestReason::UnboundedIdentifier { field: "case_id" }
    );
}

#[test]
fn ingest_rejects_raw_text_in_answer_ref() {
    let first = MOCK_RECORDS.lines().next().expect("first record");
    let value: serde_json::Value = serde_json::from_str(first).expect("json");
    let answer = value["terminal"]["completed"]["answer_ref"]
        .as_str()
        .expect("answer ref")
        .to_string();
    let text = first.replacen(&answer, "the model said hello", 1);
    assert_eq!(
        ingest_jsonl(text.as_bytes()).expect_err("raw text").reason,
        IngestReason::RawTextInReference {
            field: "answer_ref"
        }
    );
}

#[test]
fn parse_errors_never_echo_the_line() {
    let text = "{\"run_id\": \"sk-live-SECRET\", oops\n";
    let error = ingest_jsonl(text.as_bytes()).expect_err("invalid json");
    assert!(!format!("{error:?}").contains("sk-live-SECRET"));
}

#[test]
fn collect_error_is_wrapped() {
    let first = MOCK_RECORDS.lines().next().expect("first record");
    let error = collect_jsonl(first.as_bytes()).expect_err("incomplete pair");
    assert!(matches!(
        error,
        CollectJsonlError::Collect(CollectError::Invalid(_))
    ));
}
