<!-- Purpose: Documents the Claude command for running project benchmarks. -->

Run benchmarks and report performance results.

Steps:

1. Check if any benchmarks exist: `ls benches/ 2>/dev/null || grep -r '#\[bench\]' src/`
   If none exist, report "No benchmarks found" and suggest adding one for a hot path.

2. Run benchmarks: `cargo bench 2>&1`

3. Parse results — for each benchmark report:
   - Name
   - Mean time (ns/µs/ms)
   - Variance / standard deviation

4. If a previous benchmark result exists in `target/criterion/`, compare against it and report regressions (>10% slower).

5. If there are no benchmarks yet and the user asked to add one:
   - Identify the hottest code path (panel loading, sorting, rendering).
   - Add a benchmark in `benches/` using the `criterion` crate.
   - Add `criterion` to `[dev-dependencies]` in Cargo.toml if not present.
   - Add `[[bench]]` entry in Cargo.toml.

Rules:
- Run benchmarks at least 3 times and report the median, not a single run.
- Do not benchmark test or debug builds — always use `cargo bench` (which uses release mode).
- A benchmark that tests the wrong thing is worse than no benchmark.
