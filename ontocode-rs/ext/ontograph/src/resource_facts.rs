use std::collections::BTreeSet;

use serde::Deserialize;
use serde::Serialize;

use crate::rust_parser::ParsedFile;

pub(crate) const MAX_RESOURCE_FACTS: usize = 256;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct IndexedResourceFact {
    pub resource_fact_id: String,
    pub kind: String,
    pub operation: String,
    pub path: String,
    pub line: usize,
    #[serde(default)]
    pub owner_node_id: Option<String>,
    #[serde(default)]
    pub owner_symbol: Option<String>,
    #[serde(default)]
    pub owner_path: Option<String>,
    #[serde(default)]
    pub owner_line: Option<usize>,
    pub status: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ResourceOperation {
    kind: &'static str,
    operation: &'static str,
}

pub(crate) fn collect_resource_facts(
    path: &str,
    content: &str,
    parsed: Option<&ParsedFile>,
) -> Vec<IndexedResourceFact> {
    let mut facts = Vec::new();
    let mut seen = BTreeSet::new();

    if let Some(parsed) = parsed {
        for call in &parsed.calls {
            let Some(operation) = operation_for_name(&call.name) else {
                continue;
            };
            push_fact(
                &mut facts,
                &mut seen,
                path,
                call.line,
                operation,
                call.enclosing_fn.as_deref(),
                (call.enclosing_fn.is_some()).then_some(call.enclosing_fn_line),
            );
        }
    } else {
        collect_fallback_call_facts(path, content, &mut facts, &mut seen);
    }

    for (line_index, raw_line) in content.lines().enumerate() {
        let line = crate::indexer::strip_comments_and_strings(raw_line);
        for operation in [
            ResourceOperation {
                kind: "handoff",
                operation: "SCM_RIGHTS",
            },
            ResourceOperation {
                kind: "handoff",
                operation: "close_on_exec",
            },
        ] {
            if contains_token(&line, operation.operation) {
                let owner = owner_for_line(content, line_index + 1);
                push_fact(
                    &mut facts,
                    &mut seen,
                    path,
                    line_index + 1,
                    operation,
                    owner.as_ref().map(|(name, _)| name.as_str()),
                    owner.as_ref().map(|(_, line)| *line),
                );
            }
        }
    }

    facts.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.operation.cmp(&right.operation))
            .then_with(|| left.resource_fact_id.cmp(&right.resource_fact_id))
    });
    facts
}

fn collect_fallback_call_facts(
    path: &str,
    content: &str,
    facts: &mut Vec<IndexedResourceFact>,
    seen: &mut BTreeSet<(String, usize, String)>,
) {
    let mut owner: Option<(String, usize)> = None;
    let mut brace_depth = 0_i32;
    for (line_index, raw_line) in content.lines().enumerate() {
        let line_number = line_index + 1;
        let line = crate::indexer::strip_comments_and_strings(raw_line);
        if let Some((kind, name)) = crate::indexer::parse_rust_symbol_line(&line)
            && kind == "fn"
        {
            owner = Some((name, line_number));
        }
        for name in [
            "open",
            "socket",
            "pipe",
            "dup",
            "pidfd_open",
            "SCM_RIGHTS",
            "pidfd_getfd",
            "fork",
            "exec",
            "close",
            "drop",
            "wait",
        ] {
            if !crate::indexer::contains_call(&line, name) {
                continue;
            }
            let Some(operation) = operation_for_name(name) else {
                continue;
            };
            push_fact(
                facts,
                seen,
                path,
                line_number,
                operation,
                owner.as_ref().map(|(name, _)| name.as_str()),
                owner.as_ref().map(|(_, line)| *line),
            );
        }
        brace_depth += crate::indexer::brace_delta(&line);
        if brace_depth <= 0 {
            owner = None;
        }
    }
}

fn push_fact(
    facts: &mut Vec<IndexedResourceFact>,
    seen: &mut BTreeSet<(String, usize, String)>,
    path: &str,
    line: usize,
    operation: ResourceOperation,
    owner_symbol: Option<&str>,
    owner_line: Option<usize>,
) {
    if !seen.insert((path.to_string(), line, operation.operation.to_string())) {
        return;
    }
    let (owner_node_id, owner_symbol, owner_path, owner_line, status, reason) =
        match (owner_symbol, owner_line) {
            (Some(symbol), Some(line)) => (
                Some(format!("symbol:{path}:{line}:fn:{symbol}")),
                Some(symbol.to_string()),
                Some(path.to_string()),
                Some(line),
                "candidate".to_string(),
                Some(
                    "syntactic resource operation; lifetime and target semantics require review"
                        .to_string(),
                ),
            ),
            _ => (
                None,
                None,
                None,
                None,
                "inconclusive".to_string(),
                Some("resource operation has no proven enclosing function owner".to_string()),
            ),
        };
    facts.push(IndexedResourceFact {
        resource_fact_id: format!("resource:{path}:{line}:{}", operation.operation),
        kind: operation.kind.to_string(),
        operation: operation.operation.to_string(),
        path: path.to_string(),
        line,
        owner_node_id,
        owner_symbol,
        owner_path,
        owner_line,
        status,
        reason,
    });
}

fn operation_for_name(name: &str) -> Option<ResourceOperation> {
    Some(match name {
        "open" => ResourceOperation {
            kind: "acquire",
            operation: "open",
        },
        "socket" => ResourceOperation {
            kind: "acquire",
            operation: "socket",
        },
        "pipe" => ResourceOperation {
            kind: "acquire",
            operation: "pipe",
        },
        "dup" => ResourceOperation {
            kind: "acquire",
            operation: "dup",
        },
        "pidfd_open" => ResourceOperation {
            kind: "acquire",
            operation: "pidfd_open",
        },
        "SCM_RIGHTS" => ResourceOperation {
            kind: "handoff",
            operation: "SCM_RIGHTS",
        },
        "pidfd_getfd" => ResourceOperation {
            kind: "handoff",
            operation: "pidfd_getfd",
        },
        "fork" => ResourceOperation {
            kind: "handoff",
            operation: "fork",
        },
        "exec" => ResourceOperation {
            kind: "handoff",
            operation: "exec",
        },
        "close" => ResourceOperation {
            kind: "release",
            operation: "close",
        },
        "drop" => ResourceOperation {
            kind: "release",
            operation: "drop",
        },
        "wait" => ResourceOperation {
            kind: "release",
            operation: "wait",
        },
        "close_on_exec" => ResourceOperation {
            kind: "release",
            operation: "close_on_exec",
        },
        _ => return None,
    })
}

fn contains_token(line: &str, token: &str) -> bool {
    line.split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .any(|candidate| candidate == token)
}

fn owner_for_line(content: &str, line_number: usize) -> Option<(String, usize)> {
    let mut owner = None;
    for (index, raw_line) in content.lines().enumerate() {
        if index + 1 >= line_number {
            break;
        }
        let line = crate::indexer::strip_comments_and_strings(raw_line);
        if let Some((kind, name)) = crate::indexer::parse_rust_symbol_line(&line)
            && kind == "fn"
        {
            owner = Some((name, index + 1));
        }
    }
    owner
}

#[cfg(test)]
#[path = "resource_facts_tests.rs"]
mod tests;
