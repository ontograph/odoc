//! Typed retrieval-shape and evidence-record model for Ontograph Graph-RAG planner.
//!
//! Defines bounded, provenance-bearing data structures for the read-only evidence planner,
//! implementing ADR_ONTOGRAPH_GRAPH_RAG_EVIDENCE_PLANNER_CONTRACT.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::time::Duration;
use std::time::Instant;

use serde::Deserialize;
use serde::Serialize;

pub(crate) const MAX_HOPS_LIMIT: usize = 5;
pub(crate) const MAX_RECORDS_LIMIT: usize = 500;
pub(crate) const MAX_ARTIFACTS_LIMIT: usize = 100;
pub(crate) const MAX_CONTEXT_BYTES_LIMIT: usize = 1024 * 1024; // 1 MiB
pub(crate) const MAX_TIMEOUT_MS_LIMIT: u64 = 60_000; // 60s

/// Explicit allowlist of approved read-only operations for the evidence planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AllowedOperation {
    SearchSymbols,
    DocsContext,
    InspectContext,
    ImpactSymbol,
    ProceduralProject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TerminationPhase {
    Before,
    After,
    During,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TerminationReason {
    Timeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TerminationMetadata {
    pub(crate) operation: AllowedOperation,
    pub(crate) phase: TerminationPhase,
    pub(crate) reason: TerminationReason,
}

/// Bounded traversal output returned by capacity-aware provider methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TraversalResult {
    pub(crate) records: Vec<EvidenceRecord>,
    pub(crate) truncated: bool,
}

/// Provider of allowlisted read-only operations for planner execution.
pub(crate) trait PlannerOperationsProvider {
    fn search_symbols(&self, query: &str, limit: usize) -> Result<Vec<EvidenceRecord>, String>;
    fn docs_context(&self, query: &str, limit: usize) -> Result<Vec<EvidenceRecord>, String>;
    fn inspect_context(&self, symbol: &str) -> Result<Option<EvidenceRecord>, String>;
    fn impact_symbol(
        &self,
        symbol: &str,
        direction: TraversalDirection,
        max_hops: usize,
    ) -> Result<Vec<EvidenceRecord>, String>;
    fn procedural_project(&self, anchor: &str, hops: usize) -> Result<Vec<EvidenceRecord>, String>;

    /// Compatibility wrapper around [`Self::impact_symbol`] that truncates returned
    /// records to `max_records` and reports whether overflow occurred.
    fn impact_symbol_bounded(
        &self,
        symbol: &str,
        direction: TraversalDirection,
        max_hops: usize,
        max_records: usize,
    ) -> Result<TraversalResult, String> {
        let mut records = self.impact_symbol(symbol, direction, max_hops)?;
        let truncated = records.len() > max_records;
        if truncated {
            records.truncate(max_records);
        }
        Ok(TraversalResult { records, truncated })
    }

    /// Compatibility wrapper around [`Self::procedural_project`] that truncates
    /// returned records to `max_records` and reports whether overflow occurred.
    fn procedural_project_bounded(
        &self,
        anchor: &str,
        hops: usize,
        max_records: usize,
    ) -> Result<TraversalResult, String> {
        let mut records = self.procedural_project(anchor, hops)?;
        let truncated = records.len() > max_records;
        if truncated {
            records.truncate(max_records);
        }
        Ok(TraversalResult { records, truncated })
    }
}

/// Core retrieval shapes recognized by the evidence planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RetrievalShape {
    /// Resolve an exact symbol, document, artifact, or KM identity.
    DirectLookup,
    /// Retrieve bounded related nodes or records around an anchor.
    Neighborhood,
    /// Retrieve an evidence-backed caller, callee, impact, or dependency path.
    DependencyPath,
    /// Retrieve source, scope, version, hash, validation, and withdrawal evidence.
    Provenance,
    /// Connect evidence across source, docs, graph, and KM artifacts.
    CrossArtifactJoin,
    /// Compare or constrain evidence by stored version, commit, or timestamp metadata.
    TemporalVersion,
}

/// Traversal direction for relational retrieval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TraversalDirection {
    Upstream,
    Downstream,
    Lateral,
    Bidirectional,
}

/// Support status for an evidence record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvidenceSupportStatus {
    Supported,
    Blocked { reason: String },
    Unresolved { reason: String },
}

/// Budget constraints governing planner execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PlannerBudget {
    pub(crate) max_hops: usize,
    pub(crate) max_records: usize,
    pub(crate) max_artifacts: usize,
    pub(crate) max_context_bytes: usize,
    pub(crate) timeout_ms: u64,
}

impl Default for PlannerBudget {
    fn default() -> Self {
        Self {
            max_hops: 2,
            max_records: 50,
            max_artifacts: 10,
            max_context_bytes: 64 * 1024,
            timeout_ms: 10_000,
        }
    }
}

impl PlannerBudget {
    pub(crate) fn validate(&self) -> Result<(), PlannerModelError> {
        if self.max_hops == 0 || self.max_hops > MAX_HOPS_LIMIT {
            return Err(PlannerModelError::InvalidBudget(format!(
                "max_hops must be between 1 and {MAX_HOPS_LIMIT}, got {}",
                self.max_hops
            )));
        }
        if self.max_records == 0 || self.max_records > MAX_RECORDS_LIMIT {
            return Err(PlannerModelError::InvalidBudget(format!(
                "max_records must be between 1 and {MAX_RECORDS_LIMIT}, got {}",
                self.max_records
            )));
        }
        if self.max_artifacts == 0 || self.max_artifacts > MAX_ARTIFACTS_LIMIT {
            return Err(PlannerModelError::InvalidBudget(format!(
                "max_artifacts must be between 1 and {MAX_ARTIFACTS_LIMIT}, got {}",
                self.max_artifacts
            )));
        }
        if self.max_context_bytes == 0 || self.max_context_bytes > MAX_CONTEXT_BYTES_LIMIT {
            return Err(PlannerModelError::InvalidBudget(format!(
                "max_context_bytes must be between 1 and {MAX_CONTEXT_BYTES_LIMIT}, got {}",
                self.max_context_bytes
            )));
        }
        if self.timeout_ms == 0 || self.timeout_ms > MAX_TIMEOUT_MS_LIMIT {
            return Err(PlannerModelError::InvalidBudget(format!(
                "timeout_ms must be between 1 and {MAX_TIMEOUT_MS_LIMIT}, got {}",
                self.timeout_ms
            )));
        }
        Ok(())
    }
}

/// Typed retrieval plan representing intent and boundary constraints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RetrievalPlan {
    pub(crate) shape: RetrievalShape,
    pub(crate) anchor: String,
    pub(crate) direction: TraversalDirection,
    #[serde(default)]
    pub(crate) relation_types: Vec<String>,
    pub(crate) project_scope: Option<String>,
    pub(crate) budget: PlannerBudget,
}

impl RetrievalPlan {
    pub(crate) fn validate(&self) -> Result<(), PlannerModelError> {
        if self.anchor.trim().is_empty() {
            return Err(PlannerModelError::InvalidAnchor(
                "anchor identity cannot be empty".to_string(),
            ));
        }
        self.budget.validate()
    }
}

/// Source file and line span reference for provenance grounding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SourceLocation {
    pub(crate) path: String,
    pub(crate) start_line: Option<usize>,
    pub(crate) end_line: Option<usize>,
}

impl SourceLocation {
    pub(crate) fn validate(&self) -> Result<(), PlannerModelError> {
        if self.path.trim().is_empty() {
            return Err(PlannerModelError::MissingProvenance(
                "source location path cannot be empty".to_string(),
            ));
        }
        if let (Some(start), Some(end)) = (self.start_line, self.end_line)
            && start > end
        {
            return Err(PlannerModelError::InvalidLineRange { start, end });
        }
        Ok(())
    }
}

/// Individual evidence record with provenance and support status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct EvidenceRecord {
    pub(crate) id: String,
    pub(crate) source_location: Option<SourceLocation>,
    pub(crate) project_scope: Option<String>,
    pub(crate) version_or_hash: Option<String>,
    pub(crate) relation: Option<String>,
    pub(crate) retrieval_reason: String,
    pub(crate) support_status: EvidenceSupportStatus,
    #[serde(default)]
    pub(crate) metadata: BTreeMap<String, String>,
}

impl EvidenceRecord {
    /// Validates record invariants, ensuring identity and provenance integrity.
    pub(crate) fn validate(&self) -> Result<(), PlannerModelError> {
        if self.id.trim().is_empty() {
            return Err(PlannerModelError::MissingIdentity);
        }
        if let Some(ref loc) = self.source_location {
            loc.validate()?;
        }
        if matches!(self.support_status, EvidenceSupportStatus::Supported) {
            let has_location = self
                .source_location
                .as_ref()
                .is_some_and(|l| !l.path.trim().is_empty());
            let has_version_or_hash = self
                .version_or_hash
                .as_ref()
                .is_some_and(|v| !v.trim().is_empty());
            if !has_location && !has_version_or_hash {
                return Err(PlannerModelError::MissingProvenance(format!(
                    "supported evidence record `{}` requires source location or version/hash",
                    self.id
                )));
            }
        }
        Ok(())
    }
}

/// A collection of evidence records returned from planner execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct EvidenceSubgraph {
    pub(crate) plan: RetrievalPlan,
    pub(crate) records: Vec<EvidenceRecord>,
    pub(crate) truncated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) termination: Option<TerminationMetadata>,
}

impl EvidenceSubgraph {
    pub(crate) fn new(plan: RetrievalPlan) -> Result<Self, PlannerModelError> {
        plan.validate()?;
        Ok(Self {
            plan,
            records: Vec::new(),
            truncated: false,
            termination: None,
        })
    }

    /// Adds an evidence record, enforcing deduplication and budget bounds.
    pub(crate) fn add_record(&mut self, record: EvidenceRecord) -> Result<(), PlannerModelError> {
        record.validate()?;
        if let Some(expected_scope) = &self.plan.project_scope
            && let Some(record_scope) = &record.project_scope
            && record_scope != expected_scope
        {
            return Err(PlannerModelError::ScopeMismatch {
                expected: expected_scope.clone(),
                found: record_scope.clone(),
            });
        }
        if self.records.iter().any(|r| r.id == record.id) {
            return Ok(());
        }
        let record_artifact_key = artifact_key(&record);
        let artifact_count = self
            .records
            .iter()
            .map(artifact_key)
            .collect::<BTreeSet<_>>();
        if artifact_count.len() >= self.plan.budget.max_artifacts
            && !artifact_count.contains(&record_artifact_key)
        {
            self.truncated = true;
            return Err(PlannerModelError::BudgetExceeded(format!(
                "max_artifacts limit of {} reached",
                self.plan.budget.max_artifacts
            )));
        }
        if self.records.len() >= self.plan.budget.max_records {
            self.truncated = true;
            return Err(PlannerModelError::BudgetExceeded(format!(
                "max_records limit of {} reached",
                self.plan.budget.max_records
            )));
        }
        let serialized_len = serde_json::to_string(&record)
            .map_err(|e| PlannerModelError::SerializationError(e.to_string()))?
            .len();
        let current_len = self.total_serialized_bytes();
        if current_len + serialized_len > self.plan.budget.max_context_bytes {
            self.truncated = true;
            return Err(PlannerModelError::BudgetExceeded(format!(
                "max_context_bytes limit of {} exceeded",
                self.plan.budget.max_context_bytes
            )));
        }
        self.records.push(record);
        Ok(())
    }

    pub(crate) fn total_serialized_bytes(&self) -> usize {
        self.records
            .iter()
            .map(|r| serde_json::to_string(r).map(|s| s.len()).unwrap_or(0))
            .sum()
    }
}

fn artifact_key(record: &EvidenceRecord) -> String {
    match (
        record.source_location.as_ref(),
        record.version_or_hash.as_ref(),
    ) {
        (Some(location), Some(version)) => format!("path:{}|hash:{version}", location.path),
        (Some(location), None) => format!("path:{}", location.path),
        (None, Some(version)) => format!("hash:{version}"),
        (None, None) => format!("id:{}", record.id),
    }
}

fn add_record_or_blocked(
    subgraph: &mut EvidenceSubgraph,
    record: EvidenceRecord,
) -> Result<(), PlannerModelError> {
    match subgraph.add_record(record.clone()) {
        Ok(()) => Ok(()),
        // ADR "Bounds and stopping": failure to reach coverage returns a
        // bounded incomplete result, so an exhausted budget marks the subgraph
        // truncated and stops emission instead of discarding what was collected.
        Err(PlannerModelError::BudgetExceeded(_)) => {
            subgraph.truncated = true;
            Ok(())
        }
        Err(error) => {
            let blocked = EvidenceRecord {
                id: format!("Blocked:AddRecord:{}", record.id),
                source_location: None,
                project_scope: subgraph.plan.project_scope.clone(),
                version_or_hash: None,
                relation: record.relation,
                retrieval_reason: "evidence record rejected by planner boundary".to_string(),
                support_status: EvidenceSupportStatus::Blocked {
                    reason: error.to_string(),
                },
                metadata: BTreeMap::new(),
            };
            match subgraph.add_record(blocked) {
                Ok(()) => Ok(()),
                Err(blocked_error) => Err(blocked_error),
            }
        }
    }
}

/// Maximum relation names reported in one relation-filter drop record.
const MAX_REPORTED_RELATIONS: usize = 8;

/// Relation filtering applies to supported traversal evidence only.
///
/// Blocked and unresolved records describe boundary conditions such as an
/// exhausted budget or a reduced traversal depth, so they stay visible even
/// when the caller narrowed the requested relation types.
///
/// Comparison is ASCII case-insensitive because relation vocabularies differ
/// by lane: the source index emits uppercase kinds such as `CALLS`, while KM
/// transitions carry author-defined names. A caller asking for `calls` means
/// the same relation as `CALLS`.
fn relation_filter_admits(record: &EvidenceRecord, relation_types: &[String]) -> bool {
    if relation_types.is_empty() {
        return true;
    }
    if !matches!(record.support_status, EvidenceSupportStatus::Supported) {
        return true;
    }
    record.relation.as_ref().is_some_and(|relation| {
        relation_types
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(relation))
    })
}

fn join_bounded(values: &BTreeSet<String>) -> String {
    values
        .iter()
        .take(MAX_REPORTED_RELATIONS)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ")
}

fn add_traversal_records(
    subgraph: &mut EvidenceSubgraph,
    records: Vec<EvidenceRecord>,
    max_hops: usize,
    relation_types: &[String],
) -> Result<(), PlannerModelError> {
    let mut admitted = 0usize;
    let mut dropped_relations = BTreeSet::new();
    for record in records {
        // A requested relation filter is part of the plan contract: evidence
        // outside the requested relations is never emitted as supported.
        if !relation_filter_admits(&record, relation_types) {
            dropped_relations.insert(
                record
                    .relation
                    .clone()
                    .unwrap_or_else(|| "<unlabeled>".to_string()),
            );
            continue;
        }
        admitted += 1;
        if let Some(hops) = record
            .metadata
            .get("hops")
            .and_then(|value| value.parse::<usize>().ok())
        {
            // Records beyond the hop budget are dropped and the subgraph is
            // marked truncated, keeping shallower evidence rather than
            // discarding the whole shape.
            if hops == 0 || hops > max_hops {
                subgraph.truncated = true;
                continue;
            }
        }
        add_record_or_blocked(subgraph, record)?;
    }
    // A filter that removed every candidate is indistinguishable from absent
    // evidence unless the planner says so. Report the requested and observed
    // relations instead of returning a silently empty traversal.
    if admitted == 0 && !dropped_relations.is_empty() {
        let requested = relation_types.iter().cloned().collect::<BTreeSet<_>>();
        let blocked = EvidenceRecord {
            id: format!("Blocked:RelationFilter:{}", subgraph.plan.anchor),
            source_location: None,
            project_scope: subgraph.plan.project_scope.clone(),
            version_or_hash: None,
            relation: None,
            retrieval_reason: format!(
                "relation filter removed every traversal record for `{}`",
                subgraph.plan.anchor
            ),
            support_status: EvidenceSupportStatus::Blocked {
                reason: format!(
                    "no traversal evidence matched requested relations [{}]; observed relations were [{}]",
                    join_bounded(&requested),
                    join_bounded(&dropped_relations)
                ),
            },
            metadata: BTreeMap::from([
                ("requested_relations".to_string(), join_bounded(&requested)),
                (
                    "observed_relations".to_string(),
                    join_bounded(&dropped_relations),
                ),
            ]),
        };
        add_record_or_blocked(subgraph, blocked)?;
    }
    Ok(())
}

fn deadline_expired(started: Instant, deadline: Duration) -> bool {
    started.elapsed() > deadline
}

fn termination_operation(operation: &str) -> AllowedOperation {
    match operation {
        "search_symbols" | "direct_lookup" => AllowedOperation::SearchSymbols,
        "docs_context" => AllowedOperation::DocsContext,
        "inspect_context" => AllowedOperation::InspectContext,
        "impact_symbol" => AllowedOperation::ImpactSymbol,
        "procedural_project" => AllowedOperation::ProceduralProject,
        _ => unreachable!("planner timeout operation must be allowlisted"),
    }
}

fn set_timeout_termination(
    subgraph: &mut EvidenceSubgraph,
    operation: &str,
    phase: TerminationPhase,
) {
    subgraph.termination = Some(TerminationMetadata {
        operation: termination_operation(operation),
        phase,
        reason: TerminationReason::Timeout,
    });
}

fn add_timeout_record(
    subgraph: &mut EvidenceSubgraph,
    operation: &str,
    timeout_ms: u64,
    phase: &str,
) -> Result<(), PlannerModelError> {
    subgraph.truncated = true;
    set_timeout_termination(
        subgraph,
        operation,
        match phase {
            "before" => TerminationPhase::Before,
            "after" | "before completion" => TerminationPhase::After,
            "during" => TerminationPhase::During,
            _ => unreachable!("planner timeout phase must be finite"),
        },
    );
    let project_scope = subgraph.plan.project_scope.clone();
    let anchor = subgraph.plan.anchor.clone();
    add_record_or_blocked(
        subgraph,
        EvidenceRecord {
            id: format!("Blocked:Timeout:{operation}:{anchor}"),
            source_location: None,
            project_scope,
            version_or_hash: None,
            relation: None,
            retrieval_reason: format!("planner deadline expired {phase} `{operation}`"),
            support_status: EvidenceSupportStatus::Blocked {
                reason: format!(
                    "planner deadline of {timeout_ms} ms expired {phase} `{operation}`"
                ),
            },
            metadata: BTreeMap::from([
                ("operation".to_string(), operation.to_string()),
                ("timeout_ms".to_string(), timeout_ms.to_string()),
            ]),
        },
    )
}

fn stop_before_operation(
    subgraph: &mut EvidenceSubgraph,
    operation: &str,
    started: Instant,
    deadline: Duration,
    timeout_ms: u64,
) -> Result<bool, PlannerModelError> {
    if !deadline_expired(started, deadline) {
        return Ok(false);
    }
    add_timeout_record(subgraph, operation, timeout_ms, "before")?;
    Ok(true)
}

fn stop_after_operation(
    subgraph: &mut EvidenceSubgraph,
    operation: AllowedOperation,
    shape: RetrievalShape,
    allowed_operations: &BTreeSet<AllowedOperation>,
    started: Instant,
    deadline: Duration,
    timeout_ms: u64,
) -> Result<bool, PlannerModelError> {
    if !deadline_expired(started, deadline) {
        return Ok(false);
    }
    let operation = next_operation_after(operation, shape, allowed_operations)
        .unwrap_or_else(|| operation_name(operation));
    add_timeout_record(subgraph, operation, timeout_ms, "after")?;
    Ok(true)
}

fn next_operation_after(
    current: AllowedOperation,
    shape: RetrievalShape,
    allowed_operations: &BTreeSet<AllowedOperation>,
) -> Option<&'static str> {
    let mut ordered = vec![
        AllowedOperation::SearchSymbols,
        AllowedOperation::DocsContext,
    ];
    match shape {
        RetrievalShape::DirectLookup => {}
        RetrievalShape::Neighborhood | RetrievalShape::DependencyPath => {
            ordered.extend([
                AllowedOperation::InspectContext,
                AllowedOperation::ImpactSymbol,
            ]);
        }
        RetrievalShape::Provenance | RetrievalShape::TemporalVersion => {
            ordered.push(AllowedOperation::InspectContext);
        }
        RetrievalShape::CrossArtifactJoin => {
            ordered.extend([
                AllowedOperation::InspectContext,
                AllowedOperation::ProceduralProject,
            ]);
        }
    }
    ordered
        .into_iter()
        .skip_while(|operation| *operation != current)
        .skip(1)
        .find(|operation| allowed_operations.contains(operation))
        .map(operation_name)
}

fn operation_name(operation: AllowedOperation) -> &'static str {
    match operation {
        AllowedOperation::SearchSymbols => "search_symbols",
        AllowedOperation::DocsContext => "docs_context",
        AllowedOperation::InspectContext => "inspect_context",
        AllowedOperation::ImpactSymbol => "impact_symbol",
        AllowedOperation::ProceduralProject => "procedural_project",
    }
}

#[allow(clippy::too_many_arguments)]
fn add_operation_error(
    subgraph: &mut EvidenceSubgraph,
    id: String,
    operation: &str,
    retrieval_reason: &str,
    error: String,
    started: Instant,
    deadline: Duration,
    timeout_ms: u64,
) -> Result<bool, PlannerModelError> {
    let timed_out = deadline_expired(started, deadline)
        || error.to_ascii_lowercase().contains("timeout")
        || error.to_ascii_lowercase().contains("timed out");
    let reason = if timed_out && !error.to_ascii_lowercase().contains("timeout") {
        format!("{error}; planner deadline of {timeout_ms} ms expired")
    } else {
        error
    };
    if timed_out {
        subgraph.truncated = true;
        set_timeout_termination(subgraph, operation, TerminationPhase::During);
    }
    let project_scope = subgraph.plan.project_scope.clone();
    add_record_or_blocked(
        subgraph,
        EvidenceRecord {
            id,
            source_location: None,
            project_scope,
            version_or_hash: None,
            relation: None,
            retrieval_reason: retrieval_reason.to_string(),
            support_status: EvidenceSupportStatus::Blocked { reason },
            metadata: BTreeMap::from([("operation".to_string(), operation.to_string())]),
        },
    )?;
    Ok(timed_out)
}

/// Errors produced when creating or validating planner structures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlannerModelError {
    MissingIdentity,
    MissingProvenance(String),
    InvalidLineRange {
        start: usize,
        end: usize,
    },
    InvalidAnchor(String),
    InvalidBudget(String),
    BudgetExceeded(String),
    ScopeMismatch {
        expected: String,
        found: String,
    },
    #[allow(dead_code)] // reserved error category; no producer yet
    TimeoutExceeded {
        timeout_ms: u64,
    },
    SerializationError(String),
}

impl fmt::Display for PlannerModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity => write!(f, "evidence record missing required stable identity"),
            Self::MissingProvenance(msg) => write!(f, "evidence record missing provenance: {msg}"),
            Self::InvalidLineRange { start, end } => {
                write!(f, "invalid line range: start line {start} > end line {end}")
            }
            Self::InvalidAnchor(msg) => write!(f, "invalid retrieval plan anchor: {msg}"),
            Self::InvalidBudget(msg) => write!(f, "invalid planner budget: {msg}"),
            Self::BudgetExceeded(msg) => write!(f, "planner budget exceeded: {msg}"),
            Self::ScopeMismatch { expected, found } => write!(
                f,
                "project scope mismatch: expected `{expected}`, found `{found}`"
            ),
            Self::TimeoutExceeded { timeout_ms } => {
                write!(f, "planner execution exceeded timeout of {timeout_ms} ms")
            }
            Self::SerializationError(msg) => write!(f, "serialization error: {msg}"),
        }
    }
}

impl std::error::Error for PlannerModelError {}

/// Bounded planner execution engine composing allowlisted read-only operations.
pub(crate) struct PlannerEngine<'a, P: PlannerOperationsProvider> {
    provider: &'a P,
    allowed_operations: BTreeSet<AllowedOperation>,
}

impl<'a, P: PlannerOperationsProvider> PlannerEngine<'a, P> {
    pub(crate) fn new(provider: &'a P) -> Self {
        let mut allowed = BTreeSet::new();
        allowed.insert(AllowedOperation::SearchSymbols);
        allowed.insert(AllowedOperation::DocsContext);
        allowed.insert(AllowedOperation::InspectContext);
        allowed.insert(AllowedOperation::ImpactSymbol);
        allowed.insert(AllowedOperation::ProceduralProject);
        Self {
            provider,
            allowed_operations: allowed,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_allowlist(
        provider: &'a P,
        allowed_operations: BTreeSet<AllowedOperation>,
    ) -> Self {
        Self {
            provider,
            allowed_operations,
        }
    }

    /// Executes a retrieval plan enforcing textual-first order, allowlisted operations,
    /// budget bounds, and adaptive stopping.
    pub(crate) fn execute(
        &self,
        plan: RetrievalPlan,
    ) -> Result<EvidenceSubgraph, PlannerModelError> {
        let mut subgraph = EvidenceSubgraph::new(plan.clone())?;
        let started = Instant::now();
        let deadline = Duration::from_millis(plan.budget.timeout_ms);

        // Phase 1: Textual-first retrieval floor.
        // Symbol lookup and documentation context ALWAYS execute before any graph traversal step.
        let mut anchor_resolved = false;
        if self
            .allowed_operations
            .contains(&AllowedOperation::SearchSymbols)
        {
            if stop_before_operation(
                &mut subgraph,
                "search_symbols",
                started,
                deadline,
                plan.budget.timeout_ms,
            )? {
                return Ok(subgraph);
            }
            match self
                .provider
                .search_symbols(&plan.anchor, plan.budget.max_records)
            {
                Ok(records) => {
                    for record in records {
                        if record.id == plan.anchor || record.id.ends_with(&plan.anchor) {
                            anchor_resolved = true;
                        }
                        add_record_or_blocked(&mut subgraph, record)?;
                    }
                    if stop_after_operation(
                        &mut subgraph,
                        AllowedOperation::SearchSymbols,
                        plan.shape,
                        &self.allowed_operations,
                        started,
                        deadline,
                        plan.budget.timeout_ms,
                    )? {
                        return Ok(subgraph);
                    }
                }
                Err(err) => {
                    if add_operation_error(
                        &mut subgraph,
                        format!("Error:SearchSymbols:{}", plan.anchor),
                        "search_symbols",
                        "textual search error",
                        err,
                        started,
                        deadline,
                        plan.budget.timeout_ms,
                    )? {
                        return Ok(subgraph);
                    }
                }
            }
        }

        if self
            .allowed_operations
            .contains(&AllowedOperation::DocsContext)
        {
            if stop_before_operation(
                &mut subgraph,
                "docs_context",
                started,
                deadline,
                plan.budget.timeout_ms,
            )? {
                return Ok(subgraph);
            }
            match self
                .provider
                .docs_context(&plan.anchor, plan.budget.max_records)
            {
                Ok(records) => {
                    for record in records {
                        add_record_or_blocked(&mut subgraph, record)?;
                    }
                    if stop_after_operation(
                        &mut subgraph,
                        AllowedOperation::DocsContext,
                        plan.shape,
                        &self.allowed_operations,
                        started,
                        deadline,
                        plan.budget.timeout_ms,
                    )? {
                        return Ok(subgraph);
                    }
                }
                Err(err) => {
                    if add_operation_error(
                        &mut subgraph,
                        format!("Error:DocsContext:{}", plan.anchor),
                        "docs_context",
                        "docs context error",
                        err,
                        started,
                        deadline,
                        plan.budget.timeout_ms,
                    )? {
                        return Ok(subgraph);
                    }
                }
            }
        }

        // Adaptive stopping for DirectLookup:
        // If the anchor is already resolved with supported provenance, stop without graph traversal.
        if plan.shape == RetrievalShape::DirectLookup {
            if !anchor_resolved {
                add_record_or_blocked(
                    &mut subgraph,
                    EvidenceRecord {
                        id: format!("Unresolved:{}", plan.anchor),
                        source_location: None,
                        project_scope: plan.project_scope.clone(),
                        version_or_hash: None,
                        relation: None,
                        retrieval_reason: "direct lookup unresolved in textual pass".to_string(),
                        support_status: EvidenceSupportStatus::Unresolved {
                            reason: format!(
                                "symbol or document anchor `{}` was not found",
                                plan.anchor
                            ),
                        },
                        metadata: BTreeMap::new(),
                    },
                )?;
            }
            if deadline_expired(started, deadline) {
                add_timeout_record(
                    &mut subgraph,
                    "direct_lookup",
                    plan.budget.timeout_ms,
                    "before completion",
                )?;
            }
            return Ok(subgraph);
        }

        // Phase 2: Relational and graph traversal based on retrieval shape.
        match plan.shape {
            RetrievalShape::DirectLookup => unreachable!(),
            RetrievalShape::Neighborhood | RetrievalShape::DependencyPath => {
                if self
                    .allowed_operations
                    .contains(&AllowedOperation::InspectContext)
                {
                    if stop_before_operation(
                        &mut subgraph,
                        "inspect_context",
                        started,
                        deadline,
                        plan.budget.timeout_ms,
                    )? {
                        return Ok(subgraph);
                    }
                    match self.provider.inspect_context(&plan.anchor) {
                        Ok(Some(record)) => {
                            add_record_or_blocked(&mut subgraph, record)?;
                        }
                        Ok(None) => {}
                        Err(err) => {
                            if add_operation_error(
                                &mut subgraph,
                                format!("Error:InspectContext:{}", plan.anchor),
                                "inspect_context",
                                "inspect context error",
                                err,
                                started,
                                deadline,
                                plan.budget.timeout_ms,
                            )? {
                                return Ok(subgraph);
                            }
                        }
                    }
                    if stop_after_operation(
                        &mut subgraph,
                        AllowedOperation::InspectContext,
                        plan.shape,
                        &self.allowed_operations,
                        started,
                        deadline,
                        plan.budget.timeout_ms,
                    )? {
                        return Ok(subgraph);
                    }
                }
                if self
                    .allowed_operations
                    .contains(&AllowedOperation::ImpactSymbol)
                {
                    if stop_before_operation(
                        &mut subgraph,
                        "impact_symbol",
                        started,
                        deadline,
                        plan.budget.timeout_ms,
                    )? {
                        return Ok(subgraph);
                    }
                    match self.provider.impact_symbol_bounded(
                        &plan.anchor,
                        plan.direction,
                        plan.budget.max_hops,
                        plan.budget
                            .max_records
                            .saturating_sub(subgraph.records.len()),
                    ) {
                        Ok(result) => {
                            subgraph.truncated |= result.truncated;
                            add_traversal_records(
                                &mut subgraph,
                                result.records,
                                plan.budget.max_hops,
                                &plan.relation_types,
                            )?;
                        }
                        Err(err) => {
                            if add_operation_error(
                                &mut subgraph,
                                format!("Error:ImpactSymbol:{}", plan.anchor),
                                "impact_symbol",
                                "impact traversal error",
                                err,
                                started,
                                deadline,
                                plan.budget.timeout_ms,
                            )? {
                                return Ok(subgraph);
                            }
                        }
                    }
                    if stop_after_operation(
                        &mut subgraph,
                        AllowedOperation::ImpactSymbol,
                        plan.shape,
                        &self.allowed_operations,
                        started,
                        deadline,
                        plan.budget.timeout_ms,
                    )? {
                        return Ok(subgraph);
                    }
                }
            }
            RetrievalShape::Provenance => {
                if self
                    .allowed_operations
                    .contains(&AllowedOperation::InspectContext)
                {
                    if stop_before_operation(
                        &mut subgraph,
                        "inspect_context",
                        started,
                        deadline,
                        plan.budget.timeout_ms,
                    )? {
                        return Ok(subgraph);
                    }
                    match self.provider.inspect_context(&plan.anchor) {
                        Ok(Some(record)) => {
                            add_record_or_blocked(&mut subgraph, record)?;
                        }
                        Ok(None) => {}
                        Err(err) => {
                            if add_operation_error(
                                &mut subgraph,
                                format!("Error:InspectContext:{}", plan.anchor),
                                "inspect_context",
                                "inspect context error",
                                err,
                                started,
                                deadline,
                                plan.budget.timeout_ms,
                            )? {
                                return Ok(subgraph);
                            }
                        }
                    }
                    if stop_after_operation(
                        &mut subgraph,
                        AllowedOperation::InspectContext,
                        plan.shape,
                        &self.allowed_operations,
                        started,
                        deadline,
                        plan.budget.timeout_ms,
                    )? {
                        return Ok(subgraph);
                    }
                }
            }
            RetrievalShape::CrossArtifactJoin => {
                if self
                    .allowed_operations
                    .contains(&AllowedOperation::InspectContext)
                {
                    if stop_before_operation(
                        &mut subgraph,
                        "inspect_context",
                        started,
                        deadline,
                        plan.budget.timeout_ms,
                    )? {
                        return Ok(subgraph);
                    }
                    match self.provider.inspect_context(&plan.anchor) {
                        Ok(Some(record)) => {
                            add_record_or_blocked(&mut subgraph, record)?;
                        }
                        Ok(None) => {}
                        Err(err) => {
                            if add_operation_error(
                                &mut subgraph,
                                format!("Error:InspectContext:{}", plan.anchor),
                                "inspect_context",
                                "inspect context error",
                                err,
                                started,
                                deadline,
                                plan.budget.timeout_ms,
                            )? {
                                return Ok(subgraph);
                            }
                        }
                    }
                    if stop_after_operation(
                        &mut subgraph,
                        AllowedOperation::InspectContext,
                        plan.shape,
                        &self.allowed_operations,
                        started,
                        deadline,
                        plan.budget.timeout_ms,
                    )? {
                        return Ok(subgraph);
                    }
                }
                if self
                    .allowed_operations
                    .contains(&AllowedOperation::ProceduralProject)
                {
                    if stop_before_operation(
                        &mut subgraph,
                        "procedural_project",
                        started,
                        deadline,
                        plan.budget.timeout_ms,
                    )? {
                        return Ok(subgraph);
                    }
                    match self.provider.procedural_project_bounded(
                        &plan.anchor,
                        plan.budget.max_hops,
                        plan.budget
                            .max_records
                            .saturating_sub(subgraph.records.len()),
                    ) {
                        Ok(result) => {
                            subgraph.truncated |= result.truncated;
                            add_traversal_records(
                                &mut subgraph,
                                result.records,
                                plan.budget.max_hops,
                                &plan.relation_types,
                            )?;
                        }
                        Err(err) => {
                            if add_operation_error(
                                &mut subgraph,
                                format!("Error:ProceduralProject:{}", plan.anchor),
                                "procedural_project",
                                "procedural projection error",
                                err,
                                started,
                                deadline,
                                plan.budget.timeout_ms,
                            )? {
                                return Ok(subgraph);
                            }
                        }
                    }
                    if stop_after_operation(
                        &mut subgraph,
                        AllowedOperation::ProceduralProject,
                        plan.shape,
                        &self.allowed_operations,
                        started,
                        deadline,
                        plan.budget.timeout_ms,
                    )? {
                        return Ok(subgraph);
                    }
                }
            }
            RetrievalShape::TemporalVersion => {
                if self
                    .allowed_operations
                    .contains(&AllowedOperation::InspectContext)
                {
                    if stop_before_operation(
                        &mut subgraph,
                        "inspect_context",
                        started,
                        deadline,
                        plan.budget.timeout_ms,
                    )? {
                        return Ok(subgraph);
                    }
                    match self.provider.inspect_context(&plan.anchor) {
                        Ok(Some(record)) => {
                            add_record_or_blocked(&mut subgraph, record)?;
                        }
                        Ok(None) => {}
                        Err(err) => {
                            if add_operation_error(
                                &mut subgraph,
                                format!("Error:InspectContext:{}", plan.anchor),
                                "inspect_context",
                                "inspect context error",
                                err,
                                started,
                                deadline,
                                plan.budget.timeout_ms,
                            )? {
                                return Ok(subgraph);
                            }
                        }
                    }
                    if stop_after_operation(
                        &mut subgraph,
                        AllowedOperation::InspectContext,
                        plan.shape,
                        &self.allowed_operations,
                        started,
                        deadline,
                        plan.budget.timeout_ms,
                    )? {
                        return Ok(subgraph);
                    }
                }
            }
        }

        Ok(subgraph)
    }
}

#[cfg(test)]
#[path = "evidence_planner_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "evidence_planner_measurement_tests.rs"]
mod measurement_tests;
