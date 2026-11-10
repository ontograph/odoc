use pretty_assertions::assert_eq;

use super::*;

#[test]
fn extracts_resource_operations_without_comments_or_strings() {
    let content = r#"
fn worker() {
    let fd = open();
    // socket(); SCM_RIGHTS
    let text = "close(); SCM_RIGHTS";
    close(fd);
}
"#;

    let parsed = crate::rust_parser::parse_rust_file(content).expect("parse");
    let facts = collect_resource_facts("src/lib.rs", content, Some(&parsed));

    assert_eq!(
        facts
            .iter()
            .map(|fact| (fact.kind.as_str(), fact.operation.as_str(), fact.line))
            .collect::<Vec<_>>(),
        vec![("acquire", "open", 3), ("release", "close", 6),]
    );
}

#[test]
fn records_boundary_markers_and_inconclusive_top_level_ownership() {
    let content = "const RIGHTS: &str = \"SCM_RIGHTS\";\nSCM_RIGHTS;\n";
    let facts = collect_resource_facts("src/lib.rs", content, None);

    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].operation, "SCM_RIGHTS");
    assert_eq!(facts[0].status, "inconclusive");
    assert_eq!(facts[0].owner_node_id, None);
}

#[test]
fn keeps_resource_fact_order_deterministic_and_deduplicated() {
    let content = "fn worker() { fork(); fork(); }\n";
    let parsed = crate::rust_parser::parse_rust_file(content).expect("parse");
    let facts = collect_resource_facts("src/lib.rs", content, Some(&parsed));

    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].resource_fact_id, "resource:src/lib.rs:1:fork");
}
