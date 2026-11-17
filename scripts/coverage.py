#!/usr/bin/env python3
"""Require exactly 100% line coverage of all production Rust and Python code.

Both collectors finish before coverage is judged, so a Rust coverage gap
does not hide Python results. Test failures and unavailable tools stop with
their original status. Reports persist in target/coverage-full; set
FFM_COVERAGE_DIR to retain them elsewhere. --rust and --python select one
collector for platform-specific CI jobs.

Rust needs cargo-llvm-cov and rustup's llvm-tools component. Python needs
coverage>=7.10 (python -m pip install coverage==7.17.0).
"""

import argparse
import errno
import os
from pathlib import Path
import subprocess
import sys


sys.path.insert(0, str(Path(__file__).resolve().parent))
import coverage_report  # noqa: E402
import lint_steps  # noqa: E402


CHECKERS = {"Rust": coverage_report.check_rust, "Python": coverage_report.check_python}


def arguments(argv):
    parser = argparse.ArgumentParser(description=__doc__)
    selection = parser.add_mutually_exclusive_group()
    selection.add_argument("--rust", action="store_true", help="measure Rust only")
    selection.add_argument("--python", action="store_true", help="measure Python only")
    return parser.parse_args(argv)


def collect(directory, options):
    """Collect native reports without judging coverage between backends."""
    reports = []
    if not options.python:
        import coverage_rust
        reports.append(("Rust", coverage_rust.run(directory / "rust")))
    if not options.rust:
        import coverage_python
        reports.append(("Python", [coverage_python.run(directory / "python")]))
    return reports


def check_reports(reports):
    """Inspect every collected report, including those after an earlier gap."""
    passed = True
    for kind, paths in reports:
        if not paths:
            print(f"coverage: {kind} collector produced no reports", file=sys.stderr)
            passed = False
            continue
        for path in paths:
            try:
                covered = CHECKERS[kind](path)
                passed = covered and passed
            except (OSError, TypeError, ValueError) as problem:
                print(f"coverage: {path}: {problem}", file=sys.stderr)
                passed = False
    return 0 if passed else 1


def main(argv=None):
    lint_steps.use_utf8_output()
    options = arguments(sys.argv[1:] if argv is None else argv)
    directory = Path(os.environ.get("FFM_COVERAGE_DIR", lint_steps.ROOT / "target/coverage-full")).resolve()
    try:
        directory.mkdir(parents=True, exist_ok=True)
        reports = collect(directory, options)
    except subprocess.CalledProcessError as problem:
        print(f"coverage: command failed: {problem.cmd}", file=sys.stderr)
        return lint_steps.exit_status(problem.returncode, problem.cmd)
    except OSError as problem:
        print(f"coverage: {problem}", file=sys.stderr)
        return 127 if problem.errno == errno.ENOENT else 126
    return check_reports(reports)


if __name__ == "__main__":
    lint_steps.exit_with(main)
