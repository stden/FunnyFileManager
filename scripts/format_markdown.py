#!/usr/bin/env python3
"""Align the tables of Markdown files, and trim their trailing spaces.

A table is a header row that starts with a pipe, a delimiter row of dashes with
optional colons, and the lines after it that hold a pipe and do not start another
block (a heading, a quote, a list item or a code fence), as in GFM. Each column
is as wide as its widest cell, and at least MIN_WIDTH, so that the delimiter
keeps its colons. Cells are padded so that the pipes line up: on the left for
right alignment, on both sides for center, on the right otherwise. Every row
keeps a leading and a trailing pipe. Cells are split as GFM splits them: at
every unescaped pipe, even in a code span, while an escaped pipe is content and
is kept as written. A body row whose cell count is not its header's is an error,
since GFM would pad it with empty cells or drop its extra ones. Fenced code
blocks are left as they are. Elsewhere, trailing spaces and tabs go, and the
file ends with exactly one newline. Spaces and tabs are the only whitespace that
formatting touches; any other character, such as a no-break space, is content
(format_markdown_tables.py states the rule). A code block that never closes runs
to the end of the file, and that part is kept byte for byte. A file with CRLF
line endings is skipped with a warning, because formatting it would rewrite its
line endings as LF.

Standard library only. Usage:
  scripts/format_markdown.py [--check] [FILE ...]   (default: every tracked *.md outside .claude/ and aidlc/)
    --check  print the files that would change and exit 1; write nothing
A ragged table row prints <file>:<line>: table row has <n> cells, header has <m>
on stderr. That file is left as it is, the others are still formatted, and the
exit status is 1 in both modes.
"""

import argparse
from pathlib import Path
import subprocess
import sys

from format_markdown_tables import (
    FORMATTING_WHITESPACE, format_table, is_table_start, opening_fence, ragged_rows, table_end,
)


ROOT = Path(__file__).resolve().parents[1]
# What the walk without git leaves out.
SKIPPED_DIRECTORIES = {".git", "target", "node_modules"}
# The AI-DLC framework owns the files under .claude/ and aidlc/, so they are not formatted here.
FRAMEWORK_PREFIXES = (".claude/", "aidlc/")


def is_closing_fence(line, fence):
    """Only the fence's own character, and at least as many of it as the opening fence."""
    marker = line.strip(FORMATTING_WHITESPACE)
    return len(marker) >= len(fence) and marker == fence[0] * len(marker)


def code_block_end(lines, start, fence):
    """The index past the code block that opens at `start`, or None when it never closes."""
    for index in range(start + 1, len(lines)):
        if is_closing_fence(lines[index], fence):
            return index + 1
    return None


def with_final_newline(lines):
    """The lines as one text, without blank lines at the end and with one newline after the last."""
    text = "\n".join(lines).rstrip("\n")
    return text + "\n" if text else ""


class RaggedTableError(ValueError):
    """Table rows whose cell count is not their header's: `rows` holds (line number, cells, header cells)."""

    def __init__(self, rows):
        super().__init__(f"table rows whose cell count is not their header's: {rows}")
        self.rows = rows


def format_markdown(text):
    """The formatted text of one Markdown file. Raises RaggedTableError, naming every such row of the text,
    when a table row's cell count is not its header's."""
    lines = text.split("\n")
    out, ragged = [], []
    index = 0
    while index < len(lines):
        fence = opening_fence(lines[index])
        if fence:
            end = code_block_end(lines, index, fence)
            if end is None:
                break  # The block never closes: the rest of the file is code.
            out.extend(lines[index:end])
        elif is_table_start(lines, index):
            end = table_end(lines, index)
            rows = ragged_rows(lines, index, end)
            ragged.extend(rows)
            out.extend(lines[index:end] if rows else format_table(lines[index:end]))
        else:
            end = index + 1
            out.append(lines[index].rstrip(FORMATTING_WHITESPACE))
        index = end
    if ragged:
        raise RaggedTableError(ragged)
    # The unclosed code block, if any, keeps its trailing blank lines: they are code.
    unclosed = lines[index:]
    return "\n".join([*out, *unclosed]) if unclosed else with_final_newline(out)


def walked_names():
    """Root-relative names of every Markdown file under the root, outside the skipped directories."""
    walked = [path.relative_to(ROOT) for path in ROOT.rglob("*.md")]
    return [path.as_posix() for path in walked if SKIPPED_DIRECTORIES.isdisjoint(path.parts)]


def fail(message):
    """Report `message` on stderr, and exit with status 2."""
    print(f"format_markdown: {message}", file=sys.stderr)
    sys.exit(2)


def markdown_names():
    """Root-relative names of the tracked Markdown files; without git, of every one under the root."""
    try:
        listed = subprocess.run(
            ["git", "ls-files", "-z", "--", "*.md"], cwd=ROOT, capture_output=True, check=True
        )
    except FileNotFoundError:
        print("format_markdown: git is not installed; walking the tree for Markdown files instead", file=sys.stderr)
        return walked_names()
    except subprocess.CalledProcessError as error:
        detail = (error.stderr or b"").decode("utf-8", "replace").strip()
        fail(f"git ls-files failed: {detail or error}")
    except OSError as error:
        fail(f"git ls-files could not run: {error}")
    return listed.stdout.decode("utf-8").split("\0")


def markdown_files():
    """The Markdown files to format: the names from markdown_names, outside the AI-DLC framework's."""
    names = (name for name in markdown_names() if name and not name.startswith(FRAMEWORK_PREFIXES))
    return [ROOT / name for name in names if (ROOT / name).is_file()]


def shown(path):
    """The path as a report names it: relative to the root when it lies under the root."""
    return path.relative_to(ROOT) if path.is_relative_to(ROOT) else path


def format_file(path, check):
    """Whether formatting changes the file at `path`; without `check`, the change is written. A CRLF file is
    skipped. RaggedTableError comes through before anything is written."""
    # Bytes, not read_text: text mode turns CRLF into LF before the check below could see it.
    text = path.read_bytes().decode("utf-8")
    if "\r\n" in text:
        print(f"format_markdown: skipped {shown(path)}: CRLF line endings would become LF", file=sys.stderr)
        return False
    formatted = format_markdown(text)
    if formatted == text:
        return False
    if not check:
        path.write_text(formatted, encoding="utf-8", newline="\n")
    return True


def report_ragged_rows(path, error):
    """Name each row of `error` on stderr, as <path>:<line>."""
    for line, cells, header in error.rows:
        print(f"{shown(path)}:{line}: table row has {cells} cells, header has {header}", file=sys.stderr)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Align Markdown tables and trim trailing spaces.")
    parser.add_argument(
        "--check", action="store_true", help="print the files that would change and exit 1; write nothing"
    )
    parser.add_argument("files", nargs="*", type=Path, help="files to format (default: every tracked *.md)")
    args = parser.parse_args(argv)
    changed, ragged = [], []
    for path in args.files or markdown_files():
        try:
            if format_file(path, args.check):
                changed.append(path)
        except RaggedTableError as error:
            report_ragged_rows(path, error)
            ragged.append(path)
    for path in changed:
        print(shown(path))
    return 1 if ragged or (args.check and changed) else 0


if __name__ == "__main__":
    sys.exit(main())
