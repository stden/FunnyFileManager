"""Native LCOV export follows the final test of each Rust configuration."""

import unittest

from sandbox import SandboxTestCase
from test_coverage_rust import FAKE_CARGO, RUNNER


class RustLineExportTests(SandboxTestCase):
    def setUp(self):
        super().setUp()
        for relative in ("scripts/coverage_rust.py", "scripts/lint_steps.py", "scripts/shell_path.py"):
            self.sandbox.copy(relative)
        self.sandbox.write("run_coverage.py", RUNNER)
        self.sandbox.fake("cargo", effect=FAKE_CARGO)

    def test_each_configuration_preserves_json_and_exports_native_source_lines_after_tests(self):
        result = self.sandbox.run("run_coverage.py")
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = self.sandbox.calls("cargo")
        self.assertEqual([call.args[1] for call in calls], ["test", "report", "test", "report", "test", "test", "report", "test", "report"])
        for call in (call for call in calls if call.args[1] == "report"):
            self.assertIn("--lcov", call.args)
            self.assertIn("--include-build-script", call.args)
            self.assertNotIn("--json", call.args)
            self.assertNotIn("--no-clean", call.args)
            for build_option in ("--all-features", "--features", "--no-default-features"):
                self.assertNotIn(build_option, call.args)
            output = self.sandbox.repo / "reports" / call.args[-1].split("/")[-1]
            self.assertEqual(output.suffix, ".lcov")
            self.assertTrue(output.with_suffix(".json").is_file())
            self.assertIn("DA:1,0", output.read_text())

    def test_failed_native_export_preserves_its_status_and_stops_later_configurations(self):
        guard = '''
            if args[1:2] == ["report"]:
                sys.exit(42)
        '''
        self.sandbox.fake("cargo", effect=guard + FAKE_CARGO)
        result = self.sandbox.run("run_coverage.py")
        self.assertEqual(result.returncode, 42, result.stderr)
        self.assertEqual([call.args[1] for call in self.sandbox.calls("cargo")], ["test", "report"])


if __name__ == "__main__":
    unittest.main()
