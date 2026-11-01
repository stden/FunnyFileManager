<!-- Purpose: Records the deletion investigation, measured scan optimization, and reproduction steps. -->

# Deletion performance

The user confirmed that deletion was the slow operation in capture
`target/profiles/ffm-20261019T134132Z-HWDBAN`, recorded on 2026-10-19 from
13:41:32 to 13:43:03 UTC. Its matching action report is
`session-1792417383403476274-1467269.json` in the configuration directory's
`ffm/performance/` folder.
The confirmed delete took **3.808896 s**, or **96.06%** of measured action
waiting time. All other actions together took 0.156246 s.

This is input-to-frame latency, excluding time waiting for keyboard input.
Deletion runs synchronously on the UI thread: the progress pre-scan, recursive
removal, progress drawing, and panel refresh all contribute. The report's
0.424476 ms of deletion rendering covers the final ordinary frame; progress
painting happens inside the operation and is included in its other work.

The CPU recording has 439 events: 18 from FFM, 419 from Node/npm, and two from
Git. Only 49 events have call chains usable by the generated SVG; 390 lack
call chains. Its displayed percentages therefore do not represent all recorded
CPU events and cannot establish deletion's CPU share. User-space CPU sampling
also excludes I/O waiting and kernel execution. The action report identifies
the delay more directly than this particular flamegraph.

Before the change, deletion's progress pre-scan fetched full metadata for every
entry, although its progress bar only needs file counts. After the change,
`src/progress/engine/report.rs` uses `DirEntry::file_type()` for deletion;
copy and cross-filesystem move scanning retain metadata and byte counts.
Both entry APIs avoid following symbolic links. Rust documents that
`metadata()` requires a metadata lookup on Unix, while `file_type()` usually
needs no additional syscall; some platforms still require one.
See the [standard-library DirEntry documentation](https://doc.rust-lang.org/std/fs/struct.DirEntry.html).

The existing deletion progress regression now also asserts that a directory
symlink contributes exactly one file to the total, while its target survives.
The optimization preserves the pre-scan and file-count progress display.

The benchmark in `benches/ops_bench/delete_progress.rs` uses 500 files of 4 KiB
each in a warm temporary fixture on `/tmp` (tmpfs on this machine). A no-op
painter enables production progress accounting and its normal throttle without
terminal output. `scan_many_small` reuses one tree and measures the real
deletion pre-scan. `delete_many_small` calls `ops::handle_delete`, including
scanning, removal, reporting, and both panel refreshes. Application and fixture
creation are outside timing, as is their teardown; the deletion timing includes
one source-existence check that verifies the operation ran.

The optimized benchmark binaries used native Rust 1.98.1, default features,
and CPU affinity to logical CPU 16. The same benchmark code was used for both
versions; the before binary used the original progress report implementation.
The preserved binaries are `/tmp/ffm-delete-progress-before` and
`/tmp/ffm-delete-progress-after`.

Final repeated measurements used 20 samples, 0.5 s warmup, and 2 s measurement
time. The group sets 20 samples itself. Results below are Criterion means and
their 95% confidence intervals:

| Measurement   |                   Before mean [95% CI] |                    After mean [95% CI] |
| ------------- | -------------------------------------: | -------------------------------------: |
| Progress scan |    285.50967 µs [284.82605, 286.58379] |       60.29043 µs [59.92988, 60.69084] |
| Full deletion | 1.71233240 ms [1.70328402, 1.72207687] | 1.46277897 ms [1.45243300, 1.47316425] |

The scan is **4.735 times faster**; full deletion elapsed time falls by
**14.57%** in this fixture. An earlier pair measured full-deletion means of
2.48159 ms before and 1.49601 ms after. That variation is why the smaller
improvement from the final repeated measurements is reported here.

The estimates are retained under
`target/criterion/delete_progress/{scan_many_small,delete_many_small}/{deletion-before-repeat,deletion-after-repeat}/estimates.json`.
Run the following commands on the corresponding before and after source
versions, keeping the benchmark code identical:

```sh
taskset -c 16 cargo bench --bench ops_bench -- \
  delete_progress --save-baseline deletion-before-repeat \
  --warm-up-time 0.5 --measurement-time 2 --noplot

taskset -c 16 cargo bench --bench ops_bench -- \
  delete_progress --save-baseline deletion-after-repeat \
  --warm-up-time 0.5 --measurement-time 2 --noplot
```

A separate syscall check used each preserved binary's single-run test mode:

```sh
for phase in before after; do
  taskset -c 16 strace -f -e trace=statx,newfstatat \
    -o "/tmp/ffm-delete-scan-$phase.strace" \
    "/tmp/ffm-delete-progress-$phase" delete_progress/scan_many_small --test
done

for phase in before after; do
  printf '%s: ' "$phase"
  awk '/file-[0-9]+\.dat/ { n++ } END { print n+0 }' \
    "/tmp/ffm-delete-scan-$phase.strace"
done
```

Metadata requests naming fixture files fell from **500 to zero**. This counts
only `statx`/`newfstatat` requests for `file-*.dat`, not every syscall or the
metadata lookup on the root directory.

These results establish the removed scan overhead on this machine. The original
deleted tree and its filesystem are unknown, so they do not guarantee a
particular improvement to the original 3.81 s operation. A comparable disposable
tree on the same filesystem is needed to measure that workload's improvement.

`cargo test -- --quiet` passed all enabled library and default integration
tests; nine library tests were ignored. The run used a temporary PATH alias
from `python` to `/usr/bin/python3` for existing LSP fixtures, without changing
the repository or global configuration. The strengthened symlink progress
assertion passed. The file-metadata budget check passed after the change and
failed against the before binary, which still made 500 requests.
`./ffm.py lint` also passed the format, spelling, complexity, duplication,
Clippy matrix, and architecture checks.
