"""Check the compiler-wrapper probe against a fake Cargo that answers, stalls or fails.

Failures and the settings have test_cargo_preflight_*.py files of their own,
and cargo_preflight_case.py holds what they share.
"""

import json
from pathlib import Path
import signal
import time
import unittest

from cargo_preflight_case import (
    ADVICE, FAIL, LOCK_NOTE, LOCK_WAIT, NO_WRAPPER, PROBE_MANIFEST, SCRIPT, STALL, PreflightTestCase,
)
from process_signals import dies_of, signal_group, waits_for_a_signal


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


if __name__ == "__main__":
    unittest.main()
