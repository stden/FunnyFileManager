"""The Rust coverage matrix must execute MCP application E2E tests explicitly."""

import unittest

from sandbox import SandboxTestCase
from test_coverage_rust import FAKE_CARGO, RUNNER, test_commands


class McpCoverageTests(SandboxTestCase):
    def setUp(self):
        super().setUp()
        for relative in ("scripts/coverage_rust.py", "scripts/lint_steps.py", "scripts/shell_path.py"):
            self.sandbox.copy(relative)
        self.sandbox.write("run_coverage.py", RUNNER)
        self.sandbox.fake("cargo", effect=FAKE_CARGO)

    def mcp_call(self, env=None):
        result = self.sandbox.run("run_coverage.py", env=env)
        self.assertEqual(result.returncode, 0, result.stderr)
        return test_commands(self.sandbox)[2]

    def test_mcp_ignored_tests_have_a_separate_target_so_cucumber_keeps_its_own_arguments(self):
        guard = '''
            if "--" in args and "--test" not in args:
                sys.stderr.write("BDD harness rejects global libtest arguments\\n")
                sys.exit(2)
        '''
        self.sandbox.fake("cargo", effect=guard + FAKE_CARGO)
        main = self.mcp_call()
        calls = test_commands(self.sandbox)
        self.assertEqual(len(calls), 5)
        application = calls[3]
        self.assertNotIn("--", main.args)
        self.assertIn("--no-clean", application.args)
        self.assertEqual(application.args[application.args.index("--test") + 1], "application_sessions")
        self.assertEqual(application.args[-2:], ["--", "--ignored"])
        self.assertEqual(main.args[main.args.index("--output-path") + 1], application.args[application.args.index("--output-path") + 1])

    def test_failed_application_e2e_stops_before_the_colorer_configuration(self):
        guard = '''
            if "--test" in args:
                sys.exit(43)
        '''
        self.sandbox.fake("cargo", effect=guard + FAKE_CARGO)
        result = self.sandbox.run("run_coverage.py")
        self.assertEqual(result.returncode, 43, result.stderr)
        self.assertEqual(len(test_commands(self.sandbox)), 4)

    def test_default_application_binary_comes_from_the_prior_root_build(self):
        call = self.mcp_call()
        expected = self.sandbox.repo / "reports/target/debug/ffm"
        self.assertEqual(call.env.get("FFM_APPLICATION_TEST_BINARY"), str(expected))

    def test_caller_coverage_target_determines_the_application_binary_location(self):
        for target in (str(self.sandbox.root / "private target"), "relative-target"):
            with self.subTest(target=target):
                self.sandbox.log.unlink(missing_ok=True)
                call = self.mcp_call({"CARGO_LLVM_COV_TARGET_DIR": target})
                self.assertEqual(call.env.get("FFM_APPLICATION_TEST_BINARY"), str(self.sandbox.repo / target / "debug/ffm"))

    def test_explicit_application_binary_is_preserved(self):
        application = str(self.sandbox.root / "already-built ffm")
        call = self.mcp_call({"FFM_APPLICATION_TEST_BINARY": application})
        self.assertEqual(call.env.get("FFM_APPLICATION_TEST_BINARY"), application)


if __name__ == "__main__":
    unittest.main()
