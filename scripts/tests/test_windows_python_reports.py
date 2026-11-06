"""Check the native Windows report text without Windows API calls."""

import contextlib
import ctypes
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]


def load(name, relative):
    spec = importlib.util.spec_from_file_location(name, ROOT / relative)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


@contextlib.contextmanager
def windows_errors(code):
    """Make the last Windows error `code` and ctypes.WinError an OSError that names it."""
    with patch.object(ctypes, "get_last_error", return_value=code, create=True), \
         patch.object(ctypes, "WinError", side_effect=lambda error: OSError(f"WinError {error}"), create=True):
        yield


class WindowsReportTests(unittest.TestCase):
    def test_performance_report_keeps_labels_and_fixed_disks(self):
        report = load("ffm_performance", "src/windows/performance.py")
        text = report.format_report("host", 12.5, 8192, 2048, [("C:", 25 * 2**30, 100 * 2**30)])
        self.assertEqual(text.splitlines(), ["Computer: host", "CPU Load: 12.5%",
                         "Memory Free: 2048.0 MB / 8192.0 MB", "C: Used: 75.0% Free: 25.0 GB / 100.0 GB"])

    def test_system_event_keeps_id_provider_level_and_single_line_message(self):
        report = load("ffm_messages", "src/windows/system_messages.py")
        xml = ('<Event xmlns="http://schemas.microsoft.com/win/2004/08/events/event"><System>'
               '<Provider Name="Provider"/><EventID>42</EventID><Level>2</Level>'
               '<TimeCreated SystemTime="2026-11-04T12:34:56.1234567Z"/></System></Event>')
        self.assertEqual(report.format_event(xml, " first\r\nsecond \n"),
                         "2026-11-04 12:34:56Z [Error] Provider (42) first second")

    def test_elevation_keeps_exact_manager_arguments_and_exit_code(self):
        helper = load("ffm_elevation", "src/ops/packages/elevated_upgrade.py")
        calls = []
        result = helper.run("C:/Program Files/choco.exe", list(helper.UPGRADE_ARGUMENTS),
                            lambda executable, args: calls.append((executable, args)) or 3010)
        self.assertEqual(result, 3010)
        self.assertEqual(calls, [("C:/Program Files/choco.exe", ["upgrade", "all", "-y"])])
        with self.assertRaises(ValueError):
            helper.run("choco.exe", ["uninstall", "all"], lambda *args: self.fail("must not elevate"))


if __name__ == "__main__":
    unittest.main()
