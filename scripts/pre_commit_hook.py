#!/usr/bin/env python3
"""Git pre-commit hook: format Rust code, then run the clippy matrix CI enforces.

Needs only python3 (no pre-commit framework, no pip packages). Install it once
per clone:
  ln -sf ../../scripts/pre_commit_hook.py .git/hooks/pre-commit

This will run on every `git commit`, also in linked worktrees, and checks the
working tree being committed. A tree without scripts/lint.py, such as a linked
worktree on a branch from before the Python scripts, stops the commit with
status 1 instead of checking another tree: run that branch's own lint, then
commit with --no-verify. Run by hand, the hook checks the checkout it belongs
to. Arguments are ignored.
Formatting changes are applied to the working tree — stage them if fmt rewrote
files (`git add -u`) and commit again.

To bypass (not recommended for normal work):
  git commit --no-verify

Faster alternative when iterating:
  SKIP_CLIPPY=1 git commit ...
"""

import os
from pathlib import Path
import sys


# git sets it for the pre-commit hook, and so tells a commit from a run by hand.
RUN_BY_GIT = "GIT_INDEX_FILE"


def scripts_directory():
    """scripts/ of the working tree to check.

    git starts the hook at the top of the working tree it commits. Every
    linked worktree shares the main checkout's .git/hooks, whose link leads
    into the main checkout's scripts/, so a commit uses the working
    directory's own scripts/. Run by hand, the hook checks the checkout it
    belongs to.
    """
    if RUN_BY_GIT in os.environ:
        return Path.cwd() / "scripts"
    return Path(__file__).resolve().parent


SCRIPTS = scripts_directory()
if not (SCRIPTS / "lint.py").is_file():
    # Checking another tree instead would pass or block this commit on code it does not hold.
    sys.exit(f"pre-commit: {SCRIPTS.parent} has no scripts/lint.py to check it with; "
             "run its own lint, then commit with --no-verify")
# First on sys.path, even under `python3 -P` or PYTHONSAFEPATH=1, which leave
# this file's directory out: no other `lint` module may stand in.
sys.path.insert(0, str(SCRIPTS))
import lint  # noqa: E402


def main():
    lint.use_utf8_output()
    print("→ Running cargo fmt (auto-format)...")
    lint.run(["cargo", "fmt", "--all"])
    if lint.has_crate("mcp"):
        print("→ Formatting mcp/ crate...")
        lint.run(["cargo", "fmt", "--all"], lint.ROOT / "mcp")
    if os.environ.get("SKIP_CLIPPY") == "1":
        print("→ Skipping clippy (SKIP_CLIPPY=1).")
        return 0
    print("→ Running Rust linter gate (clippy)...")
    status = lint.main(["clippy"])
    if status:
        return status
    print("→ Pre-commit checks passed.")
    print("   If fmt rewrote files, stage them (git add -u) before committing again.")
    return 0


if __name__ == "__main__":
    lint.exit_with(main)
