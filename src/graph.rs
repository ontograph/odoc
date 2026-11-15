//! Document graph: Document, Section and Chunk nodes plus code-span mentions.
//!
//! Edges (sorted by kind, from, to; deduplicated):
//! - `CONTAINS`: document → top-level sections, section → child sections,
//!   section → its chunks.
//! - `NEXT`: chunk → following chunk of the same document.
//! - `LINKS_TO`: chunk holding a relative Markdown link → first chunk of the
//!   target section (`x.md#anchor`, `#anchor`) or document (`x.md`).
//!   A relative `x.html` / `x.htm` link resolves to the sibling `x.md` when
//!   that document exists (rendered-book links, ADR revision 13); otherwise it
//!   is a non-Markdown link and ignored.
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
        let ordered = ordered_chunks(doc.chunks);
        for (event, range) in Parser::new_ext(doc.source, MARKDOWN_OPTIONS).into_offset_iter() {
            if !matches!(event, Event::Code(_) | Event::Start(Tag::Link { .. })) {
                continue;
            }
            let Some(from) = chunk_at(doc.chunks, range.start, ordered) else {
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

fn ordered_chunks(chunks: &[Chunk]) -> bool {
    chunks.iter().all(|c| c.span.byte_start < c.span.byte_end)
        && chunks
            .windows(2)
            .all(|pair| pair[0].span.byte_end <= pair[1].span.byte_start)
}

fn chunk_at(chunks: &[Chunk], byte: usize, ordered: bool) -> Option<&str> {
    if ordered {
        let end = chunks.partition_point(|c| c.span.byte_start <= byte);
        return end.checked_sub(1).and_then(|i| {
            let chunk = &chunks[i];
            (byte < chunk.span.byte_end).then_some(chunk.id.as_str())
        });
    }
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
        if lower.ends_with(".md") || lower.ends_with(".markdown") {
            let Some(rel) = join(doc.rel_path, &path) else {
                return Resolved::Problem(LinkProblem::OutsideRoot);
            };
            match by_path.get(rel.as_str()) {
                Some(target) => target,
                None => return Resolved::Problem(LinkProblem::MissingTarget),
            }
        } else {
            // A rendered-page link counts only when its `.md` source exists;
            // anything else stays an ignored non-Markdown link.
            let stem_len = if lower.ends_with(".html") {
                path.len() - 5
            } else if lower.ends_with(".htm") {
                path.len() - 4
            } else {
                return Resolved::Ignored;
            };
            let md = format!("{}.md", &path[..stem_len]);
            match join(doc.rel_path, &md).and_then(|rel| by_path.get(rel.as_str()).copied()) {
                Some(target) => target,
                None => return Resolved::Ignored,
            }
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

    fn old_build(docs: &[DocInput<'_>]) -> Graph {
        let by_path: HashMap<_, _> = docs.iter().map(|d| (d.rel_path, d)).collect();
        let mut edges = HashSet::new();
        let mut diagnostics = Vec::new();
        for doc in docs {
            structure_edges(doc, &mut edges);
            let lines = LineIndex::new(doc.source);
            for (event, range) in Parser::new_ext(doc.source, MARKDOWN_OPTIONS).into_offset_iter() {
                let Some(from) = doc
                    .chunks
                    .iter()
                    .find(|c| c.span.byte_start <= range.start && range.start < c.span.byte_end)
                    .map(|c| c.id.as_str())
                else {
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
                                rel_path: doc.rel_path.into(),
                                span: lines.span(range),
                                target: dest_url.into_string(),
                                problem,
                            }),
                        }
                    }
                    _ => {}
                }
            }
        }
        let mut edges: Vec<_> = edges.into_iter().collect();
        edges.sort();
        diagnostics.sort_by(|a, b| {
            (a.rel_path.as_bytes(), a.span.byte_start)
                .cmp(&(b.rel_path.as_bytes(), b.span.byte_start))
        });
        Graph { edges, diagnostics }
    }

    #[test]
    fn optimized_lookup_preserves_complete_graph_for_arbitrary_spans() {
        let source = "Preamble `intro`.\n\n# A\n\n`a` [self](#a) [html](b.html#b) [missing](gone.md) [anchor](b.md#gone) [escape](../out.md).\n\n## Nested\n\n`nested`.\n\nSetext\n======\n\n`last`.\n";
        let sections = parse_sections("a.md", source);
        let chunks = chunk_sections(source, &sections, 45);
        let other = "# B\n\nTarget.\n";
        let other_sections = parse_sections("b.md", other);
        let other_chunks = chunk_sections(other, &other_sections, CHUNK_BUDGET);
        let mut overlap = chunks.clone();
        overlap[0].span.byte_end = source.len();
        let mut unsorted = chunks.clone();
        unsorted.reverse();
        let mut empty = chunks.clone();
        empty[0].span.byte_end = empty[0].span.byte_start;
        for variant in [chunks, overlap, unsorted, empty, Vec::new()] {
            let ordered = ordered_chunks(&variant);
            for byte in 0..=source.len() + 1 {
                let old = variant
                    .iter()
                    .find(|c| c.span.byte_start <= byte && byte < c.span.byte_end)
                    .map(|c| c.id.as_str());
                assert_eq!(chunk_at(&variant, byte, ordered), old);
            }
            let docs = [
                DocInput {
                    rel_path: "a.md",
                    source,
                    sections: &sections,
                    chunks: &variant,
                },
                DocInput {
                    rel_path: "b.md",
                    source: other,
                    sections: &other_sections,
                    chunks: &other_chunks,
                },
            ];
            assert_eq!(build(&docs), old_build(&docs));
        }
    }

    #[test]
    #[ignore = "synthetic timing observation; no performance acceptance threshold"]
    fn synthetic_scan_timing() {
        let source: String = (0..1000)
            .map(|i| format!("# H{i}\n\nplain text with `code` and [link](#h0).\n\n"))
            .collect();
        let sections = parse_sections("a.md", &source);
        let chunks = chunk_sections(&source, &sections, CHUNK_BUDGET);
        let docs = [DocInput {
            rel_path: "a.md",
            source: &source,
            sections: &sections,
            chunks: &chunks,
        }];
        let start = std::time::Instant::now();
        let old = old_build(&docs);
        let old_time = start.elapsed();
        let start = std::time::Instant::now();
        let new = build(&docs);
        eprintln!("synthetic graph old={old_time:?} new={:?}", start.elapsed());
        assert_eq!(new, old);
    }

    // Census-only classification: keep ignored-branch tests aligned with resolve.
    fn link_class(from: Option<&str>, dest: &str, resolved: &Resolved<'_>) -> &'static str {
        let Some(from) = from else {
            return "outside_chunk";
        };
        match resolved {
            Resolved::Chunk(to) => return if *to == from { "same_chunk" } else { "linked" },
            Resolved::Problem(LinkProblem::MissingTarget) => return "missing_target",
            Resolved::Problem(LinkProblem::MissingAnchor) => return "missing_anchor",
            Resolved::Problem(LinkProblem::OutsideRoot) => return "outside_root",
            Resolved::Ignored => {}
        }
        if dest.contains("://") || dest.starts_with("mailto:") {
            return "external";
        }
        let path = dest.split_once('#').map_or(dest, |(path, _)| path);
        let path = path.split_once('?').map_or(path, |(path, _)| path);
        let path = percent_decode(path);
        let lower = path.to_ascii_lowercase();
        if path.is_empty() || lower.ends_with(".md") || lower.ends_with(".markdown") {
            return "ignored_markdown";
        }
        if lower.ends_with(".html") || lower.ends_with(".htm") {
            return "ignored_html";
        }
        let scheme = path.split_once(':').is_some_and(|(prefix, _)| {
            prefix
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphabetic)
                && prefix
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'-' | b'.'))
        });
        if path.starts_with("//") || scheme || (path.contains('@') && !path.contains('/')) {
            return "not_a_path";
        }
        let last = path.rsplit('/').next().unwrap_or("");
        if path.ends_with('/') || matches!(last, "." | "..") {
            "directory"
        } else if !last.contains('.') {
            "no_extension"
        } else {
            "other_extension"
        }
    }

    fn census(docs: &[DocInput<'_>]) -> std::collections::BTreeMap<&'static str, usize> {
        use std::collections::{BTreeMap, BTreeSet};
        let graph = build(docs);
        let by_path: HashMap<_, _> = docs.iter().map(|d| (d.rel_path, d)).collect();
        let mut counts: BTreeMap<_, _> = [
            "outside_chunk",
            "linked",
            "same_chunk",
            "missing_target",
            "missing_anchor",
            "outside_root",
            "external",
            "ignored_markdown",
            "ignored_html",
            "not_a_path",
            "directory",
            "no_extension",
            "other_extension",
        ]
        .into_iter()
        .map(|class| (class, 0))
        .collect();
        let mut pairs = BTreeSet::new();
        let mut destinations: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
        let mut events = 0;
        for doc in docs {
            let ordered = ordered_chunks(doc.chunks);
            for (event, range) in Parser::new_ext(doc.source, MARKDOWN_OPTIONS).into_offset_iter() {
                let Event::Start(Tag::Link { dest_url, .. }) = event else {
                    continue;
                };
                events += 1;
                let from = chunk_at(doc.chunks, range.start, ordered);
                let resolved = if from.is_some() {
                    resolve(doc, &dest_url, &by_path)
                } else {
                    Resolved::Ignored
                };
                let class = link_class(from, &dest_url, &resolved);
                *counts.get_mut(class).unwrap() += 1;
                if class == "linked"
                    && let Resolved::Chunk(to) = resolved
                {
                    pairs.insert((from.unwrap(), to));
                }
                if matches!(class, "directory" | "no_extension" | "other_extension") {
                    destinations
                        .entry(class)
                        .or_default()
                        .insert(dest_url.to_string());
                }
            }
        }
        assert_eq!(counts.values().sum::<usize>(), events);
        let problems = counts["missing_target"] + counts["missing_anchor"] + counts["outside_root"];
        assert_eq!(problems, graph.diagnostics.len());
        assert_eq!(
            pairs.len(),
            graph
                .edges
                .iter()
                .filter(|e| e.kind == EdgeKind::LinksTo)
                .count()
        );
        counts.insert("events", events);
        counts.insert("linked_pairs", pairs.len());
        counts.insert("problems", problems);
        for (class, key) in [
            ("directory", "distinct_directory"),
            ("no_extension", "distinct_no_extension"),
            ("other_extension", "distinct_other_extension"),
        ] {
            counts.insert(key, destinations.get(class).map_or(0, BTreeSet::len));
        }
        counts
    }

    #[test]
    fn link_census_classes_match_hand_counted_events() {
        let sources = [
            (
                "a.md",
                "# A\n\n[self](#a) [linked](b.md) [again](b.md) [missing](missing.md) [anchor](b.md#absent) [escape](../out.md) [web](https://host) [mail](mailto:x@y) [empty](empty.md) [html](absent.html) [email](x@y) [tel](tel:123) [upper](MAILTO:x@y) [host](//host/path) [directory](dir/) [parent](..) [encoded](dir%2F?q#f) [plain](plain) [asset](x.png)\n",
            ),
            ("b.md", "# B\n\nTarget.\n"),
            ("empty.md", ""),
            ("uncovered.md", "[outside](b.md)\n"),
        ];
        let parsed: Vec<_> = sources
            .iter()
            .map(|(path, text)| {
                let sections = parse_sections(path, text);
                let chunks = if *path == "uncovered.md" {
                    Vec::new()
                } else {
                    chunk_sections(text, &sections, CHUNK_BUDGET)
                };
                (sections, chunks)
            })
            .collect();
        let docs: Vec<_> = sources
            .iter()
            .zip(&parsed)
            .map(|((path, source), (sections, chunks))| DocInput {
                rel_path: path,
                source,
                sections,
                chunks,
            })
            .collect();
        // 20 links: outside=1, linked=2 (one pair), self=1; three problems;
        // external=2, empty Markdown=1, HTML=1, non-path=4, directory=3,
        // extensionless=1, other=1. Distinct destinations: 3, 1, 1.
        assert_eq!(
            census(&docs),
            std::collections::BTreeMap::from([
                ("outside_chunk", 1),
                ("linked", 2),
                ("same_chunk", 1),
                ("missing_target", 1),
                ("missing_anchor", 1),
                ("outside_root", 1),
                ("external", 2),
                ("ignored_markdown", 1),
                ("ignored_html", 1),
                ("not_a_path", 4),
                ("directory", 3),
                ("no_extension", 1),
                ("other_extension", 1),
                ("events", 20),
                ("linked_pairs", 1),
                ("problems", 3),
                ("distinct_directory", 3),
                ("distinct_no_extension", 1),
                ("distinct_other_extension", 1),
            ])
        );
    }

    #[test]
    #[ignore = "read-only census of an explicitly selected corpus"]
    fn link_outcome_census() {
        let root = std::env::var_os("ONTODOC_LINK_CENSUS_ROOT").expect("census root required");
        let corpus = walk(Path::new(&root), &WalkConfig::default()).unwrap();
        let parsed: Vec<_> = corpus
            .files
            .iter()
            .map(|file| {
                let sections = parse_sections(&file.rel_path, &file.text);
                let chunks = chunk_sections(&file.text, &sections, CHUNK_BUDGET);
                (sections, chunks)
            })
            .collect();
        let docs: Vec<_> = corpus
            .files
            .iter()
            .zip(&parsed)
            .map(|(file, (sections, chunks))| DocInput {
                rel_path: &file.rel_path,
                source: &file.text,
                sections,
                chunks,
            })
            .collect();
        for (class, count) in census(&docs) {
            eprintln!("{class}={count}");
        }
    }

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

    fn inline_graph(sources: &[(&str, &str)]) -> Graph {
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
        build(&docs)
    }

    fn link_targets(graph: &Graph) -> Vec<(&str, &str)> {
        graph
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::LinksTo)
            .map(|e| (e.from.as_str(), e.to.as_str()))
            .collect()
    }

    #[test]
    fn html_links_resolve_to_existing_markdown_sources_only() {
        // Reference-style links as in rendered books: `.html` maps to the
        // sibling `.md` only when it exists; otherwise ignored, never a
        // diagnostic. Non-page extensions stay ignored.
        let graph = inline_graph(&[
            (
                "src/ch01.md",
                "# One\n\nSee [two] and [deep] and [gone] and [pic] and [web].\n\n[two]: ch02.html\n[deep]: ch02.HTM#second-part\n[gone]: ch99.html#x\n[pic]: diagram.png\n[web]: https://example.com/ch02.html\n",
            ),
            ("src/ch02.md", "# Two\n\nbody\n\n## Second Part\n\nmore\n"),
        ]);
        assert_eq!(
            link_targets(&graph),
            vec![
                ("src/ch01.md#one~0", "src/ch02.md#two/second-part~0"),
                ("src/ch01.md#one~0", "src/ch02.md#two~0"),
            ]
        );
        assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
        // A resolved page with a missing anchor is a real broken link.
        let broken = inline_graph(&[("a.md", "# A\n\n[b](b.html#nope)\n"), ("b.md", "# B\n")]);
        assert_eq!(broken.diagnostics.len(), 1);
        assert_eq!(broken.diagnostics[0].problem, LinkProblem::MissingAnchor);
    }

    #[test]
    fn canonical_empty_components_do_not_become_markdown_anchors() {
        let graph = inline_graph(&[
            (
                "a.md",
                "# Links\n\n[first](b.md#) [normal](b.md#normal) [duplicate](b.md#normal-1) [child](b.md#child) [empty](c.md#) [not an anchor](b.md#@empty)\n",
            ),
            (
                "b.md",
                "intro\n\n# Normal\n\nbody\n\n# Normal\n\nsecond\n\n# Parent\n\np\n\n## !!!\n\npunct\n\n### Child\n\nx\n",
            ),
            ("c.md", "# !!!\n\nbody\n"),
        ]);
        assert_eq!(
            link_targets(&graph),
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
