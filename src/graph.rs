//! Document graph: Document, Section and Chunk nodes plus code-span mentions.
//!
//! Edges (sorted by kind, from, to; deduplicated):
//! - `CONTAINS`: document → top-level sections, section → child sections,
//!   section → its chunks.
//! - `NEXT`: chunk → following chunk of the same document.
//! - `LINKS_TO`: chunk holding a relative Markdown link → first chunk of the
//!   target section (`x.md#anchor`, `#anchor`) or document (`x.md`).
//!   Self-loops are dropped.
//! - `MENTIONS`: chunk → `code:<text>` for each inline code span; unresolved.
//!
//! URLs with a scheme and links to non-Markdown files are not corpus links and
//! are ignored. Missing files, missing anchors and paths that leave the root
//! are [`LinkDiagnostic`]s, never edges.

use std::collections::{HashMap, HashSet};

use pulldown_cmark::{Event, Parser, Tag};

use crate::chunk::Chunk;
use crate::parse::{LineIndex, MARKDOWN_OPTIONS, Section, Span};

/// One parsed document as graph input.
pub struct DocInput<'a> {
    pub rel_path: &'a str,
    pub source: &'a str,
    pub sections: &'a [Section],
    pub chunks: &'a [Chunk],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeKind {
    Contains,
    Next,
    LinksTo,
    Mentions,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Edge {
    pub kind: EdgeKind,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkProblem {
    MissingTarget,
    MissingAnchor,
    OutsideRoot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkDiagnostic {
    pub rel_path: String,
    pub span: Span,
    pub target: String,
    pub problem: LinkProblem,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Graph {
    pub edges: Vec<Edge>,
    /// Sorted by path, then byte offset.
    pub diagnostics: Vec<LinkDiagnostic>,
}

/// Builds the graph for a whole corpus.
pub fn build(docs: &[DocInput<'_>]) -> Graph {
    let by_path: HashMap<&str, &DocInput<'_>> = docs.iter().map(|d| (d.rel_path, d)).collect();
    let mut edges = HashSet::new();
    let mut diagnostics = Vec::new();

    for doc in docs {
        structure_edges(doc, &mut edges);
        let lines = LineIndex::new(doc.source);
        for (event, range) in Parser::new_ext(doc.source, MARKDOWN_OPTIONS).into_offset_iter() {
            let Some(from) = chunk_at(doc.chunks, range.start) else {
                continue;
            };
            match event {
                Event::Code(text) => {
                    edges.insert(edge(EdgeKind::Mentions, from, &format!("code:{text}")));
                }
                Event::Start(Tag::Link { dest_url, .. }) => {
                    match resolve(doc, &dest_url, &by_path) {
                        Resolved::Chunk(to) if to != from => {
                            edges.insert(edge(EdgeKind::LinksTo, from, to));
                        }
                        Resolved::Chunk(_) | Resolved::Ignored => {}
                        Resolved::Problem(problem) => diagnostics.push(LinkDiagnostic {
                            rel_path: doc.rel_path.to_string(),
                            span: lines.span(range),
                            target: dest_url.to_string(),
                            problem,
                        }),
                    }
                }
                _ => {}
            }
        }
    }

    let mut edges: Vec<Edge> = edges.into_iter().collect();
    edges.sort();
    diagnostics.sort_by(|a, b| {
        (a.rel_path.as_bytes(), a.span.byte_start).cmp(&(b.rel_path.as_bytes(), b.span.byte_start))
    });
    Graph { edges, diagnostics }
}

fn edge(kind: EdgeKind, from: &str, to: &str) -> Edge {
    Edge {
        kind,
        from: from.to_string(),
        to: to.to_string(),
    }
}

fn structure_edges(doc: &DocInput<'_>, edges: &mut HashSet<Edge>) {
    let mut stack: Vec<&Section> = Vec::new();
    for section in doc.sections {
        while stack.last().is_some_and(|s| s.level >= section.level) {
            stack.pop();
        }
        let parent = stack.last().map_or(doc.rel_path, |s| s.id.as_str());
        edges.insert(edge(EdgeKind::Contains, parent, &section.id));
        if section.level > 0 {
            stack.push(section);
        }
    }
    for chunk in doc.chunks {
        edges.insert(edge(EdgeKind::Contains, &chunk.section_id, &chunk.id));
    }
    for pair in doc.chunks.windows(2) {
        edges.insert(edge(EdgeKind::Next, &pair[0].id, &pair[1].id));
    }
}

fn chunk_at(chunks: &[Chunk], byte: usize) -> Option<&str> {
    chunks
        .iter()
        .find(|c| c.span.byte_start <= byte && byte < c.span.byte_end)
        .map(|c| c.id.as_str())
}

enum Resolved<'a> {
    Chunk(&'a str),
    Ignored,
    Problem(LinkProblem),
}

fn resolve<'a>(
    doc: &'a DocInput<'_>,
    dest: &str,
    by_path: &HashMap<&str, &'a DocInput<'_>>,
) -> Resolved<'a> {
    if dest.contains("://") || dest.starts_with("mailto:") {
        return Resolved::Ignored;
    }
    let (path, fragment) = dest.split_once('#').unwrap_or((dest, ""));
    let path = path.split_once('?').map_or(path, |(p, _)| p);
    let target: &DocInput<'_> = if path.is_empty() {
        doc
    } else {
        let path = percent_decode(path);
        let lower = path.to_ascii_lowercase();
        if !(lower.ends_with(".md") || lower.ends_with(".markdown")) {
            return Resolved::Ignored;
        }
        let Some(rel) = join(doc.rel_path, &path) else {
            return Resolved::Problem(LinkProblem::OutsideRoot);
        };
        match by_path.get(rel.as_str()) {
            Some(target) => target,
            None => return Resolved::Problem(LinkProblem::MissingTarget),
        }
    };
    let section_id = if fragment.is_empty() {
        None
    } else {
        let anchor = percent_decode(fragment).to_lowercase();
        match target.sections.iter().find(|s| s.anchor == anchor) {
            Some(s) => Some(s.id.as_str()),
            None => return Resolved::Problem(LinkProblem::MissingAnchor),
        }
    };
    target
        .chunks
        .iter()
        .find(|c| section_id.is_none_or(|id| c.section_id == id))
        .map_or(Resolved::Ignored, |c| Resolved::Chunk(c.id.as_str()))
}

/// Resolves `link` against the directory of `from`; `None` if it leaves the root.
fn join(from: &str, link: &str) -> Option<String> {
    let mut parts: Vec<&str> = if link.starts_with('/') {
        Vec::new()
    } else {
        from.split('/').collect()
    };
    if !link.starts_with('/') {
        parts.pop();
    }
    for part in link.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            p => parts.push(p),
        }
    }
    Some(parts.join("/"))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(b)) => {
                out.push(b);
                i += 3;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{CHUNK_BUDGET, chunk_sections};
    use crate::corpus::{WalkConfig, walk};
    use crate::parse::parse_sections;
    use pretty_assertions::assert_eq;
    use std::path::Path;

    fn fixture_graph() -> Graph {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graph");
        let corpus = walk(&root, &WalkConfig::default()).unwrap();
        let parsed: Vec<(Vec<Section>, Vec<Chunk>)> = corpus
            .files
            .iter()
            .map(|f| {
                let sections = parse_sections(&f.rel_path, &f.text);
                let chunks = chunk_sections(&f.text, &sections, CHUNK_BUDGET);
                (sections, chunks)
            })
            .collect();
        let docs: Vec<DocInput<'_>> = corpus
            .files
            .iter()
            .zip(&parsed)
            .map(|(f, (sections, chunks))| DocInput {
                rel_path: &f.rel_path,
                source: &f.text,
                sections,
                chunks,
            })
            .collect();
        build(&docs)
    }

    #[test]
    fn fixture_edges_match_golden_list() {
        let graph = fixture_graph();
        let got: Vec<String> = graph
            .edges
            .iter()
            .map(|e| format!("{:?} {} -> {}", e.kind, e.from, e.to))
            .collect();
        let golden = include_str!("../tests/fixtures/graph/edges.golden");
        assert_eq!(got, golden.lines().collect::<Vec<_>>());
    }

    #[test]
    fn broken_links_are_diagnostics_with_spans() {
        let graph = fixture_graph();
        let got: Vec<(&str, usize, &str, &LinkProblem)> = graph
            .diagnostics
            .iter()
            .map(|d| {
                (
                    d.rel_path.as_str(),
                    d.span.line_start,
                    d.target.as_str(),
                    &d.problem,
                )
            })
            .collect();
        // Lines from `cat -n tests/fixtures/graph/a.md`.
        assert_eq!(
            got,
            vec![
                ("a.md", 8, "#nope", &LinkProblem::MissingAnchor),
                ("a.md", 8, "missing.md", &LinkProblem::MissingTarget),
                ("a.md", 9, "../outside.md", &LinkProblem::OutsideRoot),
            ]
        );
    }

    #[test]
    fn graph_is_deterministic() {
        assert_eq!(fixture_graph(), fixture_graph());
    }

    #[test]
    fn canonical_empty_components_do_not_become_markdown_anchors() {
        let sources = [
            (
                "a.md",
                "# Links\n\n[first](b.md#) [normal](b.md#normal) [duplicate](b.md#normal-1) [child](b.md#child) [empty](c.md#) [not an anchor](b.md#@empty)\n",
            ),
            (
                "b.md",
                "intro\n\n# Normal\n\nbody\n\n# Normal\n\nsecond\n\n# Parent\n\np\n\n## !!!\n\npunct\n\n### Child\n\nx\n",
            ),
            ("c.md", "# !!!\n\nbody\n"),
        ];
        let parsed: Vec<_> = sources
            .iter()
            .map(|(path, text)| {
                let sections = parse_sections(path, text);
                let chunks = chunk_sections(text, &sections, CHUNK_BUDGET);
                (sections, chunks)
            })
            .collect();
        let docs: Vec<_> = sources
            .iter()
            .zip(&parsed)
            .map(|((path, text), (sections, chunks))| DocInput {
                rel_path: path,
                source: text,
                sections,
                chunks,
            })
            .collect();
        let graph = build(&docs);
        let targets: Vec<_> = graph
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::LinksTo)
            .map(|e| (e.from.as_str(), e.to.as_str()))
            .collect();
        assert_eq!(
            targets,
            vec![
                ("a.md#links~0", "b.md#normal-1~0"),
                ("a.md#links~0", "b.md#normal~0"),
                ("a.md#links~0", "b.md#parent/@empty/child~0"),
                ("a.md#links~0", "b.md#~0"),
                ("a.md#links~0", "c.md#@empty~0"),
            ]
        );
        assert!(graph.edges.contains(&edge(
            EdgeKind::Contains,
            "b.md#parent",
            "b.md#parent/@empty"
        )));
        assert_eq!(graph.diagnostics.len(), 1);
        assert_eq!(graph.diagnostics[0].target, "b.md#@empty");
        assert_eq!(graph.diagnostics[0].problem, LinkProblem::MissingAnchor);
    }

    #[test]
    fn join_and_decode() {
        assert_eq!(join("a/b.md", "../c.md").as_deref(), Some("c.md"));
        assert_eq!(join("a/b.md", "./d/e.md").as_deref(), Some("a/d/e.md"));
        assert_eq!(join("a/b.md", "/x.md").as_deref(), Some("x.md"));
        assert_eq!(join("b.md", "../c.md"), None);
        assert_eq!(percent_decode("My%20Notes.md"), "My Notes.md");
        assert_eq!(percent_decode("100%"), "100%");
    }
}
