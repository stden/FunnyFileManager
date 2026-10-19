<!-- Purpose: Describes the interactive CPU profiling launcher and retained evidence. -->

# Interactive profiling

On Linux, run `./x.sh` and use FFM normally. Repeat the slow operation a few
times, then exit with **F10**. Application arguments can follow `./x.sh`.
The launcher needs Cargo, rustc, Git, `perf`, and `flamegraph` on PATH.
The current user must be allowed to record the `cpu-clock:u` perf event.

The launcher always asks Cargo to check the optimized release build, keeping
debug information and symbols inside its executable. It explicitly builds for
the current host, even if Cargo is configured for a different target. Cargo
reuses unchanged build artifacts. Ordinary release builds still use the settings in Cargo.toml.
The profiling overrides follow [Cargo's profile settings](https://doc.rust-lang.org/cargo/reference/profiles.html).

Each launch prints its own directory under `target/profiles/ffm-*/`:

- `perf.data`: raw CPU samples with DWARF call stacks, recorded at 99 Hz.
- `flamegraph.svg`: the graph generated after FFM exits.
- `ffm`: the exact executable used for this recording, including debug symbols.
- `metadata.txt`: capture time, sampling/build settings, Git revision and working
  tree status, plus compiler and Cargo versions.

Keep the whole directory when requesting analysis. The executable is a separate
copy, so later builds cannot replace the symbols needed by this recording.
`cargo clean` removes these directories along with other build output.

The initial graph skips inline expansion to keep generation quick; the raw
recording remains available for more detailed analysis. For example:

```sh
DEBUGINFOD_URLS= perf report --stdio -i target/profiles/ffm-<run>/perf.data
```

CPU samples show user-space execution, including inherited worker threads and
child processes. They do not measure time blocked on I/O, kernel execution,
or heap allocations. FFM also collects [action timings](session-tools.md#action-statistics)
automatically and saves a JSON file at exit under
`$XDG_CONFIG_HOME/ffm/performance/` (`~/.config/ffm/performance/` by default).
Retain the JSON from the same session: it identifies actions with high total
or maximum waiting time and separates rendering time from other work.

For useful comparisons, note which operation was slow and repeat the same
workload before and after an optimization. Idle time waiting for keyboard
input is excluded from the action timings; CPU sampling records executing threads.
