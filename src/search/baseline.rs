//! B0: the prior attempt's documentation search, reimplemented as a baseline.
//!
//! The whole query, trimmed and ASCII-lowercased, must be a substring of the
//! document title, any heading, or the first 1 KiB of raw Markdown (each
//! ASCII-lowercased). Matches are returned in path order, unscored. Each
//! matching document is represented by its first chunk so that B0 can be
//! evaluated at section level like the other systems.

use crate::store::Generation;

use super::Hit;

const SUMMARY_BYTES: usize = 1024;

pub fn b0(generation: &Generation, query: &str, top_k: usize) -> Vec<Hit> {
    let needle = query.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for doc in &generation.documents {
        if hits.len() == top_k {
            break;
        }
        let headings: Vec<&str> = generation
            .sections
            .iter()
            .filter(|s| s.path == doc.path && s.level > 0)
            .filter_map(|s| s.heading_path.last().map(String::as_str))
            .collect();
        let title = headings.first().copied().unwrap_or(doc.path.as_str());
        let mut cut = doc.text.len().min(SUMMARY_BYTES);
        while !doc.text.is_char_boundary(cut) {
            cut -= 1;
        }
        let matches = |text: &str| text.to_ascii_lowercase().contains(&needle);
        if !(matches(title) || headings.iter().any(|h| matches(h)) || matches(&doc.text[..cut])) {
            continue;
        }
        if let Some(chunk) = generation.chunks.iter().position(|c| c.path == doc.path) {
            hits.push(Hit { chunk, score: 1.0 });
        }
    }
    hits
}
