"""Run create_desktop_shortcut.py in a sandbox HOME with fake Cargo and desktop tools."""

import os
import shutil
import signal
import stat
import subprocess
import sys
import time
import unittest

from sandbox import Sandbox


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
        self.sandbox.copy(SCRIPT)
        # The shortcut finds cargo and desktop-file-validate through it.
        self.sandbox.copy("scripts/shell_path.py")
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


class BuildTests(ShortcutTestCase):
    def setUp(self):
        super().setUp()
        self.entry = self.make_desktop()

    def test_a_missing_binary_is_built_in_the_checkout_first(self):
        # CARGO_TARGET_DIR is ignored: the entry points at the checkout's own target/.
        elsewhere = self.sandbox.root / "elsewhere"
        (elsewhere / "release").mkdir(parents=True)
        (elsewhere / "release/ffm").write_text("binary", encoding="utf-8")
        self.sandbox.fake("cargo", effect=BUILD_BINARY)
        result = self.create(env={"CARGO_TARGET_DIR": str(elsewhere)})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines(), [
            f"FFM executable not found at {self.ffm}",
            "Building FFM in release mode...",
            f"Desktop entry created at: {self.entry}",
        ])
        call = self.sandbox.calls("cargo")[0]
        self.assertEqual((call.args, call.cwd), (["build", "--release"], str(self.repo)))
        self.assertIn(f"Exec={self.ffm}\n", self.entry.read_text(encoding="utf-8"))

    def test_programs_behind_a_tilde_path_entry_are_found_as_bash_finds_them(self):
        self.sandbox.fake("cargo", effect=BUILD_BINARY)
        path = self.sandbox.tilde_path("cargo", "desktop-file-validate")
        result = self.create(env={"PATH": path})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([call.name for call in self.sandbox.calls()], ["cargo", "desktop-file-validate"])
        self.assertTrue(self.entry.is_file())

    def test_a_failing_build_status_is_ignored_when_the_binary_exists(self):
        self.sandbox.fake("cargo", status=101, effect=BUILD_BINARY)
        result = self.create()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(self.entry.is_file())

    def test_no_binary_after_the_build_stops_before_the_entry(self):
        self.sandbox.fake("cargo", status=101, stderr="error: could not compile\n")
        result = self.create()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout.splitlines(), [
            f"FFM executable not found at {self.ffm}",
            "Building FFM in release mode...",
            "Failed to build FFM. Please run 'cargo build --release' manually.",
        ])
        self.assertIn("could not compile", result.stderr)
        self.assertFalse(self.entry.exists())

    def test_the_build_moves_into_the_checkout_for_good_like_bash_cd(self):
        # After the build, relative paths resolve in the checkout, later
        # commands start there, and children see it as PWD.
        self.sandbox.fake("cargo", effect=BUILD_BINARY)
        result = self.create(cwd=self.home)
        self.assertEqual(result.returncode, 0, result.stderr)
        for call in self.sandbox.calls():
            with self.subTest(command=call.name):
                self.assertEqual((call.cwd, call.env.get("PWD")), (str(self.repo), str(self.repo)))

    def test_ctrl_c_during_the_build_ends_it_quietly_by_sigint(self):
        self.sandbox.fake("cargo", effect="import time; time.sleep(30)")
        # A session of its own, so that the SIGINT reaches only the script and cargo.
        process = subprocess.Popen(
            [sys.executable, self.repo / SCRIPT], cwd=self.repo, env=self.sandbox.env(),
            start_new_session=True, stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        )
        self.addCleanup(process.kill)
        deadline = time.monotonic() + 20
        while not self.sandbox.calls("cargo") and time.monotonic() < deadline:
            time.sleep(0.05)
        os.killpg(process.pid, signal.SIGINT)
        _, stderr = process.communicate(timeout=30)
        # Only the fake cargo's own traceback: none from the script.
        self.assertEqual(process.returncode, -signal.SIGINT)
        self.assertNotIn(SCRIPT, stderr)
        self.assertFalse(self.entry.exists())

    def test_a_cargo_that_cannot_run_is_reported_like_the_shell(self):
        cargo = self.sandbox.bin / "cargo"
        cases = {
            "missing": (None, "cargo: command not found"),
            "not executable": (0o644, f"{cargo}: Permission denied"),
        }
        for case, (mode, report) in cases.items():
            with self.subTest(case):
                if mode is None:
                    self.sandbox.use_real("cargo")
                else:
                    cargo.write_text("#!/bin/sh\n", encoding="utf-8")
                    cargo.chmod(mode)
                # Only the sandbox's bin: a real cargo stays unseen.
                result = self.create(env={"PATH": str(self.sandbox.bin)})
                self.assertEqual((result.returncode, result.stderr), (1, report + "\n"))
                self.assertEqual(result.stdout.splitlines(), [
                    f"FFM executable not found at {self.ffm}",
                    "Building FFM in release mode...",
                    "Failed to build FFM. Please run 'cargo build --release' manually.",
                ])
                self.assertFalse(self.entry.exists())


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
        shutil.copy2(self.repo / SCRIPT, checkout / SCRIPT)
        (checkout / "scripts").mkdir()
        shutil.copy2(self.repo / "scripts/shell_path.py", checkout / "scripts/shell_path.py")
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
