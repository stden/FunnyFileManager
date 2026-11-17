"""Test the RAM target helper in process: dirname(), and a noexec /dev/shm."""

import contextlib
import io
import os
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from use_ram_target_case import load_helper


class DirnameTests(unittest.TestCase):
    """In process: dirname() is a pure function."""

    def test_matches_dirname_1(self):
        # What GNU dirname prints for each path.
        expected = {
            "/dev/shm/ffm-target": "/dev/shm", "/a/b/": "/a", "/a//b": "/a", "//a//b//": "//a",
            "/a/./b": "/a/.", "missing/.": "missing", "ffm-target": ".", "a/": ".", "../a": "..",
            "/ffm": "/", "//x": "/", "/": "/", "///": "/",
            r"a\b\c": ".", "/a/\\b": "/a",
        }
        dirname = load_helper().dirname
        self.assertEqual({path: dirname(path) for path in expected}, expected)


@unittest.skipUnless(hasattr(os, "ST_NOEXEC"), "mount flags from statvfs are Linux-only")
class NoexecTests(unittest.TestCase):
    """In process, because a test cannot mount a noexec filesystem.

    os.statvfs is patched to report noexec, and subprocess.run is patched too, so
    even a broken helper cannot run mkdir on the real /dev/shm/ffm-target.
    """

    def test_a_noexec_parent_keeps_the_disk_target(self):
        helper = load_helper()
        flags = SimpleNamespace(f_flag=os.ST_NOEXEC | os.ST_NOSUID)
        # Unset and empty both mean the default /dev/shm/ffm-target.
        for environment in ({}, {"FFM_RAM_TARGET": ""}):
            with self.subTest(environment=environment):
                stdout, stderr = io.StringIO(), io.StringIO()
                with (
                    patch.dict(os.environ),
                    patch("os.statvfs", return_value=flags) as statvfs,
                    patch("subprocess.run") as run,
                    contextlib.redirect_stdout(stdout),
                    contextlib.redirect_stderr(stderr),
                ):
                    os.environ.pop("FFM_RAM_TARGET", None)
                    os.environ.update(environment)
                    status = helper.main([])
                self.assertEqual((status, stdout.getvalue()), (0, ""))
                self.assertEqual(stderr.getvalue(), (
                    "ffm: /dev/shm is mounted noexec; cargo can't exec from it.\n"
                    "ffm: keeping on-disk ./target.\n"
                ))
                self.assertEqual(Path(statvfs.call_args.args[0]), Path("/dev/shm"))
                run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
