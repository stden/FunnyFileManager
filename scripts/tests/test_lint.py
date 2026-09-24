"""Check every lint.py mode's steps, order, messages and exit status against fakes.

Spelling, failures and signals have test_lint_*.py files of their own, and
lint_case.py holds the checkout and the expected steps that they share.
"""

import os
import sys
import unittest

from lint_case import (
    ALL, ALL_LINES, ARCHITECTURE, ARCHITECTURE_LINE, CHECKS, CLIPPY_DEFAULT, CLIPPY_DEFAULT_LINE,
    CLIPPY_LINES, CLIPPY_MATRIX, FMT_CHECK, FMT_CHECK_LINES, FMT_FIX, FMT_FIX_LINES, MODE_NAMES, PREFLIGHT,
    ROOT, SELF_TEST, SPELLING, SPELLING_LINE, checks_env, gate_sandbox, steps,
)
from sandbox import Sandbox


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
