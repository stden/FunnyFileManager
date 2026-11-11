"""Verify interrupted developer commands preserve their documented signal outcome."""

from contextlib import ExitStack
from pathlib import Path
import runpy
import signal
import subprocess
import sys
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(ROOT), str(ROOT / "scripts")]
import install_mcp  # noqa: E402
import launcher_process  # noqa: E402
import lint_steps  # noqa: E402


class ProcessInterruptTests(unittest.TestCase):
    def test_launcher_waits_for_an_interrupted_child_then_ends_by_sigint(self):
        with (
            patch.object(launcher_process.signal, "signal") as handler,
            patch.object(launcher_process.signal, "getsignal", return_value=signal.SIG_DFL),
            patch.object(launcher_process, "resolve", return_value="fake-child"),
            patch.object(launcher_process.os, "kill") as kill,
            patch.object(launcher_process.subprocess, "run") as execute,
        ):
            def interrupted(*_args, **_options):
                handler.call_args.args[1](signal.SIGINT, None)
                return subprocess.CompletedProcess([], -signal.SIGINT)

            execute.side_effect = interrupted
            self.assertEqual(launcher_process.run(["fake-child"]), 128 + signal.SIGINT)
        kill.assert_called_once_with(launcher_process.os.getpid(), signal.SIGINT)
        self.assertEqual(handler.call_args.args, (signal.SIGINT, signal.SIG_DFL))

    def test_lint_step_raises_keyboard_interrupt_after_its_child_dies_of_sigint(self):
        with (
            patch.object(lint_steps.signal, "signal") as handler,
            patch.object(lint_steps.signal, "getsignal", return_value=signal.SIG_DFL),
            patch.object(lint_steps.shell_path, "resolve", return_value="fake-child"),
            patch.object(lint_steps.subprocess, "run") as execute,
        ):
            def interrupted(*_args, **_options):
                handler.call_args.args[1](signal.SIGINT, None)
                return subprocess.CompletedProcess([], -signal.SIGINT)

            execute.side_effect = interrupted
            with self.assertRaises(KeyboardInterrupt):
                lint_steps.run_step(["fake-child"])

    def test_lint_entrypoint_propagates_keyboard_interrupt_as_sigint(self):
        def interrupted():
            raise KeyboardInterrupt

        with patch.object(lint_steps.signal, "signal") as handler, patch.object(lint_steps.os, "kill") as kill:
            lint_steps.exit_with(interrupted)
        handler.assert_any_call(signal.SIGINT, signal.SIG_DFL)
        kill.assert_called_once_with(lint_steps.os.getpid(), signal.SIGINT)

    def test_installer_stops_registration_when_its_child_dies_of_sigint(self):
        result = subprocess.CompletedProcess(["installer-child"], -signal.SIGINT)
        with patch.object(install_mcp.subprocess, "run", return_value=result):
            with self.assertRaises(KeyboardInterrupt):
                install_mcp.run_step(["installer-child"])

    def test_cli_interrupt_handlers_restore_the_default_signal_before_ending(self):
        entrypoints = {
            "ffm.py": "os.chdir",
            "install_mcp.py": "argparse.ArgumentParser.parse_args",
            "create_desktop_shortcut.py": "argparse.ArgumentParser.parse_known_args",
            "scripts/k9s_eks.py": "argparse.ArgumentParser.parse_args",
        }
        for filename, boundary in entrypoints.items():
            with self.subTest(filename=filename), ExitStack() as stack:
                stack.enter_context(patch(boundary, side_effect=KeyboardInterrupt))
                handler = stack.enter_context(patch("signal.signal"))
                kill = stack.enter_context(patch("os.kill"))
                stack.enter_context(patch.object(sys, "argv", [filename]))
                runpy.run_path(str(ROOT / filename), run_name="__main__")
                self.assertEqual(handler.call_args.args, (signal.SIGINT, signal.SIG_DFL))
                kill.assert_called_once_with(launcher_process.os.getpid(), signal.SIGINT)

    def test_preflight_handles_sigterm_with_the_original_signal_status(self):
        def deliver_signal(number, handler):
            if number == signal.SIGTERM and callable(handler):
                handler(number, None)

        with (
            patch("lint.run"),
            patch("signal.signal", side_effect=deliver_signal) as handler,
            patch("os.kill") as kill,
            patch.object(sys, "argv", ["cargo_preflight.py"]),
        ):
            runpy.run_path(str(ROOT / "scripts/cargo_preflight.py"), run_name="__main__")
        handler.assert_any_call(signal.SIGTERM, signal.SIG_DFL)
        kill.assert_called_once_with(launcher_process.os.getpid(), signal.SIGTERM)


if __name__ == "__main__":
    unittest.main()
