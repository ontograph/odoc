# OntoDoc 0.1.2 Release

Date: 2026-11-17
Status: Closed
Authority: User instruction to bump version, prepare release, build binary,
commit, install locally, create GitHub release and push. Service ADR revision 21.

## Goal

Install a verified 0.1.2 binary locally and publish the matching commit, annotated
tag, binary archive and checksums to a private GitHub repository.

## Manager Tracking

```yaml
manager_loop:
  authority: true
  status: closed
  selection_policy: active_next_task-first
  active_next_task: null
  max_in_flight: 1
  dispatch_mode: none
  reopen_gate: null
tasks:
  REL-012:
    title: Version 0.1.2 is built, installed and available as a private GitHub release
    status: DONE
    classification: architecture-paperwork
    type: closeout
    validation_tier: release
    depends_on: []
    owner_files:
    - Cargo.toml
    - Cargo.lock
    - CHANGELOG.md
    - README.md
    - docs/RELEASE_0.1.2_PLAN.md
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    - .memory-bank/audit_session-2026-11-17-release-012.md
    allowed_write_set:
    - Cargo.toml
    - Cargo.lock
    - CHANGELOG.md
    - README.md
    - docs/RELEASE_0.1.2_PLAN.md
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    - .memory-bank/audit_session-2026-11-17-release-012.md
    non_goals:
    - No runtime code, ranking, schema, evaluation, donor or historical artifact changes.
    - No public repository exposure or crates.io publication.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test --lib search::dense --locked
    - CARGO_BUILD_JOBS=4 cargo test --all-targets --locked
    - cargo fmt --check
    - just --justfile /opt/d1/_wrk/ontocode/justfile compiler-lane-detached ontodoc-rel012-clippy-20261117 bash -lc 'cd /opt/d1/_wrk/ontodoc && CARGO_BUILD_JOBS=4 cargo clippy --all-targets --locked -- -D warnings'
    - CARGO_BUILD_JOBS=4 cargo build --release --locked
    - PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p test_verify_release.py
    - git diff --check
    expected_evidence:
    - Exact commands and terminal status, clean release commit and annotated tag.
    - Binary and archive hashes, complete checksums, no home build paths, extracted CLI and five-tool stdio smoke.
    - Versioned local install and default symlink, private remote commit/tag equality, downloaded release hash and smoke.
    rollback:
    - Restore the previous local binary symlink; preserve all previous versions and artifacts.
    - Before publication, revert only this release's changes; never rewrite existing history or tags.
    stop_conditions:
    - Validation fails or the candidate changes during validation.
    - Destination already exists with unrelated history or is public.
    - An asset has unexplained private build paths or fails checksums.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User request 2026-11-17 to bump, build, commit, install, release and push.
        validation_summary: All required commands exit 0; 247 library and 73 integration tests, 5 focused tests and 22 verifier tests pass; 4 ignored helpers. Clean tagged release 8e94a3d, remapped binary and repeatable package hashes, internal and external checksums, local install and downloaded five-tool smoke pass. Private repository, remote commit/tag identities, uploaded assets and owned staging cleanup verified. See .memory-bank/audit_session-2026-11-17-release-012.md and tmp/release-012/artifacts/validation.json.
    dod_status: pass
```

## Required Traceability

| Requirement ID | ADR/user requirement | Disposition | Task IDs | Validation |
| --- | --- | --- | --- | --- |
| REL-VERSION | Bump version, build and commit | implementation | REL-012 | Locked build, version smoke, commit and annotated tag |
| REL-INSTALL | Make local install | implementation | REL-012 | Checksums, installed version and scratch corpus smoke |
| REL-GITHUB | GitHub release and push | implementation | REL-012 | Private repository, remote identities, release download and smoke |
| REL-PRESERVE | Preserve evaluation custody and earlier releases | non-goal | REL-012 | Declared write set and diff inspection |

## Decisions And Non-Goals

No remote was configured and neither `er77/ontodoc` nor `ontograph/ontodoc`
existed. The authenticated account is `er77`. Use private `er77/ontodoc` as the
conservative destination for the authorized push and release; public exposure
requires a separate owner decision. Push the existing master history and the
new tag only. Historical local releases and frozen evaluation artifacts remain
unchanged. Follow the root standalone Cargo commands; the read-only donor
snapshot is never built, cleaned or edited. Remap build paths from 0.1.1 onward.

## Closeout

Release commit `8e94a3de70fc1249e15e30518bc78f7b356df462`, annotated tag
`v0.1.2`, [GitHub release](https://github.com/er77/ontodoc/releases/tag/v0.1.2).
The default local binary runs 0.1.2; the 0.1.1 installation remains available.
All required validation and downloaded package smoke passed. Exact commands,
hashes, source identity and limits are in the
[release audit](../.memory-bank/audit_session-2026-11-17-release-012.md).
Public exposure, independent compiler reproducibility and production ingress
validation remain outside this release. No runtime owner changed.
