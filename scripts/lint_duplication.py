"""Refuse copied code: the same run of lines in two places.

A block pasted instead of shared drifts: a fix lands in one copy and not the
other. This finds every run of at least MIN_LINES significant lines that
appears more than once in the production Rust and the developer scripts.
Lines are compared as the compiler would read them — indentation, blank
lines, comments and lines of bare punctuation (`}`, `);`) left out — so
reindenting a copy does not hide it. Tests are left out: a test repeats its
setup on purpose, so each reads on its own.

Standard library only. Usage:
  scripts/lint_duplication.py              # every tracked source file
  scripts/lint_duplication.py FILE ...     # just these
"""

from collections import defaultdict
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
# The shortest run reported. Shorter runs are mostly the language's own
# idioms (a `match` on the same enum, a builder chain), not a copied idea.
MIN_LINES = 10
SUFFIXES = (".rs", ".py")
# Test code, generated code and fixtures: not held to this.
LEFT_OUT = re.compile(
    r"(^|/)(tests?|test_support|fixtures|benches|examples|target[^/]*)/"
    r"|(^|/)tests?[_.][^/]*$|_tests?\.rs$|(^|/)test_[^/]*\.py$|_case\.py$"
)
# A line that says nothing by itself: punctuation, or an attribute a
# language asks for everywhere.
BARE = re.compile(r"^[\s{}()\[\];,.:]*$|^#\[(derive|must_use|test|cfg)\b")
COMMENT = {".rs": "//", ".py": "#"}
# Copies that cannot be shared, each with why. Keep it short: every entry is
# a place a fix has to be made twice.
ALLOWED = (
    (
        ("mcp/src/fsops/remove_moved.rs", "src/ops/file/remove_moved.rs"),
        "the MCP server is a separate crate that shares no code with the TUI",
    ),
)


def significant_lines(path, text):
    """(line number, normalised text) of every line that says something."""
    marker = COMMENT[path.suffix]
    lines = []
    for number, raw in enumerate(text.splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith(marker) or BARE.match(line):
            continue
        if path.suffix == ".py" and line.startswith(('"""', "'''")):
            continue
        lines.append((number, re.sub(r"\s+", " ", line)))
    return lines


def windows(files):
    """Every run of MIN_LINES lines, keyed by its text, with where it is."""
    seen = defaultdict(list)
    for path, lines in files.items():
        for start in range(len(lines) - MIN_LINES + 1):
            run = tuple(text for _, text in lines[start:start + MIN_LINES])
            seen[run].append((path, start))
    return seen


def grown(files, places):
    """How far past MIN_LINES every copy in `places` still matches the first."""
    first_path, first_start = places[0]
    first = files[first_path]
    length = MIN_LINES

    def matches(path, start):
        lines = files[path]
        return (start + length < len(lines) and first_start + length < len(first)
                and lines[start + length][1] == first[first_start + length][1])

    while all(matches(path, start) for path, start in places):
        length += 1
    return length


def allowed(copies):
    """Whether every copy is in one allowed pair of files."""
    files = {path.as_posix() for path, _, _ in copies}
    return any(files <= set(pair) for pair, _ in ALLOWED)


def duplicates(files):
    """Each copied block as [(path, first line, last line), ...], a block
    grown to the longest run its copies share."""
    covered = set()
    found = []
    for places in windows(files).values():
        if len(places) < 2 or all(place in covered for place in places):
            continue
        length = grown(files, places)
        for path, start in places:
            covered.update((path, start + step) for step in range(length - MIN_LINES + 1))
        copies = [(path, files[path][start][0], files[path][start + length - 1][0])
                  for path, start in places]
        if not allowed(copies):
            found.append(copies)
    return found


def tracked_sources():
    listed = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard",
         *(f"*{suffix}" for suffix in SUFFIXES)],
        cwd=ROOT, capture_output=True, text=True, check=True,
    )
    return [ROOT / name for name in listed.stdout.split()
            if not LEFT_OUT.search(name) and (ROOT / name).is_file()]


def load(paths):
    files = {}
    for path in paths:
        shown = path.relative_to(ROOT) if path.is_relative_to(ROOT) else path
        files[shown] = significant_lines(path, path.read_text(encoding="utf-8"))
    return files


def main(argv=None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    paths = [Path(name).resolve() for name in argv] or tracked_sources()
    found = duplicates(load(paths))
    for copies in found:
        lines = copies[0][2] - copies[0][1] + 1
        print(f"duplicated block ({lines} lines, {len(copies)} copies):", file=sys.stderr)
        for path, first, last in copies:
            print(f"  {path.as_posix()}:{first}-{last}", file=sys.stderr)
    if found:
        print(
            f"{len(found)} duplicated block(s) of {MIN_LINES}+ lines: share the code instead",
            file=sys.stderr,
        )
        return 1
    print(f"✓ duplication OK ({len(paths)} files, no block of {MIN_LINES}+ lines repeated)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
