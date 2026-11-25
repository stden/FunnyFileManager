"""Judge physical source lines from native LLVM DA records, never group totals."""

from contextlib import redirect_stderr, redirect_stdout
import importlib
from io import StringIO
from pathlib import Path
import sys
import tempfile
import unittest


FIXTURES = Path(__file__).with_name("fixtures") / "coverage"


def rust_report(covered=2, count=2, name="src/main.rs"):
    lines = "\n".join(f"DA:{line},{int(line <= covered)}" for line in range(1, count + 1))
    return f"SF:{name}\n{lines}\nLF:{count}\nLH:{covered}\nend_of_record\n"


class LcovCoverageTests(unittest.TestCase):
    def setUp(self):
        sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
        self.addCleanup(sys.path.pop, 0)
        self.checker = importlib.import_module("coverage_report")
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.path = Path(temporary.name) / "report.lcov"

    def check(self, text):
        self.path.write_text(text, encoding="utf-8")
        with redirect_stdout(StringIO()), redirect_stderr(StringIO()):
            return self.checker.check_rust(self.path)

    def test_complementary_generic_instantiations_cover_all_nine_source_lines(self):
        trace = (FIXTURES / "complementary.lcov").read_text(encoding="utf-8")
        self.assertIn("LH:8\n", trace)
        try:
            passed = self.check(trace)
        except ValueError:
            passed = False
        self.assertTrue(passed, "all nine native source lines executed across the two generic instances")

    def test_actual_unexecuted_source_line_remains_a_failure_with_its_line_number(self):
        self.path.write_text((FIXTURES / "unexecuted.lcov").read_text(), encoding="utf-8")
        error = StringIO()
        with redirect_stderr(error):
            self.assertFalse(self.checker.check_rust(self.path))
        self.assertIn("11/12 lines; missing lines: 13", error.getvalue())

    def test_multiple_files_are_all_checked(self):
        trace = rust_report(name="one.rs") + rust_report(1, 2, "two.rs")
        self.assertFalse(self.check(trace))

    def test_a_single_unexecuted_line_cannot_round_up_to_passing(self):
        self.assertFalse(self.check(rust_report(99999, 100000)))

    def test_duplicate_files_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "duplicate.*source"):
            self.check(rust_report() + rust_report())

    def test_duplicate_da_records_are_rejected_even_when_the_counts_agree(self):
        with self.assertRaisesRegex(ValueError, "duplicate.*line"):
            self.check("SF:main.rs\nDA:1,1\nDA:1,1\nend_of_record\n")

    def test_invalid_da_records_are_rejected(self):
        for value in ("0,1", "-1,1", "1,-1", "1,1.0", "1,True", "1,NaN", "1", "1,1,extra", "1, 1"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                self.check(f"SF:main.rs\nDA:{value}\nend_of_record\n")

    def test_empty_malformed_or_unterminated_reports_are_rejected(self):
        traces = (
            "", "\n", "SF: \nDA:1,1\nend_of_record\n", "SF:main.rs\nend_of_record\n",
            "SF:main.rs\nDA:1,1\n", "DA:1,1\nend_of_record\n", "end_of_record\n",
            "SF:one.rs\nSF:two.rs\nDA:1,1\nend_of_record\n", "garbage\nend_of_record\n",
            "SF:main.rs\nUNKNOWN:1\nDA:1,1\nend_of_record\n",
        )
        for trace in traces:
            with self.subTest(trace=trace), self.assertRaises(ValueError):
                self.check(trace)

    def test_supported_lcov_metadata_does_not_replace_the_native_da_counts(self):
        trace = (
            "TN:\nSF:main.rs\nFN:1,main\nFNDA:1,main\nFNF:1\nFNH:1\n"
            "BRDA:1,0,0,-\nBRDA:1,0,1,1\nBRF:2\nBRH:1\nDA:1,1\nLF:9\nLH:0\nend_of_record\n"
        )
        self.assertTrue(self.check(trace))

    def test_perfect_summary_metadata_cannot_hide_an_unexecuted_da_line(self):
        self.assertFalse(self.check("SF:main.rs\nDA:1,0\nLF:1\nLH:1\nend_of_record\n"))

    def test_malformed_metadata_fails_instead_of_being_silently_ignored(self):
        for field in ("LF:-1", "FN:0,main", "FN:1", "FNDA:-1,main", "BRDA:1,0,0", "BRDA:0,0,0,1"):
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.check(f"SF:main.rs\n{field}\nDA:1,1\nend_of_record\n")

    def test_missing_or_non_utf8_reports_fail_with_the_read_error(self):
        for data in (None, b"\xff"):
            if data is not None:
                self.path.write_bytes(data)
            with self.subTest(data=data), self.assertRaisesRegex(ValueError, "cannot read"):
                self.checker.check_rust(self.path)


if __name__ == "__main__":
    unittest.main()
