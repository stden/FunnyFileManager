"""Pin Python coverage decisions to exact native JSON line counts."""

import importlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO


def python_report(covered=2, count=2, missing=None):
    lines = {"covered_lines": covered, "num_statements": count, "percent_covered": 100}
    return {"totals": dict(lines), "files": {"ffm.py": {
        "summary": dict(lines), "missing_lines": [] if missing is None else missing,
    }}}


class CoverageReportTests(unittest.TestCase):
    def setUp(self):
        sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
        self.addCleanup(sys.path.pop, 0)
        self.checker = importlib.import_module("coverage_report")
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.path = Path(temporary.name) / "report.json"

    def check(self, kind, report):
        self.path.write_text(json.dumps(report), encoding="utf-8")
        with redirect_stdout(StringIO()), redirect_stderr(StringIO()):
            return getattr(self.checker, f"check_{kind}")(self.path)

    def test_complete_python_reports_pass(self):
        self.assertTrue(self.check("python", python_report()))

    def test_a_single_uncovered_line_never_rounds_up_to_passing(self):
        self.assertFalse(self.check("python", python_report(99999, 100000, missing=[9])))

    def test_per_file_gaps_cannot_hide_behind_perfect_totals(self):
        python = python_report()
        python["files"]["ffm.py"]["summary"]["covered_lines"] = 1
        python["files"]["ffm.py"]["missing_lines"] = [7]
        with self.assertRaises(ValueError):
            self.check("python", python)

    def test_empty_measurements_fail(self):
        with self.assertRaisesRegex(ValueError, "no lines"):
            self.check("python", python_report(0, 0))

    def test_invalid_counts_fail(self):
        for covered, count in ((-1, 2), (3, 2), (1.0, 2), (True, 2), (1, "2")):
            with self.subTest(covered=covered, count=count), self.assertRaises(ValueError):
                self.check("python", python_report(covered, count))

    def test_missing_structure_fails(self):
        reports = (
            ("python", {}), ("python", {"files": {}, "totals": {}}),
        )
        for kind, report in reports:
            with self.subTest(kind=kind, report=report), self.assertRaises(ValueError):
                self.check(kind, report)

    def test_missing_files_cannot_be_replaced_with_totals_alone(self):
        python = python_report()
        python["files"] = {}
        with self.assertRaises(ValueError):
            self.check("python", python)

    def test_python_missing_line_list_must_agree_with_its_counts(self):
        for missing in ([7], [7, 7], [0, 9], [False, 9], "7,9"):
            with self.subTest(missing=missing), self.assertRaises(ValueError):
                self.check("python", python_report(1, 3, missing=missing))

    def test_a_present_missing_line_list_cannot_be_null(self):
        report = python_report()
        report["files"]["ffm.py"]["missing_lines"] = None
        with self.assertRaises(ValueError):
            self.check("python", report)

    def test_invalid_source_entries_fail(self):
        reports = (
            ("python", {"files": {"ffm.py": None}}),
            ("python", {"files": {"ffm.py": {}}}),
            ("python", {"files": {"ffm.py": {"summary": []}}}),
            ("python", {"files": {"ffm.py": {"summary": {}}}}),
        )
        for kind, report in reports:
            with self.subTest(kind=kind, report=report), self.assertRaises(ValueError):
                self.check(kind, report)

    def test_a_report_without_optional_python_line_numbers_can_pass(self):
        report = python_report()
        del report["files"]["ffm.py"]["missing_lines"]
        self.assertTrue(self.check("python", report))

    def test_unreadable_or_invalid_json_fails_with_a_report_error(self):
        for contents in (None, "not JSON", "null", "[]"):
            if contents is not None:
                self.path.write_text(contents, encoding="utf-8")
            with self.subTest(contents=contents), self.assertRaises(ValueError):
                self.checker.check_python(self.path)

    def test_empty_python_source_names_are_rejected(self):
        report = python_report()
        report["files"][" "] = report["files"].pop("ffm.py")
        with self.assertRaises(ValueError):
            self.check("python", report)

    def check_rust_pair(self, regions=None, raw_json=None):
        self.path.unlink(missing_ok=True)
        lcov = self.path.with_suffix(".lcov")
        lcov.write_text("SF:src/main.rs\nDA:1,1\nend_of_record\n", encoding="utf-8")
        if raw_json is not None:
            self.path.write_text(json.dumps(raw_json), encoding="utf-8")
        elif regions is not None:
            self.path.write_text(
                json.dumps({"data": [{"totals": {"regions": regions}}]}),
                encoding="utf-8",
            )
        with redirect_stdout(StringIO()), redirect_stderr(StringIO()):
            return self.checker.check_rust(lcov)

    def test_rust_companion_regions_enforce_subline_floor_when_present(self):
        self.assertTrue(self.check_rust_pair())
        self.assertTrue(self.check_rust_pair(raw_json={"data": [{}]}))
        self.assertTrue(self.check_rust_pair(raw_json={"data": [{"totals": {"lines": {}}}]}))
        self.assertTrue(self.check_rust_pair(regions={"covered": 95, "count": 100}))
        self.assertFalse(self.check_rust_pair(regions={"covered": 94, "count": 100}))

    def test_invalid_rust_companion_region_payloads_fail(self):
        bad_payloads = (
            {"data": []},
            {"data": ["not-an-object"]},
            {"data": [{"totals": {"regions": {"covered": 0, "count": 0}}}]},
            {"data": [{"totals": {"regions": {"covered": 3, "count": 2}}}]},
        )
        for payload in bad_payloads:
            with self.subTest(payload=payload), self.assertRaises(ValueError):
                self.check_rust_pair(raw_json=payload)


if __name__ == "__main__":
    unittest.main()
