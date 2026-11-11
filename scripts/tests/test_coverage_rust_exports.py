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

    def test_suite_selection_runs_requested_configurations_in_canonical_order(self):
        result = self.sandbox.run(
            "run_coverage.py",
            env={"FFM_COVERAGE_SUITES": "colorer-sys-all, root-all"},
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = [call for call in self.sandbox.calls("cargo") if call.args[1] == "report"]
        self.assertEqual(
            [call.args[-1].split("/")[-1] for call in calls],
            ["rust-root-all.lcov", "rust-colorer-sys-all.lcov"],
        )

    def test_unknown_suite_selection_fails_before_cargo(self):
        result = self.sandbox.run(
            "run_coverage.py",
            env={"FFM_COVERAGE_SUITES": "root-all,bogus"},
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unknown FFM_COVERAGE_SUITES: bogus", result.stderr)
        self.assertEqual(self.sandbox.calls("cargo"), [])

    def test_prepare_target_writes_cachedir_tag_and_removes_stale_bdd_binaries(self):
        target = self.sandbox.root / "cached-target"
        deps = target / "debug" / "deps"
        deps.mkdir(parents=True)
        stale_bdd = deps / "bdd-deadbeef"
        keep_lib = deps / "librmcp-1234.rlib"
        stale_bdd.write_bytes(b"stale")
        keep_lib.write_bytes(b"cached")
        result = self.sandbox.run(
            "run_coverage.py",
            env={"CARGO_LLVM_COV_TARGET_DIR": str(target)},
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        tag = target / "CACHEDIR.TAG"
        self.assertTrue(tag.is_file())
        self.assertTrue(tag.read_text(encoding="utf-8").startswith("Signature: 8a477f597d28d172789f06886806bc55"))
        self.assertFalse(stale_bdd.exists())
        self.assertTrue(keep_lib.is_file())


if __name__ == "__main__":
    unittest.main()
