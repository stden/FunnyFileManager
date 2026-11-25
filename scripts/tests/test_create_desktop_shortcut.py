"""Run create_desktop_shortcut.py in a sandbox HOME with fake Cargo and desktop tools.

Where the entry goes, its encoding and the arguments; the build and the
application menu have test_create_desktop_shortcut_*.py files of their own,
and create_desktop_shortcut_case.py holds what they share.
"""

import os
import stat
import subprocess
import sys
import unittest

from create_desktop_shortcut_case import BUILD_BINARY, ENTRY_NAME, SCRIPT, ShortcutTestCase, current_umask


class EntryTests(ShortcutTestCase):
    def test_writes_the_entry_to_the_home_desktop(self):
        self.make_ffm()
        entry = self.make_desktop()
        result = self.create()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((result.stdout, result.stderr), (f"Desktop entry created at: {entry}\n", ""))
        self.assertEqual(entry.read_text(encoding="utf-8"), self.expected_entry())
        self.assertEqual(self.sandbox.calls("cargo"), [])
        self.assertEqual(self.sandbox.calls("xdg-user-dir"), [])

    def test_the_entry_and_the_binary_become_executable_like_chmod_plus_x(self):
        self.make_ffm()
        entry = self.make_desktop()
        self.create()
        umask = current_umask()
        self.assertEqual(stat.S_IMODE(entry.stat().st_mode), 0o777 & ~umask)
        self.assertEqual(stat.S_IMODE(self.ffm.stat().st_mode), 0o600 | (0o111 & ~umask))

    def test_the_russian_desktop_is_the_second_choice(self):
        self.make_ffm()
        entry = self.make_desktop("Рабочий стол")
        result = self.create()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, f"Desktop entry created at: {entry}\n")
        self.assertTrue(entry.is_file())
        self.assertEqual(self.sandbox.calls("xdg-user-dir"), [])

    def test_the_english_desktop_wins_over_the_russian_one(self):
        self.make_ffm()
        entry = self.make_desktop()
        russian = self.make_desktop("Рабочий стол")
        self.create()
        self.assertTrue(entry.is_file())
        self.assertFalse(russian.exists())

    def test_xdg_user_dir_names_the_desktop_otherwise(self):
        self.make_ffm()
        desktop = self.home / "Schreibtisch mit Leerzeichen"
        desktop.mkdir()
        self.sandbox.fake("xdg-user-dir", stdout=f"{desktop}\n", stderr="noise\n")
        result = self.create()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, "")
        self.assertEqual(result.stdout, f"Desktop entry created at: {desktop / ENTRY_NAME}\n")
        self.assertEqual(self.sandbox.args("xdg-user-dir"), [["DESKTOP"]])


class MissingDesktopTests(ShortcutTestCase):
    """Deliberate change: bash failed the write yet reported the entry as created."""

    def assert_write_fails(self, result, entry, reason="No such file or directory"):
        self.assertEqual((result.returncode, result.stdout), (1, ""))
        self.assertEqual(result.stderr, f"{entry}: {reason}\n")
        # It stops at once: no chmod, no validation.
        self.assertEqual(stat.S_IMODE(self.ffm.stat().st_mode), 0o600)
        self.assertEqual(self.sandbox.calls("desktop-file-validate"), [])

    def test_an_entry_path_that_is_a_directory(self):
        self.make_ffm()
        entry = self.make_desktop()
        entry.mkdir()
        self.assert_write_fails(self.create(), entry, "Is a directory")

    def test_an_xdg_user_dir_without_an_answer_falls_back_to_home_desktop(self):
        self.make_ffm()
        for answer in ({"status": 1}, {"stdout": ""}):
            with self.subTest(**answer):
                self.sandbox.fake("xdg-user-dir", **answer)
                self.assert_write_fails(self.create(), self.home / "Desktop" / ENTRY_NAME)

    def test_a_missing_xdg_user_dir_falls_back_to_home_desktop(self):
        self.make_ffm()
        self.sandbox.use_real("xdg-user-dir")
        # Only the sandbox's bin, so a real /usr/bin/xdg-user-dir stays unseen.
        result = self.create(env={"PATH": str(self.sandbox.bin)})
        self.assert_write_fails(result, self.home / "Desktop" / ENTRY_NAME)

    def test_a_desktop_that_xdg_user_dir_names_but_does_not_exist(self):
        self.make_ffm()
        missing = self.home / "Missing"
        self.sandbox.fake("xdg-user-dir", stdout=f"{missing}\n")
        self.assert_write_fails(self.create(), missing / ENTRY_NAME)


class RelativeDesktopTests(ShortcutTestCase):
    """xdg-user-dir answering a relative path: bash resolved it after its `cd`."""

    def setUp(self):
        super().setUp()
        self.sandbox.fake("xdg-user-dir", stdout="relative desk\n")
        for base in (self.home, self.repo):
            (base / "relative desk").mkdir()

    def test_after_a_build_it_resolves_in_the_checkout(self):
        self.sandbox.fake("cargo", effect=BUILD_BINARY)
        result = self.create(cwd=self.home)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.repo / "relative desk" / ENTRY_NAME).is_file())
        self.assertFalse((self.home / "relative desk" / ENTRY_NAME).exists())
        call = self.sandbox.calls("desktop-file-validate")[0]
        self.assertEqual((call.args, call.cwd), ([f"relative desk/{ENTRY_NAME}"], str(self.repo)))

    def test_without_a_build_it_resolves_where_the_script_was_started(self):
        self.make_ffm()
        result = self.create(cwd=self.home)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.home / "relative desk" / ENTRY_NAME).is_file())
        self.assertFalse((self.repo / "relative desk" / ENTRY_NAME).exists())


class EncodingTests(ShortcutTestCase):
    def test_paths_that_are_not_utf8_keep_their_own_bytes(self):
        # Outside UTF-8 mode, a UTF-8 locale gives Python a strict stdout.
        checkout = self.sandbox.root / os.fsdecode(b"check\xffout")
        home = self.sandbox.root / os.fsdecode(b"h\xffome")
        (checkout / "target/release").mkdir(parents=True)
        (checkout / "target/release/ffm").write_text("binary", encoding="utf-8")
        (home / "Desktop").mkdir(parents=True)
        self.copy_script(checkout)
        environment = self.sandbox.env({"PYTHONUTF8": "0", "LC_ALL": "C.UTF-8", "HOME": str(home)})
        # Bytes, not text: the output is deliberately not UTF-8.
        result = subprocess.run(
            [sys.executable, checkout / SCRIPT],
            cwd=checkout, env=environment, capture_output=True, timeout=60, check=False,
        )
        entry = home / "Desktop" / ENTRY_NAME
        self.assertEqual((result.returncode, result.stderr), (0, b""))
        self.assertEqual(result.stdout, b"Desktop entry created at: " + os.fsencode(entry) + b"\n")
        written = entry.read_bytes()
        self.assertIn(b"\nExec=" + os.fsencode(checkout / "target/release/ffm") + b"\n", written)
        self.assertIn(b"\nPath=" + os.fsencode(checkout) + b"\n", written)

    def test_the_russian_desktop_is_found_under_an_ascii_locale(self):
        self.make_ffm()
        entry = self.make_desktop("Рабочий стол")
        result = self.create(env={"PYTHONUTF8": "0", "LC_ALL": "C"})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, f"Desktop entry created at: {entry}\n")
        self.assertEqual(entry.read_text(encoding="utf-8"), self.expected_entry())


class ArgumentTests(ShortcutTestCase):
    def test_arguments_are_ignored_like_bash(self):
        self.make_ffm()
        entry = self.make_desktop()
        # `--he` too: it is no abbreviation of --help.
        result = self.create("extra", "--bogus", "--he")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, f"Desktop entry created at: {entry}\n")
        self.assertTrue(entry.is_file())

    def test_help_describes_the_script(self):
        result = self.create("--help")
        self.assertEqual(result.returncode, 0)
        self.assertIn("Create a Linux desktop entry", result.stdout)
        self.assertEqual(self.sandbox.calls(), [])


if __name__ == "__main__":
    unittest.main()
