# Full production coverage implementation plan

**Goal:** Enforce and achieve 100% line coverage of repository-owned production Rust and Python through the common lint gate.

**Architecture:** Keep one exact-count verdict for native Rust LCOV source-line records and Python coverage JSON, preserving LLVM JSON for diagnostics. Collect all Rust packages and feature configurations independently, include build logic and real executable entry points, and trace Python subprocesses used by script tests. Tests and external dependencies are outside production measurement; production files have no exemptions.

**Tech stack:** cargo-llvm-cov, Cargo test, coverage.py with subprocess tracing, Python 3.9-compatible developer scripts.

## Collection and enforcement

- [x] Add `coverage_rust.run(directory) -> list[Path]`: root all features, root fallback with test helpers, MCP and colorer-sys, each with native LCOV gate data and independent JSON diagnostics. Include build scripts and disable coverage-specific production probes.
- [x] Add `coverage_python.run(directory) -> Path`: explicitly inventory production Python, collect subprocess execution, combine recognized copied-source paths, and report zero-hit files.
- [x] Replace the 98% root-only command with the shared strict verdict. Reject empty/invalid reports and any uncovered production line, using exact integer counts.
- [x] Test each collector and the verdict before implementation, including 99.999% failure, missing files, child-process coverage and tool/test failure propagation.
- [x] Add coverage to full lint and fix modes and expose `lint coverage`. Keep focused format/Clippy modes focused.
- [x] Wire coverage tools and the same strict gate into CI, including native Colorer and Windows configurations.
- [x] Independently verify the source-line metric with complementary generic instantiations and a real unexecuted line; use native `DA` records rather than LLVM's grouped function summaries.

## Reach the threshold

- [x] Collect real Rust and Python baseline reports outside stored `coverage/` artifacts.
- [ ] Group missed lines by owner and add meaningful behavioral/error-path tests in parallel, preserving production behavior.
- [ ] Repair test-fixture/environment problems that prevent the complete suites from executing.
- [ ] Repeat measured runs after relevant changes until exact coverage is 100% for every required report.
- [ ] Review correctness and measurement scope; run the complete quality gate and ordinary tests before committing and pushing.

## Working-tree boundaries

Preserve concurrent work outside this task, including `src/ops/apply/tests/short_names.rs` and any independently restored profiling fixtures. Stage only reviewed coverage work and directly necessary test improvements.

## Verified checkpoints

- Python: 2168/2168 production lines in the clean fourth measurement, with zero excluded lines. Real regressions ensure executable fakes and written script doubles cannot credit untouched production files. Later collector additions require a final fresh measurement.
- Colorer-sys: 150/150 lines, including build scripts, discovery and public availability; 21 tests and crate Clippy pass.
- MCP S3: listing 74/74, tools 78/78 and transfer 121/121 lines, using a real loopback HTTP fixture and private AWS profiles.
- Root: 6225 unit tests pass after adding real permission-denial, terminal, cleanup, elevation and system-profile tests. Whole-package coverage is still being raised; the strict gate must not be described as passing yet.

Coverage artifacts remain outside the repository under `/var/tmp/ffm-full-coverage`. Each parallel root-crate worker uses its own Cargo target: replacing a running test executable can otherwise invalidate `current_exe()` in subprocess fixtures. Final measurement must use clean profiles against the final source tree.
