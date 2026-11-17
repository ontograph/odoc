# Public OntoDoc Distribution

Date: 2026-11-17
Status: In progress
Authority: User explicitly selected https://github.com/ontograph/odoc for the codebase and release and requested a README and GitHub installation shell script.

## Goal

Publish the codebase and version 0.1.2 at the selected public repository, with documented installation that verifies release checksums.

## Manager Tracking

```yaml
manager_loop:
  authority: true
  status: in-progress
  selection_policy: active_next_task-first
  active_next_task: PUB-1
  max_in_flight: 1
  dispatch_mode: none
tasks:
  PUB-1:
    title: Public main, release and checksum-verified installer work without GitHub authentication
    status: IMPLEMENTED_PENDING_VALIDATION
    classification: architecture-paperwork
    type: closeout
    validation_tier: integration
    depends_on: []
    owner_files:
      - install.sh
      - tests/test_install.py
      - README.md
      - CHANGELOG.md
      - docs/PUBLIC_RELEASE_PLAN.md
      - .memory-bank/MEMORY.md
      - .memory-bank/project_plan-current.md
      - .memory-bank/project_pending-tasks.md
      - .memory-bank/project_architecture.md
      - .memory-bank/audit_session-2026-11-17-public-release.md
    allowed_write_set:
      - install.sh
      - tests/test_install.py
      - README.md
      - CHANGELOG.md
      - docs/PUBLIC_RELEASE_PLAN.md
      - .memory-bank/MEMORY.md
      - .memory-bank/project_plan-current.md
      - .memory-bank/project_pending-tasks.md
      - .memory-bank/project_architecture.md
      - .memory-bank/audit_session-2026-11-17-public-release.md
    non_goals:
      - No runtime, Cargo version, ranking, licence, donor snapshot or evaluation changes.
      - No deletion or rewriting of earlier release commits, tags, repositories or custody.
    required_validation:
      - sh -n install.sh
      - PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p test_install.py
      - git diff --check
    expected_evidence:
      - Installer handles a fresh install, same-version reinstall, backup on upgrade and checksum/download refusal without replacing an existing binary.
      - Public main preserves both histories and existing MIT package licence; v0.1.2 retains its original build commit and binary hash.
      - Anonymous GitHub download and installation work in a temporary home, with CLI and MCP smoke, matching release/repository installer and complete asset checksums.
    rollback:
      - Preserve prior remote as previous and prior package/release assets; use ordinary revert for new source edits.
      - Restore any installer-created backup if a user elects to roll back locally.
    stop_conditions:
      - Destination changes concurrently, source/tag identities differ, secret scan has unexplained credential matches, or required checks fail.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
```

## Required Traceability

| Requirement ID | ADR/user requirement | Disposition | Task IDs | Validation |
| --- | --- | --- | --- | --- |
| PUB-CODE | Link codebase to ontograph/odoc | implementation | PUB-1 | origin URL, public main and remote commit equality |
| PUB-RELEASE | Publish release at selected URL | implementation | PUB-1 | Original v0.1.2 tag, asset hashes and anonymous download |
| PUB-INSTALL | README and installation shell script | implementation | PUB-1 | Shell syntax, installer regressions and live scratch installation |

## Decisions And Non-Goals

The selected repository is public and its only initial file is an AGPL licence template. It contains no software. Preserve that initial commit by merging histories, retain the established MIT package licence at the merge, and do not adopt or relicense any code. Keep the old private remote as `previous`; make `origin` the requested URL. Use remote `main` without force pushing. The existing `v0.1.2` build tag and binary remain unchanged; package documentation may be refreshed with an explicit packaging commit and new archive hash. Prior packages remain immutable. The user's explicit public codebase destination resolves the earlier public-exposure decision.

## Closeout

Pending execution evidence.
