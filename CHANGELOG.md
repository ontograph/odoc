# Changelog

## 0.1.1 — unreleased

MCP tool results whose text and `structuredContent` copies would together exceed the 128 KiB frame now omit `structuredContent` and send the JSON once, as text, before withholding any search text (ADR revision 21). On the agent test corpus, two `top_k: 50` searches that withheld 22 and 24 texts now return all 50. Smaller results are unchanged. Clients must fall back to the text block when `structuredContent` is absent. Search responses add `unmatched_terms`, listing query terms found in no indexed chunk of the corpus; it applies no threshold and changes no ranking. No schema, default or quality claim changes.

`ontodoc serve --auto-index` (stdio only, off by default, ADR revision 20) builds the index on the first call when none exists and rebuilds it when a call finds it stale; the call waits. A failed rebuild answers from the previous generation with `fresh: false` and is not retried for 30 seconds. Corrupt generations are never rebuilt. `serve-http` rejects the flag. Without it, behaviour is unchanged. No tool, field, schema, ranking or quality claim changes.

The MCP `search` description and server instructions now state that matching is on exact lowercase words without stemming or stop-word removal, that `scope` is a path-prefix filter, and that `abstention_score` is always returned while abstention needs a caller `threshold` with no validated cutoff. README documents the same, plus link diagnostics. Text only; no ranking, defaults, schema or quality claim changes.

Publication and every generation load now reject malformed edge kinds/endpoints, cross-document `NEXT` edges and duplicate document paths with fixed corruption reasons. Valid schema-4 artifacts and generation IDs stay unchanged. A refused active corpus requires the tested directory-removal and trusted reindex procedure in README; the same recovery covers duplicate IDs. No retrieval-quality claim follows.

Search preserves citations when budget or response bounds withhold text: `tier: null`, empty text and `text_truncated: false`. Text fitting follows rank order and continues past oversized texts. CLI and MCP both withhold selected text above 64 KiB. Only whole-result omission sets search `truncated`; clients must accept nullable tiers. `read` may return a truncated prefix. `related.omitted` counts edges excluded by item or byte caps. No schema, ranking or quality claim changes.

Neighbour lookup keeps unresolved code targets distinct from chunks with matching ID text, preventing false incoming `MENTIONS` on legal `code:` filenames. Malformed outgoing mention targets fail with `corrupt`. Boundary regressions cover the exact first 100 neighbours and deterministic truncation.

`related` returns citations for chunk neighbours (`NEXT`, `LINKS_TO`) and parent sections (`CONTAINS`). Unresolved code mentions (`MENTIONS`) omit the citation field. Missing or wrong-kind endpoints fail with a typed `corrupt` error. `verify` checks cited endpoint bytes; it does not certify graph relations.

Requests, edge order, the 100-neighbour cap and complete-response 128 KiB limits remain unchanged. Larger citation-bearing responses can truncate sooner, with `truncated: true`; there is no pagination. Stdio and HTTP share the response contract. No schema, ranking or defaults change and no reindex is required. This development version leaves both preserved 0.1.0 packages and evaluation custody untouched; it makes no new retrieval-quality claim.

## 0.1.0 corrected candidate — local preparation

Separate source and binary identities preserve the measured candidate below. HTTP response overflow now returns the same bounded typed error and request ID as stdio; an ID unable to fit the error is refused before tool dispatch. Response sizing uses a fixed-storage writer while preserving serialization errors and truncation order. Graph construction filters relevant events before binary lookup on validated chunk spans; heading ancestry is cached per section. Arbitrary public input compatibility and canonical artifact bytes have focused regression coverage.

Publication locks release explicitly at owner scope exit, preserving fail-fast contention and maintenance coverage when a duplicate descriptor survives.

No ranking, defaults or schema changed. Evaluation results below belong to the original measured binary; no held-out rerun or new retrieval-quality claim is made. The corrected package includes the complete evaluation companion. Publication remains pending destination, exposure and build-path review.

## 0.1.0 — prepared 2026-11-13, not published

Experimental standalone documentation retrieval service with five read-only MCP tools: search, read, related, verify and status. Supports stdio and loopback HTTP, separate stores per immediate docs-share folder, trusted CLI indexing, schema-4 deterministic generations, rooted reads, bounded requests and explicit freshness/citation checks.

Linux x86_64 GNU package requires glibc 2.39 or newer, Linux kernel 5.6 or newer (`openat2`), and libgcc_s. TLS and authentication belong to the operator's ingress. The deployment guide is an untested example. Capacity measurements exist without a latency acceptance target.

The once-only registered evaluation used model-judged labels: 165 answerable and 55 off-topic model-drafted queries. H1 sanity passed against an empty B0 exact-substring baseline. H2 graph alpha 0.5 versus lexical: mean nDCG@10 difference +0.032912, bootstrap 95% CI [0.016128, 0.049996]. H5 hierarchy beta 0.3: +0.032709 [0.012113, 0.053406]. These are exploratory effects, with no Holm decision or default promotion. Lexical remains default; optional fusion weights remain 0.3 and abstention remains off.

H3 at the dev-selected threshold: correct abstention 42/55, 76.36% (Wilson 95% CI [63.65%, 85.63%]); false abstention 13/165, 7.88% ([4.66%, 13.01%]). H4 has deterministic fixture coverage, not exhaustive benchmark citation verification. E1 includes intervals crossing zero; the full audit retains every contrast. Twelve unanswered queries were withdrawn before measurement, raising absolute scores and changing split membership. Held-out miss attribution is unavailable by owner-approved exception. No human-label confirmatory claim is established.

Measured binary SHA-256: `af5fd69e216cb8a8e81ba495588f50d8274725388d27d61d1050fef3a2b83b55`; candidate commit `cc116c44a4d7f854bc19c6a4b40d75098a437268`; accepted report SHA-256 `006af6e5fcae05a9693edcc254adf7381d5622dbeae211264578f32d1a9691e0`. Package retains the binary unchanged, including embedded build-directory paths. MIT covers the standalone package; dependency and historical snapshot notices retain their own licence scopes. The donor snapshot and benchmark payloads are excluded from the installation archive.

Publication destination and repository/history exposure remain undecided. No release tag or upload has occurred.
