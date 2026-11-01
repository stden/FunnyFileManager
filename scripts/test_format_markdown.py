"""Check scripts/format_markdown.py: alignments, cells, ragged rows, code blocks, spaces and imports.

Its command cases live in scripts/tests/format_markdown_commands.py, and run from here.
"""

import ast
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
# format_markdown.py beside this file, and the command cases under scripts/tests. Explicitly first on
# sys.path, as the other tests do.
sys.path[:0] = [str(ROOT / "scripts"), str(ROOT / "scripts/tests")]
import format_markdown as formatter  # noqa: E402
import format_markdown_tables as tables  # noqa: E402
from format_markdown_commands import CommandTests  # noqa: E402, F401

# Spaces that are content, never formatting whitespace: one column wide, and two.
NO_BREAK_SPACE, IDEOGRAPHIC_SPACE = "\u00a0", "\u3000"


class AlignmentTests(unittest.TestCase):
    def test_default_alignment_pads_on_the_right(self):
        table = "| Path | Contents |\n|---|---|\n| a | bb |\n"
        self.assertEqual(formatter.format_markdown(table), (
            "| Path | Contents |\n"
            "| ---- | -------- |\n"
            "| a    | bb       |\n"
        ))

    def test_left_alignment_keeps_its_colon_and_pads_on_the_right(self):
        table = "| Name | Qty |\n|:---|---|\n| apple | 3 |\n"
        self.assertEqual(formatter.format_markdown(table), (
            "| Name  | Qty |\n"
            "| :---- | --- |\n"
            "| apple | 3   |\n"
        ))

    def test_right_alignment_pads_on_the_left(self):
        table = "| Item | Count |\n|---|---:|\n| tea | 7 |\n| coffee | 120 |\n"
        self.assertEqual(formatter.format_markdown(table), (
            "| Item   | Count |\n"
            "| ------ | ----: |\n"
            "| tea    |     7 |\n"
            "| coffee |   120 |\n"
        ))

    def test_center_alignment_pads_on_both_sides_with_the_odd_space_on_the_right(self):
        table = "| Left | Center | Right |\n|:---|:---:|---:|\n| x | y | z |\n"
        self.assertEqual(formatter.format_markdown(table), (
            "| Left | Center | Right |\n"
            "| :--- | :----: | ----: |\n"
            "| x    |   y    |     z |\n"
        ))

    def test_a_delimiter_is_at_least_three_dashes_wide(self):
        self.assertEqual(
            formatter.format_markdown("| a | b |\n|-|-|\n"),
            "| a   | b   |\n| --- | --- |\n",
        )

    def test_rows_without_their_outer_pipes_get_them(self):
        self.assertEqual(
            formatter.format_markdown("| a | b\n|---|---\n1 | 2\n"),
            "| a   | b   |\n| --- | --- |\n| 1   | 2   |\n",
        )

    def test_a_list_item_after_a_table_is_not_part_of_it(self):
        self.assertEqual(
            formatter.format_markdown("| a | b |\n|---|---|\n- x | y\n"),
            "| a   | b   |\n| --- | --- |\n- x | y\n",
        )


class CellTests(unittest.TestCase):
    def test_an_escaped_pipe_stays_inside_its_cell(self):
        table = "| Expr | Meaning |\n|---|---|\n| `a \\| b` | or |\n"
        self.assertEqual(formatter.format_markdown(table), (
            "| Expr     | Meaning |\n"
            "| -------- | ------- |\n"
            "| `a \\| b` | or      |\n"
        ))

    def test_only_ascii_spaces_and_tabs_leave_the_edges_of_a_line_and_of_a_cell(self):
        self.assertEqual(tables.split_cells(" \t|\ta | b \t| \t"), ["a", "b"])
        edges = f"{NO_BREAK_SPACE}a{IDEOGRAPHIC_SPACE}"
        self.assertEqual(tables.split_cells(f"| {edges} | b |"), [edges, "b"])
        self.assertEqual(tables.split_cells(f"{edges} | {edges}"), [edges, edges])

    def test_a_unicode_space_ending_a_row_stays_with_or_without_the_closing_pipe(self):
        # A no-break space is one column wide and an ideographic space two: each cell pads to three.
        for space, cell in ((NO_BREAK_SPACE, f"y{NO_BREAK_SPACE} "), (IDEOGRAPHIC_SPACE, f"y{IDEOGRAPHIC_SPACE}")):
            for closing in ("", "|"):
                with self.subTest(space=space, closing=closing):
                    body_row = formatter.format_markdown(f"| a | b |\n|---|---|\n| x | y{space}{closing}\n")
                    self.assertEqual(body_row, f"| a   | b   |\n| --- | --- |\n| x   | {cell} |\n")
                    header = formatter.format_markdown(f"| x | y{space}{closing}\n|---|---|\n| a | b |\n")
                    self.assertEqual(header, f"| x   | {cell} |\n| --- | --- |\n| a   | b   |\n")

    def test_wide_characters_count_two_columns_each(self):
        table = "| 名前 | x |\n|---|---|\n| a | 漢字 |\n"
        self.assertEqual(formatter.format_markdown(table), (
            "| 名前 | x    |\n"
            "| ---- | ---- |\n"
            "| a    | 漢字 |\n"
        ))

    def test_display_width_counts_wide_characters_two_and_combining_marks_none(self):
        self.assertEqual(tables.display_width("a漢Ａ"), 5)
        self.assertEqual(tables.display_width("é"), 1)

    def test_every_line_of_a_table_has_the_same_display_width(self):
        table = "| 名前 | Note |\n|:--:|---|\n| 漢字 | é |\n| a | b |\n"
        widths = {tables.display_width(line) for line in formatter.format_markdown(table).split("\n")[:-1]}
        self.assertEqual(len(widths), 1)


class RaggedTableTests(unittest.TestCase):
    # Text, then the (line, cells, header cells) of each row whose cell count is not its header's.
    CASES = {
        "short and long rows, in two tables": (
            "| a | b |\n|---|---|\n| 1 |\n| 2 | 3 |\n| 4 | 5 | 6 |\n\n| c |\n|---|\n| 7 | 8 |\n",
            [(3, 1, 2), (5, 3, 2), (9, 2, 1)],
        ),
        "an unescaped pipe splits a code span, as in GFM": ("| a | b |\n|---|---|\n| `x|y` | z |\n", [(3, 3, 2)]),
        "a Unicode space after the closing pipe": (f"| a |\n|---|\n| x |{IDEOGRAPHIC_SPACE}\n", [(3, 2, 1)]),
        "a table before an unclosed code block": ("| a |\n|---|\n| 1 | 2 |\n```\n| x |\n", [(3, 2, 1)]),
    }

    def test_each_row_whose_cell_count_is_not_its_headers_is_named_by_line(self):
        for name, (text, rows) in self.CASES.items():
            with self.subTest(name):
                with self.assertRaises(formatter.RaggedTableError) as caught:
                    formatter.format_markdown(text)
                self.assertEqual(caught.exception.rows, rows)


class CodeBlockTests(unittest.TestCase):
    def test_a_fenced_block_with_pipes_stays_exactly_as_it_is(self):
        block = "```md\n| a|b   \n|-|-\n```\n"
        self.assertEqual(formatter.format_markdown(block), block)

    def test_a_tilde_block_ignores_backtick_fences_and_closes_on_a_longer_tilde_fence(self):
        block = "~~~ python\n```\n| a|b |\n|-|-|\n~~~~\n"
        self.assertEqual(formatter.format_markdown(block), block)

    def test_a_longer_backtick_fence_is_not_closed_by_a_shorter_one(self):
        block = "````\n```\n| a|b |\n````\n"
        self.assertEqual(formatter.format_markdown(block), block)

    def test_a_fence_followed_by_a_unicode_space_is_code_and_does_not_close_the_block(self):
        block = f"```\n```{IDEOGRAPHIC_SPACE}\nstill code  \n```\n"
        self.assertEqual(formatter.format_markdown(block), block)

    def test_an_unclosed_block_keeps_its_trailing_blank_lines_byte_for_byte(self):
        block = "```\n| a|b |\n\n  \n\n"
        self.assertEqual(formatter.format_markdown(block), block)

    def test_a_table_after_a_code_block_is_still_formatted(self):
        text = "```\n| x|y\n```\n\n| a|b |\n|-|-|\n"
        self.assertEqual(
            formatter.format_markdown(text),
            "```\n| x|y\n```\n\n| a   | b   |\n| --- | --- |\n",
        )


class SpacingTests(unittest.TestCase):
    def test_trailing_spaces_and_tabs_go_outside_code(self):
        self.assertEqual(formatter.format_markdown("Text  \n- item\t \n"), "Text\n- item\n")

    def test_the_file_ends_with_exactly_one_newline(self):
        for text, expected in (("# T", "# T\n"), ("# T\n\n\n", "# T\n"), ("# T\n", "# T\n"), ("", "")):
            with self.subTest(text=text):
                self.assertEqual(formatter.format_markdown(text), expected)

    def test_formatting_twice_changes_nothing(self):
        text = (
            "# Title  \n\n| Name | Notes | Count |\n|:--|:-:|--:|\n| 名前 | a\\|b | 3 |\n| x | | |\n\n"
            "```\n| kept|as is  \n```\n"
        )
        once = formatter.format_markdown(text)
        self.assertEqual(formatter.format_markdown(once), once)


class ImportTests(unittest.TestCase):
    def test_the_formatter_modules_import_nothing_they_do_not_use(self):
        for module in (formatter, tables):
            tree = ast.parse(Path(module.__file__).read_text(encoding="utf-8"))
            imports = [node for node in ast.walk(tree) if isinstance(node, (ast.Import, ast.ImportFrom))]
            imported = {(alias.asname or alias.name).split(".")[0] for node in imports for alias in node.names}
            used = {node.id for node in ast.walk(tree) if isinstance(node, ast.Name)}
            with self.subTest(module=module.__name__):
                self.assertEqual(imported - used, set())


if __name__ == "__main__":
    unittest.main()
