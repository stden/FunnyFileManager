"""Check developer checkers' command failures and platform fallbacks."""

from contextlib import redirect_stderr, redirect_stdout
import io
from pathlib import Path
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

from sandbox import SandboxTestCase

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import format_markdown  # noqa: E402
import format_markdown_tables  # noqa: E402
import pre_commit_hook  # noqa: E402
import shortcut_files  # noqa: E402
import use_ram_target  # noqa: E402


class CheckerCommandTests(SandboxTestCase):
    def test_duplication_cli_reports_both_copies_and_exits_nonzero(self):
        self.sandbox.copy("scripts/lint_duplication.py")
        block = "".join(f"value_{number} = compute({number})\n" for number in range(12))
        first = self.sandbox.write("one.py", block)
        second = self.sandbox.write("two.py", block)
        result = self.sandbox.run("scripts/lint_duplication.py", first, second)
        self.assertEqual(result.returncode, 1)
        self.assertIn("duplicated block (12 lines, 2 copies)", result.stderr)
        self.assertIn("one.py:1-12", result.stderr)
        self.assertIn("two.py:1-12", result.stderr)
        self.assertIn("1 duplicated block(s)", result.stderr)

    def test_complexity_cli_names_the_over_budget_function(self):
        self.sandbox.copy("scripts/lint_complexity.py")
        branches = "".join(f"    if value == {number}:\n        return {number}\n" for number in range(11))
        source = self.sandbox.write("busy.py", "def busy(value):\n" + branches)
        result = self.sandbox.run("scripts/lint_complexity.py", source)
        self.assertEqual(result.returncode, 1)
        self.assertIn("busy.py:1: busy has cyclomatic complexity 12", result.stderr)
        self.assertIn("1 function(s) over the complexity budget", result.stderr)

    def test_shortcut_copy_failure_reports_os_error_and_preserves_source(self):
        source = self.sandbox.write("entry.desktop", "original")
        output = io.StringIO()
        with redirect_stderr(output):
            target = shortcut_files.copy_into(self.sandbox.repo / "missing", source)
        self.assertEqual(source.read_text(), "original")
        self.assertFalse(target.exists())
        self.assertIn("cp: cannot create regular file", output.getvalue())


class CheckerFailureTests(unittest.TestCase):
    def test_markdown_git_permission_failure_reports_the_command_could_not_run(self):
        output = io.StringIO()
        with (
            patch.object(format_markdown.subprocess, "run", side_effect=PermissionError("denied")),
            redirect_stderr(output),
        ):
            with self.assertRaises(SystemExit) as stopped:
                format_markdown.markdown_names()
        self.assertEqual(stopped.exception.code, 2)
        self.assertIn("git ls-files could not run: denied", output.getvalue())

    def test_backticks_in_a_backtick_fence_info_string_are_not_an_opening_fence(self):
        self.assertIsNone(format_markdown_tables.opening_fence("```python `invalid"))

    def test_platform_without_noexec_flag_keeps_the_normal_filesystem_behavior(self):
        with patch.object(use_ram_target, "os", SimpleNamespace()):
            self.assertFalse(use_ram_target.mounted_noexec("unused"))

    def test_pre_commit_returns_a_linter_failure_without_printing_success(self):
        checker = SimpleNamespace(
            use_utf8_output=Mock(), run=Mock(), has_crate=Mock(return_value=False),
            main=Mock(return_value=23),
        )
        output = io.StringIO()
        with patch.object(pre_commit_hook, "lint", checker), redirect_stdout(output), patch.dict("os.environ", {"SKIP_CLIPPY": ""}):
            self.assertEqual(pre_commit_hook.main(), 23)
        checker.main.assert_called_once_with(["clippy"])
        self.assertNotIn("checks passed", output.getvalue())


if __name__ == "__main__":
    unittest.main()
