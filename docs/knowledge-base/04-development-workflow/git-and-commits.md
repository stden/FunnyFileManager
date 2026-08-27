<!-- Purpose: Documents the git and commit conventions for contributors and agents. -->

# Git Workflow and Commit Guidelines

## Commit Style

The project prefers **Conventional Commit** prefixes when the change is significant:

- `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `ci:`, `style:`, `perf:`, `chore:`

Simple one-line commits are also acceptable for trivial changes.

## Pull Request Expectations

Every PR should include:

- A short summary + rationale.
- Verification steps (at minimum `cargo test` and `cargo clippy -- -D warnings`; preferably with `--features test-helpers`).
- For UI changes: a screenshot or short recording is highly recommended.

Link related issues when applicable (e.g. "Closes #42").

## Branching

- Work on feature branches off `main`.
- Keep branches reasonably focused.
- Rebase on latest `main` before opening or updating a PR when possible.

## Dirty Worktree Policy

When an AI agent is working:
- Preserve any existing uncommitted user changes in the worktree.
- Do not run destructive commands (`git reset --hard`, `git checkout --`, etc.) without explicit confirmation unless the task specifically requires cleaning the tree.

## Release Process (High Level)

- Use `cargo build --release` for final binaries.
- The release profile uses fat LTO, single codegen unit, and strips symbols for small size.

See `Cargo.toml` for the exact release profile settings.

## Pre-commit Hooks (Recommended)

Two options run the **Rust linter gate** on every commit: `cargo fmt` plus
`cargo clippy -D warnings` across the same feature matrix CI uses
(production, `test-helpers`, `keepass`, `s3`, and the `mcp/` crate).

The shared entry point is `scripts/lint.sh` (also exposed as `./ffm.sh lint`).

### Option 1: pre-commit framework (recommended)

A `.pre-commit-config.yaml` is committed at the root.

It auto-formats with `cargo fmt`, then runs `scripts/lint.sh clippy`, plus a
few basic checks (trailing whitespace, YAML, large files). Formatting changes
are applied in-place and **automatically re-staged** by the framework.

**Setup (once):**

```bash
pip install pre-commit          # or brew install pre-commit / choco install pre-commit
pre-commit install
```

Then just use normal `git commit`.

Run manually:

```bash
pre-commit run --all-files
pre-commit run cargo-fmt
pre-commit run cargo-clippy
./scripts/lint.sh               # full gate without the pre-commit framework
```

Skip clippy once (slow machines / WIP):

```bash
SKIP=cargo-clippy git commit -m "wip"
```

### Option 2: Zero-dependency shell hook

A simple script lives at `scripts/pre-commit`.

```bash
chmod +x scripts/pre-commit
ln -sf ../../scripts/pre-commit .git/hooks/pre-commit
```

This version runs `cargo fmt` (main + `mcp/`) then `scripts/lint.sh clippy`.
It does **not** automatically re-stage formatting changes — `git add -u` if
fmt rewrote files. Skip clippy with `SKIP_CLIPPY=1 git commit ...`.

### Bypassing (use sparingly)

```bash
git commit --no-verify
```

### Notes for both options

- Both use your local Rust toolchain (`cargo fmt` / `clippy` / `rustfmt.toml`).
- The `mcp/` helper crate is included in the lint script and the shell hook.
- Clippy is slower than fmt alone; that is intentional so local commits match CI.
- CI enforces the same gate in the `lint` job (`.github/workflows/tests.yml`).

Using a pre-commit hook keeps formatting and clippy regressions out of PRs.

See also `.pre-commit-config.yaml`, `scripts/pre-commit`, and `scripts/lint.sh`.