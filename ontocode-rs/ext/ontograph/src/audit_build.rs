use std::collections::BTreeMap;
use std::fs;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use ontocode_extension_api::FunctionCallError;
use ontocode_extension_api::JsonToolOutput;
use ontocode_extension_api::ToolCall;
use ontocode_extension_api::ToolExecutor;
use ontocode_extension_api::ToolName;
use ontocode_extension_api::ToolOutput;
use ontocode_extension_api::ToolSpec;
use ontocode_tools::ToolExposure;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::audit_abi::AuditAbiResult;
use crate::audit_abi::run_abi_audit;
use crate::audit_concurrency::AuditConcurrencyResult;
use crate::audit_concurrency::run_concurrency_audit;
use crate::audit_coupling::AuditCouplingResult;
use crate::audit_coupling::run_coupling_audit;
use crate::audit_coverage::AuditCoverageResult;
use crate::audit_coverage::inconclusive_coverage_result;
use crate::audit_coverage::run_coverage_audit;
use crate::audit_cycles::AuditCyclesResult;
use crate::audit_cycles::run_cycles_audit;
use crate::audit_drift::AuditDriftResult;
use crate::audit_drift::run_drift_audit;
use crate::audit_error_topology::AuditErrorTopologyResult;
use crate::audit_error_topology::run_error_topology_audit;
use crate::audit_fault_simulation::AuditFaultSimulationResult;
use crate::audit_fault_simulation::FaultSimulationAuditInput;
use crate::audit_fault_simulation::run_fault_simulation_audit;
use crate::audit_fsm::AuditFsmResult;
use crate::audit_fsm::run_fsm_audit;
use crate::audit_graph_diff::AuditGraphDiffResult;
use crate::audit_graph_diff::run_graph_diff_audit;
use crate::audit_logic::AuditLogicResult;
use crate::audit_logic::run_logic_audit;
use crate::audit_migration::AuditMigrationResult;
use crate::audit_migration::run_migration_audit;
use crate::audit_path_verify::AuditPathVerifyResult;
use crate::audit_path_verify::PathVerifyAuditInput;
use crate::audit_path_verify::run_path_verify_audit;
use crate::audit_pressure::AuditPressureResult;
use crate::audit_pressure::run_pressure_audit;
use crate::audit_report::AuditReportResult;
use crate::audit_report::load_trace_boundary_source_index;
use crate::audit_report::run_report_audit;
use crate::audit_requirements::AuditRequirementsResult;
use crate::audit_requirements::run_requirements_audit;
use crate::audit_session::AuditSessionBundleResult;
use crate::audit_session::AuditSessionDedupeResult;
use crate::audit_session::AuditSessionFindingInput;
use crate::audit_session::AuditSessionStartResult;
use crate::audit_session::AuditSessionVerifyResult;
use crate::audit_session::run_audit_session_bundle;
use crate::audit_session::run_audit_session_dedupe;
use crate::audit_session::run_audit_session_start;
use crate::audit_session::run_audit_session_verify;
use crate::audit_session_dispatch::AuditSessionDispatchResult;
use crate::audit_session_dispatch::run_audit_session_dispatch;
use crate::audit_tech_debt::AuditDeadCodeResult;
use crate::audit_tech_debt::AuditHotspotsResult;
use crate::audit_tech_debt::AuditTechDebtResult;
use crate::audit_tech_debt::DEFAULT_TECH_DEBT_MIN_LINES;
use crate::audit_tech_debt::run_dead_code_audit;
use crate::audit_tech_debt::run_hotspots_audit;
use crate::audit_tech_debt::run_tech_debt_audit;
use crate::audit_test_gap::AuditTestGapResult;
use crate::audit_test_gap::AuditTestSuggestionsResult;
use crate::audit_test_gap::TestGapAuditInput;
use crate::audit_test_gap::TestSuggestionsAuditInput;
use crate::audit_test_gap::inconclusive_test_gap_result;
use crate::audit_test_gap::inconclusive_test_suggestions_result;
use crate::audit_test_gap::run_test_gap_audit;
use crate::audit_test_gap::run_test_suggestions_audit;
use crate::audit_verify_diff::AuditVerifyDiffResult;
use crate::audit_verify_diff::VerifyDiffAuditInput;
use crate::audit_verify_diff::inconclusive_verify_diff_result;
use crate::audit_verify_diff::run_verify_diff_audit;
use crate::audit_violations::AuditViolationRuleInput;
use crate::audit_violations::AuditViolationsResult;
use crate::audit_violations::run_violations_audit;
use crate::audit_worker_scope::AuditWorkerScopeResult;
use crate::audit_worker_scope::WorkerScopeAuditInput;
use crate::audit_worker_scope::inconclusive_worker_scope_result;
use crate::audit_worker_scope::inconclusive_worker_scope_result_for_action;
use crate::audit_worker_scope::run_worker_scope_audit;
use crate::manifest::repo_ontograph_root;
use crate::query::MAX_QUERY_ITEMS;
use crate::tool::MAX_FILE_BYTES;
use crate::tool::ONTOGRAPH_NAMESPACE;
use crate::tool::OntographThreadState;
use crate::tool::namespace_function_tool;
use crate::tool::parse_tool_args;

#[path = "trace_boundary.rs"]
pub(crate) mod trace_boundary;

pub(crate) const AUDIT_TOOL_NAME: &str = "audit";
pub(crate) const AUDIT_TOOL_DESCRIPTION: &str =
    "Run one bounded read-only audit report rooted at the active Ontograph workspace.";

const DEFAULT_MAX_ITEMS: usize = 20;
const MAX_DOMAIN_COUNT: usize = 32;
const MAX_DOMAIN_LENGTH: usize = 80;
const MAX_PATTERN_COUNT: usize = 32;
const MAX_PATTERN_LENGTH: usize = 80;
const MAX_SNIPPET_CHARS: usize = 240;
const DEFAULT_FORBIDDEN_DOMAINS: [&str; 7] = [
    "TODO:",
    "FIXME:",
    "XXX:",
    "HACK:",
    "todo!(",
    "unimplemented!(",
    "dbg!(",
];
const BASE_SKIP_DIRS: [&str; 6] = [
    ".git",
    ".ontoindex",
    "node_modules",
    "target",
    "third_party",
    "tmp",
];
const DEFAULT_NOISE_DIRS: [&str; 7] = [
    ".memory-bank",
    "docs",
    "test",
    "tests",
    "fixtures",
    "__fixtures__",
    "__tests__",
];
const SCAN_EXTENSIONS: [&str; 11] = [
    "rs", "toml", "md", "json", "yaml", "yml", "sh", "py", "ts", "tsx", "js",
];
const CODE_SCAN_EXTENSIONS: [&str; 5] = ["rs", "py", "ts", "tsx", "js"];
const DEFAULT_PATTERN_RULES: [PatternRuleSeed; 6] = [
    PatternRuleSeed {
        pattern: "todo!(",
        reason: "Deferred implementation marker; confirm the path is intentionally incomplete.",
    },
    PatternRuleSeed {
        pattern: "unimplemented!(",
        reason: "Immediate runtime panic placeholder; replace before production use.",
    },
    PatternRuleSeed {
        pattern: ".unwrap()",
        reason: "Unchecked failure path; verify the call cannot fail or handle the error explicitly.",
    },
    PatternRuleSeed {
        pattern: ".expect(",
        reason: "Explicit panic path; verify the invariant is strong enough for production.",
    },
    PatternRuleSeed {
        pattern: "panic!(",
        reason: "Hard failure path; confirm it is unreachable for normal user input.",
    },
    PatternRuleSeed {
        pattern: "unsafe {",
        reason: "Unsafe block; confirm the invariant is documented and locally justified.",
    },
];

#[derive(Clone, Default)]
pub(crate) struct AuditTool {
    thread_state: Arc<OntographThreadState>,
}

impl AuditTool {
    pub(crate) fn new(thread_state: Arc<OntographThreadState>) -> Self {
        Self { thread_state }
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuditArgs {
    pub action: AuditAction,
    #[serde(default)]
    pub audit_file: Option<String>,
    #[serde(default)]
    pub forbidden_domains: Option<Vec<String>>,
    #[serde(default)]
    pub patterns: Option<Vec<String>>,
    #[serde(default)]
    pub max_items: Option<usize>,
    #[serde(default)]
    pub min_lines: Option<usize>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub findings: Option<Vec<AuditSessionFindingInput>>,
    #[serde(default)]
    pub created_at_unix_seconds: Option<i64>,
    #[serde(default)]
    pub rules: Option<Vec<AuditViolationRuleInput>>,
    #[serde(default)]
    pub bundle_id: Option<String>,
    #[serde(default)]
    pub base_generation_id: Option<String>,
    #[serde(default)]
    pub target_generation_id: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub base_ref: Option<String>,
    #[serde(default)]
    pub expected_files: Option<Vec<String>>,
    #[serde(default)]
    pub expected_symbols: Option<Vec<String>>,
    #[serde(default)]
    pub expected_tests: Option<Vec<String>>,
    #[serde(default)]
    pub executed_tests: Option<Vec<String>>,
    #[serde(default)]
    pub symbol_test_expectations: Option<BTreeMap<String, Vec<String>>>,
    #[serde(default)]
    pub file_test_expectations: Option<BTreeMap<String, Vec<String>>>,
    #[serde(default)]
    pub source_schema: Option<Value>,
    #[serde(default)]
    pub target_schema: Option<Value>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub trigger_pattern: Option<String>,
    #[serde(default)]
    pub required_patterns: Option<Vec<String>>,
    #[serde(default)]
    pub forbidden_patterns: Option<Vec<String>>,
    #[serde(default)]
    pub window_lines: Option<usize>,
    #[serde(default)]
    pub target_call: Option<String>,
    #[serde(default)]
    pub return_value: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditAction {
    Build,
    Patterns,
    Rerun,
    TechDebt,
    Hotspots,
    DeadCode,
    Cycles,
    Coupling,
    Coverage,
    Report,
    TraceBoundary,
    Violations,
    Migration,
    Requirements,
    Drift,
    GraphDiff,
    Logic,
    SessionStart,
    SessionVerify,
    SessionDedupe,
    SessionBundle,
    ExtractFsm,
    ErrorTopology,
    WorkerScopeReview,
    SessionReviewWorker,
    VerifyDiff,
    TestGap,
    TestSuggestions,
    Abi,
    PathVerify,
    Pressure,
    Concurrency,
    SimulateFault,
    SessionDispatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditBuildFinding {
    pub domain: String,
    pub path: String,
    pub line: usize,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditBuildResult {
    pub action: AuditAction,
    pub forbidden_domains: Vec<String>,
    pub scanned_files: usize,
    pub findings: Vec<AuditBuildFinding>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditPatternFinding {
    pub pattern: String,
    pub reason: String,
    pub path: String,
    pub line: usize,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditPatternResult {
    pub action: AuditAction,
    pub patterns: Vec<String>,
    pub scanned_files: usize,
    pub findings: Vec<AuditPatternFinding>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditRerunStatus {
    StillOpen,
    Moved,
    Fixed,
    MissingFile,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditRerunFinding {
    pub path: String,
    pub pattern: Option<String>,
    pub recorded_line: Option<usize>,
    pub status: AuditRerunStatus,
    pub current_line: Option<usize>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditRerunResult {
    pub action: AuditAction,
    pub audit_file: String,
    pub total: usize,
    pub still_open: usize,
    pub moved: usize,
    pub fixed: usize,
    pub missing_file: usize,
    pub invalid: usize,
    pub findings: Vec<AuditRerunFinding>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(untagged)]
pub(crate) enum AuditResult {
    Build(AuditBuildResult),
    Patterns(AuditPatternResult),
    Rerun(AuditRerunResult),
    TechDebt(AuditTechDebtResult),
    Hotspots(AuditHotspotsResult),
    DeadCode(AuditDeadCodeResult),
    Cycles(AuditCyclesResult),
    Coupling(AuditCouplingResult),
    Coverage(AuditCoverageResult),
    Report(AuditReportResult),
    TraceBoundary(trace_boundary::TraceBoundaryResult),
    Violations(AuditViolationsResult),
    Migration(AuditMigrationResult),
    Requirements(AuditRequirementsResult),
    Drift(AuditDriftResult),
    GraphDiff(AuditGraphDiffResult),
    Logic(AuditLogicResult),
    SessionStart(AuditSessionStartResult),
    SessionVerify(AuditSessionVerifyResult),
    SessionDedupe(AuditSessionDedupeResult),
    SessionBundle(AuditSessionBundleResult),
    Fsm(AuditFsmResult),
    ErrorTopology(AuditErrorTopologyResult),
    WorkerScopeReview(AuditWorkerScopeResult),
    SessionReviewWorker(AuditWorkerScopeResult),
    VerifyDiff(AuditVerifyDiffResult),
    TestGap(AuditTestGapResult),
    TestSuggestions(AuditTestSuggestionsResult),
    Abi(AuditAbiResult),
    PathVerify(AuditPathVerifyResult),
    Pressure(AuditPressureResult),
    Concurrency(AuditConcurrencyResult),
    SimulateFault(AuditFaultSimulationResult),
    SessionDispatch(AuditSessionDispatchResult),
}

#[async_trait::async_trait]
impl ToolExecutor<ToolCall> for AuditTool {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        namespace_function_tool::<AuditArgs, AuditResult>(AUDIT_TOOL_NAME, AUDIT_TOOL_DESCRIPTION)
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::DirectModelOnly
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        true
    }

    async fn handle(&self, call: ToolCall) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let args = parse_tool_args::<AuditArgs>(&call, "ontograph.audit")?;
        let repo_root = self.thread_state.current_repo_root().ok_or_else(|| {
            FunctionCallError::RespondToModel(
                "ontograph.audit workspace context is unavailable for this turn".to_string(),
            )
        })?;
        let result = match args.action {
            AuditAction::Build => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                let domains = normalize_forbidden_domains(args.forbidden_domains)?;
                let scan = scan_workspace(&repo_root, &domains, max_items)?;
                AuditResult::Build(AuditBuildResult {
                    action: AuditAction::Build,
                    forbidden_domains: domains.values,
                    scanned_files: scan.scanned_files,
                    findings: scan.findings,
                    truncated: scan.truncated,
                    limits: vec![
                        format!("max_items={max_items}"),
                        format!("max_file_bytes={MAX_FILE_BYTES}"),
                        format!("max_forbidden_domains={MAX_DOMAIN_COUNT}"),
                    ],
                    notes: build_notes(domains.used_default),
                })
            }
            AuditAction::Patterns => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                let patterns = normalize_patterns(args.patterns)?;
                let scan = scan_workspace_patterns(&repo_root, &patterns, max_items)?;
                AuditResult::Patterns(AuditPatternResult {
                    action: AuditAction::Patterns,
                    patterns: patterns
                        .rules
                        .iter()
                        .map(|rule| rule.pattern.clone())
                        .collect(),
                    scanned_files: scan.scanned_files,
                    findings: scan.findings,
                    truncated: scan.truncated,
                    limits: vec![
                        format!("max_items={max_items}"),
                        format!("max_file_bytes={MAX_FILE_BYTES}"),
                        format!("max_patterns={MAX_PATTERN_COUNT}"),
                    ],
                    notes: pattern_notes(patterns.used_default),
                })
            }
            AuditAction::Rerun => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                AuditResult::Rerun(rerun_audit_file(
                    &repo_root,
                    args.audit_file.as_deref(),
                    max_items,
                )?)
            }
            AuditAction::TechDebt => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                AuditResult::TechDebt(run_tech_debt_audit(
                    &repo_root,
                    max_items,
                    args.min_lines.unwrap_or(DEFAULT_TECH_DEBT_MIN_LINES),
                    args.since.as_deref(),
                )?)
            }
            AuditAction::Hotspots => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                AuditResult::Hotspots(run_hotspots_audit(
                    &repo_root,
                    max_items,
                    args.since.as_deref(),
                )?)
            }
            AuditAction::DeadCode => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                AuditResult::DeadCode(run_dead_code_audit(&repo_root, max_items)?)
            }
            AuditAction::Cycles => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                let source_index = self
                    .thread_state
                    .load_current_source_index("ontograph.audit")?;
                AuditResult::Cycles(run_cycles_audit(&source_index, max_items)?)
            }
            AuditAction::Coupling => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                let source_index = self
                    .thread_state
                    .load_current_source_index("ontograph.audit")?;
                AuditResult::Coupling(run_coupling_audit(&source_index, max_items)?)
            }
            AuditAction::Coverage => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                match self
                    .thread_state
                    .load_current_source_index("ontograph.audit")
                {
                    Ok(source_index) => {
                        AuditResult::Coverage(run_coverage_audit(&source_index, max_items))
                    }
                    Err(err) => {
                        AuditResult::Coverage(inconclusive_coverage_result(err.to_string()))
                    }
                }
            }
            AuditAction::Report => AuditResult::Report(run_report_audit(&self.thread_state)?),
            AuditAction::TraceBoundary => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                let source_index = load_trace_boundary_source_index(&self.thread_state)?;
                AuditResult::TraceBoundary(trace_boundary::run_trace_boundary_report(
                    &source_index,
                    trace_boundary::TraceBoundaryBudget {
                        max_items,
                        ..trace_boundary::TraceBoundaryBudget::default()
                    },
                ))
            }
            AuditAction::Violations => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                let source_index = self
                    .thread_state
                    .load_current_source_index("ontograph.audit")?;
                AuditResult::Violations(run_violations_audit(&source_index, args.rules, max_items)?)
            }
            AuditAction::Migration => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                AuditResult::Migration(run_migration_audit(&self.thread_state, max_items)?)
            }
            AuditAction::Requirements => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                AuditResult::Requirements(run_requirements_audit(&self.thread_state, max_items)?)
            }
            AuditAction::Drift => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                AuditResult::Drift(run_drift_audit(&self.thread_state, max_items)?)
            }
            AuditAction::GraphDiff => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                let artifact_root = self.thread_state.current_artifact_root().ok_or_else(|| {
                    FunctionCallError::RespondToModel(
                        "ontograph.audit workspace context is unavailable for this turn"
                            .to_string(),
                    )
                })?;
                AuditResult::GraphDiff(run_graph_diff_audit(
                    &artifact_root,
                    args.base_generation_id.as_deref(),
                    args.target_generation_id.as_deref(),
                    max_items,
                )?)
            }
            AuditAction::Logic => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                AuditResult::Logic(run_logic_audit(&repo_root, max_items)?)
            }
            AuditAction::SessionStart => AuditResult::SessionStart(run_audit_session_start(
                self.thread_state
                    .current_artifact_root()
                    .as_ref()
                    .ok_or_else(|| {
                        FunctionCallError::RespondToModel(
                            "ontograph.audit workspace context is unavailable for this turn"
                                .to_string(),
                        )
                    })?,
                args.session_id,
                args.findings.unwrap_or_default(),
                args.created_at_unix_seconds,
            )?),
            AuditAction::SessionVerify => AuditResult::SessionVerify(run_audit_session_verify(
                self.thread_state
                    .current_artifact_root()
                    .as_ref()
                    .ok_or_else(|| {
                        FunctionCallError::RespondToModel(
                            "ontograph.audit workspace context is unavailable for this turn"
                                .to_string(),
                        )
                    })?,
            )?),
            AuditAction::SessionDedupe => AuditResult::SessionDedupe(run_audit_session_dedupe(
                self.thread_state
                    .current_artifact_root()
                    .as_ref()
                    .ok_or_else(|| {
                        FunctionCallError::RespondToModel(
                            "ontograph.audit workspace context is unavailable for this turn"
                                .to_string(),
                        )
                    })?,
            )?),
            AuditAction::SessionBundle => AuditResult::SessionBundle(run_audit_session_bundle(
                self.thread_state
                    .current_artifact_root()
                    .as_ref()
                    .ok_or_else(|| {
                        FunctionCallError::RespondToModel(
                            "ontograph.audit workspace context is unavailable for this turn"
                                .to_string(),
                        )
                    })?,
            )?),
            AuditAction::ExtractFsm => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                AuditResult::Fsm(run_fsm_audit(&repo_root, max_items)?)
            }
            AuditAction::ErrorTopology => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                AuditResult::ErrorTopology(run_error_topology_audit(&repo_root, max_items)?)
            }
            AuditAction::WorkerScopeReview => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                let bundle_id = args.bundle_id.ok_or_else(|| {
                    FunctionCallError::RespondToModel(
                        "ontograph.audit bundle_id is required for action `worker_scope_review`"
                            .to_string(),
                    )
                })?;
                let scope = args.scope.ok_or_else(|| {
                    FunctionCallError::RespondToModel(
                        "ontograph.audit scope is required for action `worker_scope_review`"
                            .to_string(),
                    )
                })?;
                match self
                    .thread_state
                    .load_current_source_index("ontograph.audit")
                {
                    Ok(source_index) => {
                        let artifact_root =
                            self.thread_state.current_artifact_root().ok_or_else(|| {
                                FunctionCallError::RespondToModel(
                                "ontograph.audit workspace context is unavailable for this turn"
                                    .to_string(),
                            )
                            })?;
                        AuditResult::WorkerScopeReview(run_worker_scope_audit(
                            WorkerScopeAuditInput {
                                action: AuditAction::WorkerScopeReview,
                                store: &source_index,
                                root: &artifact_root,
                                bundle_id,
                                scope,
                                base_ref: args.base_ref,
                                executed_tests: args.executed_tests,
                                max_items,
                            },
                        )?)
                    }
                    Err(err) => AuditResult::WorkerScopeReview(inconclusive_worker_scope_result(
                        bundle_id,
                        err.to_string(),
                    )),
                }
            }
            AuditAction::SessionReviewWorker => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                let bundle_id = args.bundle_id.ok_or_else(|| {
                    FunctionCallError::RespondToModel(
                        "ontograph.audit bundle_id is required for action `session_review_worker`"
                            .to_string(),
                    )
                })?;
                let scope = args.scope.ok_or_else(|| {
                    FunctionCallError::RespondToModel(
                        "ontograph.audit scope is required for action `session_review_worker`"
                            .to_string(),
                    )
                })?;
                match self
                    .thread_state
                    .load_current_source_index("ontograph.audit")
                {
                    Ok(source_index) => {
                        let artifact_root =
                            self.thread_state.current_artifact_root().ok_or_else(|| {
                                FunctionCallError::RespondToModel(
                                "ontograph.audit workspace context is unavailable for this turn"
                                    .to_string(),
                            )
                            })?;
                        AuditResult::SessionReviewWorker(run_worker_scope_audit(
                            WorkerScopeAuditInput {
                                action: AuditAction::SessionReviewWorker,
                                store: &source_index,
                                root: &artifact_root,
                                bundle_id,
                                scope,
                                base_ref: args.base_ref,
                                executed_tests: args.executed_tests,
                                max_items,
                            },
                        )?)
                    }
                    Err(err) => AuditResult::SessionReviewWorker(
                        inconclusive_worker_scope_result_for_action(
                            AuditAction::SessionReviewWorker,
                            bundle_id,
                            err.to_string(),
                        ),
                    ),
                }
            }
            AuditAction::VerifyDiff => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                match self
                    .thread_state
                    .load_current_source_index("ontograph.audit")
                {
                    Ok(source_index) => {
                        AuditResult::VerifyDiff(run_verify_diff_audit(VerifyDiffAuditInput {
                            store: &source_index,
                            scope: args.scope.unwrap_or_else(|| "unstaged".to_string()),
                            base_ref: args.base_ref,
                            expected_files: args.expected_files,
                            expected_symbols: args.expected_symbols,
                            expected_tests: args.expected_tests,
                            executed_tests: args.executed_tests,
                            max_items,
                        })?)
                    }
                    Err(err) => {
                        AuditResult::VerifyDiff(inconclusive_verify_diff_result(err.to_string()))
                    }
                }
            }
            AuditAction::TestGap => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                match self
                    .thread_state
                    .load_current_source_index("ontograph.audit")
                {
                    Ok(source_index) => {
                        AuditResult::TestGap(run_test_gap_audit(TestGapAuditInput {
                            store: &source_index,
                            scope: args.scope.unwrap_or_else(|| "unstaged".to_string()),
                            base_ref: args.base_ref,
                            executed_tests: args.executed_tests,
                            symbol_test_expectations: args.symbol_test_expectations,
                            file_test_expectations: args.file_test_expectations,
                            max_items,
                        })?)
                    }
                    Err(err) => AuditResult::TestGap(inconclusive_test_gap_result(err.to_string())),
                }
            }
            AuditAction::TestSuggestions => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                match self
                    .thread_state
                    .load_current_source_index("ontograph.audit")
                {
                    Ok(source_index) => AuditResult::TestSuggestions(run_test_suggestions_audit(
                        TestSuggestionsAuditInput {
                            test_gap: TestGapAuditInput {
                                store: &source_index,
                                scope: args.scope.unwrap_or_else(|| "unstaged".to_string()),
                                base_ref: args.base_ref,
                                executed_tests: args.executed_tests,
                                symbol_test_expectations: args.symbol_test_expectations,
                                file_test_expectations: args.file_test_expectations,
                                max_items,
                            },
                        },
                    )?),
                    Err(err) => AuditResult::TestSuggestions(inconclusive_test_suggestions_result(
                        err.to_string(),
                    )),
                }
            }
            AuditAction::Abi => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                AuditResult::Abi(run_abi_audit(
                    args.source_schema,
                    args.target_schema,
                    max_items,
                )?)
            }
            AuditAction::PathVerify => {
                AuditResult::PathVerify(run_path_verify_audit(PathVerifyAuditInput {
                    repo_root,
                    path: args.path,
                    trigger_pattern: args.trigger_pattern,
                    required_patterns: args.required_patterns,
                    forbidden_patterns: args.forbidden_patterns,
                    window_lines: args.window_lines,
                })?)
            }
            AuditAction::Pressure => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                AuditResult::Pressure(run_pressure_audit(&repo_root, max_items)?)
            }
            AuditAction::Concurrency => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                AuditResult::Concurrency(run_concurrency_audit(&repo_root, max_items)?)
            }
            AuditAction::SimulateFault => {
                let max_items = args
                    .max_items
                    .unwrap_or(DEFAULT_MAX_ITEMS)
                    .clamp(1, MAX_QUERY_ITEMS);
                AuditResult::SimulateFault(run_fault_simulation_audit(FaultSimulationAuditInput {
                    repo_root,
                    path: args.path,
                    target_call: args.target_call,
                    return_value: args.return_value,
                    max_items,
                    window_lines: args.window_lines,
                })?)
            }
            AuditAction::SessionDispatch => {
                let bundle_id = args.bundle_id.ok_or_else(|| {
                    FunctionCallError::RespondToModel(
                        "ontograph.audit bundle_id is required for action `session_dispatch`"
                            .to_string(),
                    )
                })?;
                AuditResult::SessionDispatch(run_audit_session_dispatch(
                    self.thread_state
                        .current_artifact_root()
                        .as_ref()
                        .ok_or_else(|| {
                            FunctionCallError::RespondToModel(
                                "ontograph.audit workspace context is unavailable for this turn"
                                    .to_string(),
                            )
                        })?,
                    bundle_id,
                )?)
            }
        };
        let value = serde_json::to_value(result).map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "failed to serialize ontograph.audit result: {err}"
            ))
        })?;
        Ok(Box::new(JsonToolOutput::new(value)))
    }
}

#[derive(Debug)]
struct NormalizedDomains {
    values: Vec<String>,
    values_lower: Vec<String>,
    used_default: bool,
}

#[derive(Debug, Clone)]
struct PatternRule {
    pattern: String,
    pattern_lower: String,
    reason: String,
}

#[derive(Debug, Clone, Copy)]
struct PatternRuleSeed {
    pattern: &'static str,
    reason: &'static str,
}

#[derive(Debug)]
struct NormalizedPatterns {
    rules: Vec<PatternRule>,
    used_default: bool,
}

#[derive(Debug, Default)]
struct AuditBuildScan {
    scanned_files: usize,
    findings: Vec<AuditBuildFinding>,
    truncated: bool,
}

#[derive(Debug, Default)]
struct AuditPatternScan {
    scanned_files: usize,
    findings: Vec<AuditPatternFinding>,
    truncated: bool,
}

#[derive(Debug, Clone)]
struct ParsedAuditFinding {
    path: Option<String>,
    pattern: Option<String>,
    recorded_line: Option<usize>,
    snippet: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct AuditRerunCounts {
    still_open: usize,
    moved: usize,
    fixed: usize,
    missing_file: usize,
    invalid: usize,
}

fn normalize_forbidden_domains(
    domains: Option<Vec<String>>,
) -> Result<NormalizedDomains, FunctionCallError> {
    let values = domains
        .unwrap_or_default()
        .into_iter()
        .map(|domain| domain.trim().to_string())
        .filter(|domain| !domain.is_empty())
        .collect::<Vec<_>>();

    let used_default = values.is_empty();
    let values = if used_default {
        DEFAULT_FORBIDDEN_DOMAINS
            .iter()
            .map(|domain| (*domain).to_string())
            .collect()
    } else {
        values
    };

    if values.len() > MAX_DOMAIN_COUNT {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit forbidden_domains exceeds the maximum of {MAX_DOMAIN_COUNT} items"
        )));
    }

    if let Some(domain) = values
        .iter()
        .find(|domain| domain.len() > MAX_DOMAIN_LENGTH)
    {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit forbidden domain exceeds the maximum of {MAX_DOMAIN_LENGTH} characters: {domain}"
        )));
    }

    Ok(NormalizedDomains {
        values_lower: values.iter().map(|domain| domain.to_lowercase()).collect(),
        values,
        used_default,
    })
}

fn build_notes(used_default_domains: bool) -> Vec<String> {
    let mut notes = vec![
        "This first native slice scans repo-owned source/config files only.".to_string(),
        "Scratch and build-noise roots are excluded: .git, .ontocode/ontograph, .ontoindex, node_modules, target, third_party, tmp.".to_string(),
    ];
    if used_default_domains {
        notes.push(
            "Default domains also skip doc/test-heavy roots: .memory-bank, docs, test, tests, fixtures, __fixtures__, __tests__.".to_string(),
        );
    }
    notes
}

fn pattern_notes(used_default_patterns: bool) -> Vec<String> {
    let mut notes = vec![
        "This native slice scans repo-owned code files only.".to_string(),
        "Scratch and build-noise roots are excluded: .git, .ontocode/ontograph, .ontoindex, node_modules, target, third_party, tmp.".to_string(),
    ];
    if used_default_patterns {
        notes.push(
            "Default patterns also skip doc/test-heavy roots: .memory-bank, docs, test, tests, fixtures, __fixtures__, __tests__.".to_string(),
        );
    }
    notes
}

fn rerun_notes() -> Vec<String> {
    vec![
        "This native slice rechecks one saved audit findings file against the active workspace only."
            .to_string(),
        "Saved findings may use either native `path` or donor-style `file` keys.".to_string(),
        format!(
            "Both the saved audit file and matched workspace files are capped at {MAX_FILE_BYTES} bytes."
        ),
    ]
}

fn scan_workspace(
    repo_root: &Path,
    domains: &NormalizedDomains,
    max_items: usize,
) -> Result<AuditBuildScan, FunctionCallError> {
    let mut targets = Vec::new();
    collect_scan_targets(
        repo_root,
        repo_root,
        domains.used_default,
        &SCAN_EXTENSIONS,
        &mut targets,
    )?;
    targets.sort();

    let mut scan = AuditBuildScan {
        scanned_files: targets.len(),
        findings: Vec::new(),
        truncated: false,
    };
    for path in targets {
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => continue,
        };
        for (line_index, line) in content.lines().enumerate() {
            let line_lower = line.to_lowercase();
            for (domain, domain_lower) in domains.values.iter().zip(&domains.values_lower) {
                if !line_lower.contains(domain_lower) {
                    continue;
                }
                if scan.findings.len() == max_items {
                    scan.truncated = true;
                    return Ok(scan);
                }
                scan.findings.push(AuditBuildFinding {
                    domain: domain.clone(),
                    path: relative_slash_path(repo_root, &path).map_err(|err| {
                        FunctionCallError::RespondToModel(format!(
                            "ontograph.audit failed to normalize a matched path: {err}"
                        ))
                    })?,
                    line: line_index + 1,
                    snippet: truncate_chars(line.trim(), MAX_SNIPPET_CHARS),
                });
            }
        }
    }
    Ok(scan)
}

fn scan_workspace_patterns(
    repo_root: &Path,
    patterns: &NormalizedPatterns,
    max_items: usize,
) -> Result<AuditPatternScan, FunctionCallError> {
    let mut targets = Vec::new();
    collect_scan_targets(
        repo_root,
        repo_root,
        patterns.used_default,
        &CODE_SCAN_EXTENSIONS,
        &mut targets,
    )?;
    targets.sort();

    let mut scan = AuditPatternScan {
        scanned_files: targets.len(),
        findings: Vec::new(),
        truncated: false,
    };
    for path in targets {
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => continue,
        };
        for (line_index, line) in content.lines().enumerate() {
            let line_lower = line.to_lowercase();
            for rule in &patterns.rules {
                if !line_lower.contains(&rule.pattern_lower) {
                    continue;
                }
                if scan.findings.len() == max_items {
                    scan.truncated = true;
                    return Ok(scan);
                }
                scan.findings.push(AuditPatternFinding {
                    pattern: rule.pattern.clone(),
                    reason: rule.reason.clone(),
                    path: relative_slash_path(repo_root, &path).map_err(|err| {
                        FunctionCallError::RespondToModel(format!(
                            "ontograph.audit failed to normalize a matched path: {err}"
                        ))
                    })?,
                    line: line_index + 1,
                    snippet: truncate_chars(line.trim(), MAX_SNIPPET_CHARS),
                });
            }
        }
    }
    Ok(scan)
}

fn collect_scan_targets(
    repo_root: &Path,
    path: &Path,
    use_default_domains: bool,
    extensions: &[&str],
    targets: &mut Vec<PathBuf>,
) -> Result<(), FunctionCallError> {
    let entries = fs::read_dir(path).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.audit failed to walk the workspace: {err}"
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "ontograph.audit failed to walk the workspace: {err}"
            ))
        })?;
        let file_type = entry.file_type().map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "ontograph.audit failed to inspect a workspace entry: {err}"
            ))
        })?;
        if file_type.is_symlink() {
            continue;
        }
        let entry_path = entry.path();
        if file_type.is_dir() {
            if should_skip_dir(repo_root, &entry_path, use_default_domains) {
                continue;
            }
            collect_scan_targets(
                repo_root,
                &entry_path,
                use_default_domains,
                extensions,
                targets,
            )?;
            continue;
        }
        if !file_type.is_file() || !should_scan_file(&entry_path, extensions) {
            continue;
        }
        let metadata = entry.metadata().map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "ontograph.audit failed to inspect a workspace file: {err}"
            ))
        })?;
        if metadata.len() as usize > MAX_FILE_BYTES {
            continue;
        }
        if entry_path.strip_prefix(repo_root).is_ok() {
            targets.push(entry_path);
        }
    }
    Ok(())
}

fn normalize_patterns(
    patterns: Option<Vec<String>>,
) -> Result<NormalizedPatterns, FunctionCallError> {
    let values = patterns
        .unwrap_or_default()
        .into_iter()
        .map(|pattern| pattern.trim().to_string())
        .filter(|pattern| !pattern.is_empty())
        .collect::<Vec<_>>();

    let used_default = values.is_empty();
    let rules = if used_default {
        DEFAULT_PATTERN_RULES
            .iter()
            .map(|rule| PatternRule {
                pattern: rule.pattern.to_string(),
                pattern_lower: rule.pattern.to_lowercase(),
                reason: rule.reason.to_string(),
            })
            .collect::<Vec<_>>()
    } else {
        values
            .into_iter()
            .map(|pattern| PatternRule {
                pattern_lower: pattern.to_lowercase(),
                reason: "Custom pattern supplied by the caller.".to_string(),
                pattern,
            })
            .collect::<Vec<_>>()
    };

    if rules.len() > MAX_PATTERN_COUNT {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit patterns exceeds the maximum of {MAX_PATTERN_COUNT} items"
        )));
    }

    if let Some(rule) = rules
        .iter()
        .find(|rule| rule.pattern.len() > MAX_PATTERN_LENGTH)
    {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit pattern exceeds the maximum of {MAX_PATTERN_LENGTH} characters: {}",
            rule.pattern
        )));
    }

    Ok(NormalizedPatterns {
        rules,
        used_default,
    })
}

fn rerun_audit_file(
    repo_root: &Path,
    audit_file: Option<&str>,
    max_items: usize,
) -> Result<AuditRerunResult, FunctionCallError> {
    let audit_file =
        normalize_repo_relative_path(repo_root, audit_file.unwrap_or_default(), "audit_file")?;
    let audit_path = repo_root.join(&audit_file);
    let metadata = fs::metadata(&audit_path).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.audit failed to inspect audit_file {audit_file}: {err}"
        ))
    })?;
    if metadata.len() as usize > MAX_FILE_BYTES {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit audit_file exceeds the maximum of {MAX_FILE_BYTES} bytes: {audit_file}"
        )));
    }

    let raw = fs::read_to_string(&audit_path).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.audit failed to read audit_file {audit_file}: {err}"
        ))
    })?;
    let parsed = parse_saved_audit_findings(&raw)?;
    let total = parsed.len();

    let mut grouped = BTreeMap::<String, Vec<ParsedAuditFinding>>::new();
    let mut findings = Vec::with_capacity(total);
    let mut counts = AuditRerunCounts::default();
    for finding in parsed {
        let Some(path) = finding.path.clone() else {
            push_rerun_finding(
                &mut findings,
                &mut counts,
                AuditRerunFinding {
                    path: String::new(),
                    pattern: finding.pattern,
                    recorded_line: finding.recorded_line,
                    status: AuditRerunStatus::Invalid,
                    current_line: None,
                    detail: Some("missing file path".to_string()),
                },
            );
            continue;
        };
        match normalize_repo_relative_path(repo_root, &path, "finding.path") {
            Ok(path) => grouped.entry(path).or_default().push(finding),
            Err(_) => push_rerun_finding(
                &mut findings,
                &mut counts,
                AuditRerunFinding {
                    path,
                    pattern: finding.pattern,
                    recorded_line: finding.recorded_line,
                    status: AuditRerunStatus::Invalid,
                    current_line: None,
                    detail: Some(
                        "file path must stay inside the active workspace root".to_string(),
                    ),
                },
            ),
        }
    }

    for (path, group) in grouped {
        let content = match read_rerun_workspace_file(&repo_root.join(&path)) {
            Ok(content) => Some(content),
            Err(detail) => {
                let status = if detail.is_some() {
                    AuditRerunStatus::Invalid
                } else {
                    AuditRerunStatus::MissingFile
                };
                for finding in &group {
                    push_rerun_finding(
                        &mut findings,
                        &mut counts,
                        AuditRerunFinding {
                            path: path.clone(),
                            pattern: finding.pattern.clone(),
                            recorded_line: finding.recorded_line,
                            status: status.clone(),
                            current_line: None,
                            detail: detail.clone(),
                        },
                    );
                }
                None
            }
        };
        let Some(content) = content else {
            continue;
        };

        for finding in group {
            let (status, current_line, detail) = classify_rerun_finding(&finding, &content);
            push_rerun_finding(
                &mut findings,
                &mut counts,
                AuditRerunFinding {
                    path: path.clone(),
                    pattern: finding.pattern,
                    recorded_line: finding.recorded_line,
                    status,
                    current_line,
                    detail,
                },
            );
        }
    }

    findings.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(
                left.recorded_line
                    .unwrap_or(0)
                    .cmp(&right.recorded_line.unwrap_or(0)),
            )
            .then(
                left.current_line
                    .unwrap_or(0)
                    .cmp(&right.current_line.unwrap_or(0)),
            )
    });
    let truncated = findings.len() > max_items;
    if truncated {
        findings.truncate(max_items);
    }

    Ok(AuditRerunResult {
        action: AuditAction::Rerun,
        audit_file,
        total,
        still_open: counts.still_open,
        moved: counts.moved,
        fixed: counts.fixed,
        missing_file: counts.missing_file,
        invalid: counts.invalid,
        findings,
        truncated,
        limits: vec![
            format!("max_items={max_items}"),
            format!("max_file_bytes={MAX_FILE_BYTES}"),
        ],
        notes: rerun_notes(),
    })
}

fn parse_saved_audit_findings(raw: &str) -> Result<Vec<ParsedAuditFinding>, FunctionCallError> {
    let value = serde_json::from_str::<Value>(raw).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.audit audit_file must be valid JSON: {err}"
        ))
    })?;
    let findings = match value {
        Value::Array(findings) => findings,
        Value::Object(mut object) => match object.remove("findings") {
            Some(Value::Array(findings)) => findings,
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "ontograph.audit audit_file must be a JSON array or an object with a findings array".to_string(),
                ));
            }
        },
        _ => {
            return Err(FunctionCallError::RespondToModel(
                "ontograph.audit audit_file must be a JSON array or an object with a findings array".to_string(),
            ));
        }
    };
    Ok(findings.iter().map(parse_saved_audit_finding).collect())
}

fn parse_saved_audit_finding(value: &Value) -> ParsedAuditFinding {
    ParsedAuditFinding {
        path: value
            .get("path")
            .or_else(|| value.get("file"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        pattern: value
            .get("pattern")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        recorded_line: value
            .get("line")
            .or_else(|| value.get("recorded_line"))
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value > 0),
        snippet: value
            .get("snippet")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
    }
}

fn normalize_repo_relative_path(
    repo_root: &Path,
    raw_path: &str,
    field_name: &str,
) -> Result<String, FunctionCallError> {
    let raw_path = raw_path.trim();
    if raw_path.is_empty() {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit {field_name} is required"
        )));
    }
    let path = Path::new(raw_path);
    let relative_path = if path.is_absolute() {
        path.strip_prefix(repo_root).map_err(|_| {
            FunctionCallError::RespondToModel(format!(
                "ontograph.audit {field_name} must stay inside the active workspace root"
            ))
        })?
    } else {
        path
    };

    let mut normalized = PathBuf::new();
    for component in relative_path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(FunctionCallError::RespondToModel(format!(
                    "ontograph.audit {field_name} must stay inside the active workspace root"
                )));
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit {field_name} is required"
        )));
    }
    relative_slash_path(repo_root, &repo_root.join(normalized)).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.audit failed to normalize {field_name}: {err}"
        ))
    })
}

fn read_rerun_workspace_file(path: &Path) -> Result<String, Option<String>> {
    let metadata = fs::metadata(path).map_err(|_| None)?;
    if metadata.len() as usize > MAX_FILE_BYTES {
        return Err(Some(format!(
            "workspace file exceeds the maximum of {MAX_FILE_BYTES} bytes"
        )));
    }
    fs::read_to_string(path).map_err(|_| None)
}

fn classify_rerun_finding(
    finding: &ParsedAuditFinding,
    content: &str,
) -> (AuditRerunStatus, Option<usize>, Option<String>) {
    let lines = content.lines().collect::<Vec<_>>();
    if let Some(snippet) = finding.snippet.as_deref() {
        if let Some(recorded_line) = finding.recorded_line
            && recorded_line <= lines.len()
            && lines[recorded_line - 1].contains(snippet)
        {
            return (AuditRerunStatus::StillOpen, Some(recorded_line), None);
        }
        if let Some(index) = lines.iter().position(|line| line.contains(snippet)) {
            return (AuditRerunStatus::Moved, Some(index + 1), None);
        }
        return (AuditRerunStatus::Fixed, None, None);
    }
    if let Some(pattern) = finding.pattern.as_deref() {
        if let Some(recorded_line) = finding.recorded_line
            && recorded_line <= lines.len()
            && lines[recorded_line - 1].contains(pattern)
        {
            return (AuditRerunStatus::StillOpen, Some(recorded_line), None);
        }
        if let Some(index) = lines.iter().position(|line| line.contains(pattern)) {
            return (AuditRerunStatus::Moved, Some(index + 1), None);
        }
        return (AuditRerunStatus::Fixed, None, None);
    }
    (
        AuditRerunStatus::Invalid,
        None,
        Some("finding has neither snippet nor pattern".to_string()),
    )
}

fn push_rerun_finding(
    findings: &mut Vec<AuditRerunFinding>,
    counts: &mut AuditRerunCounts,
    finding: AuditRerunFinding,
) {
    match finding.status {
        AuditRerunStatus::StillOpen => counts.still_open += 1,
        AuditRerunStatus::Moved => counts.moved += 1,
        AuditRerunStatus::Fixed => counts.fixed += 1,
        AuditRerunStatus::MissingFile => counts.missing_file += 1,
        AuditRerunStatus::Invalid => counts.invalid += 1,
    }
    findings.push(finding);
}

fn should_skip_dir(repo_root: &Path, path: &Path, use_default_domains: bool) -> bool {
    if path == repo_ontograph_root(repo_root) {
        return true;
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    BASE_SKIP_DIRS.contains(&name) || (use_default_domains && DEFAULT_NOISE_DIRS.contains(&name))
}

fn should_scan_file(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extensions.contains(&extension))
}

fn relative_slash_path(
    repo_root: &Path,
    path: &Path,
) -> Result<String, std::path::StripPrefixError> {
    Ok(path
        .strip_prefix(repo_root)?
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}
