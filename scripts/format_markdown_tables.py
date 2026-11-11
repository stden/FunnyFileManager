"""Purpose: GFM table layout for scripts/format_markdown.py: cell splitting, column widths, alignment markers and padding.

One rule says what is formatting and what is content. Formatting whitespace is the ASCII space and tab,
at the edges of a line and at the edges of a cell: only there does the layout remove whitespace or add
its own. Every other character is content and survives byte for byte: a no-break space (U+00A0) or an
ideographic space (U+3000) at the end of a row stays in its cell, with or without a closing pipe, and
one after the closing pipe is a cell of its own, as GFM reads it.
"""

import re
import unicodedata


# The narrowest column: wide enough for the colons of `:-:`.
MIN_WIDTH = 3
LEFT, RIGHT, CENTER, DEFAULT = "left", "right", "center", "default"
# The only whitespace that formatting removes or adds, at the edges of a line or of a cell.
FORMATTING_WHITESPACE = " \t"
# The formatting whitespace that may start a line, as a pattern.
INDENT = f"[{FORMATTING_WHITESPACE}]*"
# A pipe that no backslash escapes.
UNESCAPED_PIPE = re.compile(r"(?<!\\)\|")
DELIMITER_CELL = re.compile(r":?-+:?")
# The opening line of a fenced code block: its fence, then its info string.
FENCE = re.compile(INDENT + r"(`{3,}|~{3,})(.*)")
# A line that starts another block, which ends a table as it does in GFM.
BLOCK_START = re.compile(INDENT + r"(?:[#>]|[-+*] |\d+[.)] )")


def opening_fence(line):
    """The fence that opens a code block on `line`, or None. A backtick info string has no backtick."""
    match = FENCE.match(line)
    if match is None:
        return None
    fence, info = match.groups()
    if fence[0] == "`" and "`" in info:
        return None
    return fence


def char_width(char):
    """Two columns for an East Asian wide or fullwidth character, none for a combining mark."""
    if unicodedata.category(char) in ("Mn", "Me"):
        return 0
    if unicodedata.east_asian_width(char) in ("W", "F"):
        return 2
    return 1


def display_width(text):
    return sum(char_width(char) for char in text)


def split_cells(line):
    """The cells of one table row, without the formatting whitespace at their edges. An escaped pipe stays in
    its cell, as written."""
    text = line.strip(FORMATTING_WHITESPACE)
    cells = UNESCAPED_PIPE.split(text)
    if text.startswith("|"):
        cells = cells[1:]
    if text.endswith("|") and not text.endswith("\\|"):
        cells = cells[:-1]
    return [cell.strip(FORMATTING_WHITESPACE) for cell in cells]


def alignment(delimiter):
    """The alignment that a delimiter cell asks for: colons on both sides, one side, or none."""
    starts, ends = delimiter.startswith(":"), delimiter.endswith(":")
    if starts and ends:
        return CENTER
    if ends:
        return RIGHT
    return LEFT if starts else DEFAULT


def delimiter_text(align, width):
    """`width` characters of a delimiter cell: dashes, with the colons that `align` asks for."""
    left = ":" if align in (LEFT, CENTER) else ""
    right = ":" if align in (RIGHT, CENTER) else ""
    return left + "-" * (width - len(left) - len(right)) + right


def pad(text, width, align):
    """`text` padded with spaces to `width`: before it when right-aligned, around it when centered."""
    gap = width - display_width(text)
    if align == RIGHT:
        return " " * gap + text
    if align == CENTER:
        before = gap // 2
        return " " * before + text + " " * (gap - before)
    return text + " " * gap


def join_cells(cells):
    return "|" + "|".join(f" {cell} " for cell in cells) + "|"


def column_widths(rows):
    """Each column as wide as its widest cell, and at least MIN_WIDTH."""
    return [max([MIN_WIDTH] + [display_width(cell) for cell in column]) for column in zip(*rows)]


def format_row(cells, widths, aligns):
    return join_cells([pad(cell, width, align) for cell, width, align in zip(cells, widths, aligns)])


def ragged_rows(lines, start, end):
    """(line number, cells, header cells) for each body row of the table lines[start:end] whose cell count is
    not its header's. GFM pads a short row and drops a long row's extra cells, so either hides a defect."""
    header = len(split_cells(lines[start]))
    counts = ((index + 1, len(split_cells(lines[index]))) for index in range(start + 2, end))
    return [(number, count, header) for number, count in counts if count != header]


def format_table(lines):
    """The aligned lines of one table: the header, the delimiter row, then the body rows. Its rows must all
    have the header's cell count (see ragged_rows), or the columns would cut a longer row short."""
    indent = lines[0][: len(lines[0]) - len(lines[0].lstrip(FORMATTING_WHITESPACE))]
    rows = [split_cells(line) for line in [lines[0], *lines[2:]]]
    aligns = [alignment(cell) for cell in split_cells(lines[1])]
    widths = column_widths(rows)
    delimiter = join_cells([delimiter_text(align, width) for align, width in zip(aligns, widths)])
    formatted = [format_row(row, widths, aligns) for row in rows]
    formatted.insert(1, delimiter)
    return [indent + line for line in formatted]


def is_table_start(lines, index):
    """A header row that starts with a pipe, then a delimiter row with as many cells."""
    if index + 1 >= len(lines) or not lines[index].lstrip(FORMATTING_WHITESPACE).startswith("|"):
        return False
    header = split_cells(lines[index])
    delimiter = split_cells(lines[index + 1])
    return bool(header) and len(header) == len(delimiter) and all(
        DELIMITER_CELL.fullmatch(cell) for cell in delimiter
    )


def is_table_row(line):
    """A body line of a table: it holds a pipe, and it starts no other block."""
    return "|" in line and BLOCK_START.match(line) is None and opening_fence(line) is None


def table_end(lines, start):
    """The index past the table at `start`: the rows that follow its delimiter row."""
    end = start + 2
    while end < len(lines) and is_table_row(lines[end]):
        end += 1
    return end
