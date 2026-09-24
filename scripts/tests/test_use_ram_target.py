"""Test the RAM target helper without touching the real /dev/shm."""

import contextlib
import importlib.util
import io
import os
from pathlib import Path
import re
import shlex
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from sandbox import REPO, Sandbox


SCRIPT = "scripts/use_ram_target.py"
# Evaluate the helper's stdout as the documented usage does, then show the result.
EVAL = 'eval "$("$0" "$1")" && printf %s "${CARGO_TARGET_DIR-unset}"'
# The same with the helper's stderr in $ERR, and a check instead of the value.
EVAL_QUIETLY = (
    'out=$("$0" "$1" 2>"$ERR"); echo "status $?"; eval "$out"; '
    '[ "${CARGO_TARGET_DIR-}" = "$FFM_RAM_TARGET" ] && echo exported'
)
# Free space as `df -h` prints it in the C locale: 844G, 7.8G, 512M or 0.
FREE = r"\d+(?:\.\d)?[KMGTPEZYRQ]?"


class RamTargetTests(unittest.TestCase):
    def setUp(self):
        self.sandbox = Sandbox(self)
        self.sandbox.copy(SCRIPT)
        # A space and a quote catch an export line the shell would split.
        self.parent = self.sandbox.tmp / "ram disk's"

    def run_helper(self, ram_dir, *args):
        return self.sandbox.run(SCRIPT, *args, env={"FFM_RAM_TARGET": str(ram_dir)}, input="")

    def eval_helper(self, ram_dir):
        """Run the helper under `sh`, eval its output, and return the shell's result."""
        return self.sandbox.execute(
            ["sh", "-c", EVAL, sys.executable, self.sandbox.repo / SCRIPT],
            env={"FFM_RAM_TARGET": str(ram_dir)},
        )

    def assert_exported(self, result, ram_dir):
        """One export line that the shell evaluates to exactly `ram_dir`."""
        self.assertEqual((result.returncode, result.stdout),
                         (0, f"export CARGO_TARGET_DIR={shlex.quote(str(ram_dir))}\n"))
        self.assertEqual(self.eval_helper(ram_dir).stdout, str(ram_dir))

    def test_a_writable_parent_gets_the_directory_and_one_export_line(self):
        self.parent.mkdir()
        ram_dir = self.parent / "ffm target"
        # The second run finds the directory already there.
        for attempt in (1, 2):
            with self.subTest(attempt=attempt):
                result = self.run_helper(ram_dir)
                self.assertEqual(result.returncode, 0)
                self.assertTrue(ram_dir.is_dir())
                self.assertEqual(len(result.stdout.splitlines()), 1)
                self.assertTrue(result.stdout.startswith("export CARGO_TARGET_DIR="))
                self.assertRegex(
                    result.stderr,
                    rf"\Affm: cargo target -> {re.escape(str(ram_dir))} \(tmpfs, {FREE} free\)\n\Z",
                )

    def test_eval_exports_the_directory_exactly(self):
        self.parent.mkdir()
        ram_dir = self.parent / "ffm target"
        result = self.eval_helper(ram_dir)
        self.assertEqual((result.returncode, result.stdout), (0, str(ram_dir)))

    def test_a_missing_or_non_directory_parent_keeps_the_disk_target(self):
        not_a_directory = self.sandbox.write("not a directory")
        for parent in (self.parent, not_a_directory):
            with self.subTest(parent=parent):
                result = self.run_helper(parent / "ffm-target")
                self.assertEqual((result.returncode, result.stdout), (0, ""))
                self.assertEqual(result.stderr, f"ffm: no writable {parent}; using on-disk ./target.\n")
                self.assertFalse((parent / "ffm-target").exists())
                self.assertEqual(self.eval_helper(parent / "ffm-target").stdout, "unset")

    def test_the_parent_is_the_one_dirname_prints(self):
        # dirname(1) keeps "." and "//"; a normalised "missing/." would be the grandparent.
        missing = self.sandbox.tmp / "missing"
        for ram_dir, parent in (
            (f"{missing}/.", missing),
            (f"{missing}/./ffm-target", f"{missing}/."),
            (f"{missing}//x/ffm-target", f"{missing}//x"),
        ):
            with self.subTest(ram_dir=ram_dir):
                result = self.run_helper(ram_dir)
                self.assertEqual((result.returncode, result.stdout), (0, ""))
                self.assertEqual(result.stderr, f"ffm: no writable {parent}; using on-disk ./target.\n")
                self.assertFalse(missing.exists())

    def test_a_target_mkdir_cannot_create_is_still_exported(self):
        # As in the shell script: mkdir's complaint, then the export line anyway.
        # mkdir(1) and df(1) word their complaints, and quote the path, themselves.
        self.parent.mkdir()
        cases = {
            "a file": (lambda target: target.write_text("x", encoding="utf-8"), FREE, ""),
            "a dangling symlink": (lambda target: target.symlink_to(self.sandbox.root / "gone"),
                                   "", r"df: .*a dangling symlink.*: No such file or directory\n"),
        }
        for name, (create, free, df_error) in cases.items():
            with self.subTest(name):
                target = self.parent / name
                create(target)
                result = self.run_helper(target)
                self.assert_exported(result, target)
                self.assertRegex(result.stderr, "".join((
                    rf"\Amkdir: cannot create directory .*{name}.*: File exists\n",
                    df_error,
                    rf"ffm: cargo target -> {re.escape(str(target))} \(tmpfs, {free} free\)\n\Z",
                )))

    def test_the_free_space_is_the_last_line_df_prints_without_padding(self):
        self.parent.mkdir()
        ram_dir = self.parent / "ffm target"
        self.sandbox.fake("df", stdout="Avail\n  42G\n")
        result = self.run_helper(ram_dir)
        self.assert_exported(result, ram_dir)
        self.assertEqual(result.stderr, f"ffm: cargo target -> {ram_dir} (tmpfs, 42G free)\n")
        self.assertEqual(self.sandbox.args("df")[0], ["-h", "--output=avail", str(ram_dir)])

    @unittest.skipIf(os.name == "nt" or os.geteuid() == 0, "root may write to any directory")
    def test_a_read_only_parent_keeps_the_disk_target(self):
        self.parent.mkdir(mode=0o555)
        self.addCleanup(self.parent.chmod, 0o755)
        result = self.run_helper(self.parent / "ffm-target")
        self.assertEqual((result.returncode, result.stdout), (0, ""))
        self.assertEqual(result.stderr, f"ffm: no writable {self.parent}; using on-disk ./target.\n")

    @unittest.skipIf(os.name == "nt" or os.geteuid() == 0, "root may search any directory")
    def test_a_parent_that_cannot_be_searched_still_exports_the_target(self):
        self.parent.mkdir(mode=0o222)
        self.addCleanup(self.parent.chmod, 0o755)
        target = self.parent / "ffm-target"
        result = self.run_helper(target)
        self.assert_exported(result, target)
        self.assertRegex(result.stderr, (
            # mkdir -p names the parent it could not search, not the target.
            r"\Amkdir: cannot create directory .*: Permission denied\n"
            r"df: .*ffm-target.*: Permission denied\n"
            rf"ffm: cargo target -> {re.escape(str(target))} \(tmpfs,  free\)\n\Z"
        ))

    def test_a_target_that_is_not_utf8_is_exported_byte_for_byte(self):
        self.parent.mkdir()
        ram_dir = os.fsencode(self.parent) + b"/ffm-\xff"
        stderr = self.sandbox.root / "stderr"
        result = self.sandbox.execute(
            ["sh", "-c", EVAL_QUIETLY, sys.executable, self.sandbox.repo / SCRIPT],
            # Python's strict UTF-8 output, as under a UTF-8 locale without UTF-8 mode.
            env={"FFM_RAM_TARGET": os.fsdecode(ram_dir), "ERR": str(stderr),
                 "PYTHONUTF8": "0", "PYTHONIOENCODING": "utf-8:strict"},
        )
        self.assertEqual(result.stdout, "status 0\nexported\n")
        self.assertTrue(os.path.isdir(ram_dir))
        self.assertRegex(stderr.read_bytes(), rb"\Affm: cargo target -> " + re.escape(ram_dir)
                         + rb" \(tmpfs, " + FREE.encode() + rb" free\)\n\Z")

    def test_arguments_are_ignored_as_when_the_script_was_sourced(self):
        self.parent.mkdir()
        ram_dir = self.parent / "ffm-target"
        result = self.run_helper(ram_dir, "extra", "--bogus")
        self.assert_exported(result, ram_dir)
        self.assertTrue(ram_dir.is_dir())

    def test_help_shows_the_eval_usage_and_that_the_variable_must_be_exported(self):
        self.parent.mkdir()
        for flag in ("-h", "--help"):
            with self.subTest(flag=flag):
                result = self.run_helper(self.parent / "ffm-target", flag)
                self.assertEqual((result.returncode, result.stdout), (0, ""))
                self.assertIn('eval "$(scripts/use_ram_target.py)"', result.stderr)
                self.assertIn("export FFM_RAM_TARGET=/path", result.stderr)
                self.assertIn('eval "$(FFM_RAM_TARGET=/path scripts/use_ram_target.py)"', result.stderr)
                self.assertFalse((self.parent / "ffm-target").exists())

    def test_help_gives_an_eval_nothing_to_run(self):
        # The help holds shell lines, such as one that appends to ~/.bashrc.
        result = self.sandbox.execute(
            ["sh", "-c", 'eval "$("$0" "$1" --help 2>/dev/null)" && printf %s "${CARGO_TARGET_DIR-unset}"',
             sys.executable, self.sandbox.repo / SCRIPT],
        )
        self.assertEqual((result.returncode, result.stdout, result.stderr), (0, "unset", ""))
        self.assertEqual(list(self.sandbox.home.iterdir()), [])


def load_helper():
    """Import the script by path; it does no work at import time."""
    spec = importlib.util.spec_from_file_location("use_ram_target", REPO / SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class DirnameTests(unittest.TestCase):
    """In process: dirname() is a pure function."""

    def test_matches_dirname_1(self):
        # What GNU dirname prints for each path.
        expected = {
            "/dev/shm/ffm-target": "/dev/shm", "/a/b/": "/a", "/a//b": "/a", "//a//b//": "//a",
            "/a/./b": "/a/.", "missing/.": "missing", "ffm-target": ".", "a/": ".", "../a": "..",
            "/ffm": "/", "//x": "/", "/": "/", "///": "/",
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
