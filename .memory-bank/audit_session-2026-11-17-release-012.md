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
