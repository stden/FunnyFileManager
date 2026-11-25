"""The sandbox checkout, the fake Cargo effects and the expected messages that
the tests of scripts/cargo_preflight.py share.
"""

import importlib.util
import subprocess
import sys
import unittest

from sandbox import REPO, Sandbox
from script_files import CARGO_PREFLIGHT


SCRIPT = "scripts/cargo_preflight.py"
PROBE_MANIFEST = '[package]\nname = "wrapper-probe"\nversion = "0.0.0"\nedition = "2021"\n\n[workspace]\n'
NO_WRAPPER = "none set in the environment; see Cargo's config files"
ADVICE = (
    "This repository builds without a wrapper. If you enabled one, retry with RUSTC_WRAPPER= (empty),\n"
    "or restart the cache server (sccache --stop-server).\n"
)
LOCK_NOTE = "note: the compiler wrapper answered; another Cargo process holds the package cache\n"
LOCK_WAIT = "    Blocking waiting for file lock on package cache\n"
# Fake Cargo effects for `cargo metadata`, the timed probe.
STALL = """
    if args[0] == "metadata":
        sys.stderr.write({stderr!r})
        sys.stderr.flush()
        import time
        time.sleep(60)
"""
FAIL = """
    if args[0] == "metadata":
        sys.stderr.write({stderr!r})
        sys.exit(101)
"""


def load_preflight():
    """The preflight as a module, for the one check no process can show quickly."""
    spec = importlib.util.spec_from_file_location("cargo_preflight", REPO / SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class PreflightTestCase(unittest.TestCase):
    def setUp(self):
        self.sandbox = Sandbox(self)
        # The preflight and the module that reads its limit. It runs its
        # commands through lint.py's helpers, and both find Cargo through
        # shell_path.py.
        for relative in CARGO_PREFLIGHT:
            self.sandbox.copy(relative)

    def preflight(self, effect="", cwd=None, **env):
        self.sandbox.fake("cargo", stdout="stdout is discarded\n", effect=effect)
        return self.sandbox.run(SCRIPT, cwd=cwd, env=env)

    def preflight_bytes(self, effect, **env):
        """Like preflight(), with stdout and stderr as bytes, untouched by newline translation."""
        self.sandbox.fake("cargo", effect=effect)
        return subprocess.run(
            [sys.executable, self.sandbox.repo / SCRIPT], cwd=self.sandbox.repo, env=self.sandbox.env(env),
            capture_output=True, timeout=60, check=False,
        )

    def assert_probe_removed(self):
        self.assertEqual(list(self.sandbox.tmp.iterdir()), [])
