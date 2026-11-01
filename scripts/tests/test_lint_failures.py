"""Check how lint.py ends: at the first failing step, or by a signal."""

import os
from pathlib import Path
import shlex
import signal
import unittest

from lint_case import (
    ARCHITECTURE, CHECKS, CLIPPY_DEFAULT, CLIPPY_DEFAULT_LINE, CLIPPY_MATRIX, FMT_CHECK, MARKDOWN, PREFLIGHT,
    SOURCE_CHECKS, SPELLING, checks_env, fake_preflight, gate_sandbox, steps,
)
from process_signals import dies_of, run_into_closed_pipe, signal_group, waits_for_a_signal


class FailureTests(unittest.TestCase):
    def test_the_first_failing_step_ends_lint_with_its_status(self):
        sandbox = gate_sandbox(self)
        sandbox.fake("cargo", effect="""
            if "--all-features" in args:
                sys.exit(7)
        """)
        result = sandbox.run("scripts/lint.py", env=checks_env(sandbox))
        self.assertEqual((result.returncode, result.stderr), (7, ""))
        self.assertEqual(
            steps(sandbox), PREFLIGHT + FMT_CHECK + [SPELLING, MARKDOWN] + SOURCE_CHECKS + CLIPPY_MATRIX[:2]
        )
        self.assertNotIn("✓", result.stdout)

    def test_a_failing_source_check_stops_lint_before_clippy(self):
        for index, (_, arguments, _) in enumerate(SOURCE_CHECKS):
            with self.subTest(check=arguments):
                sandbox = gate_sandbox(self)
                sandbox.fake(CHECKS, effect=f"if args == {arguments!r}: sys.exit(6)")
                result = sandbox.run("scripts/lint.py", env=checks_env(sandbox))
                self.assertEqual((result.returncode, result.stderr), (6, ""))
                self.assertEqual(
                    steps(sandbox), PREFLIGHT + FMT_CHECK + [SPELLING, MARKDOWN] + SOURCE_CHECKS[:index + 1]
                )
                self.assertNotIn("clippy", result.stdout)

    def test_a_failing_python_check_ends_lint_with_its_status(self):
        sandbox = gate_sandbox(self)
        sandbox.fake(CHECKS, effect="""
            if args == ["scripts/test_readability.py"]:
                sys.exit(3)
        """)
        result = sandbox.run("scripts/lint.py", "architecture", env=checks_env(sandbox))
        self.assertEqual(result.returncode, 3)
        self.assertEqual(steps(sandbox), PREFLIGHT + ARCHITECTURE[:2])

    def test_a_failing_preflight_ends_lint_before_any_check(self):
        sandbox = gate_sandbox(self)
        fake_preflight(sandbox, status=5)
        for mode in ("all", "clippy", "clippy-fast", "fix", "self-test", "architecture"):
            with self.subTest(mode=mode):
                sandbox.log.unlink(missing_ok=True)
                result = sandbox.run("scripts/lint.py", mode, env=checks_env(sandbox))
                self.assertEqual((result.returncode, result.stdout), (5, ""))
                self.assertEqual(steps(sandbox), PREFLIGHT)

    def test_a_command_that_cannot_start_ends_lint_with_127_or_126(self):
        missing = {"PYTHON": str(Path("/nonexistent/python"))}
        cases = {
            "cargo not found": ({}, "cargo", 127, "cargo: No such file or directory"),
            "cargo not executable": ({}, "cargo", 126, "cargo: Permission denied"),
            "no such PYTHON": (missing, None, 127, "/nonexistent/python: No such file or directory"),
        }
        for case, (env, broken, status, reason) in cases.items():
            with self.subTest(case=case):
                sandbox = gate_sandbox(self)
                if broken and status == 127:
                    (sandbox.bin / broken).unlink()
                elif broken:
                    (sandbox.bin / broken).chmod(0o644)
                # Only the sandbox's bin directory: no system cargo can stand in.
                env = {"PATH": str(sandbox.bin), "PYTHON": str(sandbox.bin / CHECKS), **env}
                result = sandbox.run("scripts/lint.py", "clippy-fast" if broken else "architecture", env=env)
                self.assertEqual((result.returncode, result.stderr), (status, f"{reason}\n"))
                self.assertNotIn("✓", result.stdout)

    def test_a_python_without_a_shebang_line_is_refused_with_126(self):
        # Run through sh instead, an empty or a broken file would pass the checks unseen.
        sandbox = gate_sandbox(self)
        ran = sandbox.root / "ran"
        plain = sandbox.root / "plain python"
        plain.write_text(f'echo "$@" >> "{ran}"\n', encoding="utf-8")
        plain.chmod(0o755)
        result = sandbox.run("scripts/lint.py", "architecture", env={"PYTHON": str(plain)})
        self.assertEqual((result.returncode, result.stderr), (126, f"{plain}: Exec format error\n"))
        self.assertFalse(ran.exists())


@unittest.skipUnless(os.name == "posix", "signal tests require POSIX process groups and fake executables")
class SignalTests(unittest.TestCase):
    def test_a_step_killed_by_signal_n_ends_lint_with_128_plus_n(self):
        # Named on stderr with the step, except for Ctrl-C and a closed pipe.
        clippy = shlex.join([CLIPPY_DEFAULT[0], *CLIPPY_DEFAULT[1]])
        reports = {
            signal.SIGTERM: f"Terminated  {clippy}\n", signal.SIGKILL: f"Killed  {clippy}\n",
            signal.SIGINT: "", signal.SIGPIPE: "",
        }
        for number, report in reports.items():
            with self.subTest(signal=number.name):
                sandbox = gate_sandbox(self)
                sandbox.fake("cargo", effect=dies_of(number, 'args[0] == "clippy"'))
                result = sandbox.run("scripts/lint.py", "clippy-fast", env=checks_env(sandbox))
                self.assertEqual((result.returncode, result.stderr), (128 + number, report))

    def test_a_reader_that_went_away_ends_lint_silently_by_sigpipe(self):
        # As `lint.sh fmt | head -1` ended: status 141 in the shell, no traceback.
        sandbox = gate_sandbox(self)
        result = run_into_closed_pipe(sandbox, "scripts/lint.py", "fmt")
        self.assertEqual((result.returncode, result.stderr), (-signal.SIGPIPE, b""))
        self.assertEqual(steps(sandbox), [])

    def test_ctrl_c_ends_lint_by_sigint_without_a_traceback(self):
        sandbox = gate_sandbox(self)
        started = sandbox.root / "clippy started"
        sandbox.fake("cargo", effect=waits_for_a_signal(started, 'args[0] == "clippy"'))
        outcome = signal_group(sandbox, ["scripts/lint.py", "clippy-fast"], started, signal.SIGINT,
                               checks_env(sandbox))
        self.assertEqual(outcome, (-signal.SIGINT, CLIPPY_DEFAULT_LINE + "\n", ""))

    def test_ctrl_c_that_the_step_handles_lets_the_gate_go_on(self):
        # Ctrl-C reaches the whole foreground group: lint and the step. lint
        # waits for the step, which cleans up and exits 0.
        sandbox = gate_sandbox(self)
        cleaned = sandbox.root / "cleaned up"
        sandbox.fake("cargo", effect=f"""
            import signal, time
            def clean_up(*_):
                time.sleep(0.5)
                open({str(cleaned)!r}, "w").close()
                sys.exit(0)
            if args[0] == "clippy":
                signal.signal(signal.SIGINT, clean_up)
                os.kill(os.getppid(), signal.SIGINT)
                os.kill(os.getpid(), signal.SIGINT)
                time.sleep(10)
        """)
        result = sandbox.run("scripts/lint.py", "clippy-fast", env=checks_env(sandbox))
        self.assertEqual((result.returncode, result.stderr), (0, ""))
        self.assertTrue(cleaned.exists())
        self.assertEqual(result.stdout.splitlines(), [CLIPPY_DEFAULT_LINE, "✓ clippy OK (default features)"])


if __name__ == "__main__":
    unittest.main()
