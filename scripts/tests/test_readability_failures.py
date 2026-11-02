"""Validate compiler rejection, diagnostic parsing, and readability-checker failures."""

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
import test_readability as checker  # noqa: E402


def compiler_result(status=0, stderr=""):
    return subprocess.CompletedProcess([], status, stdout="", stderr=stderr)


class ReadabilityFailureTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.output = Path(directory.name)
        patcher = patch.object(checker, "OUTPUT", self.output)
        patcher.start()
        self.addCleanup(patcher.stop)

    def test_broken_rustup_resolution_preserves_the_toolchain_error(self):
        failed = compiler_result(1, "toolchain unavailable")
        with patch.object(checker.shell_path, "find", return_value="/fake/rustup"):
            with patch.object(checker.subprocess, "run", return_value=failed) as execute:
                with self.assertRaisesRegex(RuntimeError, "Cannot resolve rustc.*rust-toolchain.toml"):
                    checker.tool_path("rustc", {})
        self.assertEqual(execute.call_args.args[0], ["/fake/rustup", "which", "rustc"])

    def test_diagnostics_ignore_non_json_output_without_losing_lint_codes(self):
        diagnostic = {"level": "error", "code": {"code": "clippy::too_many_lines"}}
        self.assertEqual(checker.diagnostics("compiler banner\n" + json.dumps(diagnostic)), {"clippy::too_many_lines"})

    def test_rejected_compliant_fixture_reports_the_compiler_error_and_keeps_its_log(self):
        with patch.object(checker.subprocess, "run", return_value=compiler_result(1, "unexpected rejection")):
            with self.assertRaisesRegex(RuntimeError, "tui: refactored fixture failed:.*") as failed:
                checker.check_fixture("driver", "tui", ROOT, "refactored", {})
        self.assertIn("unexpected rejection", str(failed.exception))
        self.assertEqual((self.output / "tui-refactored.jsonl").read_text(), "unexpected rejection")

    def test_negative_fixture_cannot_pass_or_fail_with_unrelated_lints(self):
        unrelated = json.dumps({"level": "error", "code": {"code": "clippy::unrelated"}})
        for result in (compiler_result(), compiler_result(1, unrelated)):
            with self.subTest(status=result.returncode), patch.object(checker.subprocess, "run", return_value=result):
                with self.assertRaisesRegex(RuntimeError, "expected rejection by exactly the readability lints") as failed:
                    checker.check_fixture("driver", "tui", ROOT, "complex", {})
                self.assertIn("missing: clippy::cognitive_complexity", str(failed.exception))
                self.assertIn("unexpected: clippy::unrelated" if result.returncode else "unexpected: none", str(failed.exception))

    def test_equivalence_compile_failure_stops_before_running_the_test_binary(self):
        with patch.object(checker.subprocess, "run", return_value=compiler_result(1, "compile failed")) as execute:
            with self.assertRaisesRegex(RuntimeError, "Cannot compile behavior comparisons"):
                checker.check_equivalence("rustc", {})
        self.assertEqual(execute.call_count, 1)
        self.assertIn("--test", execute.call_args.args[0])

    def test_equivalence_runtime_failure_preserves_the_behavior_test_output(self):
        results = [compiler_result(), compiler_result(1, "assertion failed")]
        with patch.object(checker.subprocess, "run", side_effect=results) as execute:
            with self.assertRaisesRegex(RuntimeError, "Behavior comparisons failed") as failed:
                checker.check_equivalence("rustc", {})
        self.assertIn("assertion failed", str(failed.exception))
        self.assertEqual(execute.call_count, 2)
        self.assertEqual(execute.call_args.args[0][-1], "--quiet")

    def test_main_checks_each_crate_then_compares_the_compiled_behaviors(self):
        errors = "\n".join(json.dumps({"level": "error", "code": {"code": code}}) for code in checker.LINTS)

        def compile_or_run(command, **_options):
            return compiler_result(1, errors) if any(str(part).endswith("complex.rs") for part in command) else compiler_result()

        output = io.StringIO()
        with (
            patch.object(checker.shell_path, "find", side_effect=lambda name, **_options: None if name == "rustup" else name),
            patch.object(checker.subprocess, "run", side_effect=compile_or_run) as execute,
            redirect_stdout(output),
        ):
            checker.main()
        self.assertEqual(execute.call_count, len(checker.CRATES) * 2 + 2)
        self.assertEqual(output.getvalue().count("discovers shared limits"), len(checker.CRATES))
        self.assertIn("behavior agrees (3 tests)", output.getvalue())

    def test_cli_output_directory_failure_exits_one_without_a_traceback(self):
        output = io.StringIO()
        with patch.object(Path, "mkdir", side_effect=OSError("read-only output")), redirect_stderr(output):
            with self.assertRaises(SystemExit) as stopped:
                runpy.run_path(str(ROOT / "scripts/test_readability.py"), run_name="__main__")
        self.assertEqual(stopped.exception.code, 1)
        self.assertEqual(output.getvalue(), "Readability self-test failed: read-only output\n")


if __name__ == "__main__":
    unittest.main()
