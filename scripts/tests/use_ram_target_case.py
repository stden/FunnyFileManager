"""The sandbox checkout and the shell lines that the tests of
scripts/use_ram_target.py share.
"""

import importlib.util
import shlex
import sys
import unittest

from sandbox import REPO, Sandbox


SCRIPT = "scripts/use_ram_target.py"
# Evaluate the helper's stdout as the documented usage does, then show the result.
EVAL = 'eval "$("$0" "$1")" && printf %s "${CARGO_TARGET_DIR-unset}"'
# The same with the helper's stderr in $ERR, and a check instead of the value.
EVAL_QUIETLY = (
    'out=$("$0" "$1" 2>"$ERR"); echo "status $?"; eval "$out"; '
    '[ "${CARGO_TARGET_DIR-}" = "$FFM_RAM_TARGET" ] && echo exported'
)
# Free space as `df -h` prints it in the C locale: 844G, 7.8G, 512M or 0.
FREE = r"\d+(?:\.\d)?[KMGTPEZYRQ]?"


class RamTargetTestCase(unittest.TestCase):
    def setUp(self):
        self.sandbox = Sandbox(self)
        self.sandbox.copy(SCRIPT)
        # A space and a quote catch an export line the shell would split.
        self.parent = self.sandbox.tmp / "ram disk's"

    def run_helper(self, ram_dir, *args):
        # Keep Python's child diagnostics in the native oracle's C locale.
        return self.sandbox.run(
            SCRIPT, *args, env={"FFM_RAM_TARGET": str(ram_dir), "LC_ALL": "C"}, input="",
        )

    def eval_helper(self, ram_dir):
        """Run the helper under `sh`, eval its output, and return the shell's result."""
        return self.sandbox.execute(
            ["sh", "-c", EVAL, sys.executable, self.sandbox.repo / SCRIPT],
            env={"FFM_RAM_TARGET": str(ram_dir)},
        )

    def assert_exported(self, result, ram_dir):
        """One export line that the shell evaluates to exactly `ram_dir`."""
        self.assertEqual((result.returncode, result.stdout),
                         (0, f"export CARGO_TARGET_DIR={shlex.quote(str(ram_dir))}\n"))
        self.assertEqual(self.eval_helper(ram_dir).stdout, str(ram_dir))


def load_helper():
    """Import the script by path; it does no work at import time."""
    spec = importlib.util.spec_from_file_location("use_ram_target", REPO / SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module
