//! Citation verification against current file bytes.
//!
//! - `valid`: the bytes at the cited span hash to the cited SHA-256.
//! - `moved`: they don't, but exactly one other span of the same length in
//!   the file has that hash; the new span is returned.
//! - `stale`: the hash occurs nowhere, or more than once.
//! - `missing`: the file is gone (or is not a regular file under the root).
//!
//! Reads stay beneath the root and never follow symlinks.

use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::corpus::{CorpusError, RootedCorpus, sha256_hex};
use crate::parse::LineIndex;
use crate::store::{Generation, SectionRecord, SpanRecord};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Citation {
    pub generation_id: String,
    pub path: String,
    pub heading_path: Vec<String>,
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: usize,
    pub line_end: usize,
    pub sha256: String,
}

impl Citation {
    pub fn for_section(generation: &Generation, section: &SectionRecord) -> Self {
        Self {
            generation_id: generation.generation_id.clone(),
            path: section.path.clone(),
            heading_path: section.heading_path.clone(),
            byte_start: section.span.byte_start,
            byte_end: section.span.byte_end,
            line_start: section.span.line_start,
            line_end: section.span.line_end,
            sha256: section.sha256.clone(),
        }
    }

    /// Citation for chunk `index` of `generation`.
    pub fn for_chunk(generation: &Generation, index: usize) -> Option<Self> {
        let c = generation.chunks.get(index)?;
        Some(Self {
            generation_id: generation.generation_id.clone(),
            path: c.path.clone(),
            heading_path: c.heading_path.clone(),
            byte_start: c.span.byte_start,
            byte_end: c.span.byte_end,
            line_start: c.span.line_start,
            line_end: c.span.line_end,
            sha256: c.sha256.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CitationState {
    Valid,
    Moved { span: SpanRecord },
    Stale,
    Missing,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum VerifyError {
    #[error("citation generation is not active")]
    GenerationUnavailable,
    #[error("citation is not constructible from the active generation")]
    InvalidCitation,
    #[error(transparent)]
    Filesystem(#[from] CorpusError),
}

fn lower_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

fn member(generation: &Generation, citation: &Citation) -> bool {
    let path = Path::new(&citation.path);
    let canonical = path.components().all(|c| matches!(c, Component::Normal(_)))
        && path
            .components()
            .collect::<std::path::PathBuf>()
            .as_os_str()
            == path.as_os_str();
    if citation.path.is_empty()
        || !canonical
        || !lower_hash(&citation.sha256)
        || !lower_hash(&citation.generation_id)
        || citation.byte_start >= citation.byte_end
        || citation.line_start == 0
        || citation.line_start > citation.line_end
        || !generation.files.iter().any(|f| f.path == citation.path)
    {
        return false;
    }
    let Some(doc) = generation
        .documents
        .iter()
        .find(|d| d.path == citation.path)
    else {
        return false;
    };
    if doc
        .text
        .get(citation.byte_start..citation.byte_end)
        .is_none()
    {
        return false;
    }
    generation.chunks.iter().enumerate().any(|(i, c)| {
        c.path == citation.path && Citation::for_chunk(generation, i).as_ref() == Some(citation)
    }) || generation
        .sections
        .iter()
        .any(|s| s.path == citation.path && Citation::for_section(generation, s) == *citation)
}

/// Admit the complete batch before opening any caller-selected document.
pub(crate) fn verify(
    root: &RootedCorpus,
    generation: &Generation,
    citations: &[Citation],
    max_file_bytes: u64,
) -> Result<Vec<CitationState>, VerifyError> {
    verify_with(
        generation,
        citations,
        max_file_bytes,
        &mut VerifyBudget::default(),
        |path, cap| root.read_regular(path, cap).map(|(_, bytes)| bytes),
    )
}

struct VerifyBudget {
    read_bytes: u64,
    hash_bytes: u64,
    hash_calls: u64,
}

impl Default for VerifyBudget {
    fn default() -> Self {
        Self {
            read_bytes: 128 << 20,
            hash_bytes: 128 << 20,
            hash_calls: 1_000_000,
        }
    }
}

impl VerifyBudget {
    fn charge_hash(&mut self, bytes: &[u8]) -> Result<(), CorpusError> {
        let len = u64::try_from(bytes.len())
            .map_err(|_| CorpusError::CaptureBudget("verification SHA bytes"))?;
        let remaining_bytes = self
            .hash_bytes
            .checked_sub(len)
            .ok_or(CorpusError::CaptureBudget("verification SHA bytes"))?;
        let remaining_calls = self
            .hash_calls
            .checked_sub(1)
            .ok_or(CorpusError::CaptureBudget("verification SHA calls"))?;
        self.hash_bytes = remaining_bytes;
        self.hash_calls = remaining_calls;
        Ok(())
    }
}

fn verify_with(
    generation: &Generation,
    citations: &[Citation],
    max_file_bytes: u64,
    budget: &mut VerifyBudget,
    mut read: impl FnMut(&Path, u64) -> Result<Vec<u8>, CorpusError>,
) -> Result<Vec<CitationState>, VerifyError> {
    for citation in citations {
        if citation.generation_id != generation.generation_id {
            return Err(VerifyError::GenerationUnavailable);
        }
        if !member(generation, citation) {
            return Err(VerifyError::InvalidCitation);
        }
    }
    let cap = max_file_bytes.min(2 << 20);
    let reservation = cap
        .checked_add(1)
        .ok_or(CorpusError::CaptureBudget("verification source bytes"))?;
    citations
        .iter()
        .map(|citation| {
            budget.read_bytes = budget
                .read_bytes
                .checked_sub(reservation)
                .ok_or(CorpusError::CaptureBudget("verification source bytes"))?;
            match read(Path::new(&citation.path), cap) {
                Ok(bytes) => {
                    let consumed = u64::try_from(bytes.len())
                        .ok()
                        .and_then(|len| len.checked_add(1))
                        .ok_or(CorpusError::CaptureBudget("verification source bytes"))?;
                    let refund = reservation
                        .checked_sub(consumed)
                        .ok_or(CorpusError::CaptureBudget("verification source bytes"))?;
                    budget.read_bytes += refund;
                    verify_bytes_with(&bytes, citation, |slice| {
                        budget.charge_hash(slice)?;
                        Ok(sha256_hex(slice))
                    })
                }
                Err(CorpusError::NotFound | CorpusError::Symlink | CorpusError::NotRegular) => {
                    Ok(CitationState::Missing)
                }
                Err(e) => Err(e.into()),
            }
        })
        .collect()
}

/// Verifies `citation` against the current file bytes.
pub fn verify_bytes(bytes: &[u8], citation: &Citation) -> CitationState {
    match verify_bytes_with(bytes, citation, |slice| {
        Ok::<_, std::convert::Infallible>(sha256_hex(slice))
    }) {
        Ok(state) => state,
        Err(never) => match never {},
    }
}

fn verify_bytes_with<E>(
    bytes: &[u8],
    citation: &Citation,
    mut hash: impl FnMut(&[u8]) -> Result<String, E>,
) -> Result<CitationState, E> {
    let len = citation.byte_end.saturating_sub(citation.byte_start);
    let mut hash_at = |start: usize| {
        start
            .checked_add(len)
            .and_then(|end| bytes.get(start..end))
            .map(&mut hash)
            .transpose()
    };
    if len > 0 && hash_at(citation.byte_start)?.as_deref() == Some(citation.sha256.as_str()) {
        return Ok(CitationState::Valid);
    }
    if len == 0 || len > bytes.len() {
        return Ok(CitationState::Stale);
    }
    let mut found = None;
    for start in 0..=bytes.len() - len {
        if hash_at(start)?.as_deref() == Some(citation.sha256.as_str()) {
            if found.is_some() {
                return Ok(CitationState::Stale);
            }
            found = Some(start);
        }
    }
    Ok(match found {
        Some(start) => {
            let text = String::from_utf8_lossy(bytes);
            // `start` came from the bounded candidate range, so this sum fits.
            let span = LineIndex::new(&text).span(start..start + len);
            CitationState::Moved {
                span: SpanRecord {
                    byte_start: span.byte_start,
                    byte_end: span.byte_end,
                    line_start: span.line_start,
                    line_end: span.line_end,
                },
            }
        }
        None => CitationState::Stale,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{IndexConfig, build};
    use pretty_assertions::assert_eq;
    use std::fs;

    const CAP: u64 = 1 << 20;

    fn setup() -> (tempfile::TempDir, Generation, Citation) {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("a.md"),
            "# A\n\nFirst.\n\n## B\n\nTarget text.\n",
        )
        .unwrap();
        let generation = build(dir.path(), &IndexConfig::default()).unwrap();
        let index = generation
            .chunks
            .iter()
            .position(|c| c.id == "a.md#a/b~0")
            .unwrap();
        let citation = Citation::for_chunk(&generation, index).unwrap();
        (dir, generation, citation)
    }

    fn state(root: &Path, generation: &Generation, citation: &Citation) -> CitationState {
        verify(
            &RootedCorpus::open(root).unwrap(),
            generation,
            std::slice::from_ref(citation),
            CAP,
        )
        .unwrap()
        .remove(0)
    }

    #[test]
    fn four_citation_states() {
        let (dir, generation, cite) = setup();
        let path = dir.path().join("a.md");
        assert_eq!((cite.line_start, cite.line_end), (5, 7));
        assert_eq!(state(dir.path(), &generation, &cite), CitationState::Valid);

        // Edit above the span: same bytes, shifted by 7 bytes and 2 lines.
        fs::write(&path, "# A\n\nFirst.\n\nExtra\n\n## B\n\nTarget text.\n").unwrap();
        assert_eq!(
            state(dir.path(), &generation, &cite),
            CitationState::Moved {
                span: SpanRecord {
                    byte_start: cite.byte_start + 7,
                    byte_end: cite.byte_end + 7,
                    line_start: 7,
                    line_end: 9,
                }
            }
        );

        // Edit inside the span.
        fs::write(&path, "# A\n\nFirst.\n\n## B\n\nTarget TEXT.\n").unwrap();
        assert_eq!(state(dir.path(), &generation, &cite), CitationState::Stale);

        fs::remove_file(&path).unwrap();
        assert_eq!(
            state(dir.path(), &generation, &cite),
            CitationState::Missing
        );
    }

    #[test]
    fn duplicated_span_is_stale_not_moved() {
        let (dir, generation, cite) = setup();
        let path = dir.path().join("a.md");
        fs::write(
            &path,
            "# A\n\n## B\n\nTarget text.\n\nFirst.\n\n## B\n\nTarget text.\n",
        )
        .unwrap();
        assert_eq!(state(dir.path(), &generation, &cite), CitationState::Stale);
    }

    #[test]
    fn fabricated_paths_refuse_but_replaced_admitted_files_are_missing() {
        let (dir, generation, cite) = setup();
        let real = dir.path().join("a.md");
        std::os::unix::fs::symlink(&real, dir.path().join("link.md")).unwrap();
        let root = RootedCorpus::open(dir.path()).unwrap();
        for path in ["link.md", "../a.md", "/etc/passwd", "./a.md", "a.md/", ""] {
            let mut bad = cite.clone();
            bad.path = path.into();
            assert!(matches!(
                verify(&root, &generation, &[bad], CAP),
                Err(VerifyError::InvalidCitation)
            ));
        }
        fs::remove_file(&real).unwrap();
        std::os::unix::fs::symlink("link.md", &real).unwrap();
        assert_eq!(
            state(dir.path(), &generation, &cite),
            CitationState::Missing
        );
        fs::remove_file(&real).unwrap();
        fs::create_dir(&real).unwrap();
        assert_eq!(
            state(dir.path(), &generation, &cite),
            CitationState::Missing
        );
        fs::remove_dir(&real).unwrap();
        rustix::fs::mknodat(
            rustix::fs::CWD,
            &real,
            rustix::fs::FileType::Fifo,
            rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
            0,
        )
        .unwrap();
        assert_eq!(
            state(dir.path(), &generation, &cite),
            CitationState::Missing
        );
        fs::remove_file(&real).unwrap();
        let _socket = std::os::unix::net::UnixListener::bind(&real).unwrap();
        assert_eq!(
            state(dir.path(), &generation, &cite),
            CitationState::Missing
        );
    }

    #[test]
    fn admitted_non_directory_ancestor_is_missing_not_io() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/a.md"), "# A\n\ntext\n").unwrap();
        let generation = build(dir.path(), &IndexConfig::default()).unwrap();
        let citation = Citation::for_chunk(&generation, 0).unwrap();
        fs::remove_dir_all(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub"), "not a directory").unwrap();
        assert_eq!(
            state(dir.path(), &generation, &citation),
            CitationState::Missing
        );
    }

    #[test]
    fn every_field_and_entire_batch_admit_before_target_reads() {
        let (dir, generation, cite) = setup();
        let root = RootedCorpus::open(dir.path()).unwrap();
        let mut changes = Vec::new();
        let mut c = cite.clone();
        c.path = "unindexed.md".into();
        changes.push(c);
        let mut c = cite.clone();
        c.heading_path.push("forged".into());
        changes.push(c);
        let mut c = cite.clone();
        c.byte_start += 1;
        changes.push(c);
        let mut c = cite.clone();
        c.byte_end -= 1;
        changes.push(c);
        let mut c = cite.clone();
        c.line_start += 1;
        changes.push(c);
        let mut c = cite.clone();
        c.line_end += 1;
        changes.push(c);
        let mut c = cite.clone();
        c.sha256 = "0".repeat(64);
        changes.push(c);
        let mut c = cite.clone();
        c.byte_end = c.byte_start;
        changes.push(c);
        for bad in changes {
            // A zero read cap would fail the first valid citation if any target were opened.
            assert!(matches!(
                verify(&root, &generation, &[cite.clone(), bad], 0),
                Err(VerifyError::InvalidCitation)
            ));
        }
        for id in ["0".repeat(64), "unknown".into()] {
            let mut bad = cite.clone();
            bad.generation_id = id;
            assert!(matches!(
                verify(&root, &generation, &[cite.clone(), bad], 0),
                Err(VerifyError::GenerationUnavailable)
            ));
        }
        assert!(matches!(
            verify(&root, &generation, std::slice::from_ref(&cite), 0),
            Err(VerifyError::Filesystem(CorpusError::TooLarge { .. }))
        ));
        let mut missing_manifest = generation.clone();
        missing_manifest.files.clear();
        assert!(!member(&missing_manifest, &cite));
        let mut missing_document = generation;
        missing_document.documents.clear();
        assert!(!member(&missing_document, &cite));
    }

    #[test]
    fn whole_sections_and_original_policy_membership_are_preserved() {
        let (dir, generation, _) = setup();
        let citation = Citation::for_section(&generation, &generation.sections[0]);
        assert!(citation.byte_end > generation.chunks[0].span.byte_end);
        fs::write(dir.path().join(".ontodocignore"), "a.md\n").unwrap();
        assert_eq!(
            state(dir.path(), &generation, &citation),
            CitationState::Valid
        );
        assert!(!member(
            &generation,
            &Citation {
                byte_end: usize::MAX,
                ..citation
            }
        ));
    }

    #[test]
    fn byte_verification_overflow_is_stale_not_a_panic() {
        let (_, _, mut citation) = setup();
        citation.byte_start = usize::MAX - 1;
        citation.byte_end = usize::MAX;
        assert_eq!(verify_bytes(b"x", &citation), CitationState::Stale);
    }

    #[test]
    fn source_reservation_precedes_reads_and_clamps_the_cap() {
        let (_, generation, citation) = setup();
        let bytes = generation.documents[0].text.as_bytes();
        let cap = bytes.len() as u64;
        for (allowance, expected_reads) in [(cap, 0), (cap + 1, 1)] {
            let mut budget = VerifyBudget {
                read_bytes: allowance,
                ..VerifyBudget::default()
            };
            let mut reads = 0;
            let result = verify_with(
                &generation,
                std::slice::from_ref(&citation),
                cap,
                &mut budget,
                |_, admitted_cap| {
                    reads += 1;
                    assert_eq!(admitted_cap, cap);
                    Ok(bytes.to_vec())
                },
            );
            assert_eq!(reads, expected_reads);
            if expected_reads == 0 {
                assert!(matches!(
                    result,
                    Err(VerifyError::Filesystem(CorpusError::CaptureBudget(_)))
                ));
            } else {
                assert_eq!(result.unwrap(), vec![CitationState::Valid]);
                assert_eq!(budget.read_bytes, 0);
            }
        }
        let mut budget = VerifyBudget::default();
        verify_with(
            &generation,
            &[citation],
            u64::MAX,
            &mut budget,
            |_, admitted_cap| {
                assert_eq!(admitted_cap, 2 << 20);
                Ok(bytes.to_vec())
            },
        )
        .unwrap();
        assert_eq!(budget.read_bytes, (128 << 20) - cap - 1);
    }

    #[test]
    fn shared_read_budget_refunds_only_success_and_refuses_midbatch() {
        let (_, generation, citation) = setup();
        let bytes = generation.documents[0].text.as_bytes();
        let cap = bytes.len() as u64 + 10;
        // One success costs content+1. The residual cannot reserve another cap+1.
        let mut budget = VerifyBudget {
            read_bytes: cap + 1,
            ..VerifyBudget::default()
        };
        let mut reads = 0;
        let result = verify_with(
            &generation,
            &[citation.clone(), citation.clone()],
            cap,
            &mut budget,
            |_, _| {
                reads += 1;
                Ok(bytes.to_vec())
            },
        );
        assert_eq!(reads, 1);
        assert_eq!(budget.read_bytes, 10);
        assert!(matches!(
            result,
            Err(VerifyError::Filesystem(CorpusError::CaptureBudget(_)))
        ));

        for failure in [
            CorpusError::NotFound,
            CorpusError::Symlink,
            CorpusError::NotRegular,
            CorpusError::FilesystemChanged,
        ] {
            let missing = matches!(
                failure,
                CorpusError::NotFound | CorpusError::Symlink | CorpusError::NotRegular
            );
            let mut failure = Some(failure);
            let mut budget = VerifyBudget {
                read_bytes: cap + 1,
                ..VerifyBudget::default()
            };
            let mut reads = 0;
            let result = verify_with(
                &generation,
                &[citation.clone(), citation.clone()],
                cap,
                &mut budget,
                |_, _| {
                    reads += 1;
                    Err(failure.take().unwrap())
                },
            );
            assert_eq!(reads, 1);
            assert_eq!(budget.read_bytes, 0);
            assert!(if missing {
                matches!(
                    result,
                    Err(VerifyError::Filesystem(CorpusError::CaptureBudget(_)))
                )
            } else {
                matches!(
                    result,
                    Err(VerifyError::Filesystem(CorpusError::FilesystemChanged))
                )
            });
        }
    }

    #[test]
    fn repeated_citations_share_sha_bytes_and_calls_without_partial_results() {
        let (_, generation, citation) = setup();
        let bytes = generation.documents[0].text.as_bytes();
        let len = (citation.byte_end - citation.byte_start) as u64;
        for (hash_bytes, hash_calls) in [(len, 2), (2 * len, 1)] {
            let mut budget = VerifyBudget {
                hash_bytes,
                hash_calls,
                ..VerifyBudget::default()
            };
            let mut reads = 0;
            let result = verify_with(
                &generation,
                &[citation.clone(), citation.clone(), citation.clone()],
                CAP,
                &mut budget,
                |_, _| {
                    reads += 1;
                    Ok(bytes.to_vec())
                },
            );
            assert_eq!(reads, 2);
            assert!(matches!(
                result,
                Err(VerifyError::Filesystem(CorpusError::CaptureBudget(_)))
            ));
        }
    }

    #[test]
    fn tiny_windows_charge_before_hash_and_bound_bytes_and_calls_independently() {
        let (_, _, mut citation) = setup();
        citation.byte_start = 0;
        citation.byte_end = 1;
        citation.sha256 = sha256_hex(b"z");
        for (hash_bytes, hash_calls) in [(2, 10), (10, 2)] {
            let mut budget = VerifyBudget {
                hash_bytes,
                hash_calls,
                ..VerifyBudget::default()
            };
            let mut actual_hashes = 0;
            let result = verify_bytes_with(b"abc", &citation, |slice| {
                budget.charge_hash(slice)?;
                actual_hashes += 1;
                Ok::<_, CorpusError>(sha256_hex(slice))
            });
            // Original position and the repeated first candidate both count.
            assert_eq!(actual_hashes, 2);
            assert!(matches!(result, Err(CorpusError::CaptureBudget(_))));
        }
        let mut hashes = 0;
        let result = verify_bytes_with(b"abc", &citation, |_| {
            hashes += 1;
            Err::<String, _>("refused before SHA")
        });
        assert_eq!(hashes, 1);
        assert_eq!(result, Err("refused before SHA"));
    }

    #[test]
    fn original_mismatch_and_relocation_charge_every_candidate() {
        let (_, _, mut citation) = setup();
        citation.byte_start = 0;
        citation.byte_end = 2;
        citation.sha256 = sha256_hex(b"bc");
        let mut budget = VerifyBudget::default();
        let state = verify_bytes_with(b"abc", &citation, |slice| {
            budget.charge_hash(slice)?;
            Ok::<_, CorpusError>(sha256_hex(slice))
        })
        .unwrap();
        assert!(
            matches!(state, CitationState::Moved { span } if span.byte_start == 1 && span.byte_end == 3)
        );
        // Original "ab", candidate "ab", candidate "bc": 3 calls, 6 bytes.
        assert_eq!(budget.hash_calls, 1_000_000 - 3);
        assert_eq!(budget.hash_bytes, (128 << 20) - 6);
    }

    #[test]
    fn checked_offsets_do_not_hash_invalid_spans() {
        let (_, _, mut citation) = setup();
        for (start, end) in [(usize::MAX - 1, usize::MAX), (usize::MAX, 0)] {
            citation.byte_start = start;
            citation.byte_end = end;
            let mut hashes = 0;
            let state = verify_bytes_with(b"", &citation, |_| {
                hashes += 1;
                Ok::<_, std::convert::Infallible>(String::new())
            })
            .unwrap();
            assert_eq!(hashes, 0);
            assert_eq!(state, CitationState::Stale);
        }
    }
}
