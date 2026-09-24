<!-- Purpose: Records the approved fixes and verification for the five review findings. -->

# Review findings implementation plan

**Goal:** Fix all five confirmed bugs without changing the surrounding workflows.

**Architecture:** Keep effects in their current owners. Stage a replacement beside
its destination and rename only after the copy and options succeed; reject session
JSON that its loader cannot read. Keep LSP validation pure and represent a required
post-control sample explicitly in the process refresh state.

**Tech stack:** Rust, Cargo, existing filesystem helpers, in-memory process fixtures.

- [x] Overwrite safety (`src/ops/file.rs`, a focused replacement child, owner tests):
  reproduce a refused overwrite with an existing destination, then stage before
  replacement. Cover successful overwrite, failed preparation, symlinks, moves,
  and preservation of the existing folder-merge policies.
- [x] Session persistence (`src/sessions.rs`, owner tests): reproduce an accepted
  import whose serialized JSON exceeds `MAX_FILE_BYTES`. Refuse it before writing
  and verify that the previous file and in-memory committed store remain usable.
- [x] Batch LSP (`src/config/lsp/catalog.json`, catalog tests): assert that the
  resolved batch command uses stdio and add the missing `--stdio` argument.
- [x] LSP conflicts (`src/models/lsp_editor/problems.rs`, owner tests): reproduce
  disabling the first of three existing claimants. Classify a pair as pre-existing
  when both servers already claimed that extension; keep new conflicts blocking.
- [x] Process refresh (`src/models/tasks/schedule.rs`, `src/ops/tasks/control.rs`,
  owner tests): reproduce a successful renice with an older sample in flight.
  Discard the older answer and request one fresh explicit sample, including while
  held, without changing manual-refresh coalescing or stopped-worker behavior.
- [x] Review all changes against the five failures, then inspect correctness,
  failure cleanup, enum transitions and repository lint requirements.
- [x] Run focused regressions, `cargo test`,
  `cargo nextest run --features test-helpers`, and `./ffm.py lint`.
  Supply a temporary `python` alias for the existing Python-based LSP fixtures.
- Delivery: format, commit with a Conventional Commit subject, pull again and push.

Each code task starts with a failing behavioral regression and finishes with the
regression passing. Filesystem tests use `TempDir`; process tests use injected hosts
and must not change the machine's processes. Independent owners can be edited in
parallel, but Cargo runs are coordinated by the parent agent.
