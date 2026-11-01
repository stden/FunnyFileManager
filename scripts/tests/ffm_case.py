"""The sandbox checkout and the expected output that the tests of ffm.py share.

scripts/cargo_preflight.py and scripts/lint.py are recording fakes without the
executable bit, so only `<interpreter> <file>` can run them, as ffm.py must.
"""

import subprocess
import sys
import unittest

from sandbox import Sandbox
from script_files import LAUNCHER


def status(message):
    return f"\033[0;32m[FFM]\033[0m {message}\n"


def error(message):
    return f"\033[0;31m[ERROR]\033[0m {message}\n"


def warning(message):
    return f"\033[1;33m[WARNING]\033[0m {message}\n"


RUST_MISSING = (
    error("Rust is not installed!")
    + "Please install Rust from https://rustup.rs/\n"
    + "Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh\n"
)
# Every command line that runs the Rust check first; [] is no command at all.
RUST_CHECKED = (
    ["build"], ["release"], ["test"], ["lint"], ["coverage"],
    ["install"], ["reinstall"], [], ["somewhere"], ["--version"],
)
BUILD_STARTED = status("Building FunnyFileManager...")
BUILDING = BUILD_STARTED + status("Build completed successfully!")
STARTING = status("Starting FunnyFileManager...")
NO_BINARY = warning("FFM binary not found, building...")
HELP = """\
FunnyFileManager Build & Run Script

Usage: ./ffm.py [command] [options]

Commands:
  (no command)     Build (if needed) and run FFM in debug mode
  build [mode]     Build FFM (mode: debug|release, default: debug)
  release          Build in release mode and run
  run              Run FFM (assumes already built)
  test             Run all tests
  lint [mode]      Rust linter gate (fmt + spelling + markdown + complexity +
                   duplication + clippy; see scripts/lint.py)
  coverage         CI's line-coverage gate (llvm-cov; see scripts/coverage.py)
  clean            Clean build artifacts
  colorer          Install Colorer + Far's syntax schemes, then rebuild
  install          Rebuild and install FFM to ~/.cargo/bin (--force)
  reinstall        Same as install
  help             Show this help message

Examples:
  ./ffm.py                  # Build and run in debug mode
  ./ffm.py release          # Build and run in release mode
  ./ffm.py build release    # Only build in release mode
  ./ffm.py test             # Run tests
  ./ffm.py lint             # the whole gate, clippy feature matrix included
  ./ffm.py lint clippy-fast # all targets, default features
"""
CARGO_MISSING = "cargo: No such file or directory\n"


class LauncherTest(unittest.TestCase):
    """A sandbox checkout holding ffm.py, with its helper scripts and Cargo faked."""

    def setUp(self):
        self.sandbox = Sandbox(self)
        self.repo = self.sandbox.repo
        # ffm.py, the launcher modules it imports, and shell_path.py, through
        # which it finds its programs.
        for relative in LAUNCHER:
            self.sandbox.copy(relative)
        for name in ("cargo_preflight", "lint", "coverage"):
            helper = self.sandbox.fake(name, path=self.repo / f"scripts/{name}.py")
            helper.chmod(0o644)
        self.sandbox.fake("cargo")

    def ffm(self, *args, env=None, cwd=None):
        return self.sandbox.run("ffm.py", *args, env=env, cwd=cwd)

    def ffm_bytes(self, *args, env=None, unset=(), stdout=subprocess.PIPE):
        """Run ffm.py in the sandbox with raw bytes out; `unset` drops variables."""
        environment = self.sandbox.env(env)
        for name in unset:
            del environment[name]
        return subprocess.run(
            [sys.executable, self.repo / "ffm.py", *args], cwd=self.repo, env=environment,
            stdout=stdout, stderr=subprocess.PIPE, timeout=60, check=False,
        )

    def ffm_with_stdout(self, redirection, *args):
        """Run ffm.py with its stdout redirected by sh, e.g. ">&-" closes it."""
        script = f'exec "$@" {redirection}'
        return self.sandbox.execute(["sh", "-c", script, "sh", sys.executable, self.repo / "ffm.py", *args])

    def names(self):
        return [call.name for call in self.sandbox.calls()]

    def forget_calls(self):
        self.sandbox.log.unlink(missing_ok=True)

    def hide(self, name):
        """Remove `name`'s stub; the returned env keeps the system's copy off PATH too."""
        (self.sandbox.bin / name).unlink()
        return {"PATH": str(self.sandbox.bin)}

    def place_binary(self, mode, *, status=0, stdout=""):
        """A recording ffm at target/<mode>/ffm, as if Cargo had built it."""
        path = self.repo / "target" / mode / "ffm"
        return self.sandbox.fake("ffm-binary", path=path, status=status, stdout=stdout)

    def fake_cargo_that_builds(self, *, status=0):
        """Fake Cargo whose `build` leaves a recording ffm at target/<mode>/ffm."""
        built = self.sandbox.fake("ffm-binary", path=self.sandbox.root / "built-ffm", status=status)
        self.sandbox.fake("cargo", effect=f"""
            if args[:1] == ["build"]:
                import shutil
                mode = "release" if "--release" in args else "debug"
                target = os.path.join({str(self.repo)!r}, "target", mode)
                os.makedirs(target, exist_ok=True)
                shutil.copy2({str(built)!r}, os.path.join(target, "ffm"))
        """)

