<!-- Purpose: Records the plan for hardening repository automation surfaces. -->

# Automation Hardening Plan

## Goal

Stabilize repository automation so local agents, command helpers, and CI all use the same assumptions and verification path.

## Plan

1. Fix repository hygiene issues.
   - Remove merge conflicts from checked-in documentation and workflow files.
   - Ensure workflow YAML is syntactically valid and matches the Rust test layout.

2. Standardize command entry points.
   - Keep `ffm.cmd` for Cargo-oriented developer tasks.
   - Keep agent wrappers focused on repository inspection, implementation, and verification.
   - Add or update prompt-command documents when a repeated workflow becomes common.

3. Align verification paths.
   - Use `cargo build`, `cargo test --features test-helpers`, and `cargo clippy -- -D warnings` as the baseline validation set.
   - Treat coverage tools as optional, because they are platform-dependent in this repository.

4. Add repo-local skills only where they save repeated rediscovery.
   - Start with one narrow skill if a repeated workflow emerges, such as test hardening or release preparation.
   - Validate the skill with the standard Codex skill validator before relying on it.

5. Extend CI incrementally.
   - Keep the current test workflow minimal and working.
   - Add formatting or lint jobs only after the base workflow is consistently green.

## Exit Criteria

- The checked-in workflow files are conflict-free and valid.
- Local helper commands exist for the supported agent CLIs.
- The full test suite passes with `--features test-helpers`.
- The repo has written guidance describing the expected automation surfaces and the follow-up implementation sequence.
