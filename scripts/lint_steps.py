"""Run the linter gate's steps, and end the process, as the bash scripts did.

lint.py re-exports these helpers: pre_commit_hook.py, cargo_preflight.py and
coverage.py reach them as lint.run, lint.exit_with and so on.
"""

import errno
import os
from pathlib import Path
import shlex
import signal
import subprocess
import sys

import shell_path


ROOT = Path(__file__).resolve().parents[1]
# The status of a step that cannot start: a file it needs is missing, or another reason.
NOT_FOUND = 127
NOT_EXECUTABLE = 126
# A step killed by one of these is not reported: Ctrl-C, or a reader that went
# away. Windows has no SIGPIPE.
QUIET_SIGNALS = (signal.SIGINT, getattr(signal, "SIGPIPE", None))


# The helpers from here to use_utf8_output() also serve pre_commit_hook.py and
# cargo_preflight.py, so the three run a step and end the one same way.
# TODO(DRY): scripts/launcher_process.py (for ffm.py) and install_mcp.py carry
# their own copy of this step runner. Move them into one shared module once
# every script port has landed.
def run_step(command, cwd=ROOT, **options):
    """Run one step to the end and return its exit status.

    - The step runs in `cwd`, with PWD naming it for tools that read PWD.
    - A bare program name is looked up as bash would from `cwd` (shell_path).
    - Ctrl-C reaches the step as well, and the step decides what it means:
      the script waits for it. If the step dies of it, KeyboardInterrupt is
      raised; if the step exits instead, its status counts as usual.
    - A step that cannot start gets 127 when a file it needs is missing, 126
      otherwise, and the reason is printed on stderr.
    """
    executable = shell_path.resolve(str(command[0]), cwd)
    interrupted = False

    def note_interrupt(_signal_number, _frame):
        nonlocal interrupted
        interrupted = True

    handler = signal.getsignal(signal.SIGINT)
    # A Ctrl-C ignored on entry stays ignored, here and in the step.
    if handler is not signal.SIG_IGN:
        signal.signal(signal.SIGINT, note_interrupt)
    try:
        environment = {**os.environ, "PWD": str(cwd)}
        returncode = subprocess.run(
            command, executable=executable, cwd=cwd, env=environment, check=False, **options
        ).returncode
    except OSError as error:
        print(f"{error.filename}: {error.strerror}", file=sys.stderr)
        return NOT_FOUND if error.errno == errno.ENOENT else NOT_EXECUTABLE
    finally:
        signal.signal(signal.SIGINT, handler)
    if interrupted and returncode == -signal.SIGINT:
        raise KeyboardInterrupt
    return exit_status(returncode, command)


def exit_status(returncode, command):
    """A finished step's status: its own, or 128 + N after signal N.

    The signal is named on stderr with the step, unless it is in QUIET_SIGNALS.
    """
    if returncode >= 0:
        return returncode
    signal_number = -returncode
    if signal_number not in QUIET_SIGNALS:
        print(f"{signal.strsignal(signal_number)}  {shlex.join(map(str, command))}", file=sys.stderr)
    return 128 + signal_number


def run(command, cwd=ROOT, **options):
    """Run a step that must pass: a failure ends the script with the step's status."""
    status = run_step(command, cwd, **options)
    if status:
        sys.exit(status)


def exit_with(main):
    """Exit with main()'s status. Ctrl-C ends the process by SIGINT, without a
    traceback, so that a calling script sees the interrupt and stops as well.

    A reader that goes away, as `| head` does, ends it by SIGPIPE, silently,
    as it ended the bash scripts: CPython starts with SIGPIPE ignored.
    """
    if hasattr(signal, "SIGPIPE"):  # Windows has none
        signal.signal(signal.SIGPIPE, signal.SIG_DFL)
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        signal.signal(signal.SIGINT, signal.SIG_DFL)
        os.kill(os.getpid(), signal.SIGINT)


def use_utf8_output():
    """UTF-8 whatever the locale, for the → and ✓ marks and the em dash.

    A path that is not UTF-8 goes out as its own bytes. stdout is line
    buffered, so each line is out before the next step writes its own.
    """
    if sys.stdout:
        sys.stdout.reconfigure(encoding="utf-8", errors="surrogateescape", line_buffering=True)
    if sys.stderr:
        sys.stderr.reconfigure(encoding="utf-8", errors="surrogateescape")
