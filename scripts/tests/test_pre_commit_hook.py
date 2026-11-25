"""Check the git pre-commit hook: format the root and mcp/ crates, then lint's clippy gate.

What blocks a commit is in test_pre_commit_hook_blocking.py, and
pre_commit_hook_case.py holds the checkout that both share.
"""

import sys
import unittest

from lint_case import PREFLIGHT, checks_env, steps
from pre_commit_hook_case import FORMAT, FORMAT_LINES, HOOK, PASSED, PASSED_LINES, HookTestCase


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
