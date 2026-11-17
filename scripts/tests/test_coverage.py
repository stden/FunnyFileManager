"""Exercise the complete coverage gate with collectors that write native reports."""

import json
import unittest

from sandbox import SandboxTestCase
from script_files import LINT
from test_coverage_report import python_report
from test_coverage_lcov import rust_report


COLLECTOR = '''import subprocess

def run(directory):
    directory.mkdir(parents=True, exist_ok=True)
    with (directory.parent / "collectors").open("a", encoding="utf-8") as output:
        output.write({kind!r} + "\\n")
    if {status}:
        raise subprocess.CalledProcessError({status}, ["{kind}-test"])
    report = directory / {filename!r}
    report.write_text({report!r}, encoding="utf-8")
    return {returned}
'''


class CoverageGateTests(SandboxTestCase):
    def setUp(self):
        super().setUp()
        for script in ("scripts/coverage.py", "scripts/coverage_report.py", "scripts/coverage_lcov.py", *LINT):
            self.sandbox.copy(script)
        self.output = self.sandbox.repo / "target/coverage-full"
        self.collector("rust", rust_report())
        self.collector("python", python_report())

    def collector(self, kind, report, status=0, returned=None):
        returned = returned or ("[report]" if kind == "rust" else "report")
        self.sandbox.write(f"scripts/coverage_{kind}.py", COLLECTOR.format(
            kind=kind, report=report if isinstance(report, str) else json.dumps(report), status=status, returned=returned,
            filename="report.lcov" if kind == "rust" else "report.json",
        ))

    def gate(self, *args, **environment):
        return self.sandbox.run("scripts/coverage.py", *args, env=environment)

    def measured(self):
        return (self.output / "collectors").read_text(encoding="utf-8").splitlines()

    def test_all_production_lines_covered_pass_and_reports_persist(self):
        result = self.gate()
        self.assertEqual((result.returncode, result.stderr), (0, ""))
        self.assertEqual(self.measured(), ["rust", "python"])
        self.assertTrue((self.output / "rust/report.lcov").is_file())
        self.assertTrue((self.output / "python/report.json").is_file())
        self.assertIn("100.00%", result.stdout)

    def test_a_rust_gap_still_collects_python_before_failing(self):
        self.collector("rust", rust_report(99999, 100000))
        result = self.gate()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(self.measured(), ["rust", "python"])
        self.assertIn("src/main.rs", result.stderr)
        self.assertIn("99999/100000", result.stderr)

    def test_a_python_gap_names_its_uncovered_lines(self):
        self.collector("python", python_report(1, 3, missing=[7, 19]))
        result = self.gate()
        self.assertEqual(result.returncode, 1)
        self.assertIn("ffm.py", result.stderr)
        self.assertIn("7, 19", result.stderr)

    def test_report_directory_can_be_overridden(self):
        self.output = self.sandbox.root / "saved coverage reports"
        result = self.gate(FFM_COVERAGE_DIR=str(self.output))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.measured(), ["rust", "python"])

    def test_backend_selection_runs_only_the_requested_collector(self):
        for kind in ("rust", "python"):
            with self.subTest(kind=kind):
                self.output = self.sandbox.root / kind
                result = self.gate(f"--{kind}", FFM_COVERAGE_DIR=str(self.output))
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(self.measured(), [kind])

    def test_a_failed_backend_propagates_its_status_without_a_traceback(self):
        for kind in ("rust", "python"):
            with self.subTest(kind=kind):
                self.collector(kind, {}, status=73)
                result = self.gate(f"--{kind}")
                self.assertEqual(result.returncode, 73)
                self.assertIn(f"{kind}-test", result.stderr)
                self.assertNotIn("Traceback", result.stderr)

    def test_an_empty_backend_report_list_fails_after_both_collectors(self):
        self.collector("rust", {}, returned="[]")
        result = self.gate()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(self.measured(), ["rust", "python"])
        self.assertIn("no reports", result.stderr)

    def test_a_malformed_report_fails_without_hiding_the_other_backend(self):
        self.collector("rust", "malformed LCOV")
        result = self.gate()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(self.measured(), ["rust", "python"])
        self.assertIn("unterminated LCOV record", result.stderr)
        self.assertNotIn("Traceback", result.stderr)

    def test_an_unavailable_backend_tool_returns_127(self):
        self.sandbox.write("scripts/coverage_rust.py", "def run(directory):\n    raise FileNotFoundError(2, 'missing tool', 'cargo')\n")
        result = self.gate("--rust")
        self.assertEqual(result.returncode, 127)
        self.assertIn("cargo", result.stderr)
        self.assertNotIn("Traceback", result.stderr)

    def test_help_runs_no_collector(self):
        for option in ("-h", "--help"):
            result = self.gate(option)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("100%", result.stdout)
        self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()
