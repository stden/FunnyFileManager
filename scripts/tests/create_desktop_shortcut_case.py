"""The sandbox checkout, the fake build and the expected entry that the tests
of create_desktop_shortcut.py share.
"""

import os
import shutil
import unittest

from sandbox import Sandbox
from script_files import SHORTCUT


SCRIPT = "create_desktop_shortcut.py"
ENTRY_NAME = "funny-file-manager.desktop"
# Creates the release binary the way a successful `cargo build --release` would.
BUILD_BINARY = """
    os.makedirs("target/release", exist_ok=True)
    open("target/release/ffm", "w").close()
"""


# Runs the script with os.chmod refusing target/release/ffm, as the kernel
# refuses a file another user owns. The sandbox cannot make such a file: its
# tests may run as root, whom no owner check stops.
DENY_FFM_CHMOD = """
import os, runpy, sys
allowed_chmod = os.chmod
def chmod(path, mode, **options):
    if os.fspath(path).endswith("/target/release/ffm"):
        raise PermissionError(1, "Operation not permitted", os.fspath(path))
    allowed_chmod(path, mode, **options)
os.chmod = chmod
sys.argv = sys.argv[1:]
runpy.run_path(sys.argv[0], run_name="__main__")
"""


def current_umask():
    umask = os.umask(0)
    os.umask(umask)
    return umask


class ShortcutTestCase(unittest.TestCase):
    def setUp(self):
        self.sandbox = Sandbox(self)
        # The script, the module that it imports, and shell_path.py, through
        # which it finds cargo and desktop-file-validate.
        for relative in SHORTCUT:
            self.sandbox.copy(relative)
        self.home = self.sandbox.home
        self.repo = self.sandbox.repo
        self.ffm = self.repo / "target/release/ffm"
        self.sandbox.fake("desktop-file-validate")

    def make_ffm(self):
        self.sandbox.write("target/release/ffm", "binary")
        self.ffm.chmod(0o600)

    def make_desktop(self, name="Desktop"):
        desktop = self.home / name
        desktop.mkdir()
        return desktop / ENTRY_NAME

    def create(self, *args, env=None, cwd=None):
        return self.sandbox.run(SCRIPT, *args, env=env, cwd=cwd)

    def expected_entry(self):
        return "\n".join([
            "[Desktop Entry]",
            "Version=1.0",
            "Type=Application",
            "Name=Funny File Manager",
            "Comment=Terminal-based file manager inspired by Far Manager",
            f"Exec={self.ffm}",
            f"Path={self.repo}",
            "Icon=utilities-file-manager",
            "Terminal=true",
            "Categories=Utility;FileManager;",
            "",
        ])

    def copy_script(self, checkout):
        """Copy the script and the modules that it imports into another checkout."""
        for relative in SHORTCUT:
            (checkout / relative).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(self.repo / relative, checkout / relative)
