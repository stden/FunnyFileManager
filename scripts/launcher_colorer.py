"""./ffm.py colorer: build Colorer, the engine of Far's FarColorer plugin, and
fetch Far's syntax schemes beside it.
"""

import os
import subprocess
import sys

from launcher_output import fail, status
from launcher_paths import env_or_home
from launcher_process import run, run_or_exit
from shell_path import find


COLORER_REPOSITORIES = "https://github.com/colorer"


def fetch(checkout, label):
    """Fast-forward a Colorer clone, keeping it if that fails, or make a shallow one."""
    if os.path.isdir(f"{checkout}/.git"):
        status(f"Updating {label}...")
        run(["git", "-C", checkout, "pull", "--ff-only", "--quiet"])
        return
    status(f"Cloning {label}...")
    url = f"{COLORER_REPOSITORIES}/{os.path.basename(checkout)}"
    run_or_exit(["git", "clone", "--depth", "1", url, checkout])


def colorer_command(_args):
    """Install Colorer, the engine of Far's FarColorer plugin, plus Far's schemes.

    Kept out of `cargo build` on purpose. Cloning and CMake-building a C++
    library would turn every build of the file manager into something that
    needs the network and several minutes; this does it once, deliberately,
    and the build script then finds the result.
    """
    if not find("cmake"):
        fail(
            "cmake is required to build Colorer.",
            "  Debian/Ubuntu: sudo apt-get install cmake g++ libicu-dev libxml2-dev zlib1g-dev libminizip-dev",
        )
    colorer_home = f"{env_or_home('XDG_DATA_HOME', '.local/share')}/ffm"
    library = f"{colorer_home}/Colorer-library"
    try:
        os.makedirs(colorer_home, exist_ok=True)
    except OSError as problem:
        # Reported, not fatal: the clone that follows fails with its own reason.
        print(f"mkdir: cannot create directory '{problem.filename}': {problem.strerror}", file=sys.stderr)
    fetch(library, "Colorer-library")
    status("Building Colorer-library (this takes a few minutes)...")
    configure = [
        "cmake", "-S", library, "-B", f"{library}/_build", "-DCMAKE_BUILD_TYPE=Release",
        "-DCOLORER_USE_VCPKG=OFF", "-DCOLORER_BUILD_TOOLS=OFF", "-DCOLORER_BUILD_TEST=OFF",
    ]
    if run(configure, stdout=subprocess.DEVNULL):
        fail("CMake configure failed. Missing libicu-dev / libxml2-dev / zlib1g-dev / libminizip-dev?")
    run_or_exit(["cmake", "--build", f"{library}/_build", "--parallel"])
    fetch(f"{colorer_home}/Colorer-schemes", "the syntax schemes")
    status(f"Colorer installed under {colorer_home}")
    return status("Rebuild to pick it up:  ./ffm.py install")
