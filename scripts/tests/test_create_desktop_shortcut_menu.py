"""Check what follows the entry: the validator, the applications menu copy and chmod."""

import stat
import sys
import unittest

from create_desktop_shortcut_case import DENY_FFM_CHMOD, ENTRY_NAME, SCRIPT, ShortcutTestCase, current_umask


class IntegrationTests(ShortcutTestCase):
    def setUp(self):
        super().setUp()
        self.make_ffm()
        self.entry = self.make_desktop()
        self.applications = self.home / ".local/share/applications"

    def test_the_validator_checks_the_entry_and_its_verdict_is_advisory(self):
        self.sandbox.fake("desktop-file-validate", status=1, stdout="error: invalid key\n")
        result = self.create()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.sandbox.args("desktop-file-validate"), [[str(self.entry)]])
        self.assertEqual(result.stdout.splitlines(), [
            f"Desktop entry created at: {self.entry}",
            "error: invalid key",
        ])

    def test_without_a_validator_nothing_is_validated(self):
        self.sandbox.use_real("desktop-file-validate")
        result = self.create(env={"PATH": str(self.sandbox.bin)})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.sandbox.calls("desktop-file-validate"), [])

    def test_the_entry_is_copied_into_an_existing_applications_menu(self):
        self.applications.mkdir(parents=True)
        result = self.create()
        self.assertEqual(result.returncode, 0, result.stderr)
        copied = self.applications / ENTRY_NAME
        self.assertEqual(result.stdout.splitlines(), [
            f"Desktop entry created at: {self.entry}",
            f"Application menu entry created at: {copied}",
        ])
        self.assertEqual(copied.read_text(encoding="utf-8"), self.expected_entry())

    def test_a_new_menu_entry_gets_the_source_mode_less_the_umask(self):
        self.applications.mkdir(parents=True)
        self.create()
        copied = self.applications / ENTRY_NAME
        self.assertEqual(stat.S_IMODE(copied.stat().st_mode), 0o777 & ~current_umask())

    def test_an_existing_menu_entry_keeps_its_mode_like_cp(self):
        copied = self.applications / ENTRY_NAME
        self.applications.mkdir(parents=True)
        copied.write_text("old entry\n", encoding="utf-8")
        copied.chmod(0o600)
        result = self.create()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(copied.read_text(encoding="utf-8"), self.expected_entry())
        self.assertEqual(stat.S_IMODE(copied.stat().st_mode), 0o600)

    def test_a_failed_copy_is_reported_and_the_run_still_succeeds(self):
        # cp cannot put a file where a directory is; bash went on regardless.
        copied = self.applications / ENTRY_NAME
        copied.mkdir(parents=True)
        result = self.create()
        self.assertEqual(result.returncode, 0, result.stderr)
        reason = f"cp: cannot overwrite directory '{copied}' with non-directory\n"
        self.assertEqual(result.stderr, reason)
        self.assertEqual(result.stdout.splitlines(), [
            f"Desktop entry created at: {self.entry}",
            f"Application menu entry created at: {copied}",
        ])

    def test_a_failed_chmod_is_reported_and_the_run_still_succeeds(self):
        self.applications.mkdir(parents=True)
        result = self.sandbox.execute([sys.executable, "-c", DENY_FFM_CHMOD, self.repo / SCRIPT])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            result.stderr,
            f"chmod: changing permissions of '{self.ffm}': Operation not permitted\n",
        )
        self.assertEqual(result.stdout.splitlines(), [
            f"Desktop entry created at: {self.entry}",
            f"Application menu entry created at: {self.applications / ENTRY_NAME}",
        ])
        self.assertEqual(self.sandbox.args("desktop-file-validate"), [[str(self.entry)]])
        self.assertEqual(stat.S_IMODE(self.ffm.stat().st_mode), 0o600)

    def test_a_validator_that_cannot_start_is_reported_like_the_shell(self):
        # `command -v` also finds a validator without execute permission.
        validator = self.sandbox.bin / "desktop-file-validate"
        cases = {
            "not executable": (0o644, "#!/bin/sh\n", "Permission denied"),
            "missing interpreter": (0o755, "#!/nonexistent/interpreter\n",
                                    "cannot execute: required file not found"),
        }
        for case, (mode, text, reason) in cases.items():
            with self.subTest(case):
                validator.write_text(text, encoding="utf-8")
                validator.chmod(mode)
                # Only the sandbox's bin: a real validator stays unseen.
                result = self.create(env={"PATH": str(self.sandbox.bin)})
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stderr, f"{validator}: {reason}\n")
                self.assertEqual(result.stdout, f"Desktop entry created at: {self.entry}\n")

    def test_the_applications_menu_is_never_created(self):
        result = self.create()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.home / ".local").exists())
        self.assertNotIn("Application menu entry", result.stdout)


if __name__ == "__main__":
    unittest.main()
