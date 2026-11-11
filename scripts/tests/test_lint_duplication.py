"""Test the duplication gate: what counts as a copy, and what does not."""

from pathlib import Path
import sys
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))
import lint_duplication  # noqa: E402

BLOCK = "".join(f"    let value_{n} = compute({n});\n" for n in range(12))


def found(**sources):
    files = {
        Path(name.replace("__", "/") + ".rs"): lint_duplication.significant_lines(
            Path(name + ".rs"), text
        )
        for name, text in sources.items()
    }
    return lint_duplication.duplicates(files)


class DuplicationTests(unittest.TestCase):
    def test_a_block_in_two_files_is_found_with_its_lines(self):
        copies = found(a="fn a() {\n" + BLOCK + "}\n", b="fn b() {\n" + BLOCK + "}\n")
        self.assertEqual(len(copies), 1)
        self.assertEqual({(path.as_posix(), first) for path, first, _ in copies[0]},
                         {("a.rs", 2), ("b.rs", 2)})

    def test_indentation_blank_lines_and_comments_do_not_hide_a_copy(self):
        reindented = "\n// moved here\n".join(line.strip() for line in BLOCK.splitlines())
        self.assertEqual(len(found(a=BLOCK, b=reindented)), 1)

    def test_a_short_repeat_is_not_a_copy(self):
        short = "".join(BLOCK.splitlines(keepends=True)[:5])
        self.assertEqual(found(a=short, b=short), [])

    def test_a_copy_is_reported_once_at_its_full_length(self):
        longer = BLOCK + "    let extra = 1;\n    let more = 2;\n"
        copies = found(a=longer, b=longer)
        self.assertEqual(len(copies), 1)
        _, first, last = copies[0][0]
        self.assertEqual(last - first + 1, 14)

    def test_an_allowed_pair_is_not_reported(self):
        allowed = found(
            mcp__src__fsops__remove_moved=BLOCK, src__ops__file__remove_moved=BLOCK
        )
        self.assertEqual(allowed, [])

    def test_tests_are_left_out(self):
        for name in ("src/ops/tests.rs", "src/ops/tests/copy.rs", "scripts/tests/x.py",
                     "src/a_tests.rs", "scripts/test_lint.py", "src/test_support/fake.rs"):
            self.assertTrue(lint_duplication.LEFT_OUT.search(name), name)
        self.assertFalse(lint_duplication.LEFT_OUT.search("src/ops/file/transfer.rs"))

    def test_discovery_skips_removed_tracked_sources(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            rust = root / "present.rs"
            python = root / "present.py"
            rust.touch()
            python.touch()
            with patch.object(lint_duplication, "ROOT", root), \
                    patch.object(lint_duplication.subprocess, "run") as run:
                run.return_value.stdout = "present.rs\nremoved.rs\npresent.py\n"
                self.assertEqual(lint_duplication.tracked_sources(), [rust, python])

    def test_an_explicit_missing_source_still_raises(self):
        with TemporaryDirectory() as directory:
            with self.assertRaises(FileNotFoundError):
                lint_duplication.main([str(Path(directory) / "missing.rs")])

    def test_the_repository_has_no_copied_block(self):
        self.assertEqual(lint_duplication.main([]), 0)


if __name__ == "__main__":
    unittest.main()
