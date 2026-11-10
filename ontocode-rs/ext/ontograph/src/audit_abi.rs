use std::collections::BTreeMap;

use ontocode_extension_api::FunctionCallError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::audit_build::AuditAction;
use crate::query::MAX_QUERY_ITEMS;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditAbiStatus {
    Compatible,
    Incompatible,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditAbiFindingKind {
    FieldAdded,
    FieldRemoved,
    TypeChanged,
    NullabilityChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditAbiFinding {
    pub kind: AuditAbiFindingKind,
    pub field: String,
    pub compatible: bool,
    pub source_type: Option<String>,
    pub target_type: Option<String>,
    pub source_nullable: Option<bool>,
    pub target_nullable: Option<bool>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditAbiResult {
    pub action: AuditAction,
    pub status: AuditAbiStatus,
    pub source_name: Option<String>,
    pub target_name: Option<String>,
    pub findings: Vec<AuditAbiFinding>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct NormalizedSchema {
    name: Option<String>,
    fields: BTreeMap<String, NormalizedField>,
}

#[derive(Debug, Clone, Deserialize)]
struct NormalizedField {
    #[serde(rename = "type")]
    type_name: String,
    #[serde(default)]
    nullable: bool,
}

pub(crate) fn run_abi_audit(
    source_schema: Option<Value>,
    target_schema: Option<Value>,
    max_items: usize,
) -> Result<AuditAbiResult, FunctionCallError> {
    let source = parse_schema(source_schema, "source_schema")?;
    let target = parse_schema(target_schema, "target_schema")?;
    let mut findings = compare_fields(&source.fields, &target.fields);
    let truncated = findings.len() > max_items;
    findings.truncate(max_items);
    let status = if findings.iter().any(|finding| !finding.compatible) {
        AuditAbiStatus::Incompatible
    } else {
        AuditAbiStatus::Compatible
    };

    Ok(AuditAbiResult {
        action: AuditAction::Abi,
        status,
        source_name: source.name,
        target_name: target.name,
        findings,
        truncated,
        limits: vec![
            format!("max_items={max_items}"),
            format!("max_schema_fields={MAX_QUERY_ITEMS}"),
        ],
        notes: vec![
            "audit.abi compares caller-supplied normalized JSON schema objects only."
                .to_string(),
            "Field additions are compatibility warnings; removals, type changes, and stricter nullability are incompatible.".to_string(),
        ],
    })
}

fn parse_schema(value: Option<Value>, name: &str) -> Result<NormalizedSchema, FunctionCallError> {
    let value = value.ok_or_else(|| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.audit {name} is required for action `abi`"
        ))
    })?;
    if !value.is_object() {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit {name} must be a normalized JSON schema object"
        )));
    }
    let schema: NormalizedSchema = serde_json::from_value(value).map_err(|err| {
        FunctionCallError::RespondToModel(format!("ontograph.audit {name} is malformed: {err}"))
    })?;
    if schema.fields.len() > MAX_QUERY_ITEMS {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit {name} has too many fields; max_schema_fields={MAX_QUERY_ITEMS}"
        )));
    }
    Ok(schema)
}

fn compare_fields(
    source: &BTreeMap<String, NormalizedField>,
    target: &BTreeMap<String, NormalizedField>,
) -> Vec<AuditAbiFinding> {
    let mut findings = Vec::new();
    for (field, source_field) in source {
        match target.get(field) {
            Some(target_field) => {
                if source_field.type_name != target_field.type_name {
                    findings.push(AuditAbiFinding {
                        kind: AuditAbiFindingKind::TypeChanged,
                        field: field.clone(),
                        compatible: false,
                        source_type: Some(source_field.type_name.clone()),
                        target_type: Some(target_field.type_name.clone()),
                        source_nullable: Some(source_field.nullable),
                        target_nullable: Some(target_field.nullable),
                        detail: format!(
                            "field `{field}` type changed from `{}` to `{}`",
                            source_field.type_name, target_field.type_name
                        ),
                    });
                }
                if source_field.nullable && !target_field.nullable {
                    findings.push(AuditAbiFinding {
                        kind: AuditAbiFindingKind::NullabilityChanged,
                        field: field.clone(),
                        compatible: false,
                        source_type: Some(source_field.type_name.clone()),
                        target_type: Some(target_field.type_name.clone()),
                        source_nullable: Some(source_field.nullable),
                        target_nullable: Some(target_field.nullable),
                        detail: format!("field `{field}` became non-nullable"),
                    });
                }
            }
            None => findings.push(AuditAbiFinding {
                kind: AuditAbiFindingKind::FieldRemoved,
                field: field.clone(),
                compatible: false,
                source_type: Some(source_field.type_name.clone()),
                target_type: None,
                source_nullable: Some(source_field.nullable),
                target_nullable: None,
                detail: format!("field `{field}` was removed"),
            }),
        }
    }

    for (field, target_field) in target {
        if !source.contains_key(field) {
            findings.push(AuditAbiFinding {
                kind: AuditAbiFindingKind::FieldAdded,
                field: field.clone(),
                compatible: true,
                source_type: None,
                target_type: Some(target_field.type_name.clone()),
                source_nullable: None,
                target_nullable: Some(target_field.nullable),
                detail: format!("field `{field}` was added"),
            });
        }
    }
    findings
}
