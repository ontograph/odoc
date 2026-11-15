# Donor review evidence

Date: 2026-11-14
Scope: Completed donor plan, ADR18 response behavior and census evidence.

## Review and decision

GPT Luna runtime review found no concrete blocker after inspecting current search, shared response fitting, related omission accounting and CLI/MCP/HTTP tests. Receipt: `agent-results/01a1d904-d346-7c33-b889-aa80918475ea/1`. OntoIndex navigation reported dirty source omissions; current source was authoritative. No tests or builds were run by that reviewer.

Strengths: citations survive text pressure, fit decisions use actual serialization, omission counts are explicit, and regression fixtures exercise large hits and citation-only overflow. Weaknesses: contradictory closure prose, a speculative error prediction retained after its baseline was disproved, and evidence stored only in ignored temporary files. The regex scanner duplicates Markdown parsing and failed calibration. Extending that approximation would add maintenance without trustworthy edge counts.

The selected follow-up corrects those records and embeds the exact existing result below. It adds no runtime complexity. Production-parser reuse is the preferred direction only if a later approved link study needs calibrated counts. Incremental byte accounting and graph caching remain unjustified without synthetic profiling. No performance improvement is claimed. Historical MCP wire identity remains unproven; historical Rust validation is not a current-tree green claim.

GPT Luna evidence review independently confirmed stale closure and summary prose, misleading calibration wording, and agreement between census and runtime edge order. Receipt: `agent-results/01a1d904-efdc-7dc0-b065-146c8355dff2/1`. Those documentation findings were corrected without reopening completed tasks.

Validation: both strict plan validators passed; `git diff --check` and the inline Python exact-JSON/hash/reference checks exited 0. One initial plan validation rejected an underspecified complexity field; correcting it to the required `complex` passed. Rust checks were skipped because no Rust changed.

## Evidence custody and limits

`sha256sum -c SHA256SUMS` in `tmp/donor-candidates/census/` exited 0 for both files in this review. Script SHA-256: `d6f8f4f1dc93617eeefff292e83c7728ab3a5476b29c9bf4446cf61ff7185c79`. Result SHA-256: `bf59439dbf1af14610118061273b55b1e45fb491ace29c96edba60d8f86ce042`.

The JSON block preserves the result bytes, including the final newline. It is a historical measurement, not a fresh corpus check. The ignored script is not archived here; reproducing the original run still requires that local script, its binary and matching stores. Failed calibration and unconfirmed preparation identity remain explicit. No evaluation payload was accessed or rerun.

## Retained census result

```json
{
  "corpora": [
    {
      "approved_preparation_match": "unconfirmed; available preparation pin differs from selected approved pin",
      "chunks_above_bytes": {
        "131072": 0,
        "32768": 0,
        "65536": 0
      },
      "fresh": true,
      "generation_id": "375917fc2e8cd756e785fdc118e6f5d12f4967c69963f89469de1b010d38ca95",
      "index_sha256": "b88c3f33aec4547ecc252a8cb7d255e8f943010cb70c13f3832cca29b7aca7ce",
      "largest_chunk": {
        "bytes": 10269,
        "id": "appendix-02-operators.md#appendix-b-operators-and-symbols/operators~1"
      },
      "link_diagnostic_count": 17,
      "link_scan": {
        "additional_unique_edges": null,
        "approximate_ignored_occurrences": {
          "html_without_indexed_md": 1
        },
        "calibrated": false,
        "outcome": "uncalibrated; no edge-gain claim",
        "scanned_diagnostics": {},
        "stored_diagnostics": {
          "MissingAnchor": 17
        }
      },
      "meta_sha256": "d6ae064623ad8d72a5772621fa576944dabb1f46d8a1b88a597ff0beb1cfdda0",
      "neighbour_cuts": [
        {
          "by_kind": {
            "CONTAINS": 1,
            "LINKS_TO": 108,
            "MENTIONS": 13,
            "NEXT": 1
          },
          "chunk_id": "SUMMARY.md#the-rust-programming-language~1",
          "omitted_by_kind": {
            "LINKS_TO": 10,
            "MENTIONS": 13
          },
          "total": 123
        },
        {
          "by_kind": {
            "CONTAINS": 1,
            "MENTIONS": 129,
            "NEXT": 2
          },
          "chunk_id": "appendix-02-operators.md#appendix-b-operators-and-symbols/operators~1",
          "omitted_by_kind": {
            "MENTIONS": 32
          },
          "total": 132
        }
      ],
      "root": "tmp/corpora/rust-book/src",
      "schema_version": 4
    },
    {
      "approved_preparation_match": "unconfirmed; available preparation pin differs from selected approved pin",
      "chunks_above_bytes": {
        "131072": 0,
        "32768": 1,
        "65536": 0
      },
      "fresh": true,
      "generation_id": "4e546b16f9fd4533f597a435487d1ba7175070d33992138f4c4a49889464446b",
      "index_sha256": "a5c77c58c7657a11f646983d48284bf2cf75babac0a155d3843c50d533d788a3",
      "largest_chunk": {
        "bytes": 33444,
        "id": "IMPLEMENTATION_PLAN_GLM_5_2_MAX_ONTOINDEX_MCP.md#glm-52-max-ontoindex-mcp-compatibility-project-plan/manager-tracking~1"
      },
      "link_diagnostic_count": 10,
      "link_scan": {
        "additional_unique_edges": null,
        "approximate_ignored_occurrences": {
          "directory": 52,
          "no_extension": 12,
          "other_extension": 1
        },
        "calibrated": false,
        "outcome": "uncalibrated; no edge-gain claim",
        "scanned_diagnostics": {
          "MissingTarget": 9
        },
        "stored_diagnostics": {
          "MissingTarget": 10
        }
      },
      "meta_sha256": "6fae2017dcffe2c66e69a38835fe9a7a7997fedb1913765c9f6ec050c1a7ccbd",
      "neighbour_cuts": [],
      "root": "tmp/corpora/ontoindex",
      "schema_version": 4
    }
  ],
  "limits": "Benchmark corpora only. Inline-link scanner approximates parser. No ranking or quality claim.",
  "script_sha256": "d6f8f4f1dc93617eeefff292e83c7728ab3a5476b29c9bf4446cf61ff7185c79"
}
```
