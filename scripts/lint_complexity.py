"""Hold the developer scripts to the complexity budget Clippy holds the Rust to.

Clippy measures every Rust function (clippy.toml: cognitive complexity,
nesting, length). The Python scripts had no such gate, so this is one, with
no dependency beyond the standard library: `ast` gives each function's
McCabe cyclomatic complexity — one path, plus one for every branch — and the
depth of its nested blocks.

Usage: scripts/lint_complexity.py [file ...]   (default: every tracked *.py)
"""

import ast
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
# McCabe's own advice. Counted a little more strictly than the `mccabe`
# package counts (an `assert` and a comprehension's `if` are branches too).
MAX_COMPLEXITY = 10
# Blocks inside a function, the same budget as clippy.toml's
# excessive-nesting-threshold.
MAX_NESTING = 4

# Statements that open a nested block.
BLOCKS = (
    ast.If, ast.For, ast.AsyncFor, ast.While, ast.Try, ast.With, ast.AsyncWith, ast.Match,
)
# Nodes that add a path through a function; `BoolOp` adds one per extra operand.
BRANCHES = (
    ast.If, ast.IfExp, ast.For, ast.AsyncFor, ast.While, ast.ExceptHandler, ast.Assert,
    ast.comprehension, ast.match_case,
)
FUNCTIONS = (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda)


def own_nodes(function):
    """The nodes of `function`, without those of the functions nested in it:
    each is measured on its own, as Clippy measures a closure's parent."""
    pending = list(ast.iter_child_nodes(function))
    while pending:
        node = pending.pop()
        if isinstance(node, FUNCTIONS) or isinstance(node, ast.ClassDef):
            continue
        yield node
        pending.extend(ast.iter_child_nodes(node))


def complexity(function):
    """McCabe's number: 1, plus each branch, plus each extra `and`/`or` operand."""
    total = 1
    for node in own_nodes(function):
        if isinstance(node, BRANCHES):
            total += 1
        elif isinstance(node, ast.BoolOp):
            total += len(node.values) - 1
        if isinstance(node, ast.comprehension):
            total += len(node.ifs)
    return total


def nesting(node, depth=0):
    """The deepest block inside `node`, not counting nested functions."""
    deepest = depth
    for child in ast.iter_child_nodes(node):
        if isinstance(child, FUNCTIONS) or isinstance(child, ast.ClassDef):
            continue
        inner = depth + 1 if isinstance(child, BLOCKS) else depth
        deepest = max(deepest, nesting(child, inner))
    return deepest


def findings(path, source):
    """`path:line: name ...` for every function over budget in `source`."""
    tree = ast.parse(source, filename=str(path))
    for node in ast.walk(tree):
        if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            continue
        where = f"{path}:{node.lineno}: {node.name}"
        score = complexity(node)
        if score > MAX_COMPLEXITY:
            yield f"{where} has cyclomatic complexity {score} (max {MAX_COMPLEXITY})"
        depth = nesting(node)
        if depth > MAX_NESTING:
            yield f"{where} nests blocks {depth} deep (max {MAX_NESTING})"


def tracked_scripts():
    listed = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "*.py"], cwd=ROOT, capture_output=True, text=True, check=True
    )
    return [ROOT / name for name in listed.stdout.split()]


def main(argv=None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    paths = [Path(name) for name in argv] or tracked_scripts()
    problems = []
    for path in paths:
        source = path.read_text(encoding="utf-8")
        shown = path.relative_to(ROOT) if path.is_relative_to(ROOT) else path
        problems.extend(findings(shown, source))
    for problem in problems:
        print(problem, file=sys.stderr)
    if problems:
        print(f"{len(problems)} function(s) over the complexity budget", file=sys.stderr)
        return 1
    print(f"✓ complexity OK ({len(paths)} Python files)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
