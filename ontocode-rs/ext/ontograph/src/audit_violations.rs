use ontocode_extension_api::FunctionCallError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::audit_build::AuditAction;
use crate::query::GraphSccInputRequest;
use crate::query::OntographReadQuery;
use crate::query::QueryBudget;

const MAX_VIOLATION_ROWS: usize = 100;
const MAX_RULES: usize = 32;
const MAX_PREFIX_LENGTH: usize = 160;

#[derive(Debug, Clone, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuditViolationRuleInput {
    pub rule_id: String,
    pub from_path_prefix: String,
    pub forbidden_to_path_prefix: String,
    #[serde(default)]
    pub severity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditViolationFinding {
    pub rule_id: String,
    pub severity: Option<String>,
    pub from_path: String,
    pub to_path: String,
    pub from_symbol: String,
    pub to_symbol: String,
    pub relationship_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditViolationSkippedRule {
    pub rule_id: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditViolationsResult {
    pub action: AuditAction,
    pub findings: Vec<AuditViolationFinding>,
    pub skipped_rules: Vec<AuditViolationSkippedRule>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
struct AuditViolationRule {
    rule_id: String,
    from_path_prefix: String,
    forbidden_to_path_prefix: String,
    severity: Option<String>,
}

pub(crate) fn run_violations_audit(
    query: &impl OntographReadQuery,
    rules: Option<Vec<AuditViolationRuleInput>>,
    max_items: usize,
) -> Result<AuditViolationsResult, FunctionCallError> {
    let max_items = max_items.clamp(1, MAX_VIOLATION_ROWS);
    let (rules, skipped_rules) = normalize_rules(rules)?;
    let edges = query
        .graph_scc_inputs(GraphSccInputRequest {
            budget: QueryBudget::bounded(
                QueryBudget::default().timeout_ms,
                crate::query::MAX_QUERY_ITEMS,
                QueryBudget::default().max_serialized_bytes,
            ),
        })
        .map_err(|_| {
            FunctionCallError::RespondToModel(
                "ontograph.audit failed to read boundary violation graph facts".to_string(),
            )
        })?
        .edges;
    if edges.truncated {
        return Err(FunctionCallError::RespondToModel(
            "ontograph.audit violation input facts are truncated; refusing to report partial boundary violations"
                .to_string(),
        ));
    }

    let mut findings = Vec::new();
    for edge in edges.items {
        for rule in &rules {
            if edge.from_path.starts_with(&rule.from_path_prefix)
                && edge.to_path.starts_with(&rule.forbidden_to_path_prefix)
            {
                findings.push(AuditViolationFinding {
                    rule_id: rule.rule_id.clone(),
                    severity: rule.severity.clone(),
                    from_path: edge.from_path.clone(),
                    to_path: edge.to_path.clone(),
                    from_symbol: edge.from_symbol.clone(),
                    to_symbol: edge.to_symbol.clone(),
                    relationship_id: edge.relationship_id.clone(),
                });
            }
        }
    }
    findings.sort_by(|left, right| {
        left.rule_id
            .cmp(&right.rule_id)
            .then(left.from_path.cmp(&right.from_path))
            .then(left.to_path.cmp(&right.to_path))
            .then(left.relationship_id.cmp(&right.relationship_id))
    });
    let truncated = findings.len() > max_items;
    findings.truncate(max_items);

    Ok(AuditViolationsResult {
        action: AuditAction::Violations,
        findings,
        skipped_rules,
        truncated,
        limits: vec![
            format!("max_items={max_items}"),
            format!("max_violation_rows={MAX_VIOLATION_ROWS}"),
            format!("max_rules={MAX_RULES}"),
        ],
        notes: vec![
            "This native slice checks caller-supplied path-prefix rules over resolved graph CALLS facts only."
                .to_string(),
            "Unresolved calls are not guessed into boundary violations.".to_string(),
            "Rules are local request data only; no repo config, DSL, or semantic owner inference is used."
                .to_string(),
        ],
    })
}

fn normalize_rules(
    rules: Option<Vec<AuditViolationRuleInput>>,
) -> Result<(Vec<AuditViolationRule>, Vec<AuditViolationSkippedRule>), FunctionCallError> {
    let rules = rules.unwrap_or_default();
    if rules.len() > MAX_RULES {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit violation rules exceed the maximum of {MAX_RULES} items"
        )));
    }

    let mut accepted = Vec::new();
    let mut skipped = Vec::new();
    for rule in rules {
        let rule_id = rule.rule_id.trim().to_string();
        let from_path_prefix = normalize_prefix(&rule.from_path_prefix);
        let forbidden_to_path_prefix = normalize_prefix(&rule.forbidden_to_path_prefix);
        let severity = rule
            .severity
            .map(|severity| severity.trim().to_string())
            .filter(|severity| !severity.is_empty());
        if rule_id.is_empty() {
            skipped.push(AuditViolationSkippedRule {
                rule_id: None,
                reason: "missing rule_id".to_string(),
            });
            continue;
        }
        if from_path_prefix.is_empty() {
            skipped.push(AuditViolationSkippedRule {
                rule_id: Some(rule_id),
                reason: "missing from_path_prefix".to_string(),
            });
            continue;
        }
        if forbidden_to_path_prefix.is_empty() {
            skipped.push(AuditViolationSkippedRule {
                rule_id: Some(rule_id),
                reason: "missing forbidden_to_path_prefix".to_string(),
            });
            continue;
        }
        if from_path_prefix.len() > MAX_PREFIX_LENGTH
            || forbidden_to_path_prefix.len() > MAX_PREFIX_LENGTH
        {
            skipped.push(AuditViolationSkippedRule {
                rule_id: Some(rule_id),
                reason: format!("path prefixes must be at most {MAX_PREFIX_LENGTH} characters"),
            });
            continue;
        }
        accepted.push(AuditViolationRule {
            rule_id,
            from_path_prefix,
            forbidden_to_path_prefix,
            severity,
        });
    }
    Ok((accepted, skipped))
}

fn normalize_prefix(prefix: &str) -> String {
    prefix.trim().trim_start_matches('/').replace('\\', "/")
}
