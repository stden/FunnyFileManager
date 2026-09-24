"""How scripts/k9s_eks.py speaks and runs programs, as the shell functions it
replaced did: messages after the k9s-eks: prefix, programs found on PATH as
bash finds them, and exit statuses as the shell reports them.
"""

import os
import signal
import subprocess
import sys
from typing import NoReturn

import shell_path


MESSAGE_PREFIX = "k9s-eks: "
MISSING_COMMAND = 127
USAGE_ERROR = 2
# The shell names the signal that killed a foreground command, except these.
UNREPORTED_SIGNALS = (signal.SIGINT, signal.SIGPIPE)
# `ls` ends like the old `aws | tr` did when its reader stopped early.
KILLED_BY_SIGPIPE = 128 + signal.SIGPIPE


def complain(message):
    """Print `message` on stderr, after the script's prefix."""
    print(MESSAGE_PREFIX + message, file=sys.stderr)


def fail(message: str, status: int) -> NoReturn:
    """Print `message` on stderr and end the script with `status`."""
    complain(message)
    raise SystemExit(status)


def has_command(command):
    """True if `command` is on PATH; otherwise say that it is missing."""
    if shell_path.find(command):
        return True
    complain(f"missing required command: {command}")
    return False


def run(command, **options):
    """subprocess.run for aws, kubectl, k9s or fzf, found on PATH as bash finds them."""
    return subprocess.run(command, executable=shell_path.find(command[0]), check=False, **options)


def require(command):
    if not has_command(command):
        raise SystemExit(MISSING_COMMAND)


def shell_status(returncode):
    """A child's exit status as the shell reports it: 128+N if signal N killed it."""
    return 128 - returncode if returncode < 0 else returncode


def run_tool(command):
    """Run kubectl or k9s in the foreground; return its status as the shell would.

    Like the shell, name the signal that killed it ("Killed", "Terminated").
    """
    returncode = run(command).returncode
    if returncode < 0 and -returncode not in UNREPORTED_SIGNALS:
        print(signal.strsignal(-returncode), file=sys.stderr)
    return shell_status(returncode)


def write_stdout(data):
    """Write bytes to stdout; False if the reader has gone."""
    try:
        sys.stdout.buffer.write(data)
        sys.stdout.buffer.flush()
    except BrokenPipeError:
        # Point stdout at /dev/null, so that Python's flush at exit stays quiet.
        os.dup2(os.open(os.devnull, os.O_WRONLY), sys.stdout.fileno())
        return False
    return True


def echo(output):
    """Show aws's output on stderr, byte for byte, plus a newline."""
    sys.stderr.buffer.write(output + b"\n")
    sys.stderr.buffer.flush()
