"""The checks that the linter gate's modes string together: format, spelling,
Markdown, Clippy and the architecture contracts.

Each prints its → line and runs its steps through lint_steps.run, so the first
failing step ends the gate with that step's status.
"""

import os
import subprocess
import sys

import shell_path
from lint_steps import NOT_FOUND, ROOT, run, run_step


# Crates besides the root one that the gate covers when their manifest exists.
EXTRA_CRATES = ("mcp", "colorer-sys")
TYPOS_MISSING = "typos is not installed — run: cargo install typos-cli --locked"


def python():
    """The interpreter for the Python checks: $PYTHON, else the one running lint."""
    return os.environ.get("PYTHON") or sys.executable


def has_crate(crate):
    return (ROOT / crate / "Cargo.toml").is_file()


def format_crates(check):
    """Run `cargo fmt --all` in every crate; with `check`, only verify the format."""
    command = ["cargo", "fmt", "--all", *(["--", "--check"] if check else [])]
    print("→", *command)
    run(command)
    if has_crate("mcp"):
        print("→", *command, "(mcp/)")
        run(command, ROOT / "mcp")
    if has_crate("colorer-sys"):
        run(command, ROOT / "colorer-sys")


def format_markdown_files(check):
    """Align the Markdown tables and trim trailing spaces; with `check`, only verify them."""
    run([python(), "scripts/format_markdown.py", *(["--check"] if check else [])])


def require_typos():
    """Stop with 127 unless typos is on PATH: a missing checker fails the gate
    instead of skipping it."""
    if not shell_path.find("typos", cwd=ROOT):
        print(TYPOS_MISSING, file=sys.stderr)
        sys.exit(NOT_FOUND)


# Misspellings in docs, comments, strings and identifiers; typos.toml holds the
# project words.
def run_spelling():
    require_typos()
    print("→ typos")
    run(["typos"])


def self_test_spelling():
    """The spell check must reject a real misspelling and accept the project words."""
    require_typos()
    run(["typos"], stdout=subprocess.DEVNULL)
    misspelling = b"teh\n"  # spellchecker:disable-line
    if run_step(["typos", "-"], input=misspelling, stdout=subprocess.DEVNULL) == 0:
        print("spelling self-test: typos accepted a known misspelling", file=sys.stderr)
        sys.exit(1)
    run(["typos", "-"], input=b"Ratatui draws a Detai&led menu\n")
    print("✓ spelling self-test OK")


# Lint levels in Cargo.toml and thresholds in clippy.toml apply to every target.
def run_clippy(*options):
    # Check production, tests, examples and benchmarks with the same limits.
    print(f"→ cargo clippy --all-targets {' '.join(options)} -- -D warnings")
    run(["cargo", "clippy", "--all-targets", *options, "--", "-D", "warnings"])


def run_clippy_matrix():
    run_clippy()
    # Combined features include helper-gated BDD targets and feature interactions.
    run_clippy("--all-features")
    # Compile the built-in highlighter without the optional Colorer adapter.
    run_clippy("--no-default-features", "--features", "test-helpers")
    for crate in EXTRA_CRATES:
        if has_crate(crate):
            print(f"→ cargo clippy --all-targets -- -D warnings ({crate}/)")
            run(["cargo", "clippy", "--all-targets", "--", "-D", "warnings"], ROOT / crate)
    run_architecture()


def run_architecture():
    print("→ architecture contracts (ownership, dependencies, unsafe allowlist, test separation, file limits)")
    run([
        "cargo", "test", "--test", "module_boundaries_test",
        "--test", "unsafe_boundary_test", "--test", "file_line_limit_test",
    ])
    run([python(), "scripts/test_readability.py"])
    run([python(), "scripts/test_recording.py"])
