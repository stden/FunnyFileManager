<!-- Purpose: Documents the Claude command for performance profiling. -->

Profile the application to find performance bottlenecks.

Steps:

1. Build a release build with debug info for profiling:
   Add to Cargo.toml under `[profile.release]` if not present:
   ```toml
   [profile.release]
   debug = 1
   ```
   Then run `cargo build --release -j1`.

2. On Windows: use `cargo flamegraph` or suggest VS profiler.
   On Linux/Mac: run `cargo flamegraph -- [args]` to generate a flamegraph.
   If `cargo-flamegraph` is not installed: `cargo install flamegraph`.

3. Alternatively, add timing instrumentation:
   - Identify the suspected hot function.
   - Add `let t = std::time::Instant::now(); ... eprintln!("{:.2?}", t.elapsed());` around it.
   - Run the app, perform the slow operation, and check stderr output.
   - Remove the instrumentation after profiling.

4. Report the top 3 hottest functions and their approximate % of total time.

5. Suggest specific optimizations based on findings:
   - Hot filesystem I/O → cache results
   - Hot string allocations → use `Cow<str>` or pre-allocate
   - Hot sorting → use `sort_unstable_by_key` instead of `sort_by`
   - Hot clone() → use references or `Arc`

Rules:
- Profile the release build, not debug — debug builds are 10-100x slower and misleading.
- Only optimize the actual bottleneck, not guesses.
- Measure before and after every optimization to confirm it helped.
