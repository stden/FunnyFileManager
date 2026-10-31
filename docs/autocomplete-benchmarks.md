# Command-line autocomplete benchmarks

Run `cargo bench --bench autocomplete_bench -- --noplot` from the repository root.
Criterion keeps machine-readable measurements under `target/criterion/`.
Use `--save-baseline before` and later `--baseline before` to compare changes
on the same machine with the same toolchain and profile.

Fixtures are created outside timed iterations. Command completion warms the
process-wide PATH cache first; executable names still depend on the host PATH.
Directory completion uses `cd project f`: whitespace selects the existing
built-in-only path, so these timings exclude fish startup and completion.
Ranking clones its input outside the timed routine; ranking and consuming the
owned input remain timed. These are engine measurements, not terminal-rendering
or physical key-to-screen latency. Filesystem caches are warm after Criterion's
warm-up; first-use PATH scanning and cold-disk latency are not measured.

## Initial Windows measurements

2026-09-07, Rust 1.98.1, optimized Cargo bench profile, working tree based on
`e5a1f82` with the 30-line refactor. Criterion used a one-second warm-up and a
two-second measurement target per case. Directory cases use 20 samples; other
cases use 100. Values below are Criterion's central time estimates from one run,
not performance guarantees or evidence of an improvement over the parent commit.

| Scenario | Time |
|---|---:|
| Empty command | 2.20 µs |
| Command prefix `c`, warm PATH | 104.58 µs |
| Fuzzy command `srt` | 2.74 µs |
| `sort e` | 1.03 µs |
| `bookmark go ` | 0.60 µs |
| `mark a` | 0.49 µs |
| Directory, 10 candidates | 95.37 µs |
| Directory, 100 candidates | 272.02 µs |
| Directory, 1,000 candidates | 1.99 ms |
| Prefix ranking, 100 / 1,000 / 10,000 candidates | 30.74 µs / 308.17 µs / 2.97 ms |
| Fuzzy ranking, 100 / 1,000 / 10,000 candidates | 28.37 µs / 252.76 µs / 2.52 ms |
| No-match ranking, 100 / 1,000 / 10,000 candidates | 23.22 µs / 228.18 µs / 2.49 ms |

The measured ranking cost grows roughly with the candidate count. On this host,
the common `c` prefix costs much more than a subcommand because it also searches
cached executable names. Compare isolated ranking and full directory completion
before deciding whether matcher work or filesystem access needs optimization.

## Prefix lookup and source merging

`execs::matching_in` requires sorted names, as returned by the PATH scan.
It finds the first possible match with `partition_point`, then visits only
the matching range: O(L log n + kL), for n names, k results and string length L.
The `autocomplete/path_prefix_range` benchmark compares that implementation
with the previous full scan on the same synthetic names, including a miss and
a prefix with ten matches at 1,000, 10,000 and 100,000 names.

Directory/fish and built-in/PATH candidate merging preserve first-seen order
in a vector and use a hash set for membership: expected O(nL), rather than
repeated vector membership scans with worst-case O(n²L). Ranking still sorts
the merged candidates separately; this change does not truncate the results.

Panel frame comparison on Windows, 2026-10-31, `release-fast`, 120×40,
10 Criterion samples with 0.2 s warm-up and 0.5 s measurement: the 100,000-entry
case changed from 1.31 ms to 0.36 ms after totals caching. The 10,000-entry case
changed from 0.41 ms to 0.32 ms. These short local runs demonstrate scaling,
not a guaranteed latency; small cases showed noise and a roughly 6% regression
at 1,000 entries. Reproduce with `ui_bench` filtered to `frame_by_entries`.
