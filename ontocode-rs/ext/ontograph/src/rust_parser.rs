//! `syn`-backed Rust parsing for the source indexer.
//!
//! Owns syntax-tree extraction of symbol definitions and call sites, per
//! `ADR_ONTOGRAPH_PHASE6_RUST_PARSER.md`. `indexer.rs` keeps the walk, the
//! artifact shapes, and the line-scanner fallback for files this module cannot
//! parse.
//!
//! What name-based resolution can and cannot prove bounds everything here.
//! `syn` gives syntax, not types: for `client.run()` it can see the method name
//! but not the type of `client`. A method call is therefore reported as a
//! candidate by name only, and the caller records it as ambiguous unless
//! exactly one definition in scope carries that name. Selecting arbitrarily
//! among candidates would produce a graph that looks complete and is wrong,
//! which is worse for `impact.*` and `audit.*` than an honest gap.

use syn::Expr;
use syn::ImplItem;
use syn::Item;
use syn::spanned::Spanned;

/// One definition found in a parsed file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedSymbol {
    pub name: String,
    pub kind: String,
    pub line: usize,
    /// Last source line of the whole item.
    pub end_line: usize,
    /// Type or module owner for methods and nested definitions when known.
    pub owner: Option<String>,
}

/// How a call site was written, which bounds what resolution may claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParsedCallForm {
    /// `run()` — an unqualified call.
    Bare,
    /// `Type::run()` — path-qualified; resolvable when the path root is known.
    Path,
    /// `client.run()` — receiver-qualified; the receiver type is unknown here.
    Method,
}

/// One call site found in a parsed file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedCall {
    pub name: String,
    pub form: ParsedCallForm,
    /// The path prefix for a qualified call, such as Type in Type::run().
    pub qualifier: Option<String>,
    pub line: usize,
    /// Name of the enclosing function definition, when the call is inside one.
    pub enclosing_fn: Option<String>,
    /// Line of the enclosing function definition.
    pub enclosing_fn_line: usize,
}

/// One explicit import brought into scope by a `use` item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedImport {
    /// Name used by the importing file.
    pub local_name: String,
    /// Full source path, excluding a leading `crate`, `self`, or `super`.
    pub source_path: Vec<String>,
}

/// Everything the indexer needs from one successfully parsed file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ParsedFile {
    pub symbols: Vec<ParsedSymbol>,
    pub calls: Vec<ParsedCall>,
    pub imports: Vec<ParsedImport>,
}

/// Parses one Rust file into symbols and call sites.
///
/// Returns `None` when `syn` cannot parse the file, which is the signal for the
/// caller to fall back to the line scanner rather than dropping the file from
/// the index.
pub(crate) fn parse_rust_file(content: &str) -> Option<ParsedFile> {
    let file = syn::parse_file(content).ok()?;
    let mut parsed = ParsedFile::default();
    for item in &file.items {
        collect_item(item, &mut parsed);
    }
    parsed.symbols.sort_by(|left, right| {
        left.line
            .cmp(&right.line)
            .then_with(|| left.name.cmp(&right.name))
    });
    parsed.calls.sort_by(|left, right| {
        left.line
            .cmp(&right.line)
            .then_with(|| left.name.cmp(&right.name))
    });
    Some(parsed)
}

fn collect_item(item: &Item, parsed: &mut ParsedFile) {
    match item {
        Item::Fn(item_fn) => {
            let line = item_fn.sig.ident.span().start().line;
            parsed.symbols.push(ParsedSymbol {
                name: item_fn.sig.ident.to_string(),
                kind: "fn".to_string(),
                line,
                end_line: item_fn.span().end().line,
                owner: None,
            });
            collect_calls_in_block(
                &item_fn.block,
                Some(item_fn.sig.ident.to_string()),
                line,
                parsed,
            );
            collect_nested_items(&item_fn.block, parsed);
        }
        Item::Struct(item_struct) => parsed.symbols.push(ParsedSymbol {
            name: item_struct.ident.to_string(),
            kind: "struct".to_string(),
            line: item_struct.ident.span().start().line,
            end_line: item_struct.span().end().line,
            owner: None,
        }),
        Item::Enum(item_enum) => parsed.symbols.push(ParsedSymbol {
            name: item_enum.ident.to_string(),
            kind: "enum".to_string(),
            line: item_enum.ident.span().start().line,
            end_line: item_enum.span().end().line,
            owner: None,
        }),
        Item::Trait(item_trait) => parsed.symbols.push(ParsedSymbol {
            name: item_trait.ident.to_string(),
            kind: "trait".to_string(),
            line: item_trait.ident.span().start().line,
            end_line: item_trait.span().end().line,
            owner: None,
        }),
        Item::Type(item_type) => parsed.symbols.push(ParsedSymbol {
            name: item_type.ident.to_string(),
            kind: "type".to_string(),
            line: item_type.ident.span().start().line,
            end_line: item_type.span().end().line,
            owner: None,
        }),
        Item::Const(item_const) => parsed.symbols.push(ParsedSymbol {
            name: item_const.ident.to_string(),
            kind: "const".to_string(),
            line: item_const.ident.span().start().line,
            end_line: item_const.span().end().line,
            owner: None,
        }),
        Item::Mod(item_mod) => {
            parsed.symbols.push(ParsedSymbol {
                name: item_mod.ident.to_string(),
                kind: "mod".to_string(),
                line: item_mod.ident.span().start().line,
                end_line: item_mod.span().end().line,
                owner: None,
            });
            // Nested modules are structure the line scanner cannot see.
            if let Some((_brace, items)) = item_mod.content.as_ref() {
                for nested in items {
                    collect_item(nested, parsed);
                }
            }
        }
        Item::Impl(item_impl) => {
            // `impl` bodies are the main source of method definitions the line
            // scanner attributes to no owner.
            for impl_item in &item_impl.items {
                if let ImplItem::Fn(method) = impl_item {
                    let line = method.sig.ident.span().start().line;
                    parsed.symbols.push(ParsedSymbol {
                        name: method.sig.ident.to_string(),
                        kind: "fn".to_string(),
                        line,
                        end_line: method.span().end().line,
                        owner: impl_type_name(item_impl),
                    });
                    collect_calls_in_block(
                        &method.block,
                        Some(method.sig.ident.to_string()),
                        line,
                        parsed,
                    );
                }
            }
        }
        Item::Use(item_use) => collect_use_names(&item_use.tree, parsed),
        _ => {}
    }
}

fn impl_type_name(item_impl: &syn::ItemImpl) -> Option<String> {
    let syn::Type::Path(type_path) = item_impl.self_ty.as_ref() else {
        return None;
    };
    type_path
        .path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

fn collect_use_names(tree: &syn::UseTree, parsed: &mut ParsedFile) {
    collect_use_tree(tree, &mut Vec::new(), parsed);
}

fn collect_use_tree(tree: &syn::UseTree, prefix: &mut Vec<String>, parsed: &mut ParsedFile) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_use_tree(&path.tree, prefix, parsed);
            prefix.pop();
        }
        syn::UseTree::Name(name) => {
            let mut source_path = prefix.clone();
            source_path.push(name.ident.to_string());
            parsed.imports.push(ParsedImport {
                local_name: name.ident.to_string(),
                source_path,
            });
        }
        syn::UseTree::Rename(rename) => {
            let mut source_path = prefix.clone();
            source_path.push(rename.ident.to_string());
            parsed.imports.push(ParsedImport {
                local_name: rename.rename.to_string(),
                source_path,
            });
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_use_tree(item, prefix, parsed);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

fn collect_calls_in_block(
    block: &syn::Block,
    enclosing_fn: Option<String>,
    enclosing_fn_line: usize,
    parsed: &mut ParsedFile,
) {
    let mut calls = Vec::new();
    visit_block(block, &mut calls);
    for (name, form, qualifier, line) in calls {
        parsed.calls.push(ParsedCall {
            name,
            form,
            qualifier,
            line,
            enclosing_fn: enclosing_fn.clone(),
            enclosing_fn_line,
        });
    }
}

fn collect_nested_items(block: &syn::Block, parsed: &mut ParsedFile) {
    for statement in &block.stmts {
        let syn::Stmt::Item(item) = statement else {
            continue;
        };
        collect_item(item, parsed);
    }
}

fn visit_block(
    block: &syn::Block,
    calls: &mut Vec<(String, ParsedCallForm, Option<String>, usize)>,
) {
    for statement in &block.stmts {
        match statement {
            syn::Stmt::Local(local) => {
                if let Some(init) = local.init.as_ref() {
                    visit_expr(&init.expr, calls);
                }
            }
            syn::Stmt::Expr(expr, _semi) => visit_expr(expr, calls),
            syn::Stmt::Item(item) => {
                // Nested functions are collected separately with their own
                // enclosing-function metadata by collect_nested_items.
                let _ = item;
            }
            syn::Stmt::Macro(_) => {}
        }
    }
}

fn visit_expr(expr: &Expr, calls: &mut Vec<(String, ParsedCallForm, Option<String>, usize)>) {
    match expr {
        Expr::Call(call) => {
            if let Some((name, form, qualifier)) = callee_name(&call.func) {
                calls.push((name, form, qualifier, call.func.span().start().line));
            }
            visit_expr(&call.func, calls);
            for argument in &call.args {
                visit_expr(argument, calls);
            }
        }
        Expr::MethodCall(method) => {
            calls.push((
                method.method.to_string(),
                ParsedCallForm::Method,
                None,
                method.method.span().start().line,
            ));
            visit_expr(&method.receiver, calls);
            for argument in &method.args {
                visit_expr(argument, calls);
            }
        }
        Expr::Binary(binary) => {
            visit_expr(&binary.left, calls);
            visit_expr(&binary.right, calls);
        }
        Expr::Unary(unary) => visit_expr(&unary.expr, calls),
        Expr::Paren(paren) => visit_expr(&paren.expr, calls),
        Expr::Group(group) => visit_expr(&group.expr, calls),
        Expr::Reference(reference) => visit_expr(&reference.expr, calls),
        Expr::Try(expression) => visit_expr(&expression.expr, calls),
        Expr::Await(expression) => visit_expr(&expression.base, calls),
        Expr::Field(field) => visit_expr(&field.base, calls),
        Expr::Cast(cast) => visit_expr(&cast.expr, calls),
        Expr::Let(expression) => visit_expr(&expression.expr, calls),
        Expr::Assign(assign) => {
            visit_expr(&assign.left, calls);
            visit_expr(&assign.right, calls);
        }
        Expr::Index(index) => {
            visit_expr(&index.expr, calls);
            visit_expr(&index.index, calls);
        }
        Expr::Return(expression) => {
            if let Some(inner) = expression.expr.as_ref() {
                visit_expr(inner, calls);
            }
        }
        Expr::If(expression) => {
            visit_expr(&expression.cond, calls);
            visit_block(&expression.then_branch, calls);
            if let Some((_token, branch)) = expression.else_branch.as_ref() {
                visit_expr(branch, calls);
            }
        }
        Expr::Match(expression) => {
            visit_expr(&expression.expr, calls);
            for arm in &expression.arms {
                visit_expr(&arm.body, calls);
            }
        }
        Expr::Block(expression) => visit_block(&expression.block, calls),
        Expr::ForLoop(expression) => {
            visit_expr(&expression.expr, calls);
            visit_block(&expression.body, calls);
        }
        Expr::While(expression) => {
            visit_expr(&expression.cond, calls);
            visit_block(&expression.body, calls);
        }
        Expr::Loop(expression) => visit_block(&expression.body, calls),
        Expr::Unsafe(expression) => visit_block(&expression.block, calls),
        Expr::Async(expression) => visit_block(&expression.block, calls),
        Expr::Closure(closure) => visit_expr(&closure.body, calls),
        Expr::Tuple(tuple) => {
            for element in &tuple.elems {
                visit_expr(element, calls);
            }
        }
        Expr::Array(array) => {
            for element in &array.elems {
                visit_expr(element, calls);
            }
        }
        Expr::Struct(structure) => {
            for field in &structure.fields {
                visit_expr(&field.expr, calls);
            }
        }
        _ => {}
    }
}

/// Returns the called name and how it was written.
///
/// A single-segment path is a bare call. A multi-segment path such as
/// `Type::run` is path-qualified: the final segment names the callee and the
/// preceding segments are the qualifier.
fn callee_name(func: &Expr) -> Option<(String, ParsedCallForm, Option<String>)> {
    let Expr::Path(path) = func else {
        return None;
    };
    let last = path.path.segments.last()?;
    let form = if path.path.segments.len() > 1 {
        ParsedCallForm::Path
    } else {
        ParsedCallForm::Bare
    };
    let qualifier = (path.path.segments.len() > 1).then(|| {
        path.path
            .segments
            .iter()
            .take(path.path.segments.len() - 1)
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::")
    });
    Some((last.ident.to_string(), form, qualifier))
}

#[cfg(test)]
#[path = "rust_parser_tests.rs"]
mod rust_parser_tests;
