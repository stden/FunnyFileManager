"""Build target/<mode>/ffm with Cargo, and hand this process over to it."""

import errno
import os
import sys
from typing import NoReturn

from launcher_output import fail, status, warning
from launcher_paths import ROOT, binary
from launcher_process import NOT_EXECUTABLE, NOT_FOUND, run


def build(mode):
    """Build ffm: release when `mode` is "release", debug for any other word."""
    status("Building FunnyFileManager...")
    release = ["--release"] if mode == "release" else []
    if run(["cargo", "build", *release]):
        fail("Build failed!")
    return status("Build completed successfully!")


def launch(mode, args) -> NoReturn:
    """Replace this process with target/<mode>/ffm, given `args`.

    A missing binary is built first, always in debug, and then the debug
    binary runs.
    """
    path = binary(mode)
    if not os.path.isfile(path):
        warning("FFM binary not found, building...")
        build("debug")
        path = binary("debug")
    status("Starting FunnyFileManager...")
    # The full path to exec, the short one as argv[0].
    executable = f"{ROOT}/{path}"
    try:
        os.execv(executable, [path, *args])
    except OSError as problem:
        print(f"{executable}: {problem.strerror}", file=sys.stderr)
        sys.exit(NOT_FOUND if problem.errno == errno.ENOENT else NOT_EXECUTABLE)
