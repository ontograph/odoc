use pretty_assertions::assert_eq;

use super::*;

#[test]
fn impl_methods_and_nested_modules_are_extracted() {
    let parsed = parse_rust_file(
        "struct Type;\nimpl Type {\n    fn method(&self) {}\n}\nmod inner {\n    pub fn nested() {}\n}\n",
    )
    .expect("parse");

    let symbols = parsed
        .symbols
        .iter()
        .map(|symbol| (symbol.kind.clone(), symbol.name.clone(), symbol.line))
        .collect::<Vec<_>>();
    // The line scanner attributes `impl` methods to no owner and cannot see
    // items nested inside a module body.
    assert_eq!(
        symbols,
        vec![
            ("struct".to_string(), "Type".to_string(), 1),
            ("fn".to_string(), "method".to_string(), 3),
            ("mod".to_string(), "inner".to_string(), 5),
            ("fn".to_string(), "nested".to_string(), 6),
        ]
    );
}

#[test]
fn symbols_record_the_last_line_of_the_whole_item() {
    let parsed = parse_rust_file(
        "fn a() {\n    x();\n}\nstruct T;\nimpl T {\n    fn m(&self) {\n    }\n}\n",
    )
    .expect("parse");

    let spans = parsed
        .symbols
        .iter()
        .map(|symbol| (symbol.name.clone(), symbol.line, symbol.end_line))
        .collect::<Vec<_>>();
    assert_eq!(
        spans,
        vec![
            ("a".to_string(), 1, 3),
            ("T".to_string(), 4, 4),
            ("m".to_string(), 6, 7),
        ]
    );
}

#[test]
fn call_forms_are_distinguished_by_syntax() {
    let parsed =
        parse_rust_file("fn caller() {\n    bare();\n    Type::path();\n    client.method();\n}\n")
            .expect("parse");

    let calls = parsed
        .calls
        .iter()
        .map(|call| (call.name.clone(), call.form, call.qualifier.clone()))
        .collect::<Vec<_>>();
    assert_eq!(
        calls,
        vec![
            ("bare".to_string(), ParsedCallForm::Bare, None),
            (
                "path".to_string(),
                ParsedCallForm::Path,
                Some("Type".to_string())
            ),
            ("method".to_string(), ParsedCallForm::Method, None),
        ]
    );
}

#[test]
fn calls_record_their_enclosing_function() {
    let parsed = parse_rust_file("fn outer() {\n    inner();\n}\nfn inner() {}\n").expect("parse");

    let call = parsed.calls.first().expect("one call");
    assert_eq!(call.name, "inner");
    assert_eq!(call.enclosing_fn.as_deref(), Some("outer"));
    assert_eq!(call.enclosing_fn_line, 1);
}

#[test]
fn nested_expression_positions_are_visited() {
    let parsed = parse_rust_file(
        "fn caller() {\n    if guard() {\n        for _ in iter() {\n            body();\n        }\n    }\n}\n",
    )
    .expect("parse");

    let mut names = parsed
        .calls
        .iter()
        .map(|call| call.name.clone())
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(
        names,
        vec!["body".to_string(), "guard".to_string(), "iter".to_string()]
    );
}

#[test]
fn qualified_paths_preserve_all_segments_before_the_callee() {
    let parsed = parse_rust_file("fn caller() { crate::module::run(); }\n").expect("parse");
    let call = parsed.calls.first().expect("one call");
    assert_eq!(call.name, "run");
    assert_eq!(call.form, ParsedCallForm::Path);
    assert_eq!(call.qualifier.as_deref(), Some("crate::module"));
}

#[test]
fn use_imports_are_recorded() {
    let parsed = parse_rust_file("use crate::a::Thing;\nuse crate::b::{One, Two as Renamed};\n")
        .expect("parse");

    assert_eq!(
        parsed.imports,
        vec![
            ParsedImport {
                local_name: "Thing".to_string(),
                source_path: vec!["crate".to_string(), "a".to_string(), "Thing".to_string()],
            },
            ParsedImport {
                local_name: "One".to_string(),
                source_path: vec!["crate".to_string(), "b".to_string(), "One".to_string()],
            },
            ParsedImport {
                local_name: "Renamed".to_string(),
                source_path: vec!["crate".to_string(), "b".to_string(), "Two".to_string()],
            },
        ]
    );
}

#[test]
fn unparsable_source_returns_none_so_the_caller_can_fall_back() {
    // A file `syn` cannot parse must not drop out of the index; the caller
    // falls back to the line scanner on `None`.
    assert_eq!(parse_rust_file("fn broken( {"), None);
}
