"""Check scripts/coverage.py, the line-coverage gate, against a fake cargo-llvm-cov."""

import re
import unittest

from sandbox import SandboxTestCase
from script_files import LINT


# A fake Cargo: `llvm-cov nextest` exits with NEXTEST_STATUS; `llvm-cov
# report` writes SUMMARY to the file after --output-path.
FAKE_CARGO = """
    if args[1:2] == ["nextest"]:
        sys.exit({nextest_status})
    if args[1:2] == ["report"]:
        with open(args[args.index("--output-path") + 1], "w", encoding="utf-8") as out:
            out.write({summary!r})
"""
# llvm-cov's JSON export, cut down to what the gate reads.
SUMMARY = '{{"data": [{{"totals": {{"lines": {{"count": {count}, "covered": {covered}, "percent": 0}}}}}}]}}'
# Paths as llvm-cov reports them: absolute.
LEFT_OUT = (
    "/w/src/main.rs", "/w/src/editor.rs", "/w/src/bootstrap.rs", "/w/src/clipboard/system.rs",
    "/w/src/ops/shell/tty.rs", "/w/src/highlight/colorer.rs", "/w/src/ops/tests/copy.rs",
    "/w/src/ui/tests_info_row_messages.rs", "/w/src/status/tests.rs", "/w/src/app_tests.rs",
    "/w/src/test_support.rs", "/w/src/test_support/processes.rs", "/w/tests/tui_bdd/session.rs",
)
MEASURED = (
    "/w/src/app.rs", "/w/src/ops/shell.rs", "/w/src/editor/spawn.rs", "/w/src/highlight/builtin.rs",
    "/w/src/testing.rs", "/w/src/clipboard.rs", "/w/src/models/latest.rs",
)


class CoverageGateTests(SandboxTestCase):
    def setUp(self):
        super().setUp()
        for script in ("scripts/coverage.py", *LINT):
            self.sandbox.copy(script)

    def gate(self, *, covered=98, count=100, nextest_status=0, summary=None, env=None):
        summary = summary if summary is not None else SUMMARY.format(count=count, covered=covered)
        self.sandbox.fake("cargo", effect=FAKE_CARGO.format(nextest_status=nextest_status, summary=summary))
        return self.sandbox.run("scripts/coverage.py", env=env)

    def test_the_suite_runs_under_llvm_cov_then_the_exact_totals_are_read(self):
        result = self.gate(covered=98, count=100)
        self.assertEqual(result.returncode, 0, result.stderr)
        tests, report = self.sandbox.args("cargo")
        regex = tests[tests.index("--ignore-filename-regex") + 1]
        self.assertEqual(
            tests,
            ["llvm-cov", "nextest", "--features", "test-helpers", "--ignore-filename-regex", regex,
             "--show-missing-lines"],
        )
        self.assertEqual(
            report[:-1],
            ["llvm-cov", "report", "--ignore-filename-regex", regex, "--json", "--summary-only", "--output-path"],
        )
        self.assertTrue(result.stdout.endswith("✓ line coverage 98.00% of 100 measured lines (98 covered), floor 98%\n"))
        self.assertEqual(result.stderr, "")

    def test_the_measurement_leaves_out_entry_points_host_glue_and_tests_only(self):
        self.gate()
        regex = re.compile(self.sandbox.args("cargo")[0][5])
        for path in LEFT_OUT:
            with self.subTest(left_out=path):
                self.assertTrue(regex.search(path))
        for path in MEASURED:
            with self.subTest(measured=path):
                self.assertFalse(regex.search(path))

    def test_a_share_just_under_the_floor_fails_and_is_not_rounded_up(self):
        # llvm-cov's table shows 97.998% as "98.00%".
        result = self.gate(covered=97998, count=100000)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(
            result.stderr, "✗ line coverage 97.99% of 100000 measured lines (97998 covered) is under the 98% floor\n"
        )

    def test_a_failing_test_stops_the_gate_with_its_status(self):
        result = self.gate(nextest_status=100)
        self.assertEqual(result.returncode, 100)
        self.assertEqual([args[:2] for args in self.sandbox.args("cargo")], [["llvm-cov", "nextest"]])

    def test_no_measured_lines_fail(self):
        result = self.gate(covered=0, count=0)
        self.assertEqual(result.returncode, 1)
        self.assertIn("measured no lines", result.stderr)

    def test_a_summary_it_cannot_read_fails_with_the_reason(self):
        for summary in ("not json", '{"data": []}', '{"data": [{"totals": {"lines": {"count": "x"}}}]}'):
            with self.subTest(summary=summary):
                result = self.gate(summary=summary)
                self.assertEqual(result.returncode, 1)
                self.assertIn("cannot read llvm-cov's summary", result.stderr)

    def test_without_a_wrapper_cargo_gets_env_as_one(self):
        # An empty RUSTC_WRAPPER means "none" and would outrank the pass-through.
        for extra in ({}, {"RUSTC_WRAPPER": ""}, {"CARGO_BUILD_RUSTC_WRAPPER": ""}):
            with self.subTest(env=extra):
                self.sandbox.log.unlink(missing_ok=True)
                self.assertEqual(self.gate(env=extra).returncode, 0)
                for call in self.sandbox.calls("cargo"):
                    self.assertEqual(call.env["CARGO_BUILD_RUSTC_WRAPPER"], "/usr/bin/env")
                    self.assertNotIn("RUSTC_WRAPPER", call.env)

    def test_a_wrapper_the_caller_chose_is_kept(self):
        for extra in ({"RUSTC_WRAPPER": "sccache"}, {"CARGO_BUILD_RUSTC_WRAPPER": "/opt/wrap"}):
            with self.subTest(env=extra):
                self.sandbox.log.unlink(missing_ok=True)
                self.assertEqual(self.gate(env=extra).returncode, 0)
                call = self.sandbox.calls("cargo")[0]
                self.assertEqual({name: call.env.get(name) for name in extra}, extra)
                self.assertEqual(len(call.env.keys() & {"RUSTC_WRAPPER", "CARGO_BUILD_RUSTC_WRAPPER"}), 1)

    def test_no_env_on_path_stops_before_cargo(self):
        result = self.gate(env={"PATH": str(self.sandbox.bin)})
        self.assertEqual(result.returncode, 1)
        self.assertIn("`env` is not on PATH; set CARGO_BUILD_RUSTC_WRAPPER", result.stderr)
        self.assertEqual(self.sandbox.calls("cargo"), [])

    def test_help_runs_nothing(self):
        for flag in ("-h", "--help"):
            with self.subTest(flag=flag):
                result = self.sandbox.run("scripts/coverage.py", flag)
                self.assertEqual(result.returncode, 0)
                self.assertTrue(result.stdout.startswith("Run the line-coverage gate CI enforces"))
        self.assertEqual(self.sandbox.calls(), [])


if __name__ == "__main__":
    unittest.main()
