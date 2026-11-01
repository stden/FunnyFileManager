"""Test how ./ffm.py starts ffm: release, run, and no command at all."""

import os
import signal
import unittest

from ffm_case import BUILD_STARTED, BUILDING, CARGO_MISSING, NO_BINARY, STARTING, LauncherTest, error


class LaunchTests(LauncherTest):
    def test_release_builds_then_runs_the_release_binary(self):
        self.fake_cargo_that_builds(status=7)
        result = self.ffm("release", "--", "-x", "two words", cwd=self.sandbox.home)
        self.assertEqual((result.returncode, result.stdout), (7, BUILDING + STARTING))
        self.assertEqual(self.sandbox.args("cargo"), [["build", "--release"]])
        self.assertEqual(self.sandbox.args("ffm-binary"), [["--", "-x", "two words"]])
        self.assertEqual(self.sandbox.calls("ffm-binary")[0].cwd, str(self.repo))

    def test_a_failed_release_build_runs_nothing(self):
        self.place_binary("release")
        self.sandbox.fake("cargo", status=101)
        result = self.ffm("release")
        self.assertEqual(result.returncode, 1)
        self.assertTrue(result.stdout.endswith(error("Build failed!")))
        self.assertEqual(self.sandbox.calls("ffm-binary"), [])

    def test_a_release_build_that_leaves_no_binary_falls_back_to_debug(self):
        # The release build left nothing in target/release (CARGO_TARGET_DIR
        # elsewhere): a debug build follows, and the debug binary runs.
        self.place_binary("debug", status=4)
        result = self.ffm("release")
        self.assertEqual(self.sandbox.args("cargo"), [["build", "--release"], ["build"]])
        self.assertEqual((result.returncode, result.stdout), (4, BUILDING + NO_BINARY + BUILDING + STARTING))
        self.assertEqual(self.sandbox.args("ffm-binary"), [[]])

    def test_a_binary_still_missing_after_the_build_exits_127(self):
        expected = f"{self.repo / 'target/debug/ffm'}: No such file or directory\n"
        for args in (["release"], ["run"], []):
            with self.subTest(args=args):
                result = self.ffm(*args)
                self.assertEqual((result.returncode, result.stderr), (127, expected))
                self.assertTrue(result.stdout.endswith(NO_BINARY + BUILDING + STARTING))

    def test_a_binary_that_cannot_be_executed_exits_126(self):
        binary = self.place_binary("debug")
        binary.chmod(0o644)
        plain_file = self.ffm("run")
        binary.unlink()
        binary.mkdir()
        directory = self.ffm("run")
        for result in (plain_file, directory):
            self.assertEqual(result.returncode, 126)
            self.assertEqual(result.stderr, f"{binary}: Permission denied\n")

    def test_ffm_is_executed_by_its_full_path(self):
        # A #! program sees the path exec was given as $0.
        binary = self.sandbox.write("target/debug/ffm", '#!/bin/sh\necho "$0"\n')
        binary.chmod(0o755)
        result = self.ffm("run", cwd=self.sandbox.home)
        self.assertEqual((result.returncode, result.stdout), (0, STARTING + f"{binary}\n"))

    def test_every_program_sees_the_checkout_root_as_pwd(self):
        self.place_binary("debug")
        for pwd in (str(self.sandbox.home), "/", None):
            with self.subTest(PWD=pwd):
                self.forget_calls()
                self.ffm(env={"PWD": pwd} if pwd else None, cwd=self.sandbox.home)
                self.assertEqual({call.env["PWD"] for call in self.sandbox.calls()}, {str(self.repo)})
                self.assertEqual(self.names(), ["cargo_preflight", "ffm-binary"])

    def test_ffm_starts_with_sigpipe_and_sigxfsz_at_their_defaults(self):
        if not os.path.exists("/proc/self/status"):
            self.skipTest("reads the signal masks from Linux /proc")
        binary = self.sandbox.write("target/debug/ffm", "#!/bin/sh\nexec grep '^SigIgn' /proc/self/status\n")
        binary.chmod(0o755)
        result = self.ffm("run")
        ignored = int(result.stdout.removeprefix(STARTING).split()[1], 16)
        for number in (signal.SIGPIPE, signal.SIGXFSZ):
            self.assertFalse(ignored & 1 << (number - 1), number.name)

    def test_run_starts_a_built_binary_without_cargo(self):
        self.place_binary("debug", status=5, stdout="ffm output\n")
        result = self.ffm("run", "-h", "--", "x")
        self.assertEqual((result.returncode, result.stdout), (5, STARTING + "ffm output\n"))
        self.assertEqual(self.names(), ["ffm-binary"])
        self.assertEqual(self.sandbox.args("ffm-binary"), [["-h", "--", "x"]])

    def test_run_builds_a_missing_binary_first(self):
        self.fake_cargo_that_builds()
        result = self.ffm("run")
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, NO_BINARY + BUILDING + STARTING)
        self.assertEqual(self.names(), ["cargo", "ffm-binary"])

    def test_run_without_a_binary_or_cargo_fails(self):
        result = self.ffm("run", env=self.hide("cargo"))
        self.assertEqual((result.returncode, result.stderr), (1, CARGO_MISSING))
        self.assertEqual(result.stdout, NO_BINARY + BUILD_STARTED + error("Build failed!"))

    def test_a_failed_build_runs_no_ffm(self):
        self.sandbox.fake("cargo", status=101)
        for args in (["run"], [], ["somewhere"]):
            with self.subTest(args=args):
                self.forget_calls()
                result = self.ffm(*args)
                self.assertEqual((result.returncode, result.stderr), (1, ""))
                self.assertTrue(result.stdout.endswith(BUILD_STARTED + error("Build failed!")), result.stdout)
                self.assertEqual(self.sandbox.args("cargo"), [["build"]])

    def test_no_command_runs_the_built_debug_binary(self):
        self.place_binary("debug")
        result = self.ffm(cwd=self.sandbox.home)
        self.assertEqual((result.returncode, result.stdout), (0, STARTING))
        self.assertEqual(self.names(), ["cargo_preflight", "ffm-binary"])
        call = self.sandbox.calls("ffm-binary")[0]
        self.assertEqual((call.args, call.cwd), ([], str(self.repo)))

    def test_no_command_builds_a_missing_binary_without_a_warning(self):
        self.fake_cargo_that_builds()
        result = self.ffm()
        self.assertEqual((result.returncode, result.stdout), (0, BUILDING + STARTING))
        self.assertEqual(self.sandbox.args("cargo"), [["build"]])

    def test_unknown_words_and_leading_options_all_go_to_ffm(self):
        self.place_binary("debug")
        for args in (
            ["somewhere", "-h", "--x"], ["--version"], ["-x", "-h"], ["--"],
            ["--", "run"], [""], ["-"], ["--help=x"],
        ):
            with self.subTest(args=args):
                self.forget_calls()
                result = self.ffm(*args)
                self.assertEqual((result.returncode, result.stdout), (0, STARTING))
                self.assertEqual(self.sandbox.args("ffm-binary"), [args])


if __name__ == "__main__":
    unittest.main()
