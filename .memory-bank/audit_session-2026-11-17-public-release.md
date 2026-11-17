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
