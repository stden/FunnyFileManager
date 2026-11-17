<!-- Purpose: Records the measured content-search bottleneck and reproducible optimization checks. -->

# Content search performance

The October 17, 2026 profiling session identified content search as the useful
optimization target. One search reached the existing ten-second deadline and
accounted for 96.71% of recorded foreground waiting. Other interactions in that
session were already fast: panel movement averaged about 1–1.5 ms and editor
movement less than 0.25 ms.

## What the profile proves

The CPU recording contains 251 samples. FFM and its worker threads account for
167; a separate npm installer accounts for 77 and the language server for 7.
`ops::menu::search::text::fold` appears in 75 FFM samples, and lossy UTF-8
iteration in another 15. The installer ran before the slow search.

CPU sampling excludes blocked and kernel time. Incomplete callchains also mean
the SVG omits some child-process samples. These figures identify expensive
functions; they do not predict how much faster the original directory walk
will finish. The query and original corpus are deliberately absent from action
statistics, so the same interactive search cannot be reconstructed from them.

## Reproducible measurement

`benches/content_search.rs` runs the public Find file operation against one
approximately 8 MiB file in an isolated temporary directory. Fixture creation
happens before measurement. Every iteration checks both the result count and
successful completion, so reaching the time limit cannot count as a fast run.
Cases cover ASCII with an absent or late match, mixed Unicode and malformed
UTF-8 without NUL bytes. Measurements use Rust 1.98.1, default features and the
optimized bench profile. The compiler, features, profile and corpus are the same
across measurements. This measures warm-cache, single-file search throughput;
it does not measure cold storage, many small files or recursive traversal.

Three preserved executables separate the recorded scanner from changes already
present upstream and this branch's incremental change:

- `recorded1`: the original scanner, preserved before the optimization work.
- `upstream1`: upstream commit `7fc0b4c6dbbf218b59e3e83d8ebecaf372f2ce60`.
  This version already reuses the folded output buffer, decodes valid UTF-8
  chunks directly, and handles malformed bytes through the same output buffer.
  It lowercases ASCII one byte at a time.
- `merged1`: that upstream scanner with ASCII appended in one operation, then
  lowercased using the standard `make_ascii_lowercase` on the appended range.
  Existing output and the upstream Unicode and decoding paths are preserved.

Each executable used the identical benchmark harness on CPU 16, in the order
above, with 20 samples, 300 ms warm-up and a two-second measurement target per
case. Criterion extended some runs to collect the requested samples. The table
uses the saved `mean` estimates, rather than the displayed regression slope:

| Corpus                | Recorded, mean ms | Upstream, mean ms | Merged, mean ms | Incremental result vs upstream         |
| --------------------- | ----------------: | ----------------: | --------------: | -------------------------------------- |
| ASCII, absent         |            52.883 |            12.150 |           2.368 | 5.13× faster                           |
| ASCII, late match     |            52.165 |            12.080 |           2.329 | 5.19× faster                           |
| Unicode, absent       |            73.381 |            68.815 |          70.387 | 2.29% more time                        |
| Invalid UTF-8, absent |            69.881 |            25.729 |          25.886 | 0.61% more time; overlapping intervals |

The complete recorded-to-merged ASCII speedups are 22.33× and 22.40×. These
include upstream's improvements; this branch adds the 5.13× and 5.19× speedups
relative to upstream. The ASCII confidence intervals are disjoint in both
comparisons. Upstream also accounts for the large malformed UTF-8 improvement.
For that case, upstream's 95% confidence interval is 25.562–25.950 ms and the
merged interval is 25.549–26.441 ms: no regression was detected in this run.

Unicode shows a possible small cost relative to upstream. The first pair has
disjoint 95% confidence intervals: 68.781–68.854 ms upstream versus
69.774–71.074 ms merged. A Unicode-only repeat with identical settings, saved
as `upstream2` and `merged2`, measured 69.171 ms versus 70.087 ms, or 1.32% more
time. Its intervals overlap: 68.839–69.705 ms versus 69.619–70.651 ms.
Both merged means are higher, so these runs do not establish neutral Unicode
performance. The retained ASCII optimization comes with a possible roughly
1–2% Unicode cost; the observed mean increases were 1.32% and 2.29%.

To reproduce the comparison, keep the harness identical and build it against
each scanner version. Choose an available CPU on the host and use a distinct
baseline name for each run:

```sh
taskset -c 16 cargo bench --bench content_search -- --save-baseline recorded1 --sample-size 20 --warm-up-time 0.3 --measurement-time 2
taskset -c 16 cargo bench --bench content_search -- --save-baseline upstream1 --sample-size 20 --warm-up-time 0.3 --measurement-time 2
taskset -c 16 cargo bench --bench content_search -- --save-baseline merged1 --sample-size 20 --warm-up-time 0.3 --measurement-time 2
```

Criterion saves estimates and raw samples in `target/criterion/content_search/`.
Its `--baseline` and `--save-baseline` options are mutually exclusive; comparing
the saved estimates avoids a redundant measurement. Local Colorer build
overrides, if needed, must be identical for both runs.

## Compatibility checks

The scanner keeps its bounded buffers, deadline checks, NUL detection and
character-wise Unicode folding. Tests cover arbitrary short reads, invalid
bytes, incomplete UTF-8 suffixes, expanding lowercase characters and Greek
sigma across chunk boundaries. The behavioral cases were first run against
the original implementation. Tests for the upstream streaming interface also
check output-buffer reuse and retaining incomplete input. A regression case
appends ASCII and Unicode to an existing mixed-case prefix and verifies that
the earlier prefix stays unchanged.

## Verification on October 17, 2026

- `cargo test --lib ops::menu::`: all 83 tests passed.
- `./ffm.py lint`: passed, including the full Clippy matrix and architecture checks.
- `cargo nextest run --features test-helpers --no-fail-fast`: 6,683 passed,
  two pre-existing failures and 11 skipped.
- `cargo test --features test-helpers,s3 --no-fail-fast`: completed with the
  same two pre-existing failures. All search tests passed.

The existing failures are `normalize_strips_verbatim_prefix_from_a_path_buf`
and `tracked_files_carry_no_private_data`. Their source and fixtures are
unchanged by this branch.
