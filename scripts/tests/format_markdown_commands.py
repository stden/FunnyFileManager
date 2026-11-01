"""Check the command of scripts/format_markdown.py: its exit status, its output, what it writes, git, and
its file as a program.

scripts/test_format_markdown.py imports CommandTests and runs it. The name has no test_ prefix, so the
self-test's discovery of scripts/tests does not run these cases a second time.
"""

import io
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/format_markdown.py"
# format_markdown.py beside scripts/. Explicitly first on sys.path, as the other tests do.
sys.path.insert(0, str(ROOT / "scripts"))
import format_markdown as formatter  # noqa: E402

# A table whose body row has one cell more than its header, and what the command reports after the file's name.
RAGGED = "| a | b |\n|---|---|\n| 1 | 2 | 3 |\n"
RAGGED_REPORT = ":3: table row has 3 cells, header has 2\n"


class CommandTests(unittest.TestCase):
    def run_script(self, *args):
        return subprocess.run(
            [sys.executable, str(SCRIPT), *args], capture_output=True, text=True, encoding="utf-8", check=False
        )

    def test_check_names_each_file_that_would_change_and_writes_nothing(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "notes.md"
            path.write_text("| a | b |\n|-|-|\n", encoding="utf-8")
            result = self.run_script("--check", str(path))
            self.assertEqual((result.returncode, result.stderr), (1, ""))
            self.assertEqual(result.stdout.splitlines(), [str(path)])
            self.assertEqual(path.read_text(encoding="utf-8"), "| a | b |\n|-|-|\n")

    def test_check_passes_a_file_that_is_already_formatted(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "notes.md"
            path.write_text("| a   | b   |\n| --- | --- |\n", encoding="utf-8")
            result = self.run_script("--check", str(path))
            self.assertEqual((result.returncode, result.stdout, result.stderr), (0, "", ""))

    def test_without_check_the_file_is_rewritten_and_named(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "notes.md"
            path.write_text("| a | b |\n|-|-|", encoding="utf-8")
            result = self.run_script(str(path))
            self.assertEqual((result.returncode, result.stderr), (0, ""))
            self.assertEqual(result.stdout.splitlines(), [str(path)])
            self.assertEqual(path.read_text(encoding="utf-8"), "| a   | b   |\n| --- | --- |\n")

    def test_a_ragged_row_fails_the_run_after_the_other_files_are_formatted(self):
        with tempfile.TemporaryDirectory() as directory:
            ragged, plain = Path(directory) / "ragged.md", Path(directory) / "plain.md"
            ragged.write_text(RAGGED, encoding="utf-8")
            plain.write_text("| a | b |\n|-|-|\n", encoding="utf-8")
            result = self.run_script(str(ragged), str(plain))
            self.assertEqual(
                (result.returncode, result.stdout, result.stderr), (1, f"{plain}\n", f"{ragged}{RAGGED_REPORT}")
            )
            self.assertEqual(ragged.read_text(encoding="utf-8"), RAGGED)
            self.assertEqual(plain.read_text(encoding="utf-8"), "| a   | b   |\n| --- | --- |\n")

    def test_check_names_the_ragged_rows_of_every_file_and_exits_1(self):
        with tempfile.TemporaryDirectory() as directory:
            paths = [Path(directory) / name for name in ("one.md", "two.md")]
            for path in paths:
                path.write_text(RAGGED, encoding="utf-8")
            result = self.run_script("--check", *map(str, paths))
            self.assertEqual((result.returncode, result.stdout), (1, ""))
            self.assertEqual(result.stderr, "".join(f"{path}{RAGGED_REPORT}" for path in paths))

    def test_a_crlf_file_is_skipped_with_a_warning_and_never_rewritten(self):
        crlf = b"| a | b |\r\n|-|-|\r\n"
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "notes.md"
            path.write_bytes(crlf)
            result = self.run_script(str(path))
            kept = path.read_bytes()
        self.assertEqual((result.returncode, result.stdout), (0, ""))
        self.assertIn(f"skipped {path}", result.stderr)
        self.assertEqual(kept, crlf)

    def test_the_default_list_holds_the_tracked_markdown_files_but_no_framework_files(self):
        listed = [path.relative_to(formatter.ROOT).as_posix() for path in formatter.markdown_files()]
        self.assertIn("README.md", listed)
        self.assertEqual([name for name in listed if name.startswith(formatter.FRAMEWORK_PREFIXES)], [])

    def test_without_git_the_tree_is_walked_and_the_walk_is_named(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ("docs/a.md", "target/skipped.md", ".claude/agents/x.md", "aidlc/spaces/y.md"):
                (root / name).parent.mkdir(parents=True, exist_ok=True)
                (root / name).write_text("# a\n", encoding="utf-8")
            stderr = io.StringIO()
            with patch.object(formatter, "ROOT", root), patch("subprocess.run", side_effect=FileNotFoundError):
                with redirect_stderr(stderr):
                    self.assertEqual(formatter.markdown_files(), [root / "docs" / "a.md"])
        self.assertIn("walking the tree", stderr.getvalue())

    def test_the_formatter_runs_as_a_program(self):
        # The other tests start it with sys.executable, which skips both checks.
        self.assertEqual(SCRIPT.read_text(encoding="utf-8").splitlines()[0], "#!/usr/bin/env python3")
        staged = subprocess.run(
            ["git", "ls-files", "--stage", "--", "scripts/format_markdown.py"],
            cwd=ROOT, capture_output=True, text=True, check=True,
        )
        self.assertTrue(staged.stdout.startswith("100755 "), f"git records {staged.stdout!r}")

    def test_a_failing_git_ls_files_exits_2_and_names_the_git_error(self):
        failure = subprocess.CalledProcessError(128, ["git", "ls-files"], stderr=b"fatal: not a git repository\n")
        stderr = io.StringIO()
        with patch("subprocess.run", side_effect=failure), redirect_stderr(stderr):
            with self.assertRaises(SystemExit) as caught:
                formatter.markdown_files()
        self.assertEqual(caught.exception.code, 2)
        self.assertIn("fatal: not a git repository", stderr.getvalue())
