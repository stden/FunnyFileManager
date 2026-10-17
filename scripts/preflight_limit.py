"""FFM_WRAPPER_PROBE_SECS, the time that cargo_preflight.py gives the compiler
wrapper, read as timeout(1) reads a duration.
"""

import os
import re
import sys


DEFAULT_LIMIT = "20"
# A duration as timeout(1) reads it: optional blanks and sign, then a C
# floating-point number (decimal, hexadecimal or inf), then an optional unit.
# A leading "-" made it an option of timeout(1), and so invalid.
DURATION = re.compile(
    r"(?!-)\s*(?P<number>[+-]?(?:"
    r"(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?"
    r"|0[xX](?:[\da-fA-F]+\.?[\da-fA-F]*|\.[\da-fA-F]+)(?:[pP][+-]?\d+)?"
    r"|(?i:inf(?:inity)?)))(?P<unit>[smhd]?)",
    re.ASCII,
)
DURATION_UNITS = {"": 1, "s": 1, "m": 60, "h": 60 * 60, "d": 24 * 60 * 60}


def limit_text():
    """FFM_WRAPPER_PROBE_SECS as the caller wrote it, or the default."""
    return os.environ.get("FFM_WRAPPER_PROBE_SECS") or DEFAULT_LIMIT


def limit_seconds(text):
    """The limit in seconds, read as timeout(1) reads a duration; None means no limit.

    A duration timeout(1) refused ends the preflight with status 1.
    """
    match = DURATION.fullmatch(text)
    seconds = parse_number(match["number"]) * DURATION_UNITS[match["unit"]] if match else -1
    if seconds < 0:  # -0 passes, as it did for timeout(1)
        sys.exit(f"error: FFM_WRAPPER_PROBE_SECS must be seconds, optionally suffixed s, m, h or d; not {text!r}")
    # timeout(1) waited for ever on 0, and in effect on inf.
    return seconds if 0 < seconds < float("inf") else None


def parse_number(number):
    """A number DURATION matched: decimal, hexadecimal or inf."""
    return float.fromhex(number) if "x" in number.lower() else float(number)
