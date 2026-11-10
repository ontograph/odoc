//! Markdown sections: the evaluation unit.
//!
//! A section runs from its heading to the next heading of equal or higher
//! level, so a parent's extent contains its children. Its *own blocks* stop at
//! the first heading of any level; chunks are packed from own blocks only, so
//! no text is indexed twice. Content before the first heading is a level-0
//! preamble section with an empty slug path. YAML front matter is excluded.
//!
//! Section id = `<rel_path>#<slug path>`, where the slug path joins the GitHub
//! anchors of the heading path with `/`, encoding an empty allocated anchor
//! component as `@empty` (ID-only; the Markdown anchor stays unchanged).
//! It does not depend on the chunk
//! budget or on line numbers.

use std::collections::HashMap;
use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::corpus::sha256_hex;

/// A byte span with its 1-based inclusive line span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub id: String,
    pub rel_path: String,
    /// Heading texts from the outermost ancestor to this section.
    pub heading_path: Vec<String>,
    /// 1–6 for headings, 0 for the preamble.
    pub level: u8,
    /// GitHub anchor of this section's heading; empty for the preamble.
    pub anchor: String,
    /// Full extent, including child sections.
    pub span: Span,
    /// SHA-256 of `source[span.byte_start..span.byte_end]`.
    pub sha256: String,
    /// Own top-level blocks (heading first), trailing whitespace trimmed.
    pub blocks: Vec<Range<usize>>,
}

/// Maps byte offsets to 1-based line numbers.
pub struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(source.match_indices('\n').map(|(i, _)| i + 1));
        Self { starts }
    }

    /// 1-based line containing `byte`.
    pub fn line(&self, byte: usize) -> usize {
        self.starts.partition_point(|&s| s <= byte)
    }

    /// Span of `range`; the end line is the line of the last byte.
    pub fn span(&self, range: Range<usize>) -> Span {
        Span {
            byte_start: range.start,
            byte_end: range.end,
            line_start: self.line(range.start),
            line_end: self.line(range.end.saturating_sub(1).max(range.start)),
        }
    }
}

struct Block {
    range: Range<usize>,
    heading: Option<(u8, String)>,
}

/// Splits `source` into sections in document order.
pub fn parse_sections(rel_path: &str, source: &str) -> Vec<Section> {
    let blocks = top_level_blocks(source);
    let lines = LineIndex::new(source);
    let mut slugger = Slugger::default();
    let mut stack: Vec<(u8, String, String)> = Vec::new();
    let mut sections: Vec<Section> = Vec::new();

    for block in blocks {
        match block.heading {
            Some((level, text)) => {
                while stack.last().is_some_and(|(l, _, _)| *l >= level) {
                    stack.pop();
                }
                let anchor = slugger.slug(&text);
                stack.push((level, text, anchor.clone()));
                let slug_path: Vec<&str> = stack
                    .iter()
                    .map(|(_, _, s)| if s.is_empty() { "@empty" } else { s.as_str() })
                    .collect();
                sections.push(Section {
                    id: format!("{rel_path}#{}", slug_path.join("/")),
                    rel_path: rel_path.to_string(),
                    heading_path: stack.iter().map(|(_, t, _)| t.clone()).collect(),
                    level,
                    anchor,
                    span: lines.span(block.range.clone()),
                    sha256: String::new(),
                    blocks: vec![block.range],
                });
            }
            None => match sections.last_mut() {
                Some(section) => section.blocks.push(block.range),
                None => sections.push(Section {
                    id: format!("{rel_path}#"),
                    rel_path: rel_path.to_string(),
                    heading_path: Vec::new(),
                    level: 0,
                    anchor: String::new(),
                    span: lines.span(block.range.clone()),
                    sha256: String::new(),
                    blocks: vec![block.range],
                }),
            },
        }
    }

    // Extent end: last own block of the last section before the next heading
    // of equal or higher level. The preamble owns only its own blocks.
    let own_ends: Vec<usize> = sections
        .iter()
        .map(|s| s.blocks.last().map_or(s.span.byte_end, |b| b.end))
        .collect();
    for i in 0..sections.len() {
        let level = sections[i].level;
        let mut last = i;
        if level > 0 {
            while last + 1 < sections.len() && sections[last + 1].level > level {
                last += 1;
            }
        }
        let start = sections[i].span.byte_start;
        let span = lines.span(start..own_ends[last]);
        sections[i].sha256 = sha256_hex(&source.as_bytes()[span.byte_start..span.byte_end]);
        sections[i].span = span;
    }
    sections
}

/// Top-level blocks with trailing whitespace trimmed; front matter excluded.
fn top_level_blocks(source: &str) -> Vec<Block> {
    let options = Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;
    let mut blocks = Vec::new();
    let mut depth = 0usize;
    let mut skip = false;
    let mut heading: Option<(u8, String)> = None;

    for (event, range) in Parser::new_ext(source, options).into_offset_iter() {
        match event {
            Event::Start(tag) => {
                if depth == 0 {
                    skip = matches!(tag, Tag::MetadataBlock(_));
                    if let Tag::Heading { level, .. } = tag {
                        heading = Some((level as u8, String::new()));
                    }
                }
                depth += 1;
            }
            Event::End(tag) => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    if !skip {
                        let heading = heading.take().map(|(l, t)| (l, t.trim().to_string()));
                        if matches!(tag, TagEnd::Heading(_)) || heading.is_none() {
                            blocks.push(Block {
                                range: trim_end(source, range),
                                heading,
                            });
                        }
                    }
                    skip = false;
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, buf)) = heading.as_mut() {
                    buf.push_str(&text);
                }
            }
            _ if depth == 0 => blocks.push(Block {
                range: trim_end(source, range),
                heading: None,
            }),
            _ => {}
        }
    }
    blocks.retain(|b| !b.range.is_empty());
    blocks
}

fn trim_end(source: &str, range: Range<usize>) -> Range<usize> {
    let trimmed = source[range.clone()].trim_end().len();
    range.start..range.start + trimmed
}

/// GitHub anchor generation (github-slugger semantics).
#[derive(Default)]
struct Slugger {
    seen: HashMap<String, u32>,
}

impl Slugger {
    fn slug(&mut self, text: &str) -> String {
        let base = slugify(text);
        let mut result = base.clone();
        while self.seen.contains_key(&result) {
            let n = self.seen.entry(base.clone()).or_insert(0);
            *n += 1;
            result = format!("{base}-{n}");
        }
        self.seen.insert(result.clone(), 0);
        result
    }
}

fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ' '))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    const SAMPLE: &str = include_str!("../tests/fixtures/markdown/sample.md");

    #[test]
    fn sections_have_ids_levels_and_hand_checked_line_spans() {
        let sections = parse_sections("docs/sample.md", SAMPLE);
        let got: Vec<(&str, u8, usize, usize)> = sections
            .iter()
            .map(|s| (s.id.as_str(), s.level, s.span.line_start, s.span.line_end))
            .collect();
        // Line numbers from `cat -n tests/fixtures/markdown/sample.md`.
        // Front matter (1-4) and headings inside fences (17, 21, 26) are excluded.
        assert_eq!(
            got,
            vec![
                ("docs/sample.md#", 0, 6, 6),
                ("docs/sample.md#guide", 1, 8, 37),
                ("docs/sample.md#guide/install", 2, 12, 28),
                ("docs/sample.md#guide/install-1", 2, 30, 32),
                ("docs/sample.md#guide/setext-title", 2, 34, 37),
                ("docs/sample.md#api-reference", 1, 39, 42),
                ("docs/sample.md#qa-other", 1, 44, 46),
            ]
        );
        assert_eq!(sections[3].heading_path, vec!["Guide", "Install"]);
        assert_eq!(sections[6].heading_path, vec!["Q&A: Other"]);
        // Own blocks of `guide` stop at its first child heading (line 12).
        let guide_own_end = sections[1].blocks.last().unwrap().end;
        assert_eq!(LineIndex::new(SAMPLE).line(guide_own_end - 1), 10);
    }

    #[test]
    fn section_hashes_match_span_bytes() {
        for s in parse_sections("docs/sample.md", SAMPLE) {
            let bytes = &SAMPLE.as_bytes()[s.span.byte_start..s.span.byte_end];
            assert_eq!(s.sha256, sha256_hex(bytes), "{}", s.id);
            assert!(!bytes.is_empty());
        }
    }

    #[test]
    fn inserting_a_paragraph_keeps_section_ids() {
        let edited = SAMPLE.replacen("Guide body.\n", "Guide body.\n\nInserted paragraph.\n", 1);
        let ids = |src: &str| -> Vec<String> {
            parse_sections("a.md", src)
                .into_iter()
                .map(|s| s.id)
                .collect()
        };
        assert_eq!(ids(SAMPLE), ids(&edited));
    }

    #[test]
    fn empty_id_components_do_not_change_markdown_anchors() {
        for (source, suffixes, anchors) in [
            ("Intro.\n\n# !!!\nBody.", vec!["", "@empty"], vec!["", ""]),
            (
                "# Parent\n## !!!\n### Child",
                vec!["parent", "parent/@empty", "parent/@empty/child"],
                vec!["parent", "", "child"],
            ),
            ("# !!!\n## ???", vec!["@empty", "@empty/-1"], vec!["", "-1"]),
            (
                "# !!!\n# -1\n# ???",
                vec!["@empty", "-1", "-2"],
                vec!["", "-1", "-2"],
            ),
            (
                "# a\n# a\n# a-1\n# a",
                vec!["a", "a-1", "a-1-1", "a-2"],
                vec!["a", "a-1", "a-1-1", "a-2"],
            ),
            (
                "# !!!\n# @empty\n# empty\n# Über Straße",
                vec!["@empty", "empty", "empty-1", "über-straße"],
                vec!["", "empty", "empty-1", "über-straße"],
            ),
        ] {
            for path in ["a.md", "a#part~0.md"] {
                let sections = parse_sections(path, source);
                assert_eq!(
                    sections.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
                    suffixes
                        .iter()
                        .map(|s| format!("{path}#{s}"))
                        .collect::<Vec<_>>()
                );
                assert_eq!(
                    sections
                        .iter()
                        .map(|s| s.anchor.as_str())
                        .collect::<Vec<_>>(),
                    anchors
                );
                let ids: std::collections::HashSet<_> = sections.iter().map(|s| &s.id).collect();
                assert_eq!(ids.len(), sections.len());
            }
        }
    }

    #[test]
    fn slugs_follow_github_rules() {
        assert_eq!(slugify("Q&A: Other"), "qa-other");
        assert_eq!(slugify("Hello  World_x"), "hello--world_x");
        assert_eq!(slugify("Über Straße"), "über-straße");
        let mut s = Slugger::default();
        let got: Vec<String> = ["a", "a", "a-1", "a"].iter().map(|t| s.slug(t)).collect();
        // github-slugger: a, a-1, a-1-1, a-2.
        assert_eq!(got, vec!["a", "a-1", "a-1-1", "a-2"]);
    }

    #[test]
    fn line_index_maps_bytes_to_lines() {
        let idx = LineIndex::new("ab\ncd\n");
        assert_eq!((idx.line(0), idx.line(2), idx.line(3)), (1, 1, 2));
        let span = idx.span(3..5);
        assert_eq!((span.line_start, span.line_end), (2, 2));
    }

    #[test]
    fn empty_and_front_matter_only_documents_have_no_sections() {
        assert!(parse_sections("a.md", "").is_empty());
        assert!(parse_sections("a.md", "---\nx: 1\n---\n").is_empty());
    }
}
