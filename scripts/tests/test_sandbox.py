"""Prove the script-test sandbox isolates tests from the real machine."""

import os
import shutil
import sys
import unittest
from unittest.mock import patch

from sandbox import GUARDED, REFUSED, REPO, Sandbox, SandboxTestCase


class GuardTests(SandboxTestCase):
    def test_every_guarded_command_resolves_to_a_refusing_stub(self):
        path = self.sandbox.env()["PATH"]
        for name in GUARDED:
            with self.subTest(name=name):
                self.assertEqual(shutil.which(name, path=path), str(self.sandbox.bin / name))

    def test_an_unexpected_call_is_refused_and_recorded(self):
        result = self.sandbox.execute(["cargo", "build", "--release"])
        self.assertEqual(result.returncode, REFUSED)
        self.assertIn("cargo is not faked", result.stderr)
        self.assertEqual(self.sandbox.args("cargo"), [["build", "--release"]])

    def test_git_may_not_leave_the_machine(self):
        self.sandbox.use_real("git")
        result = self.sandbox.execute(["git", "ls-remote", "https://example.invalid/x.git"])
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("transport 'https' not allowed", result.stderr)

    def test_the_environment_is_built_from_scratch(self):
        self.sandbox.fake("probe")
        leaked = {"RUSTC_WRAPPER": "x", "CARGO_TARGET_DIR": "/elsewhere", "SKIP_CLIPPY": "1"}
        with patch.dict(os.environ, leaked):
            self.sandbox.execute(["probe"], env={"FFM_MCP_ROOT": "kept"})
        env = self.sandbox.calls("probe")[0].env
        self.assertTrue(leaked.keys().isdisjoint(env))
        self.assertEqual(env["HOME"], str(self.sandbox.home))
        self.assertEqual(env["FFM_MCP_ROOT"], "kept")
        self.assertEqual(env["PATH"].split(os.pathsep)[0], str(self.sandbox.bin))

    def test_a_real_ffm_on_the_system_path_skips_the_test(self):
        system = self.sandbox.root / "system bin"
        self.sandbox.fake("ffm", path=system / "ffm")
        with patch("sandbox.SYSTEM_PATH", (str(system),)), self.assertRaises(unittest.SkipTest):
            Sandbox(self)

    def test_subprocess_coverage_configuration_reaches_the_sandbox(self):
        with patch.dict(os.environ, {"COVERAGE_PROCESS_CONFIG": "parent configuration"}):
            self.assertEqual(self.sandbox.env().get("COVERAGE_PROCESS_CONFIG"), "parent configuration")
            overridden = self.sandbox.env({"COVERAGE_PROCESS_CONFIG": "child configuration"})
            self.assertEqual(overridden["COVERAGE_PROCESS_CONFIG"], "child configuration")


class FakeTests(SandboxTestCase):
    def test_calls_is_empty_before_any_call(self):
        self.assertEqual(self.sandbox.calls(), [])

    def test_a_fake_records_its_call_and_answers_as_told(self):
        self.sandbox.fake("aws", stdout="cluster-a\n", stderr="note\n", status=3)
        result = self.sandbox.execute(["aws", "eks", "two words"], cwd=self.sandbox.home)
        self.assertEqual((result.returncode, result.stdout, result.stderr), (3, "cluster-a\n", "note\n"))
        call = self.sandbox.calls("aws")[0]
        self.assertEqual((call.args, call.cwd), (["eks", "two words"], str(self.sandbox.home)))

    def test_an_effect_sees_the_arguments_and_may_choose_the_status(self):
        self.sandbox.fake("cargo", effect="""
            if args == ["-V"]:
                sys.exit(5)
        """)
        self.assertEqual(self.sandbox.execute(["cargo", "-V"]).returncode, 5)
        self.assertEqual(self.sandbox.execute(["cargo", "metadata"]).returncode, 0)
        self.assertEqual(self.sandbox.args("cargo"), [["-V"], ["metadata"]])

    def test_a_fake_may_stand_at_any_path(self):
        helper = self.sandbox.fake("lint", path=self.sandbox.repo / "scripts/deep dir/lint.py", status=4)
        self.assertEqual(helper, self.sandbox.repo / "scripts/deep dir/lint.py")
        self.assertFalse((self.sandbox.bin / "lint").exists())
        self.assertTrue(helper.is_symlink())
        self.assertTrue(helper.resolve().is_relative_to(self.sandbox.bin))
        result = self.sandbox.execute([helper, "--check"])
        self.assertEqual(result.returncode, 4)
        self.assertEqual(self.sandbox.args("lint"), [["--check"]])


class GitTests(SandboxTestCase):
    def test_git_runs_the_real_git_past_the_refusing_stub(self):
        self.sandbox.git("init", "--quiet", "--initial-branch=main", self.sandbox.repo)
        self.assertEqual(self.sandbox.git("rev-parse", "--is-inside-work-tree"), "true\n")
        self.assertEqual(self.sandbox.calls("git"), [])

    def test_git_commits_as_the_sandbox_identity(self):
        self.sandbox.git("init", "--quiet", self.sandbox.repo)
        self.sandbox.write("README", "seed\n")
        self.sandbox.git("add", "README")
        self.sandbox.git("commit", "--quiet", "--message=seed")
        self.assertEqual(self.sandbox.git("log", "--format=%an <%ae>"), "Sandbox <sandbox@example.com>\n")

    def test_a_failing_git_raises_with_its_command_and_stderr(self):
        with self.assertRaises(AssertionError) as raised:
            self.sandbox.git("-C", self.sandbox.root / "missing", "status")
        self.assertIn(f"git -C {self.sandbox.root / 'missing'} status failed:", str(raised.exception))
        self.assertIn("missing", str(raised.exception).splitlines()[-1])


class CheckoutTests(SandboxTestCase):
    def test_copy_replaces_a_python_double_with_a_genuine_source_file(self):
        relative = "scripts/lint.py"
        self.sandbox.fake("lint", path=self.sandbox.repo / relative)
        copied = self.sandbox.copy(relative)
        self.assertFalse(copied.is_symlink())
        self.assertEqual(copied.read_bytes(), (REPO / relative).read_bytes())

    def test_copy_keeps_the_file_mode(self):
        relative = "scripts/test_lint_policy.py"
        copied = self.sandbox.copy(relative)
        self.assertEqual(copied.read_bytes(), (REPO / relative).read_bytes())
        self.assertEqual(copied.stat().st_mode, (REPO / relative).stat().st_mode)

    def test_run_uses_this_interpreter_inside_the_checkout_copy(self):
        self.sandbox.write("probe.py", 'import os, sys\nprint(sys.executable, os.getcwd(), *sys.argv[1:], sep="\\n")\n')
        result = self.sandbox.run("probe.py", "an argument")
        self.assertEqual(result.stdout.splitlines(), [sys.executable, str(self.sandbox.repo), "an argument"])


if __name__ == "__main__":
    unittest.main()
