// The current fixture schema cannot encode stale or missing sources, scope
// mismatches, or marker violations; those rollout cases require a schema change.
use pretty_assertions::assert_eq;

use super::*;

fn passage(
    passage_id: &str,
    path: &str,
    start_line: usize,
    end_line: usize,
    terms: &[&str],
    token_cost: usize,
) -> ReplayPassage {
    ReplayPassage {
        passage_id: passage_id.to_string(),
        path: path.to_string(),
        start_line,
        end_line,
        terms: terms.iter().map(|term| (*term).to_string()).collect(),
        token_cost,
    }
}

fn demand(demand_id: &str, description: &str, satisfying: &[&str]) -> ReplayDemand {
    ReplayDemand {
        demand_id: demand_id.to_string(),
        description: description.to_string(),
        satisfying_passages: satisfying
            .iter()
            .map(|passage_id| (*passage_id).to_string())
            .collect(),
    }
}

/// Two-hop corpus: the question wording only matches the first hop, so the
/// second hop is reachable only when a variant induces demands.
fn fixture() -> ReplayFixture {
    ReplayFixture {
        corpus_id: "lineage-replay-v1".to_string(),
        prompt_id: "lineage-replay-prompt-v1".to_string(),
        model_settings: "temperature=0;top_p=1;seed=0".to_string(),
        passages: vec![
            passage(
                "p-dispatch",
                "ontocode-rs/core/src/dispatch.rs",
                10,
                24,
                &["dispatch", "lease", "reservation"],
                40,
            ),
            passage(
                "p-revision",
                "ontocode-rs/core/src/revision.rs",
                5,
                18,
                &["revision", "tracking", "hash"],
                35,
            ),
            passage(
                "p-noise",
                "ontocode-rs/core/src/unrelated.rs",
                1,
                9,
                &["telemetry", "banner"],
                60,
            ),
        ],
        cases: vec![ReplayCase {
            case_id: "case-dispatch-revision".to_string(),
            question_terms: vec!["dispatch".to_string(), "lease".to_string()],
            demands: vec![
                demand("d-dispatch", "dispatch lease", &["p-dispatch"]),
                demand("d-revision", "revision tracking hash", &["p-revision"]),
            ],
            required_passages: ["p-dispatch", "p-revision"]
                .iter()
                .map(|passage_id| (*passage_id).to_string())
                .collect(),
        }],
        max_selected_passages: 2,
    }
}

#[test]
fn demand_induction_recovers_the_second_hop_that_baseline_misses() {
    let report = run_replay(&fixture()).expect("valid fixture");

    let baseline = report
        .metrics_for(ReplayVariant::Baseline)
        .expect("baseline metrics");
    let decomposition = report
        .metrics_for(ReplayVariant::DecompositionOnly)
        .expect("decomposition metrics");

    assert_eq!(baseline.demand_recall_bp, 5_000);
    assert_eq!(baseline.answer_quality_bp, 0);
    assert_eq!(baseline.failure_count, 1);

    assert_eq!(decomposition.demand_recall_bp, 10_000);
    assert_eq!(decomposition.answer_quality_bp, 10_000);
    assert_eq!(decomposition.passage_coverage_bp, 10_000);
    assert_eq!(decomposition.failure_count, 0);
}

#[test]
fn only_the_provenance_variant_records_source_spans_and_demand_identity() {
    let report = run_replay(&fixture()).expect("valid fixture");

    for variant in [ReplayVariant::Baseline, ReplayVariant::DecompositionOnly] {
        let metrics = report.metrics_for(variant).expect("metrics");
        assert_eq!(metrics.provenance_bearing_selections, 0);
    }

    let provenance = report
        .metrics_for(ReplayVariant::ProvenanceAware)
        .expect("provenance metrics");
    assert_eq!(provenance.provenance_bearing_selections, 2);

    let outcomes = report
        .outcomes
        .get("provenance_aware")
        .expect("provenance outcomes");
    // Rank follows score: `d-revision` matches three passage terms while
    // `d-dispatch` matches two, so the revision hop is exposed first.
    let selections = &outcomes[0].selections;
    assert_eq!(
        selections[0].source_span.as_deref(),
        Some("ontocode-rs/core/src/revision.rs:5-18")
    );
    assert_eq!(selections[0].demand_id.as_deref(), Some("d-revision"));
    assert_eq!(selections[0].rank, 1);
    assert_eq!(selections[0].score, 3);
    assert_eq!(
        selections[1].source_span.as_deref(),
        Some("ontocode-rs/core/src/dispatch.rs:10-24")
    );
    assert_eq!(selections[1].demand_id.as_deref(), Some("d-dispatch"));
    assert_eq!(selections[1].rank, 2);
    assert_eq!(selections[1].score, 2);
}

#[test]
fn token_and_latency_cost_are_recorded_per_variant() {
    let report = run_replay(&fixture()).expect("valid fixture");

    let baseline = report
        .metrics_for(ReplayVariant::Baseline)
        .expect("baseline metrics");
    let decomposition = report
        .metrics_for(ReplayVariant::DecompositionOnly)
        .expect("decomposition metrics");

    // Baseline scores three passages once; demand induction scores them once
    // per demand, which is the cost of the recall gain.
    assert_eq!(baseline.total_latency_units, 3);
    assert_eq!(decomposition.total_latency_units, 6);
    assert_eq!(baseline.total_token_cost, 40);
    assert_eq!(decomposition.total_token_cost, 75);
}

#[test]
fn attribution_credits_demand_induction_when_it_drives_the_recall_gain() {
    let report = run_replay(&fixture()).expect("valid fixture");

    assert_eq!(
        report.attribution(),
        ReplayAttribution::DemandInduction { gain_bp: 5_000 }
    );
}

#[test]
fn attribution_credits_grounding_when_recall_is_already_saturated() {
    let mut fixture = fixture();
    // Give the baseline question terms that reach both hops, so the only
    // remaining difference between variants is retained provenance.
    fixture.cases[0].question_terms = vec![
        "dispatch".to_string(),
        "lease".to_string(),
        "revision".to_string(),
        "tracking".to_string(),
    ];

    let report = run_replay(&fixture).expect("valid fixture");
    let baseline = report
        .metrics_for(ReplayVariant::Baseline)
        .expect("baseline metrics");
    assert_eq!(baseline.demand_recall_bp, 10_000);
    assert_eq!(
        report.attribution(),
        ReplayAttribution::Grounding {
            additional_grounded_selections: 2
        }
    );
}

#[test]
fn replay_is_byte_identical_across_runs() {
    let fixture = fixture();
    let first = run_replay(&fixture).expect("valid fixture");
    let second = run_replay(&fixture).expect("valid fixture");

    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_string(&first).expect("serialize first"),
        serde_json::to_string(&second).expect("serialize second")
    );
}

#[test]
fn recorded_metric_table_matches_the_published_evidence_artifact() {
    // Locks the numbers published in
    // `.memory-bank/knowledge-hub/LINEAGERAG_REPLAY_EVALUATION.md`.
    let report = run_replay(&fixture()).expect("valid fixture");
    let observed = report
        .metrics
        .iter()
        .map(|metrics| {
            (
                metrics.variant,
                metrics.answer_quality_bp,
                metrics.demand_recall_bp,
                metrics.passage_coverage_bp,
                metrics.total_token_cost,
                metrics.total_latency_units,
                metrics.failure_count,
                metrics.provenance_bearing_selections,
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        observed,
        vec![
            (ReplayVariant::Baseline, 0, 5_000, 5_000, 40, 3, 1, 0),
            (
                ReplayVariant::DecompositionOnly,
                10_000,
                10_000,
                10_000,
                75,
                6,
                0,
                0
            ),
            (
                ReplayVariant::ProvenanceAware,
                10_000,
                10_000,
                10_000,
                75,
                6,
                0,
                2
            ),
        ]
    );
}

#[test]
fn report_retains_the_recorded_reproducibility_settings() {
    let report = run_replay(&fixture()).expect("valid fixture");

    assert_eq!(report.corpus_id, "lineage-replay-v1");
    assert_eq!(report.prompt_id, "lineage-replay-prompt-v1");
    assert_eq!(report.model_settings, "temperature=0;top_p=1;seed=0");
    assert_eq!(report.metrics.len(), ReplayVariant::ALL.len());
    assert_eq!(report.outcomes.len(), ReplayVariant::ALL.len());
}

#[test]
fn empty_corpus_fails_closed() {
    let mut without_cases = fixture();
    without_cases.cases.clear();
    assert_eq!(
        run_replay(&without_cases),
        Err(ReplayFixtureError::EmptyCorpus)
    );

    let mut without_passages = fixture();
    without_passages.passages.clear();
    assert_eq!(
        run_replay(&without_passages),
        Err(ReplayFixtureError::EmptyCorpus)
    );
}

#[test]
fn missing_model_settings_fails_closed() {
    let mut blank_settings = fixture();
    blank_settings.model_settings = "   ".to_string();
    assert_eq!(
        run_replay(&blank_settings),
        Err(ReplayFixtureError::MissingModelSettings)
    );

    let mut blank_prompt = fixture();
    blank_prompt.prompt_id = String::new();
    assert_eq!(
        run_replay(&blank_prompt),
        Err(ReplayFixtureError::MissingModelSettings)
    );
}

#[test]
fn duplicate_passage_id_fails_closed() {
    let mut fixture = fixture();
    let duplicate = fixture.passages[0].clone();
    fixture.passages.push(duplicate);

    assert_eq!(
        run_replay(&fixture),
        Err(ReplayFixtureError::DuplicatePassageId("p-dispatch".into()))
    );
}

#[test]
fn unknown_required_passage_fails_closed() {
    let mut fixture = fixture();
    fixture.cases[0]
        .required_passages
        .insert("p-missing".to_string());

    assert_eq!(
        run_replay(&fixture),
        Err(ReplayFixtureError::UnknownPassageReference {
            case_id: "case-dispatch-revision".to_string(),
            passage_id: "p-missing".to_string(),
        })
    );
}

#[test]
fn unknown_demand_passage_fails_closed() {
    let mut fixture = fixture();
    fixture.cases[0].demands[1]
        .satisfying_passages
        .insert("p-absent".to_string());

    assert_eq!(
        run_replay(&fixture),
        Err(ReplayFixtureError::UnknownPassageReference {
            case_id: "case-dispatch-revision".to_string(),
            passage_id: "p-absent".to_string(),
        })
    );
}

#[test]
fn zero_selection_budget_fails_closed() {
    let mut fixture = fixture();
    fixture.max_selected_passages = 0;

    assert_eq!(
        run_replay(&fixture),
        Err(ReplayFixtureError::ZeroSelectionBudget)
    );
}

#[test]
fn selection_budget_bounds_selections_and_reports_uncovered_demands() {
    let mut fixture = fixture();
    fixture.max_selected_passages = 1;

    let report = run_replay(&fixture).expect("valid fixture");
    let outcome = &report
        .outcomes
        .get("provenance_aware")
        .expect("provenance outcomes")[0];

    assert_eq!(outcome.selections.len(), 1);
    assert_eq!(outcome.selections[0].passage_id, "p-revision");
    assert_eq!(outcome.uncovered_demands, vec!["d-dispatch".to_string()]);
    assert!(!outcome.answered_correctly);
}

#[test]
fn fixture_errors_render_actionable_messages() {
    assert_eq!(
        ReplayFixtureError::EmptyCorpus.to_string(),
        "replay fixture requires at least one case and passage"
    );
    assert_eq!(
        ReplayFixtureError::DuplicatePassageId("p-dispatch".into()).to_string(),
        "replay fixture has duplicate passage id `p-dispatch`"
    );
    assert_eq!(
        ReplayFixtureError::UnknownPassageReference {
            case_id: "case-1".to_string(),
            passage_id: "p-missing".to_string(),
        }
        .to_string(),
        "case `case-1` references unknown passage `p-missing`"
    );
    assert_eq!(
        ReplayFixtureError::ZeroSelectionBudget.to_string(),
        "replay fixture requires a non-zero passage budget"
    );
    assert_eq!(
        ReplayFixtureError::MissingModelSettings.to_string(),
        "replay fixture requires recorded prompt and model settings"
    );
}

/// Thresholds used by the published artifact, fixed before comparison.
///
/// Recall and coverage must be complete, and the recall gain may cost at most
/// twice the baseline token spend. Provider evidence is not demanded here so
/// the offline gate can be measured on its own; the test below covers the
/// provider requirement separately.
fn published_thresholds() -> RolloutThresholds {
    RolloutThresholds {
        min_demand_recall_bp: 10_000,
        min_passage_coverage_bp: 10_000,
        max_token_cost_ratio_bp: 20_000,
        require_provider_backed: false,
    }
}

#[test]
fn the_measured_lineage_path_clears_the_published_rollout_thresholds() {
    let report = run_replay(&fixture()).expect("valid fixture");

    // 75 tokens against a 40-token baseline is 18_750 bp, inside the 20_000 cap.
    assert_eq!(
        report.evaluate(published_thresholds()),
        RolloutVerdict::Ready
    );
}

#[test]
fn a_run_that_never_called_a_provider_cannot_satisfy_a_provider_backed_gate() {
    let report = run_replay(&fixture()).expect("valid fixture");
    assert!(
        !report.provider_backed,
        "the offline harness must never claim provider evidence"
    );

    let thresholds = RolloutThresholds {
        require_provider_backed: true,
        ..published_thresholds()
    };

    // The metrics pass, so this proves the gate rejects on missing provider
    // evidence specifically rather than on a weak measurement.
    assert_eq!(
        report.evaluate(thresholds),
        RolloutVerdict::ProviderEvidenceMissing
    );
}

#[test]
fn each_unmet_threshold_names_the_measurement_that_failed() {
    let report = run_replay(&fixture()).expect("valid fixture");

    let recall = RolloutThresholds {
        min_demand_recall_bp: 10_001,
        ..published_thresholds()
    };
    let coverage = RolloutThresholds {
        min_passage_coverage_bp: 10_001,
        ..published_thresholds()
    };
    let token_cost = RolloutThresholds {
        max_token_cost_ratio_bp: 18_749,
        ..published_thresholds()
    };

    assert_eq!(
        vec![
            report.evaluate(recall),
            report.evaluate(coverage),
            report.evaluate(token_cost),
        ],
        vec![
            RolloutVerdict::RecallBelowThreshold {
                measured_bp: 10_000,
                required_bp: 10_001,
            },
            RolloutVerdict::CoverageBelowThreshold {
                measured_bp: 10_000,
                required_bp: 10_001,
            },
            RolloutVerdict::TokenCostAboveThreshold {
                measured_bp: 18_750,
                allowed_bp: 18_749,
            },
        ]
    );
}

#[test]
fn an_incomplete_run_cannot_authorize_rollout() {
    let mut report = run_replay(&fixture()).expect("valid fixture");
    report
        .metrics
        .retain(|metrics| metrics.variant != ReplayVariant::ProvenanceAware);

    // Absence of the variant production would enable is inconclusive, never a
    // pass by default.
    assert_eq!(
        report.evaluate(published_thresholds()),
        RolloutVerdict::Inconclusive
    );
}

#[test]
fn fallback_counts_are_published_per_variant() {
    let report = run_replay(&fixture()).expect("valid fixture");

    // The baseline leaves the second hop uncovered; both demand-inducing
    // variants cover it.
    assert_eq!(
        report.fallback_counts(),
        BTreeMap::from([
            ("baseline".to_string(), 1),
            ("decomposition_only".to_string(), 0),
            ("provenance_aware".to_string(), 0),
        ])
    );
}

#[test]
fn the_fixture_digest_is_stable_and_changes_when_the_corpus_changes() {
    let baseline = fixture();
    let repeated = fixture();
    assert_eq!(baseline.digest(), repeated.digest());

    let mut edited = fixture();
    edited.max_selected_passages += 1;
    assert_ne!(
        baseline.digest(),
        edited.digest(),
        "a settings change must invalidate the published digest"
    );

    // The published artifact records this exact value.
    assert_eq!(
        run_replay(&baseline).expect("valid fixture").fixture_digest,
        baseline.digest()
    );
    // Locks the digest string published in the evaluation artifact, so editing
    // the corpus without republishing the artifact fails here.
    assert_eq!(baseline.digest(), "bd2ce66352a83cad");
}

#[test]
fn checked_in_rollout_corpus_validates_and_replays_deterministically() {
    let fixture = load_rollout_corpus().expect("checked-in rollout corpus parses");
    fixture
        .validate()
        .expect("checked-in rollout corpus validates");
    assert_eq!(fixture.cases.len(), 8);

    let first = run_replay(&fixture).expect("valid rollout corpus");
    let second = run_replay(&fixture).expect("valid rollout corpus");
    assert_eq!(first, second);
}

#[test]
fn rollout_corpus_digest_covers_raw_bytes_fixture_settings_and_case_order() {
    let fixture = load_rollout_corpus().expect("checked-in rollout corpus parses");
    let raw = ROLLOUT_CORPUS_JSON.as_bytes();
    let settings = fixture.settings();
    let digest = |bytes: &[u8], fixture: &ReplayFixture, settings: &ReplaySettings| {
        corpus_digest(bytes, fixture, settings).expect("serializable rollout corpus")
    };
    let baseline = digest(raw, &fixture, &settings);
    assert_eq!(baseline, digest(raw, &fixture, &settings));

    let mut edited_raw = raw.to_vec();
    let corpus_marker = b"lineage-rollout-v1";
    let marker_start = edited_raw
        .windows(corpus_marker.len())
        .position(|window| window == corpus_marker)
        .expect("corpus id in source bytes");
    edited_raw[marker_start + corpus_marker.len() - 1] = b'2';
    assert_ne!(baseline, digest(&edited_raw, &fixture, &settings));

    let mut changed_budget = settings.clone();
    changed_budget.max_selected_passages += 1;
    assert_ne!(baseline, digest(raw, &fixture, &changed_budget));

    let mut changed_prompt = settings.clone();
    changed_prompt.prompt_id.push_str("-changed");
    assert_ne!(baseline, digest(raw, &fixture, &changed_prompt));

    let mut changed_model = settings.clone();
    changed_model.model_settings.push_str(";changed=1");
    assert_ne!(baseline, digest(raw, &fixture, &changed_model));

    let mut reordered = fixture;
    reordered.cases.swap(0, 1);
    assert_ne!(baseline, digest(raw, &reordered, &settings));
}

#[test]
fn checked_in_rollout_corpus_digest_has_stable_sha256_shape() {
    let first = load_rollout_corpus().expect("checked-in rollout corpus parses");
    let second = load_rollout_corpus().expect("checked-in rollout corpus parses");
    let first_digest = corpus_digest(ROLLOUT_CORPUS_JSON.as_bytes(), &first, &first.settings())
        .expect("serializable rollout corpus");
    let second_digest = corpus_digest(ROLLOUT_CORPUS_JSON.as_bytes(), &second, &second.settings())
        .expect("serializable rollout corpus");

    assert_eq!(first_digest, second_digest);
    assert_eq!(first_digest.len(), 64);
    assert!(
        first_digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );
}
