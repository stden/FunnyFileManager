"""Check lint.py's spell check: typos over the tree, and its own self-test."""

import unittest

from lint_case import (
    FMT_CHECK, FMT_CHECK_LINES, PREFLIGHT, REJECTS_THE_MISSPELLING, SPELLING, SPELLING_LINE,
    SPELLING_SELF_TEST, TYPOS_MISSING, checks_env, gate_sandbox, steps,
)


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


if __name__ == "__main__":
    unittest.main()
