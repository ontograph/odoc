# Public GitHub Distribution

Date: 2026-11-17
Authority: User selected `https://github.com/ontograph/odoc` for codebase and release and requested a README and installation shell script from GitHub.

## Destination And Identity

The destination is public, grants the authenticated account ADMIN permission, and initially contains commit `2fbe3812ec10bfafa147c3c53f2bace2fec663e8` on `main`, with an AGPL licence template and no code. The existing standalone package is MIT; keep its licence unchanged and preserve the destination's initial commit as merge ancestry. No code is adopted from that initial commit. The selected public source repository resolves the prior public-exposure decision. Keep the private repository and old release unchanged.

`origin` now names `https://github.com/ontograph/odoc.git`; the former remote was renamed `previous`. Push without force after preserving destination history. The existing annotated `v0.1.2` and its binary stay tied to commit `8e94a3d`. Refresh public README, CHANGELOG, package metadata and installer separately, recording a packaging commit and new archive checksum. No runtime, dependency, schema, ranking, qrels or custody change is authorized.

## Installer

Root `install.sh` downloads anonymous public GitHub releases from `ontograph/odoc`. It supports `--version` and `--bin-dir`, checks Linux x86_64 and glibc 2.39+, caps downloads, verifies the named SHA-256 entry and binary version before touching installation, and extracts only the binary to a temporary file. It preserves an existing binary/symlink as a backup before atomic replacement; identical reinstall is a no-op. Temporary files are removed on exit and signals.

README documents installation, PATH, version pinning, source builds and MCP use. `tests/test_install.py` uses local curl fixtures for deterministic success, upgrade, malformed/checksum/download and platform refusal cases. No runtime symbol changed. OntoIndex's committed snapshot was initially stale at `701918f`, then refreshed to `8b58ea7`; source inspection established that no installer existed. The historical release verifier is unchanged and remains pinned to 0.1.0.

## Validation

Pending final command and public download evidence. The initial `sh -n install.sh` and nine installer tests exited 0 before adding a version-mismatch regression and rejecting multiline versions. No Rust build/test/lint is required for these shell and documentation changes; earlier binary validation belongs to the 0.1.2 release audit.

Final local checks: `sh -n install.sh` exit 0; `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p test_install.py` exit 0 (10 passed); `git diff --check` exit 0; Markdown link/fence check exit 0; plan validator PASS. OntoIndex `verify_diff` passed the ten-file declared scope; expected symbol scope was omitted because no runtime symbols changed. ShellCheck was not installed, so shell syntax and executable fixture tests are the local shell checks.

A heuristic scan of 702 historical Git blobs and new installer/test files found no matches for five credential/private-key patterns. It prints only object/path/pattern on a finding and never matched secrets. Historical project paths, authors, benchmark and snapshot provenance remain visible as authorized public source history; this limited scan is not proof that all possible sensitive strings are absent.

## Source Integration And Package

- Installer/documentation commit: `4a5cd6f`. Integration commit: `49cbabf`, with parents `4a5cd6f` and destination initial commit `2fbe381`. The expected add/add conflict in LICENSE was resolved by retaining exactly the existing standalone MIT licence. No other tracked merge changes occurred. `git merge-base --is-ancestor origin/main HEAD` passed before push. No force push or history replacement was used.
- Branch is `main`; `origin` is the selected public repository and `previous` retains the private repository. The existing annotated `v0.1.2` tag still points to original build commit `8e94a3de70fc1249e15e30518bc78f7b356df462`.
- Packaging reuses exactly binary SHA-256 `6487f1ca1622b41298ad98adff2bb9e8a2a76401b234c9f058d2ce97f6d61b67`. The source manifest records `repository: ontograph/odoc`, `visibility: public`, original build commit and `packaging_commit: 49cbabf`. All 29 recorded Cargo/toolchain/runtime source hashes remain equal to the original build inventory.
- Public packaging refreshes README and CHANGELOG and adds `install.sh`; the previous private archive is preserved. Fixed metadata, sorted USTAR members and zero gzip timestamp repeat byte-for-byte in two assemblies. Complete internal checksums and extracted CLI plus all five stdio tools passed. The only shipped home-path example remains the synthetic `/home/me/docs/x.md` in README, already reviewed in the private release audit.
- The three public assets are the binary archive, `install.sh` and `SHA256SUMS`. External checksums cover both payload assets. README and installer are also included inside the binary archive. Temporary publication staging has a unique owner marker and validated cleanup on every terminal outcome; persistent receipts are under ignored `tmp/public-release/`.

## Public Publication And Installation

- `python3 tmp/public-release/publish.py` exited 0. Its package verification, repeatable assembly, CLI/all-five-MCP smoke, atomic branch/tag push, release creation, anonymous downloads and installation checks all completed. It removed the marked staging directory `/opt/d1/tmp/odoc-public-release-ezqdwglj`; absence was rechecked.
- `git push --atomic -u origin main refs/tags/v0.1.2` exited 0. Public main initially equals packaging commit `49cbabf8607a83381f06c98b8a4eff766415b3e1`; peeled remote `v0.1.2` equals original build commit `8e94a3d`. The follow-up evidence commit advances main only.
- `gh release create v0.1.2 <archive> <SHA256SUMS> <install.sh> --repo ontograph/odoc --verify-tag --title 'OntoDoc 0.1.2' --notes-file <notes> --latest` exited 0. [Public release](https://github.com/ontograph/odoc/releases/tag/v0.1.2) reports `isDraft: false`, all three assets uploaded. Repository is public, default branch `main`; the description names the service's actual transports and local indexes.
- Public archive SHA-256 `527cb1b7b05964723fc6d1629732451424309f0183e8b01937fab0148426d5b7`, 3,705,073 bytes. Installer SHA-256 `69809296996197ea078bf7c917a13e481f18d8aeb5703b6f56adb6332061e30d`, 4,485 bytes. The archive's changed identity reflects packaging only. Binary identity is unchanged.
- `curl --disable -fsSL https://raw.githubusercontent.com/ontograph/odoc/main/install.sh -o <scratch>/install.sh` fetched bytes equal to the source and release installer. `sh <scratch>/install.sh` installed the latest release in a temporary home without GitHub authentication. The installed bytes match the original binary hash. CLI index/search/version and all five stdio tools passed.
- `curl --disable -fsSL https://raw.githubusercontent.com/ontograph/odoc/main/install.sh | sh` ran with `bash -o pipefail` in an independent temporary home and exited 0, installing the same binary. The driver removed GitHub token variables and used no authorization header or GitHub CLI for either installation path.
- All three release assets were downloaded again with anonymous `curl`; `sha256sum -c SHA256SUMS` exited 0. Archive bytes equal the staged payload; released installer equals the live main installer. Anonymous README download compared equal to source. Installer fixture checks reran on the final integration tree: 10 passed, exit 0; shell syntax and whitespace checks passed.

## Limits

The supplied binary supports Linux x86_64 GNU, glibc 2.39+, Linux 5.6+ with `openat2`, and libgcc_s. No Windows/macOS/ARM binary was built. No Rust compilation or retrieval evaluation was run during this packaging-only follow-up. Compiler reproducibility, quality improvements and production ingress readiness remain unclaimed. Earlier private release, local installed version and evaluation custody are unchanged. ShellCheck was unavailable; runnable installer regressions, `sh -n` and live anonymous installation provided shell validation.
