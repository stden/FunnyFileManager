"""Check the build that create_desktop_shortcut.py runs when target/release/ffm is missing."""

import os
import signal
import subprocess
import sys
import time
import unittest

from create_desktop_shortcut_case import BUILD_BINARY, SCRIPT, ShortcutTestCase


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


if __name__ == "__main__":
    unittest.main()
