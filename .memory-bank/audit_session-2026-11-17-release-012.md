# OntoDoc 0.1.2 Release

Date: 2026-11-17
Authority: User instruction "push version / prepare to release / make binary /
commit / make local install / make github release / push github".

## Decisions

- Bump the current clean master (`701918f`) from 0.1.1 to 0.1.2.
- No remote exists. GitHub is authenticated as `er77`; queries for
  `er77/ontodoc` and `ontograph/ontodoc` found neither repository. Create private
  `er77/ontodoc` and push master with the new annotated tag. Private visibility
  preserves the prior unresolved public history-exposure decision.
- Reuse the 0.1.1 binary package layout and versioned local installation;
  preserve prior releases and the existing 0.1.1 installation for rollback.
- Remap build directory, Cargo registry and toolchain paths. Build only the
  standalone root package. Never build or clean the read-only donor snapshot.
- No runtime edit, held-out evaluation, default change or new quality claim.
  Dense/RRF helpers already in master remain disconnected from CLI/MCP ranking.
  The bundled evaluation companion refers to the original measured 0.1.0 binary.

## Evidence

The historical package verifier remains
pinned to its original 0.1.0 identities; its synthetic tests can run for 0.1.2,
but its real-package default must not be repointed.

The first plan validation failed on missing owner paths and release-tier/lane
metadata. The corrected plan validation passed with one task and no errors.
OntoIndex's current committed snapshot is `701918f`; impact on the CLI `App`
owner reports LOW, with no direct incoming edges. No runtime symbol is edited.

| Command | Terminal result |
| --- | --- |
| `CARGO_BUILD_JOBS=4 cargo test --lib search::dense --locked` | exit 0; 5 passed |
| `CARGO_BUILD_JOBS=4 cargo test --all-targets --locked` | exit 0; 247 library and 73 integration tests passed; 4 ignored helpers |
| `cargo fmt --check` | exit 0 |
| `CARGO_BUILD_JOBS=4 cargo clippy --all-targets --locked -- -D warnings` | lane `ontodoc-rel012-clippy-20261117`, terminal exit 0; 5.614 s |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p test_verify_release.py` | exit 0; 22 passed |
| `git diff --check` | exit 0 |
| `CARGO_BUILD_JOBS=4 cargo build --release --locked` with `RUSTFLAGS` below | exit 0; release binary reports `ontodoc 0.1.2` |

Clippy ran through `just --justfile /opt/d1/_wrk/ontocode/justfile
compiler-lane-detached ontodoc-rel012-clippy-20261117 bash -lc 'cd
/opt/d1/_wrk/ontodoc && CARGO_BUILD_JOBS=4 cargo clippy --all-targets --locked --
-D warnings'`. `lane-result ... --wait 60` read its authenticated artifact
metadata and terminal result. The receipt lives under
`$ONTOCODE_HOME/artifacts/compiler-lane/ontodoc-rel012-clippy-20261117/`.
Test/build logs are under ignored `tmp/release-012/`.

`gh repo create er77/ontodoc --private --source . --remote origin` completed
with exit 0. Only an empty private repository exists at this preparation step;
commit/tag, installation, push and release evidence follows.

Release `RUSTFLAGS`: `--remap-path-prefix=$HOME/.cargo=/cargo
--remap-path-prefix=$HOME/.rustup=/rustup
--remap-path-prefix=/opt/d1/_wrk/ontodoc=/src/ontodoc
--remap-path-prefix=/home/er77/_wrk/ontodoc=/src/ontodoc`.
Both spellings of the checkout are remapped because the shell's canonical
working directory differs from the workspace alias. A byte scan requires zero
`/home/`, `/opt/d1/` and `/opt/d2/` prefixes in the built binary.

OntoIndex `verify_diff` passed the declared nine-file scope before commit;
symbol scope was not checked because no expected runtime symbols were supplied.
No runtime symbol changed. The package does not include the donor snapshot,
benchmark payloads or internal plans. Source history will remain private.

## Candidate And Local Installation

- Release commit `8e94a3de70fc1249e15e30518bc78f7b356df462`, annotated tag
  `v0.1.2`; the tagged tree was clean. The locked release build was repeated
  on that clean tree (exit 0, 0.07 s), retaining the same binary bytes.
- Binary SHA-256 `6487f1ca1622b41298ad98adff2bb9e8a2a76401b234c9f058d2ce97f6d61b67`;
  10,687,464 bytes. The forbidden build-path prefix scan passed.
- Archive `ontodoc-0.1.2-linux-x86_64-gnu.tar.gz`, SHA-256
  `77fb834c947cc996b3e5fb271c34f1ea5b1fab12ca9c992c9cf9bbd077f33a20`;
  3,702,624 bytes. USTAR members are regular files, sorted, fixed modes/owners
  and zero timestamps; gzip timestamp is zero. Two assemblies compared equal.
  Reproducible compilation across independent builds was not tested.
- Eleven package files: binary, README, CHANGELOG, LICENSE, LICENSE-APACHE,
  NOTICE, THIRD_PARTY_LICENSES, DEPLOYMENT, EVALUATION_0.1.0, source manifest
  and internal checksums. Each source-inventory hash matches its tagged Git
  blob. Every shipped file is covered by internal checksums; the external
  `SHA256SUMS` covers the archive. Package hash/coverage checks passed.
- The asset scan found one `/home/` occurrence in README: the synthetic
  `/home/me/docs/x.md` path example explaining `MissingTarget`. It contains no
  machine identity. Other shipped documentation and the binary had no private
  home/workspace prefixes. Credential-pattern scans found no matches.
- Scratch corpus CLI index/search passed. Extracted, installed and auto-index
  binaries each passed all five stdio tools, protocol-only stdout, current
  citation verification, lexical mode and `unmatched_terms` checks. These
  are smoke tests, not retrieval-quality measurements.
- Installed to `~/.local/opt/ontodoc-0.1.2`; default `~/.local/bin/ontodoc` and
  versioned `ontodoc-0.1.2` point to its binary. Internal checksum verification
  passed after installation. `ontodoc-pre-0.1.2` preserves the previous default
  symlink to the untouched 0.1.1 installation. `ontodoc --version` is 0.1.2.
- `git push --atomic -u origin master refs/tags/v0.1.2` completed successfully;
  remote master and the peeled annotated tag both match the release commit.

One-run driver and receipts are ignored artifacts in `tmp/release-012/`:
`run_release.py`, `release.log`, `artifacts/validation.json`,
`artifacts/source-manifest.json`, archive, checksums and release notes. The
driver owns a fresh marked temporary directory with validated cleanup on all
terminal paths. Existing worktrees and prior release artifacts are preserved.

The delegated read-only inspection did not execute because the selected model
was unavailable (`model_not_found`); it supplied no review evidence. The main
session inspected the release owners directly.

## GitHub Publication And Download Verification

- `python3 tmp/release-012/run_release.py` completed with exit 0. It verified
  the clean annotated tag/source bytes, assembled repeatable package bytes,
  installed locally, pushed the commit/tag, published and downloaded the release,
  and removed its owned temporary directory.
- `gh release create v0.1.2 <archive> <SHA256SUMS> --repo er77/ontodoc
  --verify-tag --title 'OntoDoc 0.1.2' --notes-file <release-notes.md> --latest`
  exited 0. Release: <https://github.com/er77/ontodoc/releases/tag/v0.1.2>.
- `gh release download v0.1.2 --repo er77/ontodoc --dir <owned scratch>` and
  `sha256sum -c SHA256SUMS` exited 0. Downloaded archive bytes equal the local
  archive. Extraction and all five MCP tools plus CLI index/search/version
  passed on the downloaded binary.
- GitHub reports `isDraft: false`, `isPrerelease: false`, both assets uploaded
  with expected byte sizes, private repository and default branch `master`.
  The archive and external `SHA256SUMS` are the two assets. The manifest and
  evaluation/deployment documentation are inside the archive.
- Cleanup checked the canonical staging directory, user ownership, marker,
  symlink state, Git marker and registered worktrees before removing it.
  `/opt/d1/tmp/ontodoc-release-012-9oko3reg` no longer exists. Persistent output
  under ignored `tmp/release-012/artifacts/` is retained intentionally.
- Final evidence/status changes are committed separately after the immutable
  release tag. No rebuild is required for documentation-only closeout.

## Limits

Distribution is private and needs GitHub authentication. Public access was not
requested explicitly and remains a separate decision. Running MCP sessions
keep their existing server process until it restarts. Independent compiler
reproducibility, production ingress deployment and new retrieval-quality
measurements were not performed. The four ignored helpers were not invoked;
all-target tests already cover their supported functional contracts.
