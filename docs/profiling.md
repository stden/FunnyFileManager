<!-- Purpose: Describes the interactive CPU profiling launchers on Linux and Windows and retained evidence. -->

# Interactive profiling

## Linux

Run `./x.sh` and use FFM normally. Repeat the slow operation a few times, then
exit with **F10**. Application arguments can follow `./x.sh`. The launcher needs
Cargo, rustc, Git, `perf`, and `flamegraph` on PATH, and the current user must
be allowed to record the `cpu-clock:u` perf event.

Each Linux launch prints its own directory under `target/profiles/ffm-*/`:

- `perf.data`: raw CPU samples with DWARF call stacks, recorded at 99 Hz.
- `flamegraph.svg`: the graph generated after FFM exits.
- `ffm`: the exact executable used for this recording, including debug symbols.
- `metadata.txt`: capture time, sampling/build settings, Git revision and working
  tree status, plus compiler and Cargo versions.

The initial graph skips inline expansion to keep generation quick; the raw
recording remains available for more detailed analysis:

```sh
DEBUGINFOD_URLS= perf report --stdio -i target/profiles/ffm-<run>/perf.data
```

## Windows

Run `.\x.cmd` (or `ffm.cmd profile` in PowerShell / Command Prompt, or `./x.sh`
in Git Bash) and use FFM normally, then exit with **F10**. Application arguments
are forwarded to `ffm.exe`. The launcher needs Python, Cargo, rustc, Git, and
Visual Studio's `VSDiagnostics.exe` (discovered via `FFM_VSDIAGNOSTICS`, `PATH`,
`vswhere`, or standard Visual Studio installations).

Each Windows launch writes a self-contained directory under `target/profiles/ffm-*/`:

- `profile.diagsession`: CPU sampling trace recorded by `DiagnosticsHub.CpuAgent.dll`
  (open directly in Visual Studio's Performance Profiler).
- `ffm.exe` and `ffm.pdb`: the exact executable and PDB symbols from this run.
- `metadata.txt`: capture time, build/target settings, Git revision and working
  tree status, plus compiler and Cargo versions.

## Build and action timings

Both launchers always ask Cargo to check the optimized release build for the
current host target while keeping debug information and symbols unstripped. Cargo
reuses unchanged build artifacts, and ordinary release builds continue to use the
settings in `Cargo.toml` (see [Cargo's profile settings](https://doc.rust-lang.org/cargo/reference/profiles.html)).

Keep the whole capture directory when requesting analysis: the snapshotted
executable (and PDB on Windows) ensures later builds cannot replace the symbols
needed by a recording. `cargo clean` removes `target/profiles/` along with other
build output.

CPU samples show user-space execution, including inherited worker threads and
child processes. They do not measure time blocked on I/O, kernel execution, or
heap allocations. FFM also collects [action timings](session-tools.md#action-statistics)
automatically and saves a JSON file at exit under `%APPDATA%/ffm/performance/` on
Windows or `$XDG_CONFIG_HOME/ffm/performance/` (`~/.config/ffm/performance/` by
default) on Unix. Retain the JSON from the same session: it identifies actions
with high total or maximum waiting time and separates rendering time from other
work.

For useful comparisons, note which operation was slow and repeat the same
workload before and after an optimization. Idle time waiting for keyboard input
is excluded from action timings; CPU sampling records executing threads.
