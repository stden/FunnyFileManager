"""Check the git pre-commit hook: format the root and mcp/ crates, then lint's clippy gate."""

import shutil
import signal
import sys
import unittest

from sandbox import SYSTEM_PATH
from test_lint import (
    CLIPPY_DEFAULT_LINE, CLIPPY_LINES, CLIPPY_MATRIX, PREFLIGHT,
    checks_env, dies_of, fake_preflight, gate_sandbox, run_into_closed_pipe, signal_group, steps,
    waits_for_a_signal,
)


HOOK = "scripts/pre_commit_hook.py"
FORMAT = [("cargo", ["fmt", "--all"], "."), ("cargo", ["fmt", "--all"], "mcp")]
FORMAT_LINES = ["→ Running cargo fmt (auto-format)...", "→ Formatting mcp/ crate..."]
PASSED = FORMAT + PREFLIGHT + CLIPPY_MATRIX
PASSED_LINES = [
    *FORMAT_LINES,
    "→ Running Rust linter gate (clippy)...",
    *CLIPPY_LINES,
    "✓ clippy OK",
    "→ Pre-commit checks passed.",
    "   If fmt rewrote files, stage them (git add -u) before committing again.",
]


class HookTestCase(unittest.TestCase):
    def setUp(self):
        self.sandbox = gate_sandbox(self, HOOK)

    def hook(self, **env):
        return self.sandbox.run(HOOK, env=checks_env(self.sandbox, **env))

    def make_repository_with_the_hook(self):
        """Commit the sandbox checkout to a git repository whose pre-commit hook links to the port."""
        sandbox = self.sandbox
        sandbox.use_real("git")
        sandbox.git("init", "--quiet", "--initial-branch=main", sandbox.repo)
        sandbox.git("-C", sandbox.repo, "add", "-A")
        sandbox.git("-C", sandbox.repo, "commit", "--quiet", "--no-verify", "--message=scripts")
        (sandbox.repo / ".git" / "hooks" / "pre-commit").symlink_to("../../scripts/pre_commit_hook.py")

    def commit_a_change(self, tree, **env):
        """`git commit` one new file in the working tree `tree`, which runs the hook."""
        (tree / "change").write_text("change\n", encoding="utf-8")
        self.sandbox.git("-C", tree, "add", "change")
        git = shutil.which("git", path=":".join(SYSTEM_PATH))
        return self.sandbox.execute([git, "commit", "--quiet", "--message=change"], cwd=tree,
                                    env=checks_env(self.sandbox, **env))


class PassingTests(HookTestCase):
    def test_formats_the_root_and_mcp_crates_then_runs_the_clippy_gate(self):
        result = self.hook()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines(), PASSED_LINES)
        self.assertEqual(steps(self.sandbox), PASSED)

    def test_never_stages_files(self):
        self.hook()
        self.assertEqual(self.sandbox.calls("git"), [])

    def test_without_an_mcp_crate_only_the_root_is_formatted(self):
        (self.sandbox.repo / "mcp" / "Cargo.toml").unlink()
        result = self.hook()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("→ Formatting mcp/ crate...", result.stdout)
        self.assertEqual(steps(self.sandbox)[:2], FORMAT[:1] + PREFLIGHT)

    def test_skip_clippy_1_only_formats(self):
        result = self.hook(SKIP_CLIPPY="1")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines(), [*FORMAT_LINES, "→ Skipping clippy (SKIP_CLIPPY=1)."])
        self.assertEqual(steps(self.sandbox), FORMAT)

    def test_any_other_skip_clippy_value_still_runs_clippy(self):
        for value in ("", "0", "true", "yes"):
            with self.subTest(SKIP_CLIPPY=value):
                self.sandbox.log.unlink(missing_ok=True)
                result = self.hook(SKIP_CLIPPY=value)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(steps(self.sandbox), PASSED)

    def test_runs_through_the_git_hooks_symlink(self):
        hooks = self.sandbox.repo / ".git" / "hooks"
        hooks.mkdir(parents=True)
        (hooks / "pre-commit").symlink_to("../../scripts/pre_commit_hook.py")
        result = self.sandbox.execute([sys.executable, hooks / "pre-commit"], env=checks_env(self.sandbox))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines(), PASSED_LINES)
        # The checkout root, not .git, whatever the link's own directory is.
        self.assertEqual(steps(self.sandbox), PASSED)

    def test_works_from_any_directory_on_the_checkout_it_belongs_to(self):
        result = self.sandbox.run(HOOK, cwd=self.sandbox.home, env=checks_env(self.sandbox))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(steps(self.sandbox), PASSED)

    def test_a_commit_runs_the_hook_through_its_git_hooks_link(self):
        self.make_repository_with_the_hook()
        result = self.commit_a_change(self.sandbox.repo)
        self.assertEqual(result.returncode, 0, result.stderr)
        # git hands a hook's stdout to its own stderr.
        self.assertEqual(result.stderr.splitlines(), PASSED_LINES)
        self.assertEqual(steps(self.sandbox), PASSED)

    def test_a_commit_in_a_linked_worktree_checks_that_worktree(self):
        # Every worktree shares the main checkout's .git/hooks, and so its link.
        self.make_repository_with_the_hook()
        worktree = self.sandbox.root / "linked worktree"
        self.sandbox.git("-C", self.sandbox.repo, "worktree", "add", "--quiet", "-b", "topic", worktree)
        result = self.commit_a_change(worktree)
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = self.sandbox.calls()
        self.assertEqual([call.cwd for call in calls[:2]], [str(worktree), str(worktree / "mcp")])
        self.assertTrue(all(call.cwd.startswith(str(worktree)) for call in calls))
        self.assertEqual(len(calls), len(PASSED))

    def test_needs_no_script_directory_on_sys_path_and_takes_no_other_lint(self):
        # `python3 -P` or PYTHONSAFEPATH=1 leave the script's directory off sys.path.
        impostor = self.sandbox.root / "site"
        impostor.mkdir()
        (impostor / "lint.py").write_text("raise SystemExit('the wrong lint module')\n", encoding="utf-8")
        result = self.hook(PYTHONSAFEPATH="1", PYTHONPATH=str(impostor))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines(), PASSED_LINES)

    def test_the_marks_are_utf8_whatever_the_output_encoding(self):
        for encoding in ("cp1252", "latin-1"):
            with self.subTest(encoding=encoding):
                result = self.hook(PYTHONUTF8="0", PYTHONIOENCODING=encoding)
                self.assertEqual((result.returncode, result.stderr), (0, ""))
                self.assertEqual(result.stdout.splitlines(), PASSED_LINES)


class BlockingTests(HookTestCase):
    def test_a_commit_in_a_worktree_without_lint_py_is_stopped_unchecked(self):
        # A branch from before the Python scripts: the main checkout's tree is
        # not the one being committed, so it is neither formatted nor linted.
        self.make_repository_with_the_hook()
        worktree = self.sandbox.root / "old worktree"
        self.sandbox.git("-C", self.sandbox.repo, "worktree", "add", "--quiet", "-b", "old", worktree)
        (worktree / "scripts/lint.py").unlink()
        head = self.sandbox.git("-C", worktree, "rev-parse", "HEAD")
        for skip_clippy in ("1", ""):
            with self.subTest(SKIP_CLIPPY=skip_clippy):
                result = self.commit_a_change(worktree, SKIP_CLIPPY=skip_clippy)
                self.assertEqual(result.returncode, 1)
                self.assertEqual(result.stderr, (
                    f"pre-commit: {worktree} has no scripts/lint.py to check it with; "
                    "run its own lint, then commit with --no-verify\n"
                ))
                self.assertEqual(self.sandbox.calls(), [])
                self.assertEqual(self.sandbox.git("-C", worktree, "rev-parse", "HEAD"), head)

    def test_a_clippy_failure_blocks_the_commit_with_lints_status(self):
        self.sandbox.fake("cargo", effect="""
            if args[0] == "clippy":
                sys.exit(9)
        """)
        result = self.hook()
        self.assertEqual(result.returncode, 9)
        self.assertNotIn("→ Pre-commit checks passed.", result.stdout)
        self.assertEqual(steps(self.sandbox), FORMAT + PREFLIGHT + CLIPPY_MATRIX[:1])

    def test_a_failing_preflight_blocks_the_commit_with_its_status(self):
        fake_preflight(self.sandbox, status=5)
        result = self.hook()
        self.assertEqual(result.returncode, 5)
        self.assertEqual(steps(self.sandbox), FORMAT + PREFLIGHT)

    def test_a_format_failure_stops_the_hook_at_once(self):
        for crate, formatted in ((".", 1), ("mcp", 2)):
            with self.subTest(crate=crate):
                self.sandbox.log.unlink(missing_ok=True)
                self.sandbox.fake("cargo", effect=f"""
                    if args[0] == "fmt" and os.path.samefile(os.getcwd(), {str(self.sandbox.repo / crate)!r}):
                        sys.exit(4)
                """)
                result = self.hook()
                self.assertEqual(result.returncode, 4)
                self.assertEqual(result.stdout.splitlines(), FORMAT_LINES[:formatted])
                self.assertEqual(steps(self.sandbox), FORMAT[:formatted])

    def test_a_missing_cargo_blocks_the_commit_with_127_naming_it(self):
        (self.sandbox.bin / "cargo").unlink()
        # Only the sandbox's bin directory: no system cargo can stand in.
        result = self.sandbox.run(HOOK, env={"PATH": str(self.sandbox.bin)})
        self.assertEqual((result.returncode, result.stderr), (127, "cargo: No such file or directory\n"))

    def test_a_step_killed_by_a_signal_blocks_the_commit_with_128_plus_n(self):
        self.sandbox.fake("cargo", effect=dies_of(signal.SIGKILL, 'args[0] == "clippy"'))
        result = self.hook()
        report = "Killed  cargo clippy --all-targets -- -D warnings\n"
        self.assertEqual((result.returncode, result.stderr), (128 + signal.SIGKILL, report))

    def test_a_reader_that_went_away_ends_the_hook_silently_by_sigpipe(self):
        result = run_into_closed_pipe(self.sandbox, HOOK, env=checks_env(self.sandbox))
        self.assertEqual((result.returncode, result.stderr), (-signal.SIGPIPE, b""))
        self.assertEqual(steps(self.sandbox), [])

    def test_ctrl_c_ends_the_hook_by_sigint_without_a_traceback(self):
        started = self.sandbox.root / "clippy started"
        self.sandbox.fake("cargo", effect=waits_for_a_signal(started, 'args[0] == "clippy"'))
        returncode, stdout, stderr = signal_group(self.sandbox, [HOOK], started, signal.SIGINT,
                                                  checks_env(self.sandbox))
        self.assertEqual((returncode, stderr), (-signal.SIGINT, ""))
        self.assertEqual(stdout.splitlines()[-1], CLIPPY_DEFAULT_LINE)


class ArgumentTests(HookTestCase):
    def test_arguments_are_ignored_even_a_help_flag(self):
        for args in (("--help",), ("-h",), ("extra",), ("extra", "--help"), ("--",), ("-x",)):
            with self.subTest(args=args):
                self.sandbox.log.unlink(missing_ok=True)
                result = self.sandbox.run(HOOK, *args, env=checks_env(self.sandbox))
                self.assertEqual((result.returncode, result.stderr), (0, ""))
                self.assertEqual(result.stdout.splitlines(), PASSED_LINES)
                self.assertEqual(steps(self.sandbox), PASSED)


if __name__ == "__main__":
    unittest.main()
