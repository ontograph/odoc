//! Chunks: the retrieval unit.
//!
//! Each section's own blocks are packed greedily into chunks of at most the
//! budget, splitting only at block boundaries. A single block larger than the
//! budget becomes its own chunk. Chunk id = `<section id>~<ordinal>`.

use crate::corpus::sha256_hex;
use crate::parse::{LineIndex, Section, Span};

/// Frozen v1 chunk budget in bytes (ADR §4); never tuned.
pub const CHUNK_BUDGET: usize = 1200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub id: String,
    pub section_id: String,
    pub ordinal: u32,
    pub rel_path: String,
    pub heading_path: Vec<String>,
    pub span: Span,
    pub sha256: String,
}

/// Chunks every section of one document, in document order.
pub fn chunk_sections(source: &str, sections: &[Section], budget: usize) -> Vec<Chunk> {
    let lines = LineIndex::new(source);
    let mut chunks = Vec::new();
    for section in sections {
        let mut groups: Vec<(usize, usize)> = Vec::new();
        for block in &section.blocks {
            match groups.last_mut() {
                Some((start, end)) if block.end - *start <= budget => *end = block.end,
                _ => groups.push((block.start, block.end)),
            }
        }
        for (ordinal, (start, end)) in (0u32..).zip(groups) {
            chunks.push(Chunk {
                id: format!("{}~{ordinal}", section.id),
                section_id: section.id.clone(),
                ordinal,
                rel_path: section.rel_path.clone(),
                heading_path: section.heading_path.clone(),
                span: lines.span(start..end),
                sha256: sha256_hex(&source.as_bytes()[start..end]),
            });
        }
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_sections;
    use pretty_assertions::assert_eq;
    use std::collections::BTreeSet;

    const SAMPLE: &str = include_str!("../tests/fixtures/markdown/sample.md");

    fn ids(source: &str, budget: usize) -> (Vec<String>, Vec<String>) {
        let sections = parse_sections("a.md", source);
        let chunks = chunk_sections(source, &sections, budget);
        (
            sections.into_iter().map(|s| s.id).collect(),
            chunks.into_iter().map(|c| c.id).collect(),
        )
    }

    #[test]
    fn chunk_spans_hash_and_map_totally_to_sections() {
        let sections = parse_sections("a.md", SAMPLE);
        let chunks = chunk_sections(SAMPLE, &sections, CHUNK_BUDGET);
        let section_ids: BTreeSet<&str> = sections.iter().map(|s| s.id.as_str()).collect();
        let covered: BTreeSet<&str> = chunks.iter().map(|c| c.section_id.as_str()).collect();
        assert_eq!(section_ids, covered);
        for c in &chunks {
            let bytes = &SAMPLE.as_bytes()[c.span.byte_start..c.span.byte_end];
            assert!(!bytes.is_empty(), "{}", c.id);
            assert_eq!(c.sha256, sha256_hex(bytes));
        }
        // Hand-checked against `cat -n`: second Install heading through "Second install.".
        let install = chunks
            .iter()
            .find(|c| c.id == "a.md#guide/install-1~0")
            .unwrap();
        assert_eq!((install.span.line_start, install.span.line_end), (30, 32));
    }

    #[test]
    fn section_ids_do_not_depend_on_budget() {
        let (s800, _) = ids(SAMPLE, 800);
        let (s1200, _) = ids(SAMPLE, 1200);
        let (s2000, _) = ids(SAMPLE, 2000);
        assert_eq!(s800, s1200);
        assert_eq!(s1200, s2000);
    }

    #[test]
    fn empty_components_and_delimiter_paths_stay_unique_across_budgets() {
        let source = "intro\n\n# Parent\n\nparent\n\n## !!!\n\nempty\n\n### Child\n\nchild\n\n# a\n# a\n# a-1\n# a\n";
        for path in ["a.md", "a#part~0.md"] {
            let sections = parse_sections(path, source);
            let expected: Vec<_> = [
                "",
                "parent",
                "parent/@empty",
                "parent/@empty/child",
                "a",
                "a-1",
                "a-1-1",
                "a-2",
            ]
            .map(|suffix| format!("{path}#{suffix}"))
            .into();
            assert_eq!(
                sections.iter().map(|s| s.id.clone()).collect::<Vec<_>>(),
                expected
            );
            for budget in [1, 800, 1200, 2000] {
                let chunks = chunk_sections(source, &sections, budget);
                let ids: BTreeSet<_> = chunks.iter().map(|c| &c.id).collect();
                assert_eq!(ids.len(), chunks.len());
                let covered: BTreeSet<_> = chunks.iter().map(|c| c.section_id.clone()).collect();
                assert_eq!(covered, expected.iter().cloned().collect());
                for c in chunks {
                    assert_eq!(c.id, format!("{}~{}", c.section_id, c.ordinal));
                }
            }
        }
    }

    #[test]
    fn inserting_a_paragraph_keeps_later_chunk_ids() {
        let edited = SAMPLE.replacen("Guide body.\n", "Guide body.\n\nInserted paragraph.\n", 1);
        let (_, before) = ids(SAMPLE, CHUNK_BUDGET);
        let (_, after) = ids(&edited, CHUNK_BUDGET);
        assert_eq!(before, after);
    }

    #[test]
    fn oversized_sections_split_only_at_block_boundaries() {
        // Blocks: heading (5 bytes), three 39-byte paragraphs, one 80-byte paragraph.
        let para = "x".repeat(39);
        let long = "y".repeat(80);
        let source = format!("# Big\n\n{para}\n\n{para}\n\n{para}\n\n{long}\n");
        let sections = parse_sections("a.md", &source);
        let section = &sections[0];
        let chunks = chunk_sections(&source, &sections, 50);
        // Budget 50: heading+p1 spans 5+2+39 = 46 <= 50; adding p2 would span 87,
        // so p2, p3 and long each start a chunk; long (80 > 50) is kept whole.
        let lens: Vec<usize> = chunks
            .iter()
            .map(|c| c.span.byte_end - c.span.byte_start)
            .collect();
        assert_eq!(lens, vec![46, 39, 39, 80]);
        let starts: BTreeSet<usize> = section.blocks.iter().map(|b| b.start).collect();
        let ends: BTreeSet<usize> = section.blocks.iter().map(|b| b.end).collect();
        for (i, c) in chunks.iter().enumerate() {
            assert!(starts.contains(&c.span.byte_start));
            assert!(ends.contains(&c.span.byte_end));
            assert_eq!(c.ordinal as usize, i);
            assert_eq!(c.id, format!("a.md#big~{i}"));
        }
    }
}
