"""Exercise the native adapters' failure paths and script entry points with fake DLLs."""

import contextlib
import ctypes
import io
import runpy
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

from test_windows_python_reports import ROOT, load, windows_errors

ERROR = 1223  # The last Windows error that every failing fake reports.
EVENT_XML = ('<Event xmlns="http://schemas.microsoft.com/win/2004/08/events/event"><System>'
             '<Provider Name="provider"/><EventID>1</EventID><Level>3</Level>'
             '<TimeCreated SystemTime="2026-11-04T00:00:00Z"/></System></Event>')


def run_script(relative, dll=None):
    """Run a script as __main__ without arguments; answer its exit code, stdout and stderr."""
    out, err, code = io.StringIO(), io.StringIO(), None
    with patch.object(sys, "argv", [relative]), patch("time.sleep"), \
         patch.object(ctypes, "WinDLL", return_value=dll, create=True), \
         contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        try:
            runpy.run_path(str(ROOT / relative), run_name="__main__")
        except SystemExit as stop:
            code = stop.code
    return code, out.getvalue(), err.getvalue()


def needs_bytes(index):
    """A failing Windows call that reports 8 bytes needed through the size pointer at argument `index`."""
    def call(*arguments):
        arguments[index]._obj.value = 8
        return 0
    return call


def working_kernel():
    return Mock(GetSystemTimes=Mock(return_value=1), GlobalMemoryStatusEx=Mock(return_value=1),
                GetLogicalDrives=Mock(return_value=1 << 2), GetDriveTypeW=Mock(return_value=3),
                GetDiskFreeSpaceExW=Mock(return_value=1))


class PerformanceFailureTests(unittest.TestCase):
    def test_each_failing_system_call_raises_its_windows_error(self):
        module = load("failure_performance", "src/windows/performance.py")
        for failing in ("GetSystemTimes", "GlobalMemoryStatusEx", "GetDiskFreeSpaceExW"):
            kernel = working_kernel()
            getattr(kernel, failing).return_value = 0
            with self.subTest(failing), windows_errors(ERROR), patch.object(module.time, "sleep"), \
                 patch.object(ctypes, "WinDLL", return_value=kernel, create=True):
                with self.assertRaisesRegex(OSError, f"^WinError {ERROR}$"):
                    module.report()

    def test_script_prints_the_report(self):
        kernel = working_kernel()
        kernel.GetLogicalDrives.return_value = 0
        with patch.dict("os.environ", {"COMPUTERNAME": "HOST"}):
            code, out, err = run_script("src/windows/performance.py", kernel)
        self.assertEqual((code, err), (None, ""))
        self.assertEqual(out.splitlines(), ["Computer: HOST", "CPU Load: 0.0%", "Memory Free: 0.0 MB / 0.0 MB"])


class ElevatedUpgradeFailureTests(unittest.TestCase):
    def setUp(self):
        self.module = load("failure_elevation", "src/ops/packages/elevated_upgrade.py")

    def elevate_failing(self, launch, wait=0):
        """Elevate through fakes where `launch` plays ShellExecuteExW; answer the kernel and the error text."""
        shell = SimpleNamespace(ShellExecuteExW=Mock(side_effect=launch))
        kernel = SimpleNamespace(WaitForSingleObject=Mock(return_value=wait), GetExitCodeProcess=Mock(),
                                 CloseHandle=Mock())
        with patch.object(ctypes, "WinDLL", side_effect=[shell, kernel], create=True), windows_errors(ERROR):
            with self.assertRaises(OSError) as raised:
                self.module.elevate("choco.exe", self.module.UPGRADE_ARGUMENTS)
        return kernel, str(raised.exception)

    def test_refused_uac_prompt_raises_the_windows_error_without_waiting(self):
        kernel, message = self.elevate_failing(lambda _info: 0)
        self.assertEqual(message, f"WinError {ERROR}")
        kernel.WaitForSingleObject.assert_not_called()

    def test_request_without_process_handle_raises_and_closes_nothing(self):
        kernel, message = self.elevate_failing(lambda _info: 1)
        self.assertEqual(message, "The UAC request returns no process handle.")
        kernel.CloseHandle.assert_not_called()

    def test_failed_wait_raises_without_reading_the_exit_code_and_closes_the_handle(self):
        def launch(pointer):
            pointer._obj.hProcess = 42
            return 1

        kernel, message = self.elevate_failing(launch, wait=0x102)
        self.assertEqual(message, f"WinError {ERROR}")
        kernel.GetExitCodeProcess.assert_not_called()
        kernel.CloseHandle.assert_called_once_with(42)

    def test_main_runs_the_discovered_executable_and_returns_its_exit_code(self):
        arguments = ["C:/choco.exe", "upgrade", "all", "-y"]
        with patch.object(self.module, "elevate", return_value=3010) as elevate:
            self.assertEqual(self.module.main(arguments), 3010)
            with patch.object(sys, "argv", ["elevated_upgrade.py", *arguments]):
                self.assertEqual(self.module.main(), 3010)
        self.assertEqual([call.args for call in elevate.call_args_list], [("C:/choco.exe", arguments[1:])] * 2)

    def test_main_reports_a_refusal_with_status_one(self):
        cases = {
            "requires the discovered Chocolatey executable": ([], None),
            "accepts only the fixed Chocolatey upgrade": (["choco.exe", "uninstall", "all"], None),
            "The operation was canceled by the user.": (["choco.exe", *self.module.UPGRADE_ARGUMENTS],
                                                         OSError("The operation was canceled by the user.")),
        }
        for message, (arguments, failure) in cases.items():
            with self.subTest(message), contextlib.redirect_stderr(io.StringIO()) as err, \
                 patch.object(self.module, "elevate", side_effect=failure) as elevate:
                self.assertEqual(self.module.main(arguments), 1)
            self.assertIn(message, err.getvalue())
            self.assertEqual(elevate.called, failure is not None)

    def test_script_without_arguments_exits_with_status_one(self):
        code, out, err = run_script("src/ops/packages/elevated_upgrade.py")
        self.assertEqual((code, out), (1, ""))
        self.assertEqual(err, "The elevated helper requires the discovered Chocolatey executable.\n")


class EventLogFailureTests(unittest.TestCase):
    def setUp(self):
        self.module = load("failure_events", "src/windows/system_messages.py")
        self.api = Mock()
        with patch.object(ctypes, "WinDLL", return_value=self.api, create=True):
            self.log = self.module.EventLog()

    def test_event_with_no_xml_size_raises_the_windows_error(self):
        self.api.EvtRender.return_value = 0
        with windows_errors(ERROR), self.assertRaisesRegex(OSError, f"^WinError {ERROR}$"):
            self.log.xml(5)
        self.api.EvtRender.assert_called_once()

    def test_failed_render_into_the_buffer_raises_the_windows_error(self):
        self.api.EvtRender.side_effect = needs_bytes(-2)
        with windows_errors(ERROR), self.assertRaisesRegex(OSError, f"^WinError {ERROR}$"):
            self.log.xml(5)
        self.assertEqual(self.api.EvtRender.call_count, 2)

    def test_provider_without_metadata_has_no_message_and_nothing_to_close(self):
        self.api.EvtOpenPublisherMetadata.return_value = 0
        self.assertEqual(self.log.message(5, "provider"), "")
        self.api.EvtFormatMessage.assert_not_called()
        self.api.EvtClose.assert_not_called()

    def test_message_that_needs_no_text_or_fails_is_empty_and_closes_its_metadata(self):
        self.api.EvtOpenPublisherMetadata.return_value = 7
        for name, side_effect in (("no text", None), ("failure", needs_bytes(-1))):
            self.api.EvtFormatMessage = Mock(return_value=0, side_effect=side_effect)
            with self.subTest(name):
                self.assertEqual(self.log.message(5, "provider"), "")
        self.assertEqual([call.args for call in self.api.EvtClose.call_args_list], [(7,), (7,)])

    def test_failed_query_raises_the_windows_error(self):
        self.api.EvtQuery.return_value = 0
        with windows_errors(ERROR), self.assertRaisesRegex(OSError, f"^WinError {ERROR}$"):
            self.log.report()

    def test_failed_next_other_than_no_more_items_raises_and_closes_the_query(self):
        self.api.EvtQuery.return_value, self.api.EvtNext.return_value = 4, 0
        with windows_errors(ERROR), self.assertRaisesRegex(OSError, f"^WinError {ERROR}$"):
            self.log.report()
        self.api.EvtClose.assert_called_once_with(4)

    def test_script_prints_the_report(self):
        def next_event(_query, _capacity, events, _timeout, _flags, count):
            events[0], count._obj.value = 5, 1
            return 1

        def render(_context, _event, _flags, _size, buffer, used, _count):
            used._obj.value = (len(EVENT_XML) + 1) * 2
            if buffer is not None:
                buffer.value = EVENT_XML
            return buffer is not None

        api = Mock(EvtQuery=Mock(return_value=4), EvtNext=Mock(side_effect=next_event),
                   EvtRender=Mock(side_effect=render), EvtOpenPublisherMetadata=Mock(return_value=0))
        code, out, err = run_script("src/windows/system_messages.py", api)
        self.assertEqual((code, err), (None, ""))
        self.assertEqual(out, "2026-11-04 00:00:00Z [Warning] provider (1) \n")


if __name__ == "__main__":
    unittest.main()
