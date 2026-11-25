<!-- Purpose: Specifies automation entrypoints and their expected responsibilities. -->

# 📐 Automation Surfaces Specification

## 📖 Purpose

Define the repository-facing automation entry points that developers and AI agents can rely on consistently across local development and CI.

## 🎯 Scope

- GitHub Actions workflows under `.github/workflows/`
- Windows helper commands such as `ffm.cmd`
- Prompt-command documents under `.claude/commands/`
- Future repo-local skills and agent metadata

## 📐 Requirements

### 🚦 CI

- The default CI workflow must run on pushes to `main` and `master`, and on pull requests.
- CI must use the stable Rust toolchain.
- CI must run the test suite with `--features test-helpers`, because several integration-style tests are gated behind that feature.
- CI configuration files must not contain unresolved merge markers.

### 💬 Local Commands

- Each Windows helper command must `cd` to the repository root before invoking tools.
- Each helper must fail fast with a readable error if the target CLI is not installed.
- Each helper must allow a custom prompt, while also providing a repo-specific default prompt.

### 💬 Prompt Commands

- Prompt-command files in `.claude/commands/` must describe reproducible steps, not vague goals.
- Commands that mention coverage or tests must use the repo's Rust feature flags correctly.
- Commands should prefer `cargo test --features test-helpers` over bare `cargo test` when full behavior coverage is expected.

### 🎓 Skills

- Repo-local skills should live in a dedicated folder and be task-specific rather than tool-agnostic.
- Each skill must contain a `SKILL.md` with explicit triggers and a short workflow.
- Skills should point to bundled references only when the repo has non-obvious procedures or constraints worth reusing.

## 🚫 Non-Goals

- Defining hosted deployment or release automation
- Defining external cloud secrets management
- Mandating a specific AI vendor for repository automation
