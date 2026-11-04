"""Exercise native adapters with fake DLL functions and process handles."""

import ctypes
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

from test_windows_python_reports import load


class NativeApiTests(unittest.TestCase):
    def test_performance_reads_cpu_memory_and_fixed_drives(self):
        module = load("native_performance", "src/windows/performance.py")
        samples = iter([(100, 200, 100), (150, 400, 200)])

        def times(*values):
            for pointer, value in zip(values, next(samples)):
                pointer._obj.dwLowDateTime = value
            return 1

        def memory(pointer):
            pointer._obj.total, pointer._obj.available = 8192 * 2**20, 2048 * 2**20
            return 1

        def disk(_drive, available, total, free):
            available._obj.value, total._obj.value, free._obj.value = 20 * 2**30, 100 * 2**30, 25 * 2**30
            return 1

        kernel = SimpleNamespace(GetSystemTimes=Mock(side_effect=times), GlobalMemoryStatusEx=Mock(side_effect=memory),
                                 GetLogicalDrives=Mock(return_value=1 << 2), GetDriveTypeW=Mock(return_value=3),
                                 GetDiskFreeSpaceExW=Mock(side_effect=disk))
        with patch.object(ctypes, "WinDLL", return_value=kernel, create=True), patch.object(module.time, "sleep"):
            text = module.report()
        self.assertIn("CPU Load: 83.3%", text)
        self.assertIn("Memory Free: 2048.0 MB / 8192.0 MB", text)
        self.assertIn("C: Used: 75.0%", text)

    def test_uac_waits_returns_status_and_always_closes_its_handle(self):
        module = load("native_elevation", "src/ops/packages/elevated_upgrade.py")

        def launch(pointer):
            info = pointer._obj
            self.assertEqual(info.lpVerb, "runas")
            self.assertEqual(info.lpFile, "C:/with spaces/choco.exe")
            self.assertEqual(info.lpParameters, "upgrade all -y")
            info.hProcess = 42
            return 1

        def code(_handle, result):
            result._obj.value = 3010
            return 1

        shell = SimpleNamespace(ShellExecuteExW=Mock(side_effect=launch))
        kernel = SimpleNamespace(WaitForSingleObject=Mock(return_value=0), GetExitCodeProcess=Mock(side_effect=code),
                                 CloseHandle=Mock())
        with patch.object(ctypes, "WinDLL", side_effect=[shell, kernel], create=True):
            self.assertEqual(module.elevate("C:/with spaces/choco.exe", module.UPGRADE_ARGUMENTS), 3010)
        kernel.CloseHandle.assert_called_once_with(42)
        kernel.GetExitCodeProcess.side_effect = None
        kernel.GetExitCodeProcess.return_value = 0
        with patch.object(ctypes, "WinDLL", side_effect=[shell, kernel], create=True), \
             patch.object(ctypes, "get_last_error", return_value=5, create=True), \
             patch.object(ctypes, "WinError", side_effect=lambda error: OSError(str(error)), create=True):
            with self.assertRaises(OSError):
                module.elevate("C:/with spaces/choco.exe", module.UPGRADE_ARGUMENTS)
        self.assertEqual(kernel.CloseHandle.call_count, 2)

    def test_event_log_releases_event_query_and_metadata_handles(self):
        module = load("native_events", "src/windows/system_messages.py")
        xml = ('<Event xmlns="http://schemas.microsoft.com/win/2004/08/events/event"><System>'
               '<Provider Name="provider"/><EventID>1</EventID><Level>3</Level>'
               '<TimeCreated SystemTime="2026-11-04T00:00:00Z"/></System></Event>')

        def next_event(_query, _capacity, events, _timeout, _flags, count):
            events[0], count._obj.value = 5, 1
            return 1

        def render(_context, _event, _flags, _size, buffer, used, _count):
            used._obj.value = (len(xml) + 1) * 2
            if buffer is not None:
                buffer.value = xml
            return buffer is not None

        def message(_meta, _event, _id, _count, _values, _flags, _size, buffer, used):
            used._obj.value = 8
            if buffer is not None:
                buffer.value = "message"
            return buffer is not None

        api = SimpleNamespace(EvtQuery=Mock(return_value=4), EvtNext=Mock(side_effect=next_event),
                              EvtRender=Mock(side_effect=render), EvtClose=Mock(),
                              EvtOpenPublisherMetadata=Mock(return_value=6), EvtFormatMessage=Mock(side_effect=message))
        with patch.object(ctypes, "WinDLL", return_value=api, create=True):
            self.assertIn("[Warning] provider (1) message", module.EventLog().report())
        self.assertEqual([call.args for call in api.EvtClose.call_args_list], [(6,), (5,), (4,)])
        api.EvtNext.side_effect, api.EvtNext.return_value = None, 0
        with patch.object(ctypes, "WinDLL", return_value=api, create=True), \
             patch.object(ctypes, "get_last_error", return_value=259, create=True):
            self.assertEqual(module.EventLog().report(), "")


if __name__ == "__main__":
    unittest.main()
