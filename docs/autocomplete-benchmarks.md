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
