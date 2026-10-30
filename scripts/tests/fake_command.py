"""The recording fake that stands in for a command in a Sandbox.

FAKE is a fake's whole program: it appends one JSON line (name, arguments,
working directory, environment) to the call log, runs the test's effect,
then writes the given output and exits with the given status. Call is one
such line, read back.
"""

from typing import NamedTuple


FAKE = """\
#!{python}
import json, os, sys
args = sys.argv[1:]
call = {{"name": {name!r}, "args": args, "cwd": os.getcwd(), "env": dict(os.environ)}}
with open({log!r}, "a", encoding="utf-8") as log:
    log.write(json.dumps(call) + "\\n")
{effect}
sys.stdout.write({stdout!r})
sys.stderr.write({stderr!r})
sys.exit({status!r})
"""


class Call(NamedTuple):
    """One recorded run of a fake command."""

    name: str
    args: list[str]
    cwd: str
    env: dict[str, str]
