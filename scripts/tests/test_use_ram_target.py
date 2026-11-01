"""Test the RAM target helper without touching the real /dev/shm.

Here it runs as a program, under a shell; test_use_ram_target_in_process.py
holds the checks that only an in-process call can make.
"""

import os
import re
import sys
import unittest

from use_ram_target_case import EVAL_QUIETLY, FREE, SCRIPT, RamTargetTestCase


class RamTargetTests(RamTargetTestCase):
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
            "a file": (lambda target: target.write_text("x", encoding="utf-8"), FREE),
            "a dangling symlink": (lambda target: target.symlink_to(self.sandbox.root / "gone"),
                                   ""),
        }
        for name, (create, free) in cases.items():
            with self.subTest(name):
                target = self.parent / name
                create(target)
                mkdir = self.sandbox.execute(["mkdir", "-p", target])
                disk_free = self.sandbox.execute(["df", "-h", "--output=avail", target])
                self.assertNotEqual(mkdir.returncode, 0)
                result = self.run_helper(target)
                self.assert_exported(result, target)
                self.assertRegex(result.stderr, "".join((
                    r"\A" + re.escape(mkdir.stderr + disk_free.stderr),
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


if __name__ == "__main__":
    unittest.main()
