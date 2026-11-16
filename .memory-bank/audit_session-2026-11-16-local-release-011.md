# Local 0.1.1 Release

Date: 2026-11-16
Authority: owner instruction "push version / prepare to release and commit / make binary / make local install" on 2026-11-16. As with 0.1.0, the release stays local: nothing was pushed, published or uploaded.

## Release Identity

- Release commit `af6efcc` ("Release 0.1.1 (local)"), annotated tag `v0.1.1`, both local. The commit dates the CHANGELOG 0.1.1 entry. It sits on `83caad9` (ADR revision 21) and the 0.1.1 work before it.
- The binary was built from a clean detached worktree at `v0.1.1` (`/opt/d2/ontodoc-release-011`, zero `git status` entries). Another session's uncommitted dense-retrieval work in the main tree (`src/search/dense.rs`, `pub mod dense`, hybrid ADR and plan) is not in the release.
- Build: `cargo build --release --locked` with Rust 1.95.0 and `--remap-path-prefix` for the Cargo registry, rustup and the build directory, as decision 4 of the 0.1.0 install required from 0.1.1 on. `grep -c /home/` on the binary prints 0; the corrected 0.1.0 binary had 317.
- Binary SHA-256 `02c01170670778a4e04b58a9260cf1ceb110e360ce5f8666059085dafcf32682`. Archive `ontodoc-0.1.1-linux-x86_64-gnu.tar.gz`, SHA-256 `76b262701df233f1ca6c8e6165638f05080fda1da9570967a796402bfe166dfe`, in `/opt/d2/ontodoc-release-011/pkg/` with a `.sha256` file.
- The package has the 0.1.0 layout: `bin/ontodoc`, README, CHANGELOG, licences, NOTICE, THIRD_PARTY_LICENSES, DEPLOYMENT, EVALUATION_0.1.0, a `source-manifest.json` with commit, tag and `published: false`, and `SHA256SUMS`.

## Validation On The Tagged Tree

| Command | Result |
| --- | --- |
| `cargo test --all-targets --locked` | exit 0; lib 242 passed, 4 ignored; every integration target passed |
| `cargo fmt --check` | exit 0 |
| Clippy lane `ontodoc-rel011-clippy-20261116`, `cargo clippy --all-targets --locked -- -D warnings` | exit_code 0, state terminal (2,528 s elapsed, mostly queued behind another session's OntoIndex test run) |
| Earlier lane `ontodoc-sr1-clippy-20261116` on `83caad9` | exit_code 0, state terminal. `git diff --stat 83caad9 v0.1.1` shows only `CHANGELOG.md`, so the tagged Rust source is identical. The other session's `src/search/dense.rs` was created at 10:17, after that lane ran at 02:12 |

## Install

- Extracted to a temporary directory under `~/.local/opt/`. `sha256sum -c SHA256SUMS` passed. Moved to `~/.local/opt/ontodoc-0.1.1/`, a path that did not exist before.
- `~/.local/bin/ontodoc-0.1.1` links to it. `~/.local/bin/ontodoc`, which the ontocode MCP entry runs, now links to it as well. The previous binary (SHA-256 prefix `518c3056`) is kept as `~/.local/bin/ontodoc.bak-20261116-pre-0.1.1`. `ontodoc-0.1.0` is unchanged.
- Smoke on scratch folders, which were then removed: `index` exit 0; `search "restore backup kubernetes"` was fresh, with top citation `guide.md` and `unmatched_terms: ["kubernetes"]`; stdio `serve --auto-index` built the index on the first `search` call and answered with `fresh: true`.

## Limits

- No public release, push, upload or remote tag exists.
- The tarball is not byte-reproducible across rebuilds without a pinned toolchain, and that was not checked.
- Running ontocode sessions keep the old server process until their MCP server restarts.
- There is no new retrieval-quality claim. The bundled `EVALUATION_0.1.0.md` describes the measured 0.1.0 binary.
