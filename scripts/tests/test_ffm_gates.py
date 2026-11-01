"""Test the gates that ./ffm.py runs: test's suites, lint and coverage, and a
gate that a signal kills.
"""

import shlex
import signal
import sys
import unittest

from ffm_case import LauncherTest, status
from lint_case import ALL_LINES

# The checks that ./ffm.py lint names in its gate label and in its help: those lint.py reports running.
LINT_CHECKS = "fmt + spelling + markdown + complexity + duplication + clippy"


class TestCommandTests(LauncherTest):
    SUITES = (
        ["test", "--features", "test-helpers"],
        ["test", "--features", "test-helpers,s3", "--test", "bdd", "--test", "bdd_ru"],
        ["test", "--features", "keepass", "--lib", "keepass"],
    )

    def test_every_feature_set_runs_in_order(self):
        result = self.ffm("test", "extra")
        self.assertEqual(result.returncode, 0)
        self.assertEqual(
            result.stdout,
            status("Running tests...")
            + status("Running BDD suites...")
            + status("Running keepass feature tests..."),
        )
        self.assertEqual(self.sandbox.args("cargo"), list(self.SUITES))

    def test_the_first_failing_suite_exits_1(self):
        for index, failing in enumerate(self.SUITES):
            with self.subTest(failing=failing):
                self.forget_calls()
                self.sandbox.fake("cargo", effect=f"if args == {failing!r}: sys.exit(101)")
                self.assertEqual(self.ffm("test").returncode, 1)
                self.assertEqual(self.sandbox.args("cargo"), list(self.SUITES[: index + 1]))

    def test_a_suite_killed_by_a_signal_exits_1(self):
        self.sandbox.fake("cargo", effect="import signal; os.kill(os.getpid(), signal.SIGTERM)")
        result = self.ffm("test")
        self.assertEqual((result.returncode, result.stderr), (1, "Terminated  cargo test --features test-helpers\n"))
        self.assertEqual(self.sandbox.args("cargo"), [self.SUITES[0]])

    def test_ctrl_c_that_the_suite_handles_lets_the_run_go_on(self):
        # Ctrl+C reaches the whole foreground group: this script and the suite.
        # The script waits for the suite, which cleans up and exits 0, and goes on.
        marker = self.sandbox.root / "cleaned up"
        self.sandbox.fake("cargo", effect=f"""
            import signal, time
            def clean_up(*_):
                time.sleep(0.5)
                open({str(marker)!r}, "w").close()
                sys.exit(0)
            if args == {self.SUITES[0]!r}:
                signal.signal(signal.SIGINT, clean_up)
                os.kill(os.getppid(), signal.SIGINT)
                os.kill(os.getpid(), signal.SIGINT)
                time.sleep(10)
        """)
        result = self.ffm("test")
        self.assertEqual((result.returncode, result.stderr), (0, ""))
        self.assertTrue(marker.exists())
        self.assertEqual(self.sandbox.args("cargo"), list(self.SUITES))

    def test_ctrl_c_that_kills_the_suite_kills_the_run_without_a_traceback(self):
        self.sandbox.fake("cargo", effect=f"""
            import signal, time
            if args == {self.SUITES[0]!r}:
                signal.signal(signal.SIGINT, signal.SIG_DFL)
                os.kill(os.getppid(), signal.SIGINT)
                time.sleep(0.3)
                os.kill(os.getpid(), signal.SIGINT)
        """)
        result = self.ffm("test")
        self.assertEqual((result.returncode, result.stderr), (-signal.SIGINT, ""))
        self.assertEqual(result.stdout, status("Running tests..."))
        self.assertEqual(self.sandbox.args("cargo"), [self.SUITES[0]])


class SignalStatusTests(LauncherTest):
    def test_a_command_killed_by_signal_n_exits_128_plus_n(self):
        lint = self.repo / "scripts/lint.py"
        coverage = self.repo / "scripts/coverage.py"
        commands = {
            "lint": ("lint", lint, shlex.join([sys.executable, str(lint), "all"])),
            "coverage": ("coverage", coverage, shlex.join([sys.executable, str(coverage)])),
            "clean": ("cargo", None, "cargo clean"),
        }
        for command, (fake, path, shown) in commands.items():
            # Named on stderr with the command, except for Ctrl-C.
            reports = {signal.SIGKILL: f"Killed  {shown}\n", signal.SIGTERM: f"Terminated  {shown}\n", signal.SIGINT: ""}
            for number, report in reports.items():
                with self.subTest(command=command, signal=number.name):
                    # Only the child gets the signal, so this script goes on.
                    self.sandbox.fake(fake, path=path, effect=f"""
                        import signal
                        signal.signal(signal.SIGINT, signal.SIG_DFL)
                        os.kill(os.getpid(), {int(number)})
                    """)
                    result = self.ffm(command)
                    self.assertEqual((result.returncode, result.stderr), (128 + number, report))


class LintTests(LauncherTest):
    def test_lint_runs_lint_py_in_the_given_mode(self):
        # "--" goes on to lint.py as the mode, which lint.py refuses.
        modes = (
            ([], "all"), ([""], "all"), (["fmt"], "fmt"), (["clippy-fast", "extra"], "clippy-fast"),
            (["--"], "--"), (["--", "fmt"], "--"),
        )
        for args, mode in modes:
            with self.subTest(args=args):
                self.forget_calls()
                result = self.ffm("lint", *args, cwd=self.sandbox.home)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout, status(f"Running Rust linter gate ({LINT_CHECKS})..."))
                self.assertEqual(self.names(), ["cargo_preflight", "lint"])
                call = self.sandbox.calls("lint")[0]
                self.assertEqual((call.args, call.cwd), ([mode], str(self.repo)))

    def test_the_help_and_the_gate_label_list_the_checks_that_lint_runs(self):
        # lint.py's last line names what it ran. The help wraps the list over two lines.
        self.assertIn(f"✓ lint OK ({LINT_CHECKS})", ALL_LINES)
        words = " ".join(self.ffm("help").stdout.split())
        self.assertIn(f"lint [mode] Rust linter gate ({LINT_CHECKS}; see scripts/lint.py)", words)

    def test_lint_exits_with_the_lint_status(self):
        self.sandbox.fake("lint", path=self.repo / "scripts/lint.py", status=3)
        self.assertEqual(self.ffm("lint").returncode, 3)


class CoverageTests(LauncherTest):
    def test_coverage_runs_coverage_py_from_the_checkout_after_the_rust_check(self):
        # Words after the command are not passed on: the gate takes none.
        result = self.ffm("coverage", "--html", cwd=self.sandbox.home)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, status("Running test coverage..."))
        self.assertEqual(self.names(), ["cargo_preflight", "coverage"])
        call = self.sandbox.calls("coverage")[0]
        self.assertEqual((call.args, call.cwd), ([], str(self.repo)))

    def test_coverage_exits_with_the_gate_status(self):
        self.sandbox.fake("coverage", path=self.repo / "scripts/coverage.py", status=3)
        self.assertEqual(self.ffm("coverage").returncode, 3)


if __name__ == "__main__":
    unittest.main()
