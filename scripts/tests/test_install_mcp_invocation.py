"""Check how install_mcp.py is started: its encoding, its arguments, and its
file as a program.
"""

import json
import os
import subprocess
import sys
import unittest

from install_mcp_case import SCRIPT, InstallerTestCase
from sandbox import REPO


class EncodingTests(InstallerTestCase):
    def test_paths_that_are_not_utf8_are_printed_as_their_own_bytes(self):
        # Outside UTF-8 mode, a UTF-8 locale gives Python a strict stdout.
        checkout = self.sandbox.root / os.fsdecode(b"check\xffout")
        home = self.sandbox.root / os.fsdecode(b"h\xffome")
        (checkout / "mcp").mkdir(parents=True)
        (home / ".codeium").mkdir(parents=True)
        self.copy_installer(checkout)
        self.fake_cargo(checkout / "target")
        binary = self.make_binary(checkout / "target")
        environment = self.sandbox.env({"PYTHONUTF8": "0", "LC_ALL": "C.UTF-8", "HOME": str(home)})
        # Bytes, not text: the output is deliberately not UTF-8.
        result = subprocess.run(
            [sys.executable, checkout / SCRIPT, "--no-build"],
            cwd=checkout, env=environment, capture_output=True, timeout=60, check=False,
        )
        self.assertEqual((result.returncode, result.stderr), (0, b""))
        lines = result.stdout.splitlines()
        gemini = checkout / ".gemini/settings.json"
        windsurf = home / ".codeium/windsurf/mcp_config.json"
        self.assertIn("  ✓ Gemini → ".encode() + os.fsencode(gemini), lines)
        self.assertIn("  ✓ Windsurf → ".encode() + os.fsencode(windsurf), lines)
        self.assertEqual(lines[-1], b"Binary: " + os.fsencode(binary))
        # JSON escapes the stray byte.
        config = json.loads(gemini.read_text(encoding="utf-8"))
        self.assertEqual(config["mcpServers"]["ffm-mcp"], self.entry(binary))

    def test_a_config_is_read_as_utf8_under_an_ascii_locale(self):
        # With UTF-8 mode off, LC_ALL=C makes the locale's encoding ASCII. A
        # read in that encoding would fail on this config and drop the other
        # server along with it.
        self.make_binary()
        other = {"command": "café ✓"}
        config = json.dumps({"mcpServers": {"other": other}}, ensure_ascii=False)
        self.sandbox.write(".cursor/mcp.json", config)
        result = self.install("--no-build", env={"PYTHONUTF8": "0", "LC_ALL": "C"})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.read_json(".cursor/mcp.json")["mcpServers"]["other"], other)


class ArgumentTests(InstallerTestCase):
    def test_an_unknown_argument_is_rejected_before_cargo_runs(self):
        self.make_binary()
        result = self.install("--no-build", "--bogus")
        self.assertEqual(result.returncode, 2)
        self.assertIn("unrecognized arguments: --bogus", result.stderr)
        self.assertEqual(self.sandbox.calls(), [])
        self.assert_no_config_written()

    def test_an_abbreviation_of_no_build_is_rejected(self):
        # argparse would otherwise read `--no` as --no-build and skip the build.
        self.make_binary()
        for argument in ("--no", "--no-b"):
            with self.subTest(argument=argument):
                result = self.install(argument)
                self.assertEqual(result.returncode, 2)
                self.assertIn(f"unrecognized arguments: {argument}", result.stderr)
                self.assertEqual(self.sandbox.calls(), [])

    def test_help_describes_the_installer_without_running_it(self):
        result = self.install("--help")
        self.assertEqual(result.returncode, 0)
        self.assertIn("Install or refresh the ffm-mcp server", result.stdout)
        self.assertIn("--no-build", result.stdout)
        # argparse writes the one usage line.
        self.assertEqual(result.stdout.lower().count("usage:"), 1, result.stdout)
        self.assertEqual(self.sandbox.calls(), [])


class EntryPointTests(unittest.TestCase):
    def test_the_installer_runs_as_a_program(self):
        # The other tests start it with sys.executable, which skips both checks.
        script = REPO / SCRIPT
        self.assertEqual(script.read_text(encoding="utf-8").splitlines()[0], "#!/usr/bin/env python3")
        self.assertTrue(os.access(script, os.X_OK), f"{script} is not executable")


if __name__ == "__main__":
    unittest.main()
