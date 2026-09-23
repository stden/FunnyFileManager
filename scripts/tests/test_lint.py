"""Check every lint.py mode's steps, order, messages and exit status against fakes."""

import os
import shlex
import signal
import subprocess
import sys
import time
import unittest
from pathlib import Path

from sandbox import Sandbox


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
    # lint.py finds its programs through shell_path.py.
    for script in ("scripts/lint.py", "scripts/shell_path.py", *scripts):
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


def dies_of(signal_number, condition):
    """A fake's effect: when `condition` holds, die of `signal_number`.

    Python handles SIGINT and ignores SIGPIPE, so the fake restores their default actions first.
    """
    return f"""
        import signal
        if {condition}:
            signal.signal(signal.SIGINT, signal.SIG_DFL)
            signal.signal(signal.SIGPIPE, signal.SIG_DFL)
            os.kill(os.getpid(), {int(signal_number)})
    """


def waits_for_a_signal(started, condition):
    """A fake's effect: when `condition` holds, create `started` and sleep until a signal ends it."""
    return f"""
        import signal, time
        if {condition}:
            signal.signal(signal.SIGINT, signal.SIG_DFL)
            open({str(started)!r}, "w").close()
            time.sleep(60)
    """


def signal_group(sandbox, command, started, signal_number, env=None):
    """Run a copied script, send `signal_number` to its process group once `started` exists.

    Returns (returncode, stdout, stderr); the script and its fakes share the group,
    as a terminal's Ctrl+C or a cancelled CI step reaches them all.
    """
    script, *args = command
    process = subprocess.Popen(
        [sys.executable, sandbox.repo / script, *args],
        cwd=sandbox.repo, env=sandbox.env(env), stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, encoding="utf-8", process_group=0,
        # A test runner that ignores SIGINT would pass that on; Ctrl+C reaches a default handler.
        preexec_fn=lambda: signal.signal(signal.SIGINT, signal.SIG_DFL),
    )
    try:
        deadline = time.monotonic() + 30
        while not started.exists() and time.monotonic() < deadline:
            time.sleep(0.05)
        os.killpg(process.pid, signal_number)
        stdout, stderr = process.communicate(timeout=30)
    finally:
        process.kill()  # does nothing once the script has ended
    return process.returncode, stdout, stderr


def run_into_closed_pipe(sandbox, script, *args, env=None):
    """Run a copied script whose stdout is a pipe nobody reads any more, as after `| head -1`.

    Returns the finished process, with stderr as bytes.
    """
    read_end, write_end = os.pipe()
    os.close(read_end)
    try:
        return subprocess.run(
            [sys.executable, sandbox.repo / script, *args], cwd=sandbox.repo, env=sandbox.env(env),
            stdout=write_end, stderr=subprocess.PIPE, timeout=60, check=False,
        )
    finally:
        os.close(write_end)


def steps(sandbox):
    """Every recorded call as (name, args, directory relative to the checkout)."""
    return [(call.name, call.args, os.path.relpath(call.cwd, sandbox.repo)) for call in sandbox.calls()]


class ModeTests(unittest.TestCase):
    MODES = {
        (): (ALL, ALL_LINES),
        ("all",): (ALL, ALL_LINES),
        ("",): (ALL, ALL_LINES),
        ("fmt",): (FMT_CHECK, FMT_CHECK_LINES + ["✓ fmt OK"]),
        ("spelling",): ([SPELLING], [SPELLING_LINE, "✓ spelling OK"]),
        ("clippy",): (PREFLIGHT + CLIPPY_MATRIX, CLIPPY_LINES + ["✓ clippy OK"]),
        ("clippy-fast",): (PREFLIGHT + [CLIPPY_DEFAULT], [CLIPPY_DEFAULT_LINE, "✓ clippy OK (default features)"]),
        ("fix",): (PREFLIGHT + FMT_FIX + CLIPPY_MATRIX,
                   FMT_FIX_LINES + CLIPPY_LINES + ["✓ lint OK after format (fmt + clippy)"]),
        ("self-test",): (PREFLIGHT + SELF_TEST, ["✓ spelling self-test OK"]),
        ("architecture",): (PREFLIGHT + ARCHITECTURE, [ARCHITECTURE_LINE]),
    }

    def test_each_mode_runs_its_steps_in_order_and_reports_success(self):
        for args, (expected_steps, expected_lines) in self.MODES.items():
            with self.subTest(mode=args):
                sandbox = gate_sandbox(self)
                result = sandbox.run("scripts/lint.py", *args, env=checks_env(sandbox))
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout.splitlines(), expected_lines)
                self.assertEqual(result.stderr, "")
                self.assertEqual(steps(sandbox), expected_steps)

    def test_crates_without_a_manifest_are_skipped(self):
        root_only = [step for step in ALL if step[2] == ROOT]
        sandbox = gate_sandbox(self, crates=())
        result = sandbox.run("scripts/lint.py", env=checks_env(sandbox))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(steps(sandbox), root_only)
        self.assertNotIn("mcp/", result.stdout)
        self.assertNotIn("colorer-sys/", result.stdout)

    def test_it_works_from_the_checkout_root_whatever_the_callers_directory(self):
        sandbox = gate_sandbox(self)
        result = sandbox.run("scripts/lint.py", "architecture", cwd=sandbox.home, env=checks_env(sandbox))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(steps(sandbox), PREFLIGHT + ARCHITECTURE)

    def test_announcements_stay_in_order_with_the_commands_output(self):
        sandbox = gate_sandbox(self)
        sandbox.fake("cargo", stdout="cargo output\n")
        result = sandbox.run("scripts/lint.py", "fmt", env=checks_env(sandbox))
        self.assertEqual(result.stdout.splitlines(), [
            FMT_CHECK_LINES[0], "cargo output",
            FMT_CHECK_LINES[1], "cargo output",
            "cargo output",
            "✓ fmt OK",
        ])


class PythonTests(unittest.TestCase):
    def test_python_checks_default_to_the_interpreter_running_lint(self):
        record = "import os, sys\nwith open('interpreters', 'a') as log:\n    log.write(sys.executable + '\\n')\n"
        for extra in ({}, {"PYTHON": ""}):
            with self.subTest(env=extra):
                sandbox = gate_sandbox(self)
                sandbox.write("scripts/test_readability.py", record)
                sandbox.write("scripts/test_recording.py", record)
                result = sandbox.run("scripts/lint.py", "architecture", env=extra)
                self.assertEqual(result.returncode, 0, result.stderr)
                used = (sandbox.repo / "interpreters").read_text(encoding="utf-8").splitlines()
                self.assertEqual(used, [sys.executable, sys.executable])

    def test_the_preflight_runs_with_lints_own_interpreter_not_python(self):
        sandbox = gate_sandbox(self)
        result = sandbox.run("scripts/lint.py", "clippy-fast", env=checks_env(sandbox))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(sandbox.calls(CHECKS), [])
        self.assertEqual(steps(sandbox)[:1], PREFLIGHT)

    def test_a_python_set_by_the_caller_reaches_every_step_resolved(self):
        # A PYTHON the caller set reaches every step resolved; an unset one stays unset.
        cases = [({}, None), ({"PYTHON": ""}, sys.executable), ({"PYTHON": "/opt/python"}, "/opt/python")]
        for extra, seen in cases:
            with self.subTest(env=extra):
                sandbox = gate_sandbox(self)
                result = sandbox.run("scripts/lint.py", "clippy-fast", env=extra)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual({call.env.get("PYTHON") for call in sandbox.calls()}, {seen})


class EnvironmentTests(unittest.TestCase):
    def test_every_step_sees_its_own_directory_as_pwd(self):
        sandbox = gate_sandbox(self)
        result = sandbox.run("scripts/lint.py", env=checks_env(sandbox, PWD=str(sandbox.home)))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(sandbox.calls()), len(ALL))
        for call in sandbox.calls():
            self.assertEqual(call.env["PWD"], call.cwd, call.args)

    def test_the_marks_are_utf8_whatever_the_output_encoding(self):
        # A Windows pipe (cp1252, cp866) or a Latin-1 locale cannot encode → or ✓.
        for encoding in ("cp1252", "cp866", "latin-1", "ascii"):
            with self.subTest(encoding=encoding):
                sandbox = gate_sandbox(self)
                result = sandbox.run(
                    "scripts/lint.py", "fmt",
                    env=checks_env(sandbox, PYTHONUTF8="0", PYTHONIOENCODING=encoding),
                )
                self.assertEqual((result.returncode, result.stderr), (0, ""))
                self.assertEqual(result.stdout.splitlines(), FMT_CHECK_LINES + ["✓ fmt OK"])


class PathTests(unittest.TestCase):
    def test_cargo_and_typos_behind_a_tilde_path_entry_are_found_as_bash_finds_them(self):
        sandbox = gate_sandbox(self)
        path = sandbox.tilde_path("cargo", "typos")
        result = sandbox.run("scripts/lint.py", env=checks_env(sandbox, PATH=path))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(steps(sandbox), ALL)

    def test_a_relative_path_entry_is_taken_from_each_steps_own_directory(self):
        # As after bash's `cd "$ROOT"` and `(cd mcp && cargo ...)`, whatever the caller's directory.
        sandbox = gate_sandbox(self)
        for crate, name in ((sandbox.home, "caller"), (sandbox.repo, "root"),
                            (sandbox.repo / "mcp", "mcp"), (sandbox.repo / "colorer-sys", "colorer")):
            sandbox.fake(f"cargo in {name}", path=crate / "tools/cargo")
        env = {"PATH": os.pathsep.join(["tools", str(sandbox.bin)])}
        result = sandbox.run("scripts/lint.py", "fmt", cwd=sandbox.home, env=env)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([call.name for call in sandbox.calls()],
                         ["cargo in root", "cargo in mcp", "cargo in colorer"])


class FailureTests(unittest.TestCase):
    def test_the_first_failing_step_ends_lint_with_its_status(self):
        sandbox = gate_sandbox(self)
        sandbox.fake("cargo", effect="""
            if "--all-features" in args:
                sys.exit(7)
        """)
        result = sandbox.run("scripts/lint.py", env=checks_env(sandbox))
        self.assertEqual((result.returncode, result.stderr), (7, ""))
        self.assertEqual(steps(sandbox), PREFLIGHT + FMT_CHECK + [SPELLING] + CLIPPY_MATRIX[:2])
        self.assertNotIn("✓", result.stdout)

    def test_a_failing_python_check_ends_lint_with_its_status(self):
        sandbox = gate_sandbox(self)
        sandbox.fake(CHECKS, effect="""
            if args == ["scripts/test_readability.py"]:
                sys.exit(3)
        """)
        result = sandbox.run("scripts/lint.py", "architecture", env=checks_env(sandbox))
        self.assertEqual(result.returncode, 3)
        self.assertEqual(steps(sandbox), PREFLIGHT + ARCHITECTURE[:2])

    def test_a_failing_preflight_ends_lint_before_any_check(self):
        sandbox = gate_sandbox(self)
        fake_preflight(sandbox, status=5)
        for mode in ("all", "clippy", "clippy-fast", "fix", "self-test", "architecture"):
            with self.subTest(mode=mode):
                sandbox.log.unlink(missing_ok=True)
                result = sandbox.run("scripts/lint.py", mode, env=checks_env(sandbox))
                self.assertEqual((result.returncode, result.stdout), (5, ""))
                self.assertEqual(steps(sandbox), PREFLIGHT)

    def test_a_command_that_cannot_start_ends_lint_with_127_or_126(self):
        missing = {"PYTHON": str(Path("/nonexistent/python"))}
        cases = {
            "cargo not found": ({}, "cargo", 127, "cargo: No such file or directory"),
            "cargo not executable": ({}, "cargo", 126, "cargo: Permission denied"),
            "no such PYTHON": (missing, None, 127, "/nonexistent/python: No such file or directory"),
        }
        for case, (env, broken, status, reason) in cases.items():
            with self.subTest(case=case):
                sandbox = gate_sandbox(self)
                if broken and status == 127:
                    (sandbox.bin / broken).unlink()
                elif broken:
                    (sandbox.bin / broken).chmod(0o644)
                # Only the sandbox's bin directory: no system cargo can stand in.
                env = {"PATH": str(sandbox.bin), "PYTHON": str(sandbox.bin / CHECKS), **env}
                result = sandbox.run("scripts/lint.py", "clippy-fast" if broken else "architecture", env=env)
                self.assertEqual((result.returncode, result.stderr), (status, f"{reason}\n"))
                self.assertNotIn("✓", result.stdout)

    def test_a_python_without_a_shebang_line_is_refused_with_126(self):
        # Run through sh instead, an empty or a broken file would pass the checks unseen.
        sandbox = gate_sandbox(self)
        ran = sandbox.root / "ran"
        plain = sandbox.root / "plain python"
        plain.write_text(f'echo "$@" >> "{ran}"\n', encoding="utf-8")
        plain.chmod(0o755)
        result = sandbox.run("scripts/lint.py", "architecture", env={"PYTHON": str(plain)})
        self.assertEqual((result.returncode, result.stderr), (126, f"{plain}: Exec format error\n"))
        self.assertFalse(ran.exists())


class SpellingTests(unittest.TestCase):
    def test_a_missing_typos_fails_the_gate_with_127_and_the_install_command(self):
        cases = {
            "all": (PREFLIGHT + FMT_CHECK, FMT_CHECK_LINES),
            "spelling": ([], []),
            "self-test": (PREFLIGHT, []),
        }
        for mode, (expected_steps, expected_lines) in cases.items():
            with self.subTest(mode=mode):
                sandbox = gate_sandbox(self)
                (sandbox.bin / "typos").unlink()
                result = sandbox.run("scripts/lint.py", mode, env=checks_env(sandbox))
                self.assertEqual((result.returncode, result.stderr), (127, TYPOS_MISSING))
                self.assertEqual(result.stdout.splitlines(), expected_lines)
                self.assertEqual(steps(sandbox), expected_steps)

    def test_a_misspelling_in_the_tree_ends_lint_with_typos_status(self):
        for mode in ("all", "spelling"):
            with self.subTest(mode=mode):
                sandbox = gate_sandbox(self)
                sandbox.fake("typos", status=2, stdout="error: a misspelling\n")
                result = sandbox.run("scripts/lint.py", mode, env=checks_env(sandbox))
                self.assertEqual(result.returncode, 2)
                self.assertTrue(result.stdout.endswith(f"{SPELLING_LINE}\nerror: a misspelling\n"), result.stdout)
                self.assertEqual(steps(sandbox)[-1], SPELLING)

    def test_the_self_test_feeds_a_misspelling_then_the_project_words(self):
        sandbox = gate_sandbox(self)
        fed = sandbox.root / "fed"
        sandbox.fake("typos", effect=f"""
            if args == ["-"]:
                text = sys.stdin.read()
                with open({str(fed)!r}, "a") as log:
                    log.write(text)
                sys.exit(2 if "teh" in text else 0)  # spellchecker:disable-line
        """)
        result = sandbox.run("scripts/lint.py", "self-test", env=checks_env(sandbox))
        self.assertEqual((result.returncode, result.stderr), (0, ""))
        self.assertEqual(fed.read_text(encoding="utf-8"), "teh\nRatatui draws a Detai&led menu\n")  # spellchecker:disable-line

    def test_the_self_test_hides_the_tree_check_and_the_rejection(self):
        sandbox = gate_sandbox(self)
        sandbox.fake("typos", stdout="typos output\n", effect=REJECTS_THE_MISSPELLING)
        result = sandbox.run("scripts/lint.py", "self-test", env=checks_env(sandbox))
        self.assertEqual(result.returncode, 0, result.stderr)
        # Only the check of the project words shows its output.
        self.assertEqual(result.stdout.splitlines(), ["typos output", "✓ spelling self-test OK"])

    def test_a_typos_that_accepts_the_misspelling_fails_the_self_test(self):
        sandbox = gate_sandbox(self)
        sandbox.fake("typos")
        result = sandbox.run("scripts/lint.py", "self-test", env=checks_env(sandbox))
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stderr, "spelling self-test: typos accepted a known misspelling\n")
        self.assertEqual(steps(sandbox), PREFLIGHT + SPELLING_SELF_TEST[:2])

    def test_a_typos_that_rejects_the_project_words_fails_the_self_test(self):
        sandbox = gate_sandbox(self)
        sandbox.fake("typos", effect="""
            if args == ["-"]:
                print("rejected:", sys.stdin.read().strip())
                sys.exit(2)
        """)
        result = sandbox.run("scripts/lint.py", "self-test", env=checks_env(sandbox))
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stdout, "rejected: Ratatui draws a Detai&led menu\n")
        self.assertEqual(steps(sandbox), PREFLIGHT + SPELLING_SELF_TEST)

    def test_a_misspelling_in_the_tree_fails_the_self_test_first(self):
        sandbox = gate_sandbox(self)
        sandbox.fake("typos", status=2)
        result = sandbox.run("scripts/lint.py", "self-test", env=checks_env(sandbox))
        self.assertEqual(result.returncode, 2)
        self.assertEqual(steps(sandbox), PREFLIGHT + [SPELLING])


class SignalTests(unittest.TestCase):
    def test_a_step_killed_by_signal_n_ends_lint_with_128_plus_n(self):
        # Named on stderr with the step, except for Ctrl-C and a closed pipe.
        clippy = shlex.join([CLIPPY_DEFAULT[0], *CLIPPY_DEFAULT[1]])
        reports = {
            signal.SIGTERM: f"Terminated  {clippy}\n", signal.SIGKILL: f"Killed  {clippy}\n",
            signal.SIGINT: "", signal.SIGPIPE: "",
        }
        for number, report in reports.items():
            with self.subTest(signal=number.name):
                sandbox = gate_sandbox(self)
                sandbox.fake("cargo", effect=dies_of(number, 'args[0] == "clippy"'))
                result = sandbox.run("scripts/lint.py", "clippy-fast", env=checks_env(sandbox))
                self.assertEqual((result.returncode, result.stderr), (128 + number, report))

    def test_a_reader_that_went_away_ends_lint_silently_by_sigpipe(self):
        # As `lint.sh fmt | head -1` ended: status 141 in the shell, no traceback.
        sandbox = gate_sandbox(self)
        result = run_into_closed_pipe(sandbox, "scripts/lint.py", "fmt")
        self.assertEqual((result.returncode, result.stderr), (-signal.SIGPIPE, b""))
        self.assertEqual(steps(sandbox), [])

    def test_ctrl_c_ends_lint_by_sigint_without_a_traceback(self):
        sandbox = gate_sandbox(self)
        started = sandbox.root / "clippy started"
        sandbox.fake("cargo", effect=waits_for_a_signal(started, 'args[0] == "clippy"'))
        outcome = signal_group(sandbox, ["scripts/lint.py", "clippy-fast"], started, signal.SIGINT,
                               checks_env(sandbox))
        self.assertEqual(outcome, (-signal.SIGINT, CLIPPY_DEFAULT_LINE + "\n", ""))

    def test_ctrl_c_that_the_step_handles_lets_the_gate_go_on(self):
        # Ctrl-C reaches the whole foreground group: lint and the step. lint
        # waits for the step, which cleans up and exits 0.
        sandbox = gate_sandbox(self)
        cleaned = sandbox.root / "cleaned up"
        sandbox.fake("cargo", effect=f"""
            import signal, time
            def clean_up(*_):
                time.sleep(0.5)
                open({str(cleaned)!r}, "w").close()
                sys.exit(0)
            if args[0] == "clippy":
                signal.signal(signal.SIGINT, clean_up)
                os.kill(os.getppid(), signal.SIGINT)
                os.kill(os.getpid(), signal.SIGINT)
                time.sleep(10)
        """)
        result = sandbox.run("scripts/lint.py", "clippy-fast", env=checks_env(sandbox))
        self.assertEqual((result.returncode, result.stderr), (0, ""))
        self.assertTrue(cleaned.exists())
        self.assertEqual(result.stdout.splitlines(), [CLIPPY_DEFAULT_LINE, "✓ clippy OK (default features)"])


class ArgumentTests(unittest.TestCase):
    def test_help_prints_every_mode_without_running_anything(self):
        for args in (("help",), ("-h",), ("--help",), ("-h", "fmt")):
            with self.subTest(args=args):
                sandbox = gate_sandbox(self)
                result = sandbox.run("scripts/lint.py", *args, env=checks_env(sandbox))
                self.assertEqual((result.returncode, result.stderr), (0, ""))
                lines = result.stdout.splitlines()
                self.assertEqual(lines[0], "Run the Rust linter gate used by CI, pre-commit, and `./ffm.py lint`.")
                self.assertEqual(lines[-1], "Exit non-zero on the first failure. Matches `.github/workflows/tests.yml` lint job.")
                for mode in MODE_NAMES[1:]:
                    self.assertTrue(any(line.startswith(f"  scripts/lint.py {mode} ") for line in lines), mode)
                self.assertEqual(sandbox.calls(), [])

    def test_an_unknown_mode_exits_2_naming_the_valid_ones_before_the_preflight(self):
        # "--" and "-x" are modes too; neither is a valid one.
        for args in (("bogus",), ("--",), ("--", "fmt"), ("-x",)):
            with self.subTest(args=args):
                sandbox = gate_sandbox(self)
                result = sandbox.run("scripts/lint.py", *args, env=checks_env(sandbox))
                expected = f"unknown mode: {args[0]} (try: {'|'.join(MODE_NAMES)}|help)\n"
                self.assertEqual((result.returncode, result.stdout, result.stderr), (2, "", expected))
                self.assertEqual(sandbox.calls(), [])

    def test_words_after_the_mode_are_ignored(self):
        for args in (("fmt", "extra"), ("fmt", "-h"), ("fmt", "--", "bogus")):
            with self.subTest(args=args):
                sandbox = gate_sandbox(self)
                result = sandbox.run("scripts/lint.py", *args, env=checks_env(sandbox))
                self.assertEqual((result.returncode, result.stderr), (0, ""))
                self.assertEqual(result.stdout.splitlines(), FMT_CHECK_LINES + ["✓ fmt OK"])
                self.assertEqual(steps(sandbox), FMT_CHECK)


class ScriptFileTests(unittest.TestCase):
    def test_lint_is_an_executable_python3_script(self):
        path = Sandbox(self).copy("scripts/lint.py")
        self.assertTrue(path.read_text(encoding="utf-8").startswith("#!/usr/bin/env python3\n"))
        self.assertTrue(os.access(path, os.X_OK))


if __name__ == "__main__":
    unittest.main()
