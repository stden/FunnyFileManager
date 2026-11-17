<!-- Purpose: Tracks the measured content-search optimization and its verification. -->

# Content Search Performance Implementation Plan

> For agentic workers: use the subagent-driven-development workflow for the independent benchmark and review tasks.

**Goal:** Reduce the measured CPU cost and elapsed time of finite content searches without changing search results or limits.

**Architecture:** Keep the bounded, synchronous search and its typed `Scan` outcomes. Preserve the concurrent upstream change that folds decoded chunks directly into a reused output buffer. Append ASCII as a slice and lowercase only the appended range with the standard library; retain the Unicode and malformed/incomplete UTF-8 rules. Benchmark through the public search operation without exposing a new runtime API.

**Tech stack:** Rust standard library, Criterion, tempfile, perf, existing Cargo gates.

## Evidence and scope

- The recorded search took 10,003.821 ms, 96.71% of measured action waiting, and reached the existing ten-second limit.
- `text::fold` accounts for 75 of 167 FFM CPU samples; lossy UTF-8 iteration accounts for another 15. CPU sampling does not measure blocked or kernel time.
- npm installation belongs to a separate child process. Routine navigation and editor movement in this recording do not justify changes.
- Preserve masks, Unicode/sigma equivalence, invalid-byte replacements, split characters, binary checks, bounded memory, result limits and deadlines.

## Task 1: Capture a reproducible baseline

Files: `Cargo.toml`, `benches/content_search.rs`.

- [x] Add finite 8 MiB ASCII absent/late-match, Unicode absent and malformed UTF-8 absent cases through the public `ops::findfile::accept` operation.
- [x] Build fixtures only in `TempDir` using explicit panels. Assert completion and result counts so a time limit cannot look like an improvement.
- [x] Run `cargo bench --bench content_search -- --save-baseline before` before changing production code; retain raw Criterion results under ignored `target/`.

## Task 2: Preserve decoding behavior

Files: `src/ops/menu/search/text/tests.rs` and `text/tests/decoding.rs`.

- [x] Exercise the scanner with reads limited to 1, 2, 3, 5 and 7 bytes over ASCII, Cyrillic, expanding lowercase, emoji and malformed bytes.
- [x] Cover an invalid prefix followed by an incomplete three-byte suffix and a match across ASCII/Unicode chunks.
- [x] Run the new behavioral cases against the baseline implementation before editing it. Existing profile and benchmark measurements reproduce the performance defect; correctness cases define unchanged behavior.

## Task 3: Reduce measured scanning costs

File: `src/ops/menu/search/text.rs`.

- [x] Resolve the rebase onto upstream `7fc0b4c6` by retaining its `push_chunk`, `fold_into`, direct lossy decoding and buffer reuse. These subsume the initial borrowed-decoder optimization.
- [x] Replace the ASCII byte/push loop with `push_str` followed by `make_ascii_lowercase` on the appended range. Keep capacity reservation and the existing Unicode iterator.
- [x] Adapt direct tests to the upstream decoder contract, test buffer reuse, and ensure folding leaves an existing mixed-case prefix untouched.
- [x] Run the merged search/menu regression tests: all 83 passed.

## Task 4: Measure, review and finish

Files: `docs/content-search-performance.md` plus the files above.

- [x] Compare the merged scanner against both the recorded-version baseline and latest upstream with identical compiler, profile, features, corpus and CPU affinity. Retain the roughly 5.1× incremental ASCII improvement and explicitly report the possible 1–2% Unicode cost; the Unicode-only repeat has overlapping confidence intervals.
- [x] Record final results and the limits of the original 251-sample profile; distinguish upstream improvements from this branch's incremental ASCII change.
- [x] Request independent review of UTF-8 and chunk-boundary invariants; add output-reuse, retained-suffix, malformed-middle and append-isolation regression cases from review.
- [x] Rerun `cargo fmt --all`, `cargo test --features test-helpers,s3 --no-fail-fast`, `cargo nextest run --features test-helpers` and `./ffm.py lint` after integrating upstream. The full lint gate passes; Nextest reports 6,683 passes, two baseline failures and 11 skips.
- [x] Distinguish the pre-existing path-normalization and private-data test failures from new failures. Both merged full test runs reach only those two known failures; all search tests pass.

Finishing workflow: commit with Conventional Commits, pull before pushing, and
push the feature branch. Rebuild with profiling symbols for the existing
launcher, then verify the checked-out branch and clean working tree.
