"""Prove policy self-tests reject broken fixtures, manifests, and compiler verdicts."""

from contextlib import redirect_stderr, redirect_stdout
import io
import json
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import lint_policy_checker as checker  # noqa: E402
import lint_policy_clippy as clippy  # noqa: E402
import test_lint_policy as policy  # noqa: E402


def diagnostic(fixture, code):
    filename = "clippy.toml" if fixture == clippy.MANIFEST_FIXTURE else f"src/{fixture}.rs"
    return json.dumps({
        "reason": "compiler-message",
        "message": {"code": {"code": code}, "spans": [{"file_name": filename, "is_primary": True}]},
    })


def compiler_result(status=0, stdout="", stderr=""):
    return subprocess.CompletedProcess([], status, stdout=stdout, stderr=stderr)


class LintPolicyFailureTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.output = Path(directory.name) / "output"
        self.manifest = '[package]\nedition = "2021"\n' + (clippy.FIXTURES / "manifests/tui.toml").read_text(encoding="utf-8")
        for module in (clippy, policy):
            patcher = patch.object(module, "OUTPUT", self.output)
            patcher.start()
            self.addCleanup(patcher.stop)

    def test_lint_tables_fail_closed_when_a_valid_section_cannot_be_copied_verbatim(self):
        manifest = ' [lints.rust]\nunsafe_code = "deny"\n'
        with self.assertRaisesRegex(RuntimeError, "tui: cannot copy the \\[lints\\] tables verbatim"):
            clippy.lint_tables("tui", manifest)

    def test_negative_fixtures_must_fail_with_only_the_expected_lint_codes(self):
        expected = {"unsafe_block": {"unsafe_code"}}
        extra = diagnostic("unsafe_block", "unsafe_code") + "\n" + diagnostic("other", "clippy::unexpected")
        for result in (compiler_result(), compiler_result(1, extra)):
            with self.subTest(status=result.returncode), patch.object(clippy.subprocess, "run", return_value=result):
                with self.assertRaisesRegex(RuntimeError, "negative fixtures must fail with exactly their lints") as failed:
                    clippy.check_negatives("tui", f"negative-{result.returncode}", expected, self.manifest)
                self.assertIn("got []" if result.returncode == 0 else "other: unexpected", str(failed.exception))

    def test_rejected_positive_fixture_reports_its_lints_and_compiler_error(self):
        result = compiler_result(1, diagnostic("checked_conversions", "clippy::unexpected"), "compiler failure")
        with patch.object(clippy.subprocess, "run", return_value=result):
            with self.assertRaisesRegex(RuntimeError, "positive fixtures must pass") as failed:
                clippy.check_positives("tui", self.manifest)
        self.assertIn("checked_conversions", str(failed.exception))
        self.assertIn("clippy::unexpected", str(failed.exception))
        self.assertIn("compiler failure", str(failed.exception))

    def test_missing_mutation_anchor_is_reported_before_claiming_the_checker_passes(self):
        mutation = ("tui", "absent mutation anchor", "replacement", "expected violation")
        with patch.object(policy, "MUTATIONS", [mutation]):
            with self.assertRaisesRegex(RuntimeError, "fixture lacks the mutation anchor 'absent mutation anchor'"):
                policy.check_checker()

    def test_mutation_expected_diagnostics_must_match_the_checker_exactly(self):
        crate, old, new, _expected = policy.MUTATIONS[0]
        with patch.object(policy, "MUTATIONS", [(crate, old, new, "wrong expected message")]):
            with self.assertRaisesRegex(RuntimeError, "expected wrong expected message") as failed:
                policy.check_checker()
        self.assertIn("got", str(failed.exception))

    def test_manifest_drift_names_the_changed_required_lint(self):
        changed = self.manifest.replace('unsafe_code = "deny"', 'unsafe_code = "allow"', 1)
        self.assertNotEqual(changed, self.manifest)
        with self.assertRaisesRegex(RuntimeError, "manifests drift from the lint policy") as failed:
            policy.check_manifests({"tui": changed})
        self.assertIn('unsafe_code is "allow"', str(failed.exception))

    def test_reason_for_an_absent_lint_cannot_be_inferred_from_another_comment(self):
        self.assertFalse(checker.has_reason('# another lint\nother = "allow"', "missing"))

    def test_diagnostic_parser_ignores_cargo_status_and_uncoded_compiler_messages(self):
        messages = [
            "cargo progress",
            json.dumps({"reason": "build-finished", "success": False}),
            json.dumps({"reason": "compiler-message", "message": {"code": None}}),
            diagnostic("unsafe_block", "unsafe_code"),
        ]
        self.assertEqual(dict(clippy.codes_by_fixture("\n".join(messages))), {"unsafe_block": {"unsafe_code"}})

    def test_main_validates_each_manifest_and_checks_generated_compiler_fixtures(self):
        def compile_fixtures(_command, **options):
            directory = Path(options["cwd"])
            if directory.name == "positive":
                return compiler_result()
            expected = policy.NEGATIVES[directory.parent.name][directory.name]
            messages = [diagnostic(name, code) for name, codes in expected.items() for code in sorted(codes)]
            return compiler_result(1, "\n".join(messages))

        output = io.StringIO()
        with patch.object(clippy.subprocess, "run", side_effect=compile_fixtures) as execute, redirect_stdout(output):
            policy.main()
        self.assertEqual(execute.call_count, sum(len(kinds) + 1 for kinds in policy.NEGATIVES.values()))
        self.assertIn("judges", output.getvalue())
        self.assertIn("manifests match the lint policy", output.getvalue())
        self.assertTrue((self.output / "tui/negative/clippy.jsonl").is_file())

    def test_cli_manifest_read_failure_exits_one_without_a_traceback(self):
        output = io.StringIO()
        with patch.object(Path, "read_text", side_effect=OSError("unreadable manifest")), redirect_stderr(output):
            with self.assertRaises(SystemExit) as stopped:
                runpy.run_path(str(ROOT / "scripts/test_lint_policy.py"), run_name="__main__")
        self.assertEqual(stopped.exception.code, 1)
        self.assertEqual(output.getvalue(), "Lint policy self-test failed: unreadable manifest\n")


if __name__ == "__main__":
    unittest.main()
