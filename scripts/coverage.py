#!/usr/bin/env python3
"""Run the line-coverage gate CI enforces: the root crate's suite under llvm-cov.

Usage: scripts/coverage.py    (arguments are ignored unless the first is -h or --help)

This file is the one definition of the gate, for CI and for `./ffm.py
coverage` alike: the features, the files left out of the measurement and
the floor. It runs every root test with `cargo llvm-cov nextest` (unit,
integration, KeePass and the TUI BDD scenarios), prints llvm-cov's table and
the uncovered lines, then fails when the measured lines fall under FLOOR.

Needs cargo-llvm-cov, cargo-nextest and the llvm-tools component:
  cargo install cargo-llvm-cov cargo-nextest --locked
  rustup component add llvm-tools

The last line gives the exact share, cut to two decimals and never rounded
up: llvm-cov's own table prints 97.998% as "98.00%", which fails a 98% gate.
"""

import json
import os
from pathlib import Path
import sys
import tempfile

# lint.py, beside this file, runs a step and ends the process as the other
# scripts do; shell_path.py finds programs as bash does. Explicitly first on
# sys.path: `python3 -P` or PYTHONSAFEPATH=1 leave this file's directory out,
# and no other `lint` or `shell_path` module may stand in.
sys.path.insert(0, str(Path(__file__).resolve().parent))
import lint  # noqa: E402
import shell_path  # noqa: E402


HELP_WORDS = ("-h", "--help")
# test-helpers compiles the helper-gated test targets; the default features
# (colorer, keepass) come with it. s3 is left out: its suite runs in CI's BDD job.
FEATURES = "test-helpers"
# Left out of the measurement (they still compile and run in the suite):
# - main.rs, bootstrap.rs, editor.rs: real-TTY and external-process entry points
# - clipboard/system.rs: the OS clipboard, which a headless runner does not have
# - ops/shell/tty.rs: drains crossterm's live event queue for cancel keys
# - highlight/colorer.rs and colorer/: the Colorer FFI wrapper; the gate does not
#   build Colorer-library, so CI's Colorer job tests it with the engine present
# - test files (tests*.rs, *_tests.rs, tests/, test_support.rs, test_support/):
#   tests are not code under test; their unreached lines are assertion-failure
#   arms that never fire in a passing suite
IGNORED = (
    r"src/(main|editor|bootstrap)\.rs$"
    r"|src/clipboard/system\.rs$"
    r"|src/ops/shell/tty\.rs$"
    r"|src/highlight/colorer(\.rs$|/)"
    r"|/tests[_./]"
    r"|_tests\.rs$"
    r"|/test_support(/|\.rs$)"
)
# Percent of the measured lines the suite must execute. What is left is nearly
# all error handling that needs fault injection to reach (the `?` on a
# create_dir, a write that fails halfway), plus lines no test can reach: a
# cfg!(windows) arm, an unreachable!, try_from on values that cannot be out of
# range. llvm also counts a line missed when any instantiation leaves it
# unexecuted. Raise the floor as those paths gain tests; never lower it to hide
# a regression.
FLOOR = 98
# The empty rustc-wrapper in .cargo/config.toml means "none" to Cargo, but
# cargo-llvm-cov runs it as ` rustc`, which fails; env(1) is a wrapper that
# changes nothing.
PASS_THROUGH = "env"


def use_pass_through_wrapper():
    """Give Cargo a compiler wrapper cargo-llvm-cov can run, unless the caller chose one.

    A wrapper in RUSTC_WRAPPER or CARGO_BUILD_RUSTC_WRAPPER, not empty, is
    kept. Otherwise CARGO_BUILD_RUSTC_WRAPPER names env(1), and an empty
    RUSTC_WRAPPER, which would outrank it, goes: both mean "no wrapper".
    """
    if os.environ.get("RUSTC_WRAPPER") or os.environ.get("CARGO_BUILD_RUSTC_WRAPPER"):
        return
    wrapper = shell_path.find(PASS_THROUGH)
    if not wrapper:
        sys.exit(f"coverage: cargo-llvm-cov needs a pass-through compiler wrapper and "
                 f"`{PASS_THROUGH}` is not on PATH; set CARGO_BUILD_RUSTC_WRAPPER")
    os.environ.pop("RUSTC_WRAPPER", None)
    os.environ["CARGO_BUILD_RUSTC_WRAPPER"] = wrapper


def percent_cut(covered, count):
    """covered/count as a percentage with two decimals, cut rather than rounded."""
    hundredths = covered * 100 * 100 // count
    return f"{hundredths // 100}.{hundredths % 100:02d}%"


def line_totals(summary_path):
    """(covered, count) of the measured lines, from llvm-cov's JSON summary."""
    try:
        with open(summary_path, encoding="utf-8") as summary:
            lines = json.load(summary)["data"][0]["totals"]["lines"]
        return int(lines["covered"]), int(lines["count"])
    except (OSError, ValueError, LookupError, TypeError) as problem:
        sys.exit(f"coverage: cannot read llvm-cov's summary {summary_path}: {problem}")


def verdict(covered, count):
    """Print the gate's result and return the exit status: 0 at or over FLOOR, 1 under."""
    if count == 0:
        print("coverage: llvm-cov measured no lines; the gate cannot pass on nothing", file=sys.stderr)
        return 1
    share = f"{percent_cut(covered, count)} of {count} measured lines ({covered} covered)"
    # Whole numbers: covered/count >= FLOOR/100 without a float.
    if covered * 100 >= FLOOR * count:
        print(f"✓ line coverage {share}, floor {FLOOR}%")
        return 0
    print(f"✗ line coverage {share} is under the {FLOOR}% floor", file=sys.stderr)
    return 1


def main(argv=None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    lint.use_utf8_output()
    if argv[:1] and argv[0] in HELP_WORDS:
        print(__doc__.strip())
        return 0
    use_pass_through_wrapper()
    measured = ["--ignore-filename-regex", IGNORED]
    tests = ["cargo", "llvm-cov", "nextest", "--features", FEATURES, *measured, "--show-missing-lines"]
    print("→", *tests)
    lint.run(tests)
    with tempfile.TemporaryDirectory(prefix="ffm-coverage-") as directory:
        summary = Path(directory) / "summary.json"
        # The same profile data again, as JSON: exact counts, not the table's rounding.
        report = ["cargo", "llvm-cov", "report", *measured, "--json", "--summary-only", "--output-path", summary]
        print("→", *report)
        lint.run(report)
        covered, count = line_totals(summary)
    return verdict(covered, count)


if __name__ == "__main__":
    lint.exit_with(main)
