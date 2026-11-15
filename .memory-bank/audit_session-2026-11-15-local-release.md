# Local 0.1.0 Install (REL-6 Closeout)

Date: 2026-11-15

## Decision

The owner was asked seven questions with proposed answers. The reply was "make only local install / approved". That approves the proposed answers and replaces publication with a local install.

1. Release the corrected candidate as 0.1.0. Evaluation results describe the original measured binary; the bundled `EVALUATION_0.1.0.md` says so.
2. and 3. Do not publish. No GitHub repository, tag, release or upload; no package or history exposure.
4. Keep the recorded home build paths: 317 `/home/` byte-prefix occurrences, all Cargo registry and Rust toolchain paths. Use `--remap-path-prefix` from 0.1.1 onwards.
5. Unreleased 0.1.1 work stays out of 0.1.0.
6. One local commit, no push.
7. No tokenizer experiment now.

## Evidence

- `sha256sum -c SHA256SUMS` in `tmp/release-010/corrected/`: both archives OK. The binary archive SHA-256 is `daccd668…4b64725`; the manifest records `published: false` and binary SHA-256 `2fc26f15…69c2a3`.
- The archive was extracted to a unique temporary directory under `~/.local/opt/`. Its internal `SHA256SUMS` check passed. It was then moved to `~/.local/opt/ontodoc-0.1.0/`, a path that did not exist before. The installed binary SHA-256 is `2fc26f1527ce59842e135ef47fd80d950541bd6389fa360c43f987dd8684c2a3`.
- Symlink `~/.local/bin/ontodoc-0.1.0`: `--version` prints `ontodoc 0.1.0`.
- Scratch folder smoke: `index` exit 0, committed. `search "restore backup"` exit 0, fresh, top citation `guide.md`. Stdio `tools/list` returned the five tools `read`, `related`, `search`, `status` and `verify`. The scratch folder was removed.
- `~/.local/bin/ontodoc` stays the unreleased 0.1.1 build with `--auto-index` used by the ontocode MCP entry. The two binaries coexist under different names.
- Local commit `116967a` holds the previously uncommitted completed-plan work, with no tag or push. `.gitignore` now ignores manager `*.integrate.lock` files. No commit hooks are configured.

## Limits

- No public release exists, and nothing was uploaded or tagged.
- The installed 0.1.0 binary is the corrected candidate and was not itself measured by the held-out evaluation.
- No Rust build or test was rerun for this closeout; the package's own validation is in `audit_session-2026-11-13-corrected-candidate.md`.
