#!/usr/bin/env python3
"""Decide a CI mutation-testing step from cargo-mutants' exit status and mutants.out.

Usage: scripts/mutants_verdict.py STATUS MUTANTS_OUT

STATUS is the exit status of `cargo mutants`; MUTANTS_OUT is the mutants.out
directory it wrote. The verdict, in this order:

- A status other than 0 (every mutant caught), 2 (missed) or 3 (timeouts)
  fails as it is: the baseline failed, the diff does not match the tree, or
  the command was wrong.
- A missed mutant fails: a changed line can break without a test noticing.
  cargo-mutants exits 3 in preference to 2 when a run has both, so the
  verdict reads missed.txt rather than trusting the status.
- Timeouts with no caught mutant fail: every mutant that built ran out of
  time, so nothing was tested. The job's CPUs or the timeout are wrong, not
  the tests.
- Otherwise a timed-out mutant passes, as one that hangs the suite has been
  detected, but each is named in a warning annotation.

A diff with no mutants writes no files and passes.
"""

from pathlib import Path
import sys


USAGE = "usage: scripts/mutants_verdict.py STATUS MUTANTS_OUT"
# cargo-mutants' exit statuses that carry a result for this script to judge.
ALL_CAUGHT = 0
MISSED = 2
TIMEOUT = 3
BAD_USAGE = 2


def listed(directory, name):
    """The mutants in `directory`/`name`, one per line; none when there is no file."""
    try:
        text = (directory / name).read_text(encoding="utf-8")
    except FileNotFoundError:
        return []
    return [line for line in text.splitlines() if line.strip()]


def verdict(status, directory):
    """Print what the step found and return its exit status."""
    if status not in (ALL_CAUGHT, MISSED, TIMEOUT):
        print(f"cargo mutants exited {status}: no mutants were judged (see its output above)", file=sys.stderr)
        return status
    missed = listed(directory, "missed.txt")
    if status == MISSED or missed:
        print(f"{len(missed)} mutant(s) survived; kill each with a test that observes the change:", file=sys.stderr)
        for mutant in missed:
            print(f"  {mutant}", file=sys.stderr)
        return MISSED
    timeouts = listed(directory, "timeout.txt")
    if status == TIMEOUT and not listed(directory, "caught.txt"):
        print(f"all {len(timeouts)} mutant(s) that built timed out, so nothing was tested:", file=sys.stderr)
        for mutant in timeouts:
            print(f"  {mutant}", file=sys.stderr)
        return TIMEOUT
    for mutant in timeouts:
        print(f"::warning title=Mutant timed out (counted as caught)::{mutant}")
    return ALL_CAUGHT


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    if argv[:1] in (["-h"], ["--help"]):
        print(__doc__.strip())
        return 0
    if len(argv) != 2 or not argv[0].isdigit():
        print(USAGE, file=sys.stderr)
        return BAD_USAGE
    return verdict(int(argv[0]), Path(argv[1]))


if __name__ == "__main__":
    sys.exit(main())
