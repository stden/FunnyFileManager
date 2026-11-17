"""The sandbox checkout, the git repository around it, and the expected steps
that the tests of the pre-commit hook share.
"""

import shutil
import unittest

from lint_case import CLIPPY_LINES, CLIPPY_MATRIX, PREFLIGHT, checks_env, gate_sandbox
from sandbox import SYSTEM_PATH


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
