//! ADR discrimination check on synthetic data only: an oracle ranking beats
//! a seeded shuffle with p < 0.01, a system compared with itself gives p = 1,
//! and the real B1..B3 rankers find planted sections far above a shuffle.
//! B0 (ADR §6) returns each matching document's first chunk, so it cannot
//! reach a planted later section; that baseline property is asserted too.
//! This is a ranking-sanity diagnostic, not a retrieval-quality claim.

use std::collections::BTreeMap;

use ontodoc::eval::metrics::{Qrels, ndcg_at};
use ontodoc::eval::run::{PerQuery, System, compare, rank_sections};
use ontodoc::eval::stats::SplitMix64;
use ontodoc::search::SearchIndex;
use ontodoc::store::{IndexConfig, build};

const DOCS: usize = 24;
const RESAMPLES: usize = 2000;
const SEED: u64 = 11;

/// Unique planted term for document `i`, e.g. `plantedqa`.
fn term(i: usize) -> String {
    let a = char::from(b'a' + (i / 26) as u8);
    let b = char::from(b'a' + (i % 26) as u8);
    format!("planted{a}{b}")
}

/// 24 documents, each with three sections; only `Target` holds its term.
fn corpus() -> (tempfile::TempDir, SearchIndex) {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..DOCS {
        let body = format!(
            "# Doc {i}\n\nShared filler about installing and configuring tools.\n\n\
             ## Intro\n\nGeneral overview text repeated across documents.\n\n\
             ## Target\n\nThe {} keyword appears only here.\n\n\
             ## Notes\n\nMore filler about configuring tools.\n",
            term(i)
        );
        std::fs::write(dir.path().join(format!("d{i:02}.md")), body).unwrap();
    }
    let generation = build(dir.path(), &IndexConfig::default()).unwrap();
    (dir, SearchIndex::try_new(generation).unwrap())
}

fn target(i: usize) -> String {
    format!("d{i:02}.md#doc-{i}/target")
}

fn qrels(i: usize) -> Qrels {
    BTreeMap::from([(target(i), 2)])
}

fn per_query(rankings: &[Vec<String>]) -> PerQuery {
    rankings
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let r: Vec<&str> = r.iter().map(String::as_str).collect();
            (format!("q{i:02}"), ndcg_at(&qrels(i), &r, 10).unwrap())
        })
        .collect()
}

fn corpus_of() -> BTreeMap<String, String> {
    (0..DOCS)
        .map(|i| (format!("q{i:02}"), "c".into()))
        .collect()
}

#[test]
fn oracle_beats_shuffle_and_rankers_find_planted_sections() {
    let (_dir, index) = corpus();
    let mut sections: Vec<String> = index
        .generation
        .sections
        .iter()
        .map(|s| s.id.clone())
        .collect();
    sections.sort();
    assert!(sections.contains(&target(0)), "{sections:?}");

    let oracle: Vec<Vec<String>> = (0..DOCS).map(|i| vec![target(i)]).collect();
    let mut rng = SplitMix64::new(SEED);
    let shuffled: Vec<Vec<String>> = (0..DOCS)
        .map(|_| {
            let mut s = sections.clone();
            for k in (1..s.len()).rev() {
                s.swap(k, rng.below(k + 1));
            }
            s.truncate(10);
            s
        })
        .collect();
    let (oracle, shuffled) = (per_query(&oracle), per_query(&shuffled));
    let of = corpus_of();

    // Oracle nDCG is 1 per query; a shuffle of 96 sections rarely hits.
    let (n, ci, p) = compare(&oracle, &shuffled, &of, RESAMPLES, SEED);
    assert_eq!(n, DOCS);
    assert!(ci.lo > 0.0 && p < 0.01, "ci {ci:?} p {p}");
    // Self-comparison: every difference is zero, so p = 1 exactly.
    let (_, _, p_self) = compare(&oracle, &oracle, &of, RESAMPLES, SEED);
    assert_eq!(p_self, 1.0);

    // B0 matches the document but cites its first section, never the target.
    let b0 = rank_sections(&index, System::B0, &term(3));
    assert_eq!(b0.len(), 1);
    assert_ne!(b0[0].0, target(3));
    assert!(b0[0].0.starts_with("d03.md#"), "{b0:?}");

    for system in [System::B1, System::B2(0.5), System::B3(0.5)] {
        let runs: Vec<Vec<String>> = (0..DOCS)
            .map(|i| {
                rank_sections(&index, system, &term(i))
                    .into_iter()
                    .map(|(id, _)| id)
                    .collect()
            })
            .collect();
        let found = per_query(&runs);
        let (_, ci, p) = compare(&found, &shuffled, &of, RESAMPLES, SEED);
        assert!(ci.lo > 0.0 && p < 0.01, "{} ci {ci:?} p {p}", system.name());
        // Deterministic: the same query gives byte-identical rankings.
        assert_eq!(
            rank_sections(&index, system, &term(0)),
            rank_sections(&index, system, &term(0))
        );
    }
}
