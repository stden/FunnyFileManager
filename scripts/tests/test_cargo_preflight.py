"""Check the compiler-wrapper probe against a fake Cargo that answers, stalls or fails."""

import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
import unittest
from unittest.mock import patch

from sandbox import REPO, Sandbox
from test_lint import dies_of, signal_group, waits_for_a_signal


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
        self.sandbox.copy(SCRIPT)
        # The preflight runs its commands through lint.py's helpers, and
        # both find Cargo through shell_path.py.
        self.sandbox.copy("scripts/lint.py")
        self.sandbox.copy("scripts/shell_path.py")

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


class AnswerTests(PreflightTestCase):
    def test_a_wrapper_that_answers_passes_silently(self):
        result = self.preflight(effect="""
            if args[0] == "metadata":
                probe = os.path.dirname(args[4])
                with open(os.path.join(os.environ["HOME"], "probe.json"), "w") as seen:
                    json.dump({
                        "manifest": open(args[4]).read(),
                        "lib": open(os.path.join(probe, "src", "lib.rs")).read(),
                    }, seen)
        """)
        self.assertEqual((result.returncode, result.stdout, result.stderr), (0, "", ""))
        version, metadata = self.sandbox.calls("cargo")
        self.assertEqual(version.args, ["-V"])
        probe = Path(metadata.args[4]).parent
        self.assertEqual(metadata.args, [
            "metadata", "--format-version=1", "--offline", "--manifest-path", str(probe / "Cargo.toml"),
        ])
        self.assertEqual(probe.parent, self.sandbox.tmp)
        self.assertTrue(probe.name.startswith("ffm-wrapper-probe."), probe.name)
        self.assertEqual(metadata.env["CARGO_TARGET_DIR"], str(probe / "target"))
        self.assertNotIn("CARGO_TARGET_DIR", version.env)
        seen = json.loads((self.sandbox.home / "probe.json").read_text(encoding="utf-8"))
        self.assertEqual(seen, {"manifest": PROBE_MANIFEST, "lib": ""})
        self.assert_probe_removed()

    def test_a_cargo_behind_a_tilde_path_entry_is_found_as_bash_finds_it(self):
        # bash found it for `cargo -V`, but timeout(1) then could not run it.
        self.sandbox.fake("cargo")
        result = self.sandbox.run(SCRIPT, env={"PATH": self.sandbox.tilde_path("cargo")})
        self.assertEqual((result.returncode, result.stderr), (0, ""))
        self.assertEqual([args[0] for args in self.sandbox.args("cargo")], ["-V", "metadata"])
        self.assert_probe_removed()

    def test_cargo_runs_from_the_checkout_root_whatever_the_callers_directory(self):
        result = self.preflight(cwd=self.sandbox.home)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([call.cwd for call in self.sandbox.calls("cargo")], [str(self.sandbox.repo)] * 2)

    def test_waiting_on_the_package_cache_lock_counts_as_an_answer(self):
        cases = {
            "cargo failed": FAIL.format(stderr=LOCK_WAIT),
            "cargo was killed at the limit": STALL.format(stderr=LOCK_WAIT),
        }
        for case, effect in cases.items():
            with self.subTest(case=case):
                result = self.preflight(effect=effect, FFM_WRAPPER_PROBE_SECS="0.5")
                self.assertEqual((result.returncode, result.stdout, result.stderr), (0, "", LOCK_NOTE))
                self.assert_probe_removed()

    def test_a_limit_of_zero_means_no_limit(self):
        for limit in ("0", " -0", "inf"):
            with self.subTest(limit=limit):
                result = self.preflight(
                    effect='if args[0] == "metadata": __import__("time").sleep(1)',
                    FFM_WRAPPER_PROBE_SECS=limit,
                )
                self.assertEqual((result.returncode, result.stderr), (0, ""))

    def test_cargo_sees_the_checkout_root_as_pwd(self):
        result = self.preflight(cwd=self.sandbox.home, PWD=str(self.sandbox.home))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([call.env["PWD"] for call in self.sandbox.calls("cargo")], [str(self.sandbox.repo)] * 2)

    def test_needs_no_script_directory_on_sys_path_and_takes_no_other_lint(self):
        # `python3 -P` or PYTHONSAFEPATH=1 leave the script's directory off sys.path.
        impostor = self.sandbox.root / "site"
        impostor.mkdir()
        (impostor / "lint.py").write_text("raise SystemExit('the wrong lint module')\n", encoding="utf-8")
        result = self.preflight(PYTHONSAFEPATH="1", PYTHONPATH=str(impostor))
        self.assertEqual((result.returncode, result.stderr), (0, ""))


class StallTests(PreflightTestCase):
    def test_a_stalled_wrapper_fails_within_the_limit(self):
        started = time.monotonic()
        result = self.preflight(effect=STALL.format(stderr="   Compiling nothing\n"), FFM_WRAPPER_PROBE_SECS="0.5")
        elapsed = time.monotonic() - started
        self.assertEqual(result.returncode, 1)
        self.assertLess(elapsed, 15)
        self.assertEqual(result.stdout, "")
        self.assertEqual(
            result.stderr,
            f"error: Cargo got no answer to `rustc -vV` within 0.5s (compiler wrapper: {NO_WRAPPER}).\n"
            "Every build would hang the same way; a stalled sccache server is the usual cause.\n"
            "   Compiling nothing\n" + ADVICE,
        )
        self.assert_probe_removed()


class FailureTests(PreflightTestCase):
    def test_a_broken_wrapper_reports_cargos_own_reason(self):
        reason = (
            "error: could not execute process `/nonexistent/compiler-wrapper rustc -vV` (never executed)\n"
            "\nCaused by:\n  No such file or directory (os error 2)\n"
        )
        result = self.preflight(effect=FAIL.format(stderr=reason), RUSTC_WRAPPER="/nonexistent/compiler-wrapper")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(
            result.stderr,
            "error: Cargo could not query the Rust compiler "
            "(compiler wrapper: RUSTC_WRAPPER=/nonexistent/compiler-wrapper):\n" + reason + ADVICE,
        )
        self.assert_probe_removed()

    def test_a_cargo_that_exits_124_or_is_killed_before_the_limit_did_not_stall(self):
        # bash took timeout(1)'s 124 and 137 for a stall, even from Cargo itself
        # or the OOM killer; the preflight knows whether the limit ran out.
        effects = {
            "exit 124": """
                if args[0] == "metadata":
                    sys.exit(124)
            """,
            "SIGKILL": dies_of(signal.SIGKILL, 'args[0] == "metadata"'),
        }
        for case, effect in effects.items():
            with self.subTest(case=case):
                result = self.preflight(effect=effect)
                self.assertEqual((result.returncode, result.stderr), (1, (
                    f"error: Cargo could not query the Rust compiler (compiler wrapper: {NO_WRAPPER}):\n"
                    + ADVICE
                )))
                self.assert_probe_removed()

    def test_only_the_last_twenty_lines_of_cargos_stderr_are_shown(self):
        lines = "".join(f"line {number}\n" for number in range(1, 26))
        result = self.preflight(effect=FAIL.format(stderr=lines))
        shown = result.stderr.splitlines()[1:-2]
        self.assertEqual(shown, [f"line {number}" for number in range(6, 26)])

    def test_cargos_stderr_is_shown_byte_for_byte_as_tail_showed_it(self):
        # Lines end at \n only: progress redraws (\r), CRLF, form feeds, U+2028,
        # bytes that are not UTF-8 and Cyrillic all pass through unchanged, even
        # when the preflight's own stderr cannot encode them.
        progress = b"".join(b"p%d\r" % number for number in range(30)) + b"done\n"
        tail = [b"crlf\r\n", b"feed\x0c vtab\x0b sep\xe2\x80\xa8 end\n", b"latin \xe9\xff\n",
                "ошибка: не удалось\n".encode("utf-8"), b"no newline"]
        earlier = b"".join(b"early %d\n" % number for number in range(20))
        effect = f"""
            if args[0] == "metadata":
                sys.stderr.flush()
                os.write(2, {earlier + progress + b"".join(tail)!r})
                sys.exit(101)
        """
        for encoding in ("utf-8", "latin-1", "ascii"):
            with self.subTest(stderr_encoding=encoding):
                result = self.preflight_bytes(effect, PYTHONUTF8="0", PYTHONIOENCODING=encoding)
                self.assertEqual(result.returncode, 1)
                shown = result.stderr.split(b"\n", 1)[1].rsplit(b"This repository", 1)[0]
                self.assertEqual(shown, b"".join(
                    [b"early %d\n" % number for number in range(6, 20)] + [progress] + tail))

    def test_the_error_names_the_wrapper_the_environment_selects(self):
        cases = [
            ({"RUSTC_WRAPPER": "sccache", "CARGO_BUILD_RUSTC_WRAPPER": "other"}, "RUSTC_WRAPPER=sccache"),
            ({"RUSTC_WRAPPER": "", "CARGO_BUILD_RUSTC_WRAPPER": "other"}, "CARGO_BUILD_RUSTC_WRAPPER=other"),
            ({"RUSTC_WRAPPER": "", "CARGO_BUILD_RUSTC_WRAPPER": ""}, NO_WRAPPER),
            ({}, NO_WRAPPER),
        ]
        for env, wrapper in cases:
            with self.subTest(env=env):
                result = self.preflight(effect=FAIL.format(stderr="boom\n"), **env)
                self.assertEqual(
                    result.stderr.splitlines()[0],
                    f"error: Cargo could not query the Rust compiler (compiler wrapper: {wrapper}):",
                )

    def test_a_failing_cargo_version_ends_the_preflight_with_its_status(self):
        result = self.preflight(effect="""
            if args == ["-V"]:
                sys.stderr.write("error: toolchain is not installed\\n")
                sys.exit(5)
        """)
        self.assertEqual(result.returncode, 5)
        self.assertEqual(result.stderr, "error: toolchain is not installed\n")
        self.assertEqual(self.sandbox.args("cargo"), [["-V"]])
        self.assert_probe_removed()

    def test_a_missing_cargo_fails_with_status_127_naming_it_as_bash_did(self):
        for cargo in ("/nonexistent/cargo", "no-such-cargo"):
            with self.subTest(cargo=cargo):
                result = self.preflight(CARGO=cargo)
                self.assertEqual(
                    (result.returncode, result.stdout, result.stderr),
                    (127, "", f"{cargo}: No such file or directory\n"),
                )
                self.assertEqual(self.sandbox.calls(), [])
                self.assert_probe_removed()

    def test_a_cargo_that_cannot_be_executed_ends_the_preflight_with_126(self):
        self.sandbox.fake("cargo").chmod(0o644)
        # Only the sandbox's bin directory: no system cargo can stand in.
        result = self.sandbox.run(SCRIPT, env={"PATH": str(self.sandbox.bin)})
        self.assertEqual((result.returncode, result.stderr), (126, "cargo: Permission denied\n"))
        self.assert_probe_removed()

    def test_a_cargo_without_a_shebang_line_cannot_start(self):
        # bash ran such a file as a script of its own; the preflight, like lint, refuses it.
        ran = self.sandbox.root / "ran"
        plain = self.sandbox.root / "plain cargo"
        plain.write_text(f'echo "$1" >> "{ran}"\n', encoding="utf-8")
        plain.chmod(0o755)
        result = self.preflight(CARGO=str(plain))
        self.assertEqual((result.returncode, result.stderr), (126, f"{plain}: Exec format error\n"))
        self.assertFalse(ran.exists())
        self.assert_probe_removed()

    def test_a_tmpdir_that_cannot_hold_the_probe_fails_as_mktemp_did(self):
        cases = {
            "missing": (self.sandbox.root / "no such dir", "No such file or directory"),
            "a file": (self.sandbox.write("a file"), "Not a directory"),
        }
        for case, (tmpdir, reason) in cases.items():
            with self.subTest(case=case):
                self.sandbox.log.unlink(missing_ok=True)
                result = self.preflight(TMPDIR=str(tmpdir))
                self.assertEqual((result.returncode, result.stdout), (1, ""))
                self.assertEqual(
                    result.stderr,
                    f"mktemp: failed to create directory via template '{tmpdir}/ffm-wrapper-probe.XXXXXX': {reason}\n",
                )
                self.assertEqual(self.sandbox.args("cargo"), [["-V"]])

    def test_a_relative_tmpdir_is_under_the_checkout_root(self):
        (self.sandbox.repo / "scratch").mkdir()
        result = self.preflight(cwd=self.sandbox.home, TMPDIR="scratch")
        self.assertEqual(result.returncode, 0, result.stderr)
        manifest = Path(self.sandbox.args("cargo")[1][4])
        self.assertEqual(manifest.parent.parent, self.sandbox.repo / "scratch")


class SignalTests(PreflightTestCase):
    def test_a_cargo_version_killed_by_signal_n_ends_the_preflight_with_128_plus_n(self):
        result = self.preflight(effect=dies_of(signal.SIGTERM, 'args == ["-V"]'))
        self.assertEqual((result.returncode, result.stderr), (128 + signal.SIGTERM, "Terminated  cargo -V\n"))

    def test_ctrl_c_or_sigterm_ends_the_preflight_by_that_signal_and_removes_the_probe(self):
        for number in (signal.SIGINT, signal.SIGTERM):
            with self.subTest(signal=number.name):
                started = self.sandbox.root / f"{number.name} probe started"
                self.sandbox.fake("cargo", effect=waits_for_a_signal(started, 'args[0] == "metadata"'))
                outcome = signal_group(self.sandbox, [SCRIPT], started, number)
                self.assertEqual(outcome, (-number, "", ""))
                self.assert_probe_removed()


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
