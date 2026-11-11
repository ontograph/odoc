//! Okapi BM25 with literature defaults.
//!
//! `idf(t) = ln(1 + (N - n_t + 0.5) / (n_t + 0.5))` (always positive);
//! `score(d) = Σ_t idf(t) · tf·(k1+1) / (tf + k1·(1 - b + b·|d|/avgdl))`
//! over unique query terms, summed in query-term order.

use std::collections::HashMap;

pub const K1: f64 = 1.2;
pub const B: f64 = 0.75;

#[derive(Debug, Clone, PartialEq)]
pub struct Bm25 {
    postings: HashMap<String, Vec<(u32, u32)>>,
    doc_len: Vec<u32>,
    avgdl: f64,
}

impl Bm25 {
    /// Indexes one token list per document; document ids are positions.
    pub fn new(docs: impl IntoIterator<Item = Vec<String>>) -> Self {
        let mut postings: HashMap<String, Vec<(u32, u32)>> = HashMap::new();
        let mut doc_len = Vec::new();
        for (doc, tokens) in (0u32..).zip(docs) {
            doc_len.push(u32::try_from(tokens.len()).unwrap_or(u32::MAX));
            let mut tf: HashMap<String, u32> = HashMap::new();
            for token in tokens {
                *tf.entry(token).or_insert(0) += 1;
            }
            for (term, count) in tf {
                postings.entry(term).or_default().push((doc, count));
            }
        }
        let total: f64 = doc_len.iter().map(|&l| f64::from(l)).sum();
        let avgdl = if doc_len.is_empty() {
            0.0
        } else {
            total / doc_len.len() as f64
        };
        Self {
            postings,
            doc_len,
            avgdl,
        }
    }

    pub fn len(&self) -> usize {
        self.doc_len.len()
    }

    pub fn is_empty(&self) -> bool {
        self.doc_len.is_empty()
    }

    pub fn idf(&self, term: &str) -> f64 {
        let n = self.postings.get(term).map_or(0, Vec::len) as f64;
        let total = self.len() as f64;
        (1.0 + (total - n + 0.5) / (n + 0.5)).ln()
    }

    /// Dense scores, one per document, for unique `terms`.
    pub fn scores(&self, terms: &[String]) -> Vec<f64> {
        let mut scores = vec![0.0; self.len()];
        for term in terms {
            let Some(postings) = self.postings.get(term) else {
                continue;
            };
            let idf = self.idf(term);
            for &(doc, tf) in postings {
                let len = f64::from(self.doc_len[doc as usize]);
                let ratio = if self.avgdl > 0.0 {
                    len / self.avgdl
                } else {
                    1.0
                };
                let tf = f64::from(tf);
                scores[doc as usize] += idf * tf * (K1 + 1.0) / (tf + K1 * (1.0 - B + B * ratio));
            }
        }
        scores
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// d0 = "a b a", d1 = "b c", d2 = "c c c d"; N = 3, avgdl = 9/3 = 3.
    pub(crate) fn toy() -> Bm25 {
        let doc = |s: &str| s.split(' ').map(String::from).collect::<Vec<_>>();
        Bm25::new([doc("a b a"), doc("b c"), doc("c c c d")])
    }

    #[test]
    fn scores_match_hand_computed_values() {
        let bm25 = toy();
        // idf(a): n=1 -> ln(1 + 2.5/1.5) = ln(2.666..) = 0.9808292530117263
        // idf(c): n=2 -> ln(1 + 1.5/2.5) = ln(1.6)     = 0.47000362924573563
        assert!((bm25.idf("a") - (8.0f64 / 3.0).ln()).abs() < 1e-12);
        assert!((bm25.idf("c") - 1.6f64.ln()).abs() < 1e-12);
        let s = bm25.scores(&["a".into(), "c".into()]);
        // d0: tf(a)=2, |d|=3: K = 1.2*(0.25+0.75*1)   = 1.2; 0.98083*2*2.2/(2+1.2) = 1.3486402228911236
        // d1: tf(c)=1, |d|=2: K = 1.2*(0.25+0.75*2/3) = 0.9; 0.47000*1*2.2/(1+0.9) = 0.5442147286003255
        // d2: tf(c)=3, |d|=4: K = 1.2*(0.25+0.75*4/3) = 1.5; 0.47000*3*2.2/(3+1.5) = 0.6893386562270789
        let want = [1.3486402228911236, 0.5442147286003255, 0.6893386562270789];
        for (got, want) in s.iter().zip(want) {
            assert!((got - want).abs() < 1e-9, "{got} vs {want}");
        }
        assert_eq!(bm25.scores(&["zzz".into()]), vec![0.0; 3]);
    }

    #[test]
    fn empty_index_scores_nothing() {
        let bm25 = Bm25::new(Vec::<Vec<String>>::new());
        assert!(bm25.is_empty());
        assert!(bm25.scores(&["a".into()]).is_empty());
    }

    #[test]
    fn empty_document_counts_toward_avgdl() {
        // d0 = "", d1 = "a"; N = 2, avgdl = 1/2.
        // idf(a): n=1 -> ln(1 + 1.5/1.5) = ln 2.
        // d1: tf=1, |d|=1, ratio=2: K = 1.2*(0.25+0.75*2) = 2.1;
        //     ln2*1*2.2/(1+2.1) = (22/31)·ln 2 = 0.49190...
        let bm25 = Bm25::new([Vec::new(), vec!["a".to_string()]]);
        assert!((bm25.idf("a") - 2f64.ln()).abs() < 1e-12);
        let s = bm25.scores(&["a".into()]);
        assert_eq!(s[0], 0.0);
        assert!((s[1] - 22.0 / 31.0 * 2f64.ln()).abs() < 1e-12, "{}", s[1]);
    }
}
