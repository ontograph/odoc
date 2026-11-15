//! Structural tiers, derived without any model.
//!
//! Every tier is a `\n`-joined list of source substrings: heading texts (ATX
//! markers and setext underlines removed) and first sentences. A tier that
//! hits its cap is cut at a UTF-8 boundary and flagged `truncated`.
//!
//! - L0 (per chunk): heading path, then the first sentence of the chunk.
//! - L1 (per section, shared by its chunks): the section heading and first
//!   sentence, then every descendant heading with its first sentence.
//! - Pseudo-document (per document): every heading and first sentence in
//!   document order; the first heading is the title.
//!
//! First sentence: the first paragraph block, up to and including the first
//! `.`, `?` or `!` followed by whitespace, or the whole block.

use std::collections::HashMap;
use std::ops::Range;

use pulldown_cmark::{Event, Parser, Tag};

use crate::chunk::Chunk;
use crate::parse::{MARKDOWN_OPTIONS, Section};

pub const L0_CAP: usize = 200;
pub const L1_CAP: usize = 1500;
pub const PSEUDO_DOC_CAP: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tier {
    pub text: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkTiers {
    pub l0: Tier,
    pub l1: Tier,
}

/// Tiers for `chunks` (same order), which must come from `sections` of `source`.
pub fn chunk_tiers(source: &str, sections: &[Section], chunks: &[Chunk]) -> Vec<ChunkTiers> {
    let paths = heading_paths(source, sections);
    let by_id: HashMap<&str, usize> = sections
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();
    let l1: Vec<Tier> = (0..sections.len())
        .map(|i| section_l1(source, sections, i))
        .collect();
    chunks
        .iter()
        .filter_map(|chunk| {
            let i = *by_id.get(chunk.section_id.as_str())?;
            let mut l0 = Builder::new(source, L0_CAP);
            for h in &paths[i] {
                l0.push(h.clone());
            }
            let own = sections[i]
                .blocks
                .iter()
                .filter(|b| b.start >= chunk.span.byte_start && b.end <= chunk.span.byte_end);
            if let Some(s) = first_sentence(source, own) {
                l0.push(s);
            }
            Some(ChunkTiers {
                l0: l0.finish(),
                l1: l1[i].clone(),
            })
        })
        .collect()
}

/// Structural pseudo-document of one document.
pub fn pseudo_document(source: &str, sections: &[Section]) -> Tier {
    let mut out = Builder::new(source, PSEUDO_DOC_CAP);
    for section in sections {
        push_outline_entry(&mut out, source, section);
    }
    out.finish()
}

fn section_l1(source: &str, sections: &[Section], i: usize) -> Tier {
    let mut out = Builder::new(source, L1_CAP);
    push_outline_entry(&mut out, source, &sections[i]);
    let level = sections[i].level;
    if level > 0 {
        for child in sections[i + 1..].iter().take_while(|s| s.level > level) {
            push_outline_entry(&mut out, source, child);
        }
    }
    out.finish()
}

fn push_outline_entry(out: &mut Builder<'_>, source: &str, section: &Section) {
    if let Some(h) = heading_text(source, section) {
        out.push(h);
    }
    if let Some(s) = first_sentence(source, section.blocks.iter()) {
        out.push(s);
    }
}

/// Cache ancestry once per section, including levels whose heading is absent.
fn heading_paths(source: &str, sections: &[Section]) -> Vec<Vec<Range<usize>>> {
    let mut stack: Vec<(u8, Option<Range<usize>>)> = Vec::new();
    sections
        .iter()
        .map(|section| {
            // The old reverse scan ignored both zero and u8::MAX. Neither may
            // clear ancestry; a missing heading still shadows its earlier level.
            if section.level > 0 && section.level < u8::MAX {
                while stack
                    .last()
                    .is_some_and(|(level, _)| *level >= section.level)
                {
                    stack.pop();
                }
                stack.push((section.level, heading_text(source, section)));
            }
            stack
                .iter()
                .filter_map(|(_, heading)| heading.clone())
                .collect()
        })
        .collect()
}

/// Source range of the heading text, without ATX markers or setext underline.
fn heading_text(source: &str, section: &Section) -> Option<Range<usize>> {
    if section.level == 0 {
        return None;
    }
    let block = section.blocks.first()?;
    let raw = source.get(block.clone())?.trim_start();
    let text = match raw.strip_prefix('#') {
        Some(_) => {
            let body = raw.trim_start_matches('#').trim();
            let closed = body.trim_end_matches('#');
            if closed.is_empty() || closed.ends_with([' ', '\t']) {
                closed.trim_end()
            } else {
                body
            }
        }
        None => raw.rsplit_once('\n').map_or(raw, |(text, _)| text).trim(),
    };
    Some(sub_range(source, text))
}

fn first_sentence<'a>(
    source: &str,
    blocks: impl Iterator<Item = &'a Range<usize>>,
) -> Option<Range<usize>> {
    let block = blocks.filter_map(|b| source.get(b.clone())).find(|text| {
        matches!(
            Parser::new_ext(text, MARKDOWN_OPTIONS).next(),
            Some(Event::Start(Tag::Paragraph))
        )
    })?;
    let text = block.trim_start();
    let bytes = text.as_bytes();
    let end = (0..bytes.len())
        .find(|&i| {
            matches!(bytes[i], b'.' | b'?' | b'!')
                && bytes.get(i + 1).is_some_and(u8::is_ascii_whitespace)
        })
        .map_or(text.len(), |i| i + 1);
    Some(sub_range(source, &text[..end]))
}

/// Range of `sub`, which must be a subslice of `source`.
fn sub_range(source: &str, sub: &str) -> Range<usize> {
    let start = (sub.as_ptr() as usize).saturating_sub(source.as_ptr() as usize);
    start..start + sub.len()
}

struct Builder<'a> {
    source: &'a str,
    cap: usize,
    tier: Tier,
}

impl<'a> Builder<'a> {
    fn new(source: &'a str, cap: usize) -> Self {
        Self {
            source,
            cap,
            tier: Tier::default(),
        }
    }

    fn push(&mut self, range: Range<usize>) {
        let piece = self.source.get(range).unwrap_or("");
        if self.tier.truncated || piece.is_empty() {
            return;
        }
        let sep = usize::from(!self.tier.text.is_empty());
        let room = self.cap.saturating_sub(self.tier.text.len() + sep);
        let mut cut = piece.len().min(room);
        while !piece.is_char_boundary(cut) {
            cut -= 1;
        }
        if cut < piece.len() {
            self.tier.truncated = true;
        }
        if cut > 0 {
            if sep == 1 {
                self.tier.text.push('\n');
            }
            self.tier.text.push_str(&piece[..cut]);
        }
    }

    fn finish(self) -> Tier {
        self.tier
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{CHUNK_BUDGET, chunk_sections};
    use crate::parse::parse_sections;
    use pretty_assertions::assert_eq;

    const SAMPLE: &str = include_str!("../tests/fixtures/markdown/sample.md");

    fn old_chunk_tiers(source: &str, sections: &[Section], chunks: &[Chunk]) -> Vec<ChunkTiers> {
        let by_id: HashMap<_, _> = sections
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.as_str(), i))
            .collect();
        let l1: Vec<_> = (0..sections.len())
            .map(|i| section_l1(source, sections, i))
            .collect();
        chunks
            .iter()
            .filter_map(|chunk| {
                let i = *by_id.get(chunk.section_id.as_str())?;
                let mut path = Vec::new();
                let mut level = u8::MAX;
                for section in sections[..=i].iter().rev() {
                    if section.level > 0 && section.level < level {
                        level = section.level;
                        path.extend(heading_text(source, section));
                    }
                }
                path.reverse();
                let mut l0 = Builder::new(source, L0_CAP);
                for h in path {
                    l0.push(h);
                }
                let own = sections[i]
                    .blocks
                    .iter()
                    .filter(|b| b.start >= chunk.span.byte_start && b.end <= chunk.span.byte_end);
                if let Some(s) = first_sentence(source, own) {
                    l0.push(s);
                }
                Some(ChunkTiers {
                    l0: l0.finish(),
                    l1: l1[i].clone(),
                })
            })
            .collect()
    }

    #[test]
    fn cached_heading_paths_preserve_complete_tiers_for_public_inputs() {
        let source = "Preamble.\n\n# A\n\nBody.\n\n## B\n\nBody b.\n\n### C\n\nBody c.\n\n# D\n\nBody d.\n\nSetext\n------\n\nLast.\n";
        let original = parse_sections("a.md", source);
        let mut unusual = original.clone();
        // A missing level-1 heading shadows A; zero and 255 leave that
        // ancestry intact. Public levels are not restricted to Markdown's six.
        unusual[2].level = 1;
        unusual[2].blocks.clear();
        unusual[3].level = 0;
        unusual[4].level = 255;
        unusual[5].level = 200;
        unusual[5].id = unusual[1].id.clone();
        for sections in [original, unusual] {
            let mut chunks = chunk_sections(source, &sections, 20);
            chunks.reverse();
            let mut unknown = chunks[0].clone();
            unknown.section_id = "unknown".into();
            chunks.insert(1, unknown);
            let new = chunk_tiers(source, &sections, &chunks);
            assert_eq!(new.len(), chunks.len() - 1);
            assert_eq!(new, old_chunk_tiers(source, &sections, &chunks));
        }
    }

    #[test]
    #[ignore = "synthetic timing observation; no performance acceptance threshold"]
    fn synthetic_scan_timing() {
        let source: String = (0..1000).map(|i| format!("# H{i}\n\nBody.\n\n")).collect();
        let sections = parse_sections("a.md", &source);
        let chunks = chunk_sections(&source, &sections, CHUNK_BUDGET);
        let start = std::time::Instant::now();
        let old = old_chunk_tiers(&source, &sections, &chunks);
        let old_time = start.elapsed();
        let start = std::time::Instant::now();
        let new = chunk_tiers(&source, &sections, &chunks);
        eprintln!("synthetic tiers old={old_time:?} new={:?}", start.elapsed());
        assert_eq!(new, old);
    }

    fn tiers_of(source: &str) -> (Vec<Chunk>, Vec<ChunkTiers>, Tier) {
        let sections = parse_sections("a.md", source);
        let chunks = chunk_sections(source, &sections, CHUNK_BUDGET);
        let tiers = chunk_tiers(source, &sections, &chunks);
        let doc = pseudo_document(source, &sections);
        (chunks, tiers, doc)
    }

    fn assert_copied(source: &str, tier: &Tier) {
        for line in tier.text.split('\n') {
            assert!(source.contains(line), "not in source: {line:?}");
        }
    }

    #[test]
    fn sample_tiers_match_hand_written_outline() {
        let (chunks, tiers, doc) = tiers_of(SAMPLE);
        let get = |id: &str| &tiers[chunks.iter().position(|c| c.id == id).unwrap()];
        assert_eq!(
            get("a.md#guide/install-1~0").l0.text,
            "Guide\nInstall\nSecond install."
        );
        assert_eq!(
            get("a.md#~0").l0.text,
            "Intro paragraph before any heading."
        );
        assert_eq!(
            get("a.md#guide~0").l1.text,
            "Guide\nGuide body.\nInstall\nRun this:\nInstall\nSecond install.\nSetext Title\nSetext body."
        );
        assert_eq!(
            doc.text,
            "Intro paragraph before any heading.\nGuide\nGuide body.\nInstall\nRun this:\n\
             Install\nSecond install.\nSetext Title\nSetext body.\nAPI Reference\n\
             Reference body.\nQ&A: Other\nLast section."
        );
        assert_eq!(tiers.len(), chunks.len());
    }

    #[test]
    fn every_tier_line_is_copied_from_source() {
        let (_, tiers, doc) = tiers_of(SAMPLE);
        for t in &tiers {
            assert_copied(SAMPLE, &t.l0);
            assert_copied(SAMPLE, &t.l1);
        }
        assert_copied(SAMPLE, &doc);
    }

    #[test]
    fn caps_cut_at_utf8_boundaries() {
        // "Héading" is 8 bytes; 8 + "\n" leaves 191 bytes of room. The sentence is
        // 150 two-byte chars, so the cut steps back from 191 to the boundary 190.
        let source = format!("# Héading\n\n{}\n", "é".repeat(150));
        let (_, tiers, _) = tiers_of(&source);
        let l0 = &tiers[0].l0;
        assert!(l0.truncated);
        assert!(l0.text.len() <= L0_CAP);
        assert_eq!(l0.text.len(), 8 + 1 + 190);
        assert_copied(&source, l0);

        // "x" + "\n" leaves 198 bytes of room. After the ASCII "a", char
        // boundaries fall at odd offsets, so the cut steps back from 198 to 197.
        let source = format!("# x\n\na{}\n", "é".repeat(150));
        let (_, tiers, _) = tiers_of(&source);
        assert_eq!(tiers[0].l0.text.len(), 1 + 1 + 1 + 2 * 98);
        assert!(tiers[0].l1.text.len() <= L1_CAP);
    }

    #[test]
    fn first_sentence_rules() {
        let one = |text: &str| {
            let r = first_sentence(text, std::iter::once(&(0..text.len()))).unwrap();
            text[r].to_string()
        };
        assert_eq!(one("First one. Second."), "First one.");
        assert_eq!(one("Why? Because."), "Why?");
        assert_eq!(one("Stop!\nNext line."), "Stop!");
        assert_eq!(one("v1.2 is out. Next"), "v1.2 is out.");
        assert_eq!(one("No terminator"), "No terminator");
        // A GFM table is a table block, not a paragraph (schema 3).
        let doc = "| A. b |\n| --- |\n| B |\n\nActual sentence. More.\n";
        let table = 0..doc.find("\n\n").unwrap();
        let para = table.end + 2..doc.len() - 1;
        let r = first_sentence(doc, [table, para].iter()).unwrap();
        assert_eq!(&doc[r], "Actual sentence.");
        let code = "```\ncode. here\n```";
        assert_eq!(
            first_sentence(code, std::iter::once(&(0..code.len()))),
            None
        );
    }

    #[test]
    fn heading_text_strips_markers() {
        for (src, want) in [
            ("## Title ##\n", "Title"),
            ("# C# #\n", "C#"),
            ("#  Spaced   \n", "Spaced"),
            ("Multi\nline\n===\n", "Multi\nline"),
        ] {
            let sections = parse_sections("a.md", src);
            let r = heading_text(src, &sections[0]).unwrap();
            assert_eq!(&src[r], want, "{src:?}");
        }
    }
}
