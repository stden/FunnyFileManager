"""What ./ffm.py prints of its own: coloured status lines, straight to stdout.

Every line goes out at once through os.write, so it stays in order with the
output of the commands that ffm.py runs.
"""

import os
import sys
from typing import NoReturn


# The file descriptor itself: sys.stdout is None when it was closed.
STDOUT = 1

GREEN = "\033[0;32m"
RED = "\033[0;31m"
YELLOW = "\033[1;33m"
NC = "\033[0m"  # No colour


def say(line):
    """Print one line and return 0, or 1 if it could not be written.

    The line is written at once, unbuffered, so it stays in order with the
    output of the commands around it. Paths keep the exact bytes the
    environment gave them, and this file's text stays UTF-8 in any locale.
    A failed write is reported on stderr and the script goes on; a closed
    pipe ends it by SIGPIPE, without a word.
    """
    data = f"{line}\n".encode("utf-8", "surrogateescape")
    try:
        while data:
            data = data[os.write(STDOUT, data):]
    except OSError as problem:
        print(f"{sys.argv[0]}: write error: {problem.strerror}", file=sys.stderr)
        return 1
    return 0


def status(message):
    return say(f"{GREEN}[FFM]{NC} {message}")


def error(message):
    return say(f"{RED}[ERROR]{NC} {message}")


def warning(message):
    return say(f"{YELLOW}[WARNING]{NC} {message}")


def fail(message, *hints) -> NoReturn:
    """Print an error and its hint lines, then stop with status 1."""
    error(message)
    for hint in hints:
        say(hint)
    sys.exit(1)
