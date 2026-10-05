"""Test ffm.py, the build-and-run launcher, against fake Cargo, git, cmake and ffm.

Help, the Rust check, build, clean, PATH lookup and output; the other
commands have test_ffm_*.py files of their own, and ffm_case.py holds the
checkout they share.
"""

import os
import signal
import unittest

from ffm_case import (
    BUILD_STARTED, BUILDING, CARGO_MISSING, HELP, RUST_CHECKED, RUST_MISSING, LauncherTest, error, status,
)


class HelpTests(LauncherTest):
    def test_help_lists_every_command_and_example(self):
        for args in (["help"], ["-h"], ["--help"], ["help", "extra"]):
            with self.subTest(args=args):
                result = self.ffm(*args)
                self.assertEqual((result.returncode, result.stdout, result.stderr), (0, HELP, ""))
        self.assertEqual(self.sandbox.calls(), [])


class RustCheckTests(LauncherTest):
    def test_a_missing_cargo_stops_every_rust_command(self):
        env = self.hide("cargo")
        for args in RUST_CHECKED:
            with self.subTest(args=args):
                result = self.ffm(*args, env=env)
                self.assertEqual((result.returncode, result.stdout, result.stderr), (1, RUST_MISSING, ""))
        self.assertEqual(self.sandbox.calls(), [])

    def test_a_failing_preflight_stops_every_rust_command_before_cargo(self):
        self.sandbox.fake("cargo_preflight", path=self.repo / "scripts/cargo_preflight.py", status=3)
        self.place_binary("debug")
        for args in RUST_CHECKED:
            with self.subTest(args=args):
                self.forget_calls()
                result = self.ffm(*args)
                self.assertEqual((result.returncode, result.stdout), (1, ""))
                self.assertEqual(self.names(), ["cargo_preflight"])

    def test_the_preflight_runs_first_from_the_checkout_root(self):
        result = self.ffm("build", cwd=self.sandbox.home)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.names(), ["cargo_preflight", "cargo"])
        self.assertEqual({call.cwd for call in self.sandbox.calls()}, {str(self.repo)})
        self.assertEqual(self.sandbox.args("cargo_preflight"), [[]])


class BuildTests(LauncherTest):
    def test_any_mode_but_release_builds_debug(self):
        for args in (["build"], ["build", "debug"], ["build", "anything"], ["build", ""]):
            with self.subTest(args=args):
                self.forget_calls()
                result = self.ffm(*args)
                self.assertEqual((result.returncode, result.stdout), (0, BUILDING))
                self.assertEqual(self.sandbox.args("cargo"), [["build"]])

    def test_release_mode_builds_release_and_ignores_extra_words(self):
        result = self.ffm("build", "release", "extra")
        self.assertEqual((result.returncode, result.stdout), (0, BUILDING))
        self.assertEqual(self.sandbox.args("cargo"), [["build", "--release"]])

    def test_status_lines_stay_in_order_with_cargo_output(self):
        self.sandbox.fake("cargo", stdout="Compiling ffm\n")
        result = self.ffm("build")
        expected = BUILD_STARTED + "Compiling ffm\n" + status("Build completed successfully!")
        self.assertEqual(result.stdout, expected)

    def test_a_failed_build_exits_1(self):
        self.sandbox.fake("cargo", status=101)
        result = self.ffm("build")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, BUILD_STARTED + error("Build failed!"))


class CleanTests(LauncherTest):
    def test_clean_runs_cargo_clean_without_the_rust_check(self):
        self.sandbox.fake("cargo", status=4)
        result = self.ffm("clean", "extra", cwd=self.sandbox.home)
        self.assertEqual((result.returncode, result.stdout), (4, status("Cleaning build artifacts...")))
        self.assertEqual(self.names(), ["cargo"])
        call = self.sandbox.calls("cargo")[0]
        self.assertEqual((call.args, call.cwd), (["clean"], str(self.repo)))

    def test_clean_without_cargo_exits_127(self):
        result = self.ffm("clean", env=self.hide("cargo"))
        self.assertEqual((result.returncode, result.stderr), (127, CARGO_MISSING))

    def test_a_cargo_that_cannot_be_executed_exits_126(self):
        (self.sandbox.bin / "cargo").chmod(0o644)
        result = self.ffm("clean", env={"PATH": str(self.sandbox.bin)})
        self.assertEqual((result.returncode, result.stderr), (126, "cargo: Permission denied\n"))


class PathLookupTests(LauncherTest):
    """Commands are looked up with a leading ~ in a PATH entry expanded."""

    def test_cargo_behind_a_tilde_entry_is_found_and_path_is_passed_on_as_it_was(self):
        self.hide("cargo")
        self.sandbox.fake("cargo", path=self.sandbox.home / ".cargo/bin/cargo")
        path = os.pathsep.join(["~/.cargo/bin", str(self.sandbox.bin)])
        for args, expected in ((["build"], [["build"]]), (["clean"], [["clean"]])):
            with self.subTest(args=args):
                self.forget_calls()
                result = self.ffm(*args, env={"PATH": path})
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(self.sandbox.args("cargo"), expected)
                self.assertEqual(self.sandbox.calls("cargo")[0].env["PATH"], path)

    def test_cmake_behind_a_tilde_entry_is_found(self):
        self.hide("cmake")
        self.sandbox.fake("cmake", path=self.sandbox.home / "tools/cmake")
        self.sandbox.fake("git")
        env = {"PATH": os.pathsep.join(["~/tools", str(self.sandbox.bin)]), "XDG_DATA_HOME": str(self.sandbox.root)}
        result = self.ffm("colorer", env=env)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.names(), ["git", "cmake", "cmake", "git"])


class OutputTests(LauncherTest):
    """A stdout that cannot be written is reported, and the script goes on."""

    def test_a_failed_write_is_reported_and_the_script_goes_on(self):
        # Fakes that leave stdout alone: Python's sys.stdout is None when fd 1 is closed.
        self.sandbox.fake("cargo_preflight", path=self.repo / "scripts/cargo_preflight.py", effect="sys.exit(0)")
        self.sandbox.fake("cargo", effect="sys.exit(0)")
        redirections = {">&-": "Bad file descriptor", ">/dev/full": "No space left on device"}
        for redirection, reason in redirections.items():
            if not os.path.exists("/dev/full") and "full" in redirection:
                continue
            with self.subTest(redirection=redirection):
                self.forget_calls()
                result = self.ffm_with_stdout(redirection, "build")
                report = f"{self.repo / 'ffm.py'}: write error: {reason}\n"
                # The last command was echo, and its status was the script's.
                self.assertEqual((result.returncode, result.stderr), (1, report * 2))
                self.assertEqual(self.names(), ["cargo_preflight", "cargo"])
                self.assertEqual(self.ffm_with_stdout(redirection, "help").returncode, 1)

    def test_a_closed_pipe_ends_the_script_silently_by_sigpipe(self):
        read_end, write_end = os.pipe()
        os.close(read_end)
        try:
            result = self.ffm_bytes("build", stdout=write_end)
        finally:
            os.close(write_end)
        self.assertEqual((result.returncode, result.stderr), (-signal.SIGPIPE, b""))
        self.assertEqual(self.names(), ["cargo_preflight"])


if __name__ == "__main__":
    unittest.main()
