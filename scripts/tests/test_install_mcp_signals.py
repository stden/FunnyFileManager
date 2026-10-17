"""Check how install_mcp.py ends when a step is killed, or on Ctrl-C."""

import os
import signal
import subprocess
import sys
import time
import unittest

from install_mcp_case import CTRL_C_ADD, KILL, SCRIPT, InstallerTestCase


class SignalTests(InstallerTestCase):
    """A step killed by signal N ends the install with 128 + N and is named on stderr."""

    def assert_killed(self, result):
        self.assertEqual(result.returncode, 128 + 9)
        self.assertTrue(result.stderr.startswith("Killed  "), result.stderr)
        self.assert_no_config_written()

    def test_a_killed_build(self):
        self.fake_cargo(self.target, build_effect=KILL)
        result = self.install()
        self.assert_killed(result)
        self.assertEqual(result.stderr, "Killed  cargo build --release -j1\n")

    def test_a_killed_add(self):
        binary = self.make_binary()
        self.sandbox.fake("claude", effect=f'if args[1] == "add": {KILL}')
        result = self.install("--no-build")
        self.assert_killed(result)
        claude = self.sandbox.bin / "claude"
        self.assertIn(f"{claude} mcp add --transport stdio ffm-mcp -- '{binary}'", result.stderr)

    def test_a_killed_cargo_metadata(self):
        self.sandbox.fake("cargo", effect=KILL)
        result = self.install("--no-build")
        self.assert_killed(result)
        self.assertEqual(result.stdout, "")


class CtrlCTests(InstallerTestCase):
    """Ctrl-C reaches the running step as well, and the step decides what it means."""

    def setUp(self):
        super().setUp()
        self.make_binary()
        self.started = self.sandbox.root / "started"

    def press_ctrl_c_during_add(self, on_sigint):
        """Run the installer; once `claude mcp add` has started, send SIGINT
        to the installer's whole process group, as a terminal does."""
        effect = CTRL_C_ADD.format(on_sigint=on_sigint, started=str(self.started))
        self.sandbox.fake("claude", effect=effect)
        process = subprocess.Popen(
            [sys.executable, self.repo / SCRIPT, "--no-build"],
            cwd=self.repo, env=self.sandbox.env(), start_new_session=True,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8",
        )
        with process:
            deadline = time.monotonic() + 30
            while not self.started.exists():
                self.assertIsNone(process.poll(), "the installer ended before `claude mcp add` started")
                self.assertLess(time.monotonic(), deadline, "`claude mcp add` never started")
                time.sleep(0.02)
            os.killpg(process.pid, signal.SIGINT)
            stdout, stderr = process.communicate(timeout=60)
        return subprocess.CompletedProcess(process.args, process.returncode, stdout, stderr)

    def test_the_install_waits_for_a_step_that_cleans_up_first(self):
        finished = self.sandbox.root / "finished"
        result = self.press_ctrl_c_during_add(
            f'time.sleep(1); open({str(finished)!r}, "w").close(); sys.exit(130)'
        )
        self.assertTrue(finished.exists(), "the step was killed before it had cleaned up")
        self.assert_stopped_at("claude", result, 130, "")

    def test_a_step_killed_by_ctrl_c_ends_the_install_by_sigint_too(self):
        result = self.press_ctrl_c_during_add(
            "signal.signal(signal.SIGINT, signal.SIG_DFL); os.kill(os.getpid(), signal.SIGINT)"
        )
        # Killed by SIGINT, so a calling script sees the interrupt; no traceback.
        self.assert_stopped_at("claude", result, -signal.SIGINT, "")

    def test_the_install_goes_on_when_a_step_exits_0_on_ctrl_c(self):
        result = self.press_ctrl_c_during_add("sys.exit(0)")
        self.assertEqual((result.returncode, result.stderr), (0, ""))
        self.assertTrue((self.repo / ".mcp.json").exists())

    def test_a_step_starts_with_the_default_ctrl_c_unless_it_was_ignored(self):
        self.sandbox.fake("claude", effect="""
            import signal
            if args[1] == "add":
                print("Ctrl-C ignored:", signal.getsignal(signal.SIGINT) is signal.SIG_IGN)
        """)
        for ignored in (False, True):
            with self.subTest(ignored=ignored):
                # The installer inherits this test's disposition.
                disposition = signal.SIG_IGN if ignored else signal.default_int_handler
                handler = signal.signal(signal.SIGINT, disposition)
                try:
                    result = self.install("--no-build")
                finally:
                    signal.signal(signal.SIGINT, handler)
                self.assertIn(f"Ctrl-C ignored: {ignored}\n", result.stdout)


if __name__ == "__main__":
    unittest.main()
