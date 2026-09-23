#!/usr/bin/env python3
"""Run the Rust linter gate used by CI, pre-commit, and `./ffm.py lint`.

Usage:
  scripts/lint.py              # fmt + spelling + clippy matrix + architecture contracts
  scripts/lint.py fmt          # format check only
  scripts/lint.py spelling     # typos over every text file (typos.toml)
  scripts/lint.py clippy       # clippy and architecture (all targets, all features, fallback)
  scripts/lint.py clippy-fast  # clippy all targets (default features)
  scripts/lint.py fix          # auto-format, then full clippy gate
  scripts/lint.py self-test    # spell check, script unit tests, readability limits, lint policy
  scripts/lint.py architecture # dependency ownership, layering, unsafe allowlist, file limits

Exit non-zero on the first failure. Matches `.github/workflows/tests.yml` lint job.
"""

import errno
import os
from pathlib import Path
import shlex
import signal
import subprocess
import sys

# shell_path.py, beside this file, finds programs as bash does. Explicitly
# first on sys.path: `python3 -P` or PYTHONSAFEPATH=1 leave this file's
# directory out, and no other `shell_path` module may stand in.
sys.path.insert(0, str(Path(__file__).resolve().parent))
import shell_path  # noqa: E402


ROOT = Path(__file__).resolve().parents[1]
PREFLIGHT = ROOT / "scripts/cargo_preflight.py"
HELP_WORDS = ("-h", "--help", "help")
# Modes that build nothing, so they skip the compiler-wrapper preflight.
PREFLIGHT_FREE = ("fmt", "spelling")
# Crates besides the root one that the gate covers when their manifest exists.
EXTRA_CRATES = ("mcp", "colorer-sys")
TYPOS_MISSING = "typos is not installed — run: cargo install typos-cli --locked"
# The status of a step that cannot start: a file it needs is missing, or another reason.
NOT_FOUND = 127
NOT_EXECUTABLE = 126
# A step killed by one of these is not reported: Ctrl-C, or a reader that went
# away. Windows has no SIGPIPE.
QUIET_SIGNALS = (signal.SIGINT, getattr(signal, "SIGPIPE", None))


# The helpers from here to use_utf8_output() also serve pre_commit_hook.py and
# cargo_preflight.py, so the three run a step and end the one same way.
# TODO(DRY): ffm.py and install_mcp.py carry their own copy of this step
# runner. Move them into one shared module once every script port has landed.
def run_step(command, cwd=ROOT, **options):
    """Run one step to the end and return its exit status.

    - The step runs in `cwd`, with PWD naming it for tools that read PWD.
    - A bare program name is looked up as bash would from `cwd` (shell_path).
    - Ctrl-C reaches the step as well, and the step decides what it means:
      the script waits for it. If the step dies of it, KeyboardInterrupt is
      raised; if the step exits instead, its status counts as usual.
    - A step that cannot start gets 127 when a file it needs is missing, 126
      otherwise, and the reason is printed on stderr.
    """
    executable = shell_path.resolve(str(command[0]), cwd)
    interrupted = False

    def note_interrupt(_signal_number, _frame):
        nonlocal interrupted
        interrupted = True

    handler = signal.getsignal(signal.SIGINT)
    # A Ctrl-C ignored on entry stays ignored, here and in the step.
    if handler is not signal.SIG_IGN:
        signal.signal(signal.SIGINT, note_interrupt)
    try:
        environment = {**os.environ, "PWD": str(cwd)}
        returncode = subprocess.run(
            command, executable=executable, cwd=cwd, env=environment, check=False, **options
        ).returncode
    except OSError as error:
        print(f"{error.filename}: {error.strerror}", file=sys.stderr)
        return NOT_FOUND if error.errno == errno.ENOENT else NOT_EXECUTABLE
    finally:
        signal.signal(signal.SIGINT, handler)
    if interrupted and returncode == -signal.SIGINT:
        raise KeyboardInterrupt
    return exit_status(returncode, command)


def exit_status(returncode, command):
    """A finished step's status: its own, or 128 + N after signal N.

    The signal is named on stderr with the step, unless it is in QUIET_SIGNALS.
    """
    if returncode >= 0:
        return returncode
    signal_number = -returncode
    if signal_number not in QUIET_SIGNALS:
        print(f"{signal.strsignal(signal_number)}  {shlex.join(map(str, command))}", file=sys.stderr)
    return 128 + signal_number


def run(command, cwd=ROOT, **options):
    """Run a step that must pass: a failure ends the script with the step's status."""
    status = run_step(command, cwd, **options)
    if status:
        sys.exit(status)


def exit_with(main):
    """Exit with main()'s status. Ctrl-C ends the process by SIGINT, without a
    traceback, so that a calling script sees the interrupt and stops as well.

    A reader that goes away, as `| head` does, ends it by SIGPIPE, silently,
    as it ended the bash scripts: CPython starts with SIGPIPE ignored.
    """
    if hasattr(signal, "SIGPIPE"):  # Windows has none
        signal.signal(signal.SIGPIPE, signal.SIG_DFL)
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        signal.signal(signal.SIGINT, signal.SIG_DFL)
        os.kill(os.getpid(), signal.SIGINT)


def use_utf8_output():
    """UTF-8 whatever the locale, for the → and ✓ marks and the em dash.

    A path that is not UTF-8 goes out as its own bytes. stdout is line
    buffered, so each line is out before the next step writes its own.
    """
    if sys.stdout:
        sys.stdout.reconfigure(encoding="utf-8", errors="surrogateescape", line_buffering=True)
    if sys.stderr:
        sys.stderr.reconfigure(encoding="utf-8", errors="surrogateescape")


def python():
    """The interpreter for the Python checks: $PYTHON, else the one running lint."""
    return os.environ.get("PYTHON") or sys.executable


def has_crate(crate):
    return (ROOT / crate / "Cargo.toml").is_file()


def format_crates(check):
    """Run `cargo fmt --all` in every crate; with `check`, only verify the format."""
    command = ["cargo", "fmt", "--all", *(["--", "--check"] if check else [])]
    print("→", *command)
    run(command)
    if has_crate("mcp"):
        print("→", *command, "(mcp/)")
        run(command, ROOT / "mcp")
    if has_crate("colorer-sys"):
        run(command, ROOT / "colorer-sys")


def require_typos():
    """Stop with 127 unless typos is on PATH: a missing checker fails the gate
    instead of skipping it."""
    if not shell_path.find("typos", cwd=ROOT):
        print(TYPOS_MISSING, file=sys.stderr)
        sys.exit(NOT_FOUND)


# Misspellings in docs, comments, strings and identifiers; typos.toml holds the
# project words.
def run_spelling():
    require_typos()
    print("→ typos")
    run(["typos"])


def self_test_spelling():
    """The spell check must reject a real misspelling and accept the project words."""
    require_typos()
    run(["typos"], stdout=subprocess.DEVNULL)
    misspelling = b"teh\n"  # spellchecker:disable-line
    if run_step(["typos", "-"], input=misspelling, stdout=subprocess.DEVNULL) == 0:
        print("spelling self-test: typos accepted a known misspelling", file=sys.stderr)
        sys.exit(1)
    run(["typos", "-"], input=b"Ratatui draws a Detai&led menu\n")
    print("✓ spelling self-test OK")


# Lint levels in Cargo.toml and thresholds in clippy.toml apply to every target.
def run_clippy(*options):
    # Check production, tests, examples and benchmarks with the same limits.
    print(f"→ cargo clippy --all-targets {' '.join(options)} -- -D warnings")
    run(["cargo", "clippy", "--all-targets", *options, "--", "-D", "warnings"])


def run_clippy_matrix():
    run_clippy()
    # Combined features include helper-gated BDD targets and feature interactions.
    run_clippy("--all-features")
    # Compile the built-in highlighter without the optional Colorer adapter.
    run_clippy("--no-default-features", "--features", "test-helpers")
    for crate in EXTRA_CRATES:
        if has_crate(crate):
            print(f"→ cargo clippy --all-targets -- -D warnings ({crate}/)")
            run(["cargo", "clippy", "--all-targets", "--", "-D", "warnings"], ROOT / crate)
    run_architecture()


def run_architecture():
    print("→ architecture contracts (ownership, dependencies, unsafe allowlist, test separation, file limits)")
    run([
        "cargo", "test", "--test", "module_boundaries_test",
        "--test", "unsafe_boundary_test", "--test", "file_line_limit_test",
    ])
    run([python(), "scripts/test_readability.py"])
    run([python(), "scripts/test_recording.py"])


def lint_all():
    format_crates(check=True)
    run_spelling()
    run_clippy_matrix()
    print("✓ lint OK (fmt + spelling + clippy)")


def lint_fmt():
    format_crates(check=True)
    print("✓ fmt OK")


def lint_spelling():
    run_spelling()
    print("✓ spelling OK")


def lint_clippy():
    run_clippy_matrix()
    print("✓ clippy OK")


def lint_clippy_fast():
    run_clippy()
    print("✓ clippy OK (default features)")


def lint_fix():
    format_crates(check=False)
    run_clippy_matrix()
    print("✓ lint OK after format (fmt + clippy)")


def self_test():
    self_test_spelling()
    # The developer scripts' unit tests (scripts/tests): fake commands, no Cargo.
    run([python(), "-m", "unittest", "discover", "-s", "scripts/tests"])
    run([python(), "scripts/test_readability.py"])
    # Runs Cargo on throwaway crates, hence the wrapper preflight.
    run([python(), "scripts/test_lint_policy.py"])


# In the order the unknown-mode message lists them.
MODES = {
    "all": lint_all,
    "fmt": lint_fmt,
    "spelling": lint_spelling,
    "clippy": lint_clippy,
    "clippy-fast": lint_clippy_fast,
    "fix": lint_fix,
    "self-test": self_test,
    "architecture": run_architecture,
}


def main(argv=None) -> int:
    """Run the mode the first argument names; later arguments are ignored."""
    argv = sys.argv[1:] if argv is None else argv
    use_utf8_output()
    mode = argv[0] if argv and argv[0] else "all"
    if mode in HELP_WORDS:
        print(__doc__.strip())
        return 0
    if mode not in MODES:
        print(f"unknown mode: {mode} (try: {'|'.join([*MODES, 'help'])})", file=sys.stderr)
        return 2
    if "PYTHON" in os.environ:
        # A PYTHON the caller set reaches every step resolved: empty names
        # the interpreter running lint.
        os.environ["PYTHON"] = python()
    if mode not in PREFLIGHT_FREE:
        # Fail in seconds, not minutes, when the compiler wrapper has stalled.
        run([sys.executable, PREFLIGHT])
    MODES[mode]()
    return 0


if __name__ == "__main__":
    exit_with(main)
