"""Check the preflight's settings: CARGO, FFM_WRAPPER_PROBE_SECS, TMPDIR and --help."""

import os
from pathlib import Path
import unittest
from unittest.mock import patch

from cargo_preflight_case import SCRIPT, STALL, PreflightTestCase, load_preflight


class SettingTests(PreflightTestCase):
    def test_cargo_selects_the_cargo_to_probe_with(self):
        chosen = self.sandbox.fake("chosen-cargo", path=self.sandbox.root / "toolchain dir" / "cargo")
        result = self.preflight(CARGO=str(chosen))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([args[0] for args in self.sandbox.args("chosen-cargo")], ["-V", "metadata"])
        self.assertEqual(self.sandbox.calls("cargo"), [])

    def test_an_empty_cargo_means_cargo_on_path(self):
        result = self.preflight(CARGO="")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([args[0] for args in self.sandbox.args("cargo")], ["-V", "metadata"])

    def test_an_invalid_limit_is_refused_after_cargo_version_as_timeout_refused_it(self):
        for limit in ("soon", "-1", "-0", "nan", "5x", "s", "1_0", "5 ", "٥"):
            with self.subTest(limit=limit):
                self.sandbox.log.unlink(missing_ok=True)
                result = self.preflight(FFM_WRAPPER_PROBE_SECS=limit)
                self.assertEqual(result.returncode, 1)
                self.assertEqual(
                    result.stderr,
                    "error: FFM_WRAPPER_PROBE_SECS must be seconds, optionally suffixed s, m, h or d;"
                    f" not {limit!r}\n",
                )
                self.assertEqual(self.sandbox.args("cargo"), [["-V"]])
                self.assert_probe_removed()

    def test_a_failing_cargo_version_wins_over_an_invalid_limit(self):
        result = self.preflight(effect='if args == ["-V"]: sys.exit(6)', FFM_WRAPPER_PROBE_SECS="soon")
        self.assertEqual((result.returncode, result.stderr), (6, ""))

    def test_a_limit_with_a_unit_suffix_is_accepted_and_shown_as_written(self):
        result = self.preflight(effect=STALL.format(stderr=""), FFM_WRAPPER_PROBE_SECS="0.01m")
        self.assertEqual(result.returncode, 1)
        self.assertIn("within 0.01ms (compiler wrapper", result.stderr)

    def test_help_describes_the_settings_without_running_cargo(self):
        for flag in ("-h", "--help"):
            with self.subTest(flag=flag):
                shown = self.sandbox.run(SCRIPT, flag)
                self.assertEqual(shown.returncode, 0, shown.stderr)
                self.assertIn("FFM_WRAPPER_PROBE_SECS", shown.stdout)
                self.assertEqual(self.sandbox.calls(), [])

    def test_other_arguments_are_ignored_as_bash_ignored_them(self):
        self.sandbox.fake("cargo")
        for args in (("extra",), ("extra", "--help"), ("--",)):
            with self.subTest(args=args):
                self.sandbox.log.unlink(missing_ok=True)
                result = self.sandbox.run(SCRIPT, *args)
                self.assertEqual((result.returncode, result.stderr), (0, ""))
                self.assertEqual([call_args[0] for call_args in self.sandbox.args("cargo")], ["-V", "metadata"])


class LimitTests(unittest.TestCase):
    """The default limit is 20 s; a process-level check would take 20 s to show it."""

    def test_an_unset_or_empty_limit_means_twenty_seconds(self):
        preflight = load_preflight()
        for environment in ({}, {"FFM_WRAPPER_PROBE_SECS": ""}):
            with self.subTest(environment=environment), patch.dict(os.environ, environment, clear=True):
                self.assertEqual(preflight.limit_text(), "20")
                self.assertEqual(preflight.limit_seconds(preflight.limit_text()), 20.0)

    def test_the_limit_is_kept_as_written_for_the_message(self):
        preflight = load_preflight()
        with patch.dict(os.environ, {"FFM_WRAPPER_PROBE_SECS": "2.5"}, clear=True):
            self.assertEqual(preflight.limit_text(), "2.5")
        self.assertEqual(preflight.limit_seconds("2.5"), 2.5)
        self.assertIsNone(preflight.limit_seconds("0"))

    def test_the_limit_reads_durations_as_timeout_did(self):
        # GNU timeout 9.4 took each of these: C floating-point numbers, hexadecimal
        # and inf included, after blanks and a sign, then one of s, m, h or d.
        preflight = load_preflight()
        cases = {
            "20": 20, "1.5s": 1.5, "2m": 120, "1h": 3600, "0.5d": 43200, "0s": None, "0m": None,
            "0x10": 16, "0x1p3": 8, "0X1P-1s": 0.5, "0x10d": 269, " 5": 5, "\t+5": 5, ".5": 0.5, "5.": 5,
            "1e1m": 600, "08": 8, " -0": None, "+0": None, "inf": None, "INFINITYs": None, "1e400": None,  # spellchecker:disable-line
        }
        for text, seconds in cases.items():
            with self.subTest(text=text):
                self.assertEqual(preflight.limit_seconds(text), seconds)

    def test_the_limit_refuses_what_timeout_refused(self):
        preflight = load_preflight()
        for text in ("soon", "-1", "-0", " -1", "nan", "5x", "s", "1_0", "5 ", "٥", "1,5", "0x", "1e", "1.5S", "+"):
            with self.subTest(text=text), self.assertRaises(SystemExit):
                preflight.limit_seconds(text)

    def test_the_probe_goes_where_mktemp_put_it(self):
        preflight = load_preflight()
        cases = [
            ({"TMPDIR": "/var/scratch"}, Path("/var/scratch")),
            ({"TMPDIR": "", "TEMP": "/elsewhere", "TMP": "/elsewhere"}, Path("/tmp")),
            ({"TEMP": "/elsewhere"}, Path("/tmp")),
            ({"TMPDIR": "scratch"}, preflight.ROOT / "scratch"),
        ]
        for environment, parent in cases:
            with self.subTest(environment=environment), patch.dict(os.environ, environment, clear=True):
                self.assertEqual(preflight.probe_parent(), parent)


if __name__ == "__main__":
    unittest.main()
