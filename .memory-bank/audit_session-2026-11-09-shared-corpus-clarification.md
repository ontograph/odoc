# Shared-Corpus HTTP Clarification

Date: 2026-11-09
Authority: user clarification, "Everyone can read the same corpus."
Scope: narrows the [HTTP challenge](audit_session-2026-11-09-http-service-challenge.md).
This records a requirement and recommendation, not approval to implement HTTP.

## Selected Access Model

One corpus, with equal read access for all service users. No tenant routing,
per-document ACLs, per-user indexes or authorization-aware ranking are needed
for this deployment. Global graph traversal and corpus-wide ranking statistics
are appropriate here; the earlier mixed-ACL concerns do not block this model.
Keep `scope` as a user-selected search filter, not a security boundary.

Equal document access does not establish anonymous Internet access. Service
admission/authentication remains a deployment decision. Users also do not gain
access to one another's queries, sessions, logs, arbitrary server files or index
publication controls merely because the documents are shared.

## Narrowed Recommendation

Start with one Rust process, one configured corpus root and one shared immutable
index generation. Clone the existing server into rmcp Streamable HTTP sessions;
do not create an index per connection or serialize all searches behind a mutex.
Retain stdio and the existing five read-only tools. Indexing stays in a separate
trusted CLI/control-plane operation with atomic generation publication.

Do not add a tenant database, document ACL engine, distributed index or per-user
storage. Additional replicas are a measured availability/capacity option, not a
prerequisite. Preserve request privacy even if the documents are public.

The previous challenge's non-tenancy findings still apply: per-request corpus
rehashing, citation-verification cost and file boundaries, input/work/wire-size
caps, admission/cancellation, reload behavior and multi-call generation
consistency. Verification must remain limited to the intended corpus, not every
file beneath a developer checkout. Authentication alone would not fix these.

## Authority And Evidence

The accepted ADR still specifies stdio. An HTTP amendment and scoped tasks are
required before implementation; this clarification does not reorder or complete
EVAL-1.3 and does not change retrieval-quality hypotheses. HTTP capacity remains
unmeasured; no user-count or throughput guarantee follows from equal access.

The previous HTTP review's 51 protected implementation/authority/input files
match their recorded hashes (`1250f8`, `python3`, exit **0**); the plan, ADR and
agent-rule SHA-256 check also exited **0** (`74fbbc`). Only the new user
requirement and its consequences were reviewed. No agents, builds, tests,
network calls, load tests or evaluation runs were started for this delta.
