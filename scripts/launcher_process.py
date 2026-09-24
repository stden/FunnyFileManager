"""How ./ffm.py runs a command: found as bash finds it, waited for through
Ctrl-C, and ended with the status that the shell would report.
"""

import errno
import os
import shlex
import signal
import subprocess
import sys

from shell_path import resolve


# The status of a command that cannot start: a file it needs is missing, or another reason.
NOT_FOUND = 127
NOT_EXECUTABLE = 126
# A command killed by one of these is not reported: Ctrl-C, or a reader that went away.
QUIET_SIGNALS = (signal.SIGINT, signal.SIGPIPE)


def die_by(signal_number):
    """End this process killed by `signal_number`, so a calling script sees it."""
    signal.signal(signal_number, signal.SIG_DFL)
    os.kill(os.getpid(), signal_number)


# TODO(DRY): scripts/lint_steps.py (for lint, the pre-commit hook and the
# preflight) and install_mcp.py carry their own copy of this command runner.
# Move them into one shared module once every script port has landed.
def run(command, **options):
    """Run a command to the end and return its exit status.

    - A bare name is looked up as find() looks it up.
    - Ctrl-C reaches the command as well, and the command decides what it
      means: this script waits for it. If the command dies of it, so does
      this script; if the command exits instead, the script goes on.
    - A command that cannot start gets 127 when a file it needs is missing,
      126 otherwise, and the reason is printed on stderr.
    """
    executable = resolve(str(command[0]))
    interrupted = False

    def note_interrupt(_signal_number, _frame):
        nonlocal interrupted
        interrupted = True

    handler = signal.getsignal(signal.SIGINT)
    # A Ctrl-C ignored on entry, as for `ffm.py &` in a script, stays ignored.
    if handler is not signal.SIG_IGN:
        signal.signal(signal.SIGINT, note_interrupt)
    try:
        returncode = subprocess.run(command, executable=executable, check=False, **options).returncode
    except OSError as problem:
        print(f"{problem.filename}: {problem.strerror}", file=sys.stderr)
        return NOT_FOUND if problem.errno == errno.ENOENT else NOT_EXECUTABLE
    finally:
        signal.signal(signal.SIGINT, handler)
    if interrupted and returncode == -signal.SIGINT:
        die_by(signal.SIGINT)
    return exit_status(returncode, command)


def exit_status(returncode, command):
    """A finished command's status: its own, or 128 + N after signal N.

    The signal is named on stderr with the command, unless it is in QUIET_SIGNALS.
    """
    if returncode >= 0:
        return returncode
    signal_number = -returncode
    if signal_number not in QUIET_SIGNALS:
        print(f"{signal.strsignal(signal_number)}  {shlex.join(map(str, command))}", file=sys.stderr)
    return 128 + signal_number


def run_or_exit(command):
    """Run a command that must pass: any failure stops the script with status 1."""
    if run(command):
        sys.exit(1)
