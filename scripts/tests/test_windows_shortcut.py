"""Check Windows shortcut decisions without a build or desktop writes."""

import base64
import contextlib
import ctypes
import io
import json
from pathlib import Path
import subprocess
import sys
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
import create_desktop_shortcut  # noqa: E402
import windows_shortcut  # noqa: E402


def run_main(root):
    """Run windows_shortcut.main(root); answer its status, stdout and stderr."""
    with contextlib.redirect_stdout(io.StringIO()) as out, contextlib.redirect_stderr(io.StringIO()) as err:
        status = windows_shortcut.main(root)
    return status, out.getvalue(), err.getvalue()


class WindowsShortcutTests(unittest.TestCase):
    def test_terminal_and_direct_shortcut_properties(self):
        root = Path("C:/checkout with spaces")
        executable = root / "target/release/ffm.exe"
        terminal = windows_shortcut.properties(root, executable, True)
        direct = windows_shortcut.properties(root, executable, False)
        self.assertEqual(terminal["target"], "wt.exe")
        self.assertEqual(terminal["arguments"], f'-d "{root}" -- "{executable}"')
        self.assertEqual(direct["target"], str(executable))
        self.assertEqual(direct["working_directory"], str(root))
        self.assertEqual(direct["icon"], str(executable) + ",0")

    def test_main_builds_only_when_missing_and_writes_both_outputs(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            executable = root / "target/release/ffm.exe"
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b"fixture")
            desktop = root / "desktop"
            desktop.mkdir()
            with patch.object(windows_shortcut, "desktop_directory", return_value=desktop), \
                 patch.object(windows_shortcut.shutil, "which", return_value=None), \
                 patch.object(windows_shortcut, "write_shortcut") as write, \
                 patch.object(windows_shortcut.subprocess, "run") as launch:
                self.assertEqual(windows_shortcut.main(root), 0)
            launch.assert_not_called()
            self.assertEqual(write.call_args.args[0], desktop / "FunnyFileManager.lnk")
            self.assertIn(str(executable), (desktop / "FunnyFileManager.bat").read_text(encoding="utf-8"))

    def test_desktop_directory_is_the_shell_desktop_folder(self):
        def shell(_window, folder, _token, _flags, buffer):
            buffer.value = "C:\\Profiles\\Tester\\Desktop"
            return 0

        folder = Mock(side_effect=shell)
        windll = SimpleNamespace(shell32=SimpleNamespace(SHGetFolderPathW=folder))
        with patch.object(ctypes, "windll", windll, create=True):
            desktop = windows_shortcut.desktop_directory()
        self.assertEqual(str(desktop), "C:\\Profiles\\Tester\\Desktop")
        self.assertEqual(folder.call_args.args[1], 0x10)

    def test_desktop_directory_names_the_failed_hresult(self):
        windll = SimpleNamespace(shell32=SimpleNamespace(SHGetFolderPathW=Mock(return_value=0x80004005)))
        with patch.object(ctypes, "windll", windll, create=True), self.assertRaises(OSError) as raised:
            windows_shortcut.desktop_directory()
        self.assertEqual(str(raised.exception), "Cannot find the Windows desktop: HRESULT 0x80004005")

    def test_shortcut_goes_to_powershell_as_an_encoded_script_with_values_in_the_environment(self):
        path = Path("C:/Desktop with spaces/FunnyFileManager.lnk")
        values = windows_shortcut.properties(Path("C:/it's here"), Path("C:/it's here/ffm.exe"), False)
        with patch.object(windows_shortcut.shutil, "which", return_value="C:/pwsh.exe"), \
             patch.object(windows_shortcut.subprocess, "run") as launch:
            windows_shortcut.write_shortcut(path, values)
        command = launch.call_args.args[0]
        script = base64.b64decode(command[-1]).decode("utf-16-le")
        self.assertEqual(command[:-1], ["C:/pwsh.exe", "-NoProfile", "-NonInteractive", "-EncodedCommand"])
        self.assertIn("CreateShortcut($p.path)", script)
        self.assertNotIn("it's here", script)
        self.assertEqual(json.loads(launch.call_args.kwargs["env"]["FFM_SHORTCUT"]), {"path": str(path), **values})
        self.assertTrue(launch.call_args.kwargs["check"])

    def test_windows_powershell_stands_in_when_pwsh_is_missing(self):
        hosts = {"powershell.exe": "C:/powershell.exe"}
        with patch.object(windows_shortcut.shutil, "which", side_effect=hosts.get), \
             patch.object(windows_shortcut.subprocess, "run") as launch:
            windows_shortcut.write_shortcut(Path("a.lnk"), {})
        self.assertEqual(launch.call_args.args[0][0], "C:/powershell.exe")

    def test_missing_powershell_host_is_an_error(self):
        with patch.object(windows_shortcut.shutil, "which", return_value=None), \
             patch.object(windows_shortcut.subprocess, "run") as launch, self.assertRaises(OSError) as raised:
            windows_shortcut.write_shortcut(Path("a.lnk"), {})
        self.assertEqual(str(raised.exception), "A PowerShell host is required for the WScript shortcut adapter.")
        launch.assert_not_called()

    def test_main_builds_a_missing_executable_before_writing_the_shortcut(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            executable = root / "target/release/ffm.exe"

            def build(*_arguments, **_options):
                executable.parent.mkdir(parents=True)
                executable.write_bytes(b"fixture")

            with patch.object(windows_shortcut, "desktop_directory", return_value=root), \
                 patch.object(windows_shortcut.shutil, "which", return_value="C:/wt.exe"), \
                 patch.object(windows_shortcut, "write_shortcut") as write, \
                 patch.object(windows_shortcut.subprocess, "run", side_effect=build) as launch:
                status, out, err = run_main(root)
        launch.assert_called_once_with(["cargo", "build", "--release"], cwd=root, check=False)
        self.assertEqual((status, err), (0, ""))
        self.assertEqual(out.splitlines()[:2],
                         [f"FFM executable not found at {executable}", "Building FFM in release mode..."])
        self.assertEqual(write.call_args.args[1]["target"], "wt.exe")

    def test_main_fails_when_the_build_leaves_no_executable(self):
        with tempfile.TemporaryDirectory() as folder, \
             patch.object(windows_shortcut.subprocess, "run"), \
             patch.object(windows_shortcut, "desktop_directory") as desktop:
            status, out, err = run_main(Path(folder))
        self.assertEqual(status, 1)
        self.assertEqual(err, "Failed to build FFM. Please run 'cargo build --release' manually.\n")
        desktop.assert_not_called()

    def test_main_reports_a_failed_desktop_or_shortcut_with_status_one(self):
        failures = {"desktop": OSError("Cannot find the Windows desktop"),
                    "shortcut": subprocess.CalledProcessError(1, "pwsh.exe")}
        for step, failure in failures.items():
            with self.subTest(step), tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                (root / "target/release").mkdir(parents=True)
                (root / "target/release/ffm.exe").write_bytes(b"fixture")
                with patch.object(windows_shortcut, "desktop_directory", return_value=root,
                                  side_effect=failure if step == "desktop" else None), \
                     patch.object(windows_shortcut, "write_shortcut",
                                  side_effect=failure if step == "shortcut" else None):
                    status, out, err = run_main(root)
                self.assertEqual((status, out, err), (1, "", f"{failure}\n"))

    def test_the_linux_script_hands_the_checkout_to_the_native_adapter_on_windows(self):
        with patch.object(create_desktop_shortcut, "os", SimpleNamespace(name="nt")), \
             patch.object(create_desktop_shortcut, "desktop_directory") as linux_desktop, \
             patch.object(windows_shortcut, "main", return_value=7) as native:
            self.assertEqual(create_desktop_shortcut.main([]), 7)
        native.assert_called_once_with(create_desktop_shortcut.ROOT)
        linux_desktop.assert_not_called()


if __name__ == "__main__":
    unittest.main()
