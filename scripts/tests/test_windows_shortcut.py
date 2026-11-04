"""Check Windows shortcut decisions without a build or desktop writes."""

from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import windows_shortcut  # noqa: E402


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


if __name__ == "__main__":
    unittest.main()
