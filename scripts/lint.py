#!/usr/bin/env python3
"""Run the Rust linter gate used by CI, pre-commit, and `./ffm.py lint`.

Usage:
  scripts/lint.py              # fmt + spelling + markdown + complexity + duplication + clippy + architecture
  scripts/lint.py fmt          # format check only
  scripts/lint.py complexity   # Python scripts' cyclomatic complexity and nesting
  scripts/lint.py duplication  # no block of 10+ lines repeated in Rust or Python
  scripts/lint.py spelling     # typos over every text file (typos.toml)
  scripts/lint.py markdown     # Markdown tables aligned, trailing spaces gone (format_markdown.py --check)
  scripts/lint.py clippy       # clippy and architecture (all targets, all features, fallback)
  scripts/lint.py clippy-fast  # clippy all targets (default features)
  scripts/lint.py fix          # auto-format (cargo fmt, then Markdown), then full clippy gate
  scripts/lint.py self-test    # spell check, script unit tests, readability limits, lint policy
  scripts/lint.py architecture # dependency ownership, layering, unsafe allowlist, file limits

Exit non-zero on the first failure. Matches `.github/workflows/tests.yml` lint job.
"""

import os
from pathlib import Path
import sys

# lint_steps.py and lint_checks.py, beside this file, hold the step runner and
# the checks; shell_path.py finds programs as bash does. Explicitly first on
# sys.path: `python3 -P` or PYTHONSAFEPATH=1 leave this file's directory out,
# and no other `lint_steps`, `lint_checks` or `shell_path` module may stand in.
sys.path.insert(0, str(Path(__file__).resolve().parent))
# pre_commit_hook.py, cargo_preflight.py and coverage.py reach the step
# runner, has_crate and ROOT through lint.
from lint_checks import (  # noqa: E402, F401
    format_crates, format_markdown_files, has_crate, python, run_architecture, run_clippy, run_clippy_matrix,
    run_spelling, self_test_spelling,
)
from lint_steps import ROOT, exit_with, run, use_utf8_output  # noqa: E402

PREFLIGHT = ROOT / "scripts/cargo_preflight.py"
HELP_WORDS = ("-h", "--help", "help")
# Modes that build nothing, so they skip the compiler-wrapper preflight.
PREFLIGHT_FREE = ("fmt", "spelling", "markdown", "complexity", "duplication")


def lint_all():
    format_crates(check=True)
    run_spelling()
    format_markdown_files(check=True)
    lint_complexity()
    lint_duplication()
    run_clippy_matrix()
    print("✓ lint OK (fmt + spelling + markdown + complexity + duplication + clippy)")


def lint_duplication():
    # Copied code drifts: a fix lands in one copy and not the other.
    run([python(), "scripts/lint_duplication.py"])


def lint_complexity():
    # Clippy holds the Rust to clippy.toml's budget; this holds the scripts.
    run([python(), "scripts/lint_complexity.py"])


def lint_fmt():
    format_crates(check=True)
    print("✓ fmt OK")


def lint_spelling():
    run_spelling()
    print("✓ spelling OK")


def lint_markdown():
    format_markdown_files(check=True)
    print("✓ markdown OK")


def lint_clippy():
    run_clippy_matrix()
    print("✓ clippy OK")


def lint_clippy_fast():
    run_clippy()
    print("✓ clippy OK (default features)")


def lint_fix():
    format_crates(check=False)
    format_markdown_files(check=False)
    run_clippy_matrix()
    print("✓ lint OK after format (fmt + markdown + clippy)")


def self_test():
    self_test_spelling()
    # The developer scripts' unit tests (scripts/tests): fake commands, no Cargo.
    run([python(), "-m", "unittest", "discover", "-s", "scripts/tests"])
    # The Markdown formatter's unit tests, beside this file.
    run([python(), "scripts/test_format_markdown.py"])
    run([python(), "scripts/test_readability.py"])
    # Runs Cargo on throwaway crates, hence the wrapper preflight.
    run([python(), "scripts/test_lint_policy.py"])


# In the order the unknown-mode message lists them.
MODES = {
    "all": lint_all,
    "fmt": lint_fmt,
    "spelling": lint_spelling,
    "markdown": lint_markdown,
    "complexity": lint_complexity,
    "duplication": lint_duplication,
    "clippy": lint_clippy,
    "clippy-fast": lint_clippy_fast,
    "fix": lint_fix,
    "self-test": self_test,
    "architecture": run_architecture,
}


def main(argv=None) -> int:
    """Run the mode the first argument names; later arguments are ignored."""
    argv = sys.argv[1:] if argv is None else argv
    use_utf8_output()
    mode = argv[0] if argv and argv[0] else "all"
    if mode in HELP_WORDS:
        print(__doc__.strip())
        return 0
    if mode not in MODES:
        print(f"unknown mode: {mode} (try: {'|'.join([*MODES, 'help'])})", file=sys.stderr)
        return 2
    if "PYTHON" in os.environ:
        # A PYTHON the caller set reaches every step resolved: empty names
        # the interpreter running lint.
        os.environ["PYTHON"] = python()
    if mode not in PREFLIGHT_FREE:
        # Fail in seconds, not minutes, when the compiler wrapper has stalled.
        run([sys.executable, PREFLIGHT])
    MODES[mode]()
    return 0


if __name__ == "__main__":
    exit_with(main)
