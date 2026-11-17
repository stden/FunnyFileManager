"""Check what makes the git pre-commit hook block a commit, and how it ends."""

import signal
import unittest

from lint_case import CLIPPY_DEFAULT_LINE, CLIPPY_MATRIX, PREFLIGHT, checks_env, fake_preflight, steps
from pre_commit_hook_case import FORMAT, FORMAT_LINES, HOOK, HookTestCase
from process_signals import dies_of, run_into_closed_pipe, signal_group, waits_for_a_signal


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


if __name__ == "__main__":
    unittest.main()
