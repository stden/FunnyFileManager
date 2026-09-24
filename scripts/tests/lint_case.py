"""The sandbox checkout and the expected steps that the tests of scripts/lint.py
and of the pre-commit hook share.
"""

import os

from sandbox import Sandbox
from script_files import LINT


# The fake that stands in for $PYTHON, so no real check or test suite starts.
CHECKS = "python-checks"
# The recording stand-in for scripts/cargo_preflight.py, tested on its own.
PREFLIGHT_FAKE = "cargo_preflight"
ROOT = "."
PREFLIGHT = [(PREFLIGHT_FAKE, [], ROOT)]
FMT_CHECK = [("cargo", ["fmt", "--all", "--", "--check"], crate) for crate in (ROOT, "mcp", "colorer-sys")]
FMT_FIX = [("cargo", ["fmt", "--all"], crate) for crate in (ROOT, "mcp", "colorer-sys")]
CLIPPY_DEFAULT = ("cargo", ["clippy", "--all-targets", "--", "-D", "warnings"], ROOT)
ARCHITECTURE = [
    ("cargo", ["test", "--test", "module_boundaries_test", "--test", "unsafe_boundary_test",
               "--test", "file_line_limit_test"], ROOT),
    (CHECKS, ["scripts/test_readability.py"], ROOT),
    (CHECKS, ["scripts/test_recording.py"], ROOT),
]
CLIPPY_MATRIX = [
    CLIPPY_DEFAULT,
    ("cargo", ["clippy", "--all-targets", "--all-features", "--", "-D", "warnings"], ROOT),
    ("cargo", ["clippy", "--all-targets", "--no-default-features", "--features", "test-helpers",
               "--", "-D", "warnings"], ROOT),
    ("cargo", ["clippy", "--all-targets", "--", "-D", "warnings"], "mcp"),
    ("cargo", ["clippy", "--all-targets", "--", "-D", "warnings"], "colorer-sys"),
    *ARCHITECTURE,
]
SPELLING = ("typos", [], ROOT)
# The tree, then a known misspelling and a line of project words on stdin.
SPELLING_SELF_TEST = [SPELLING, ("typos", ["-"], ROOT), ("typos", ["-"], ROOT)]
SELF_TEST = [
    *SPELLING_SELF_TEST,
    (CHECKS, ["-m", "unittest", "discover", "-s", "scripts/tests"], ROOT),
    (CHECKS, ["scripts/test_readability.py"], ROOT),
    (CHECKS, ["scripts/test_lint_policy.py"], ROOT),
]

FMT_CHECK_LINES = ["→ cargo fmt --all -- --check", "→ cargo fmt --all -- --check (mcp/)"]
SPELLING_LINE = "→ typos"
ALL = PREFLIGHT + FMT_CHECK + [SPELLING] + CLIPPY_MATRIX
FMT_FIX_LINES = ["→ cargo fmt --all", "→ cargo fmt --all (mcp/)"]
CLIPPY_DEFAULT_LINE = "→ cargo clippy --all-targets  -- -D warnings"
ARCHITECTURE_LINE = (
    "→ architecture contracts (ownership, dependencies, unsafe allowlist, test separation, file limits)"
)
CLIPPY_LINES = [
    CLIPPY_DEFAULT_LINE,
    "→ cargo clippy --all-targets --all-features -- -D warnings",
    "→ cargo clippy --all-targets --no-default-features --features test-helpers -- -D warnings",
    "→ cargo clippy --all-targets -- -D warnings (mcp/)",
    "→ cargo clippy --all-targets -- -D warnings (colorer-sys/)",
    ARCHITECTURE_LINE,
]
ALL_LINES = FMT_CHECK_LINES + [SPELLING_LINE] + CLIPPY_LINES + ["✓ lint OK (fmt + spelling + clippy)"]
TYPOS_MISSING = "typos is not installed — run: cargo install typos-cli --locked\n"
MODE_NAMES = ("all", "fmt", "spelling", "clippy", "clippy-fast", "fix", "self-test", "architecture")
# A typos that rejects a known misspelling on stdin, as the real one does.
REJECTS_THE_MISSPELLING = """
    if args == ["-"] and "teh" in sys.stdin.read():  # spellchecker:disable-line
        print("error: `teh` should be `the`")  # spellchecker:disable-line
        sys.exit(2)
"""


def gate_sandbox(test, *scripts, crates=("mcp", "colorer-sys")):
    """A checkout with lint.py, a stand-in preflight, `scripts` and the extra `crates`.

    The preflight, Cargo and the $PYTHON fake succeed silently, and typos
    rejects only the misspelling, unless a test replaces them.
    """
    sandbox = Sandbox(test)
    # lint.py, the modules it imports, and shell_path.py, through which it
    # finds its programs.
    for script in (*LINT, *scripts):
        sandbox.copy(script)
    for crate in crates:
        sandbox.write(f"{crate}/Cargo.toml")
    fake_preflight(sandbox)
    sandbox.fake("cargo")
    sandbox.fake("typos", effect=REJECTS_THE_MISSPELLING)
    sandbox.fake(CHECKS)
    return sandbox


def fake_preflight(sandbox, status=0):
    """A recording scripts/cargo_preflight.py that exits with `status`.

    It lacks the executable bit, so only `<interpreter> <file>` can run it.
    """
    sandbox.fake(PREFLIGHT_FAKE, path=sandbox.repo / "scripts/cargo_preflight.py", status=status).chmod(0o644)


def checks_env(sandbox, **extra):
    """The environment that points $PYTHON at the sandbox's fake."""
    return {"PYTHON": str(sandbox.bin / CHECKS), **extra}


def steps(sandbox):
    """Every recorded call as (name, args, directory relative to the checkout)."""
    return [(call.name, call.args, os.path.relpath(call.cwd, sandbox.repo)) for call in sandbox.calls()]
