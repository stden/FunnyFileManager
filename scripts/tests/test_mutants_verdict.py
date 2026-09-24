"""Check scripts/mutants_verdict.py, the verdict of CI's mutation-testing steps."""

from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

from sandbox import REPO


SCRIPT = REPO / "scripts/mutants_verdict.py"
CAUGHT = "src/a.rs:1:5: replace f -> bool with true"
MISSED = "src/a.rs:2:5: replace g -> bool with false"
HUNG = "src/a.rs:3:9: replace < with <= in h"


class MutantsVerdictTests(unittest.TestCase):
    def setUp(self):
        self.out = Path(tempfile.mkdtemp(prefix="ffm-mutants-verdict-test-"))
        self.addCleanup(shutil.rmtree, self.out, ignore_errors=True)

    def outcome(self, **lists):
        """Write mutants.out's lists: caught=[...] becomes caught.txt."""
        for name, mutants in lists.items():
            (self.out / f"{name}.txt").write_text("".join(f"{mutant}\n" for mutant in mutants), encoding="utf-8")

    def verdict(self, *args):
        return subprocess.run(
            [sys.executable, SCRIPT, *map(str, args)],
            capture_output=True, text=True, encoding="utf-8", timeout=60, check=False,
        )

    def test_every_mutant_caught_passes(self):
        self.outcome(caught=[CAUGHT], missed=[], timeout=[], unviable=[])
        result = self.verdict(0, self.out)
        self.assertEqual((result.returncode, result.stdout, result.stderr), (0, "", ""))

    def test_a_diff_without_mutants_writes_nothing_and_passes(self):
        result = self.verdict(0, self.out / "mutants.out")
        self.assertEqual((result.returncode, result.stdout, result.stderr), (0, "", ""))

    def test_a_missed_mutant_fails_and_is_named(self):
        self.outcome(caught=[CAUGHT], missed=[MISSED])
        result = self.verdict(2, self.out)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stderr, f"1 mutant(s) survived; kill each with a test that observes the change:\n  {MISSED}\n")

    def test_a_missed_mutant_behind_exit_3_still_fails(self):
        # cargo-mutants reports 3 (timeout) in preference to 2 (missed).
        self.outcome(caught=[CAUGHT], missed=[MISSED], timeout=[HUNG])
        result = self.verdict(3, self.out)
        self.assertEqual(result.returncode, 2)
        self.assertIn(MISSED, result.stderr)

    def test_exit_2_fails_even_without_a_missed_list(self):
        result = self.verdict(2, self.out)
        self.assertEqual(result.returncode, 2)

    def test_timeouts_with_nothing_caught_fail(self):
        # The symptom of two suites sharing a runner: every mutant timed out.
        for lists in ({"timeout": [HUNG, HUNG]}, {"timeout": [HUNG], "caught": [], "unviable": [CAUGHT]}):
            with self.subTest(lists=lists):
                for stale in self.out.iterdir():
                    stale.unlink()
                self.outcome(**lists)
                result = self.verdict(3, self.out)
                self.assertEqual(result.returncode, 3)
                self.assertIn("timed out, so nothing was tested", result.stderr)

    def test_a_timeout_among_caught_mutants_passes_with_a_warning(self):
        self.outcome(caught=[CAUGHT], timeout=[HUNG])
        result = self.verdict(3, self.out)
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, f"::warning title=Mutant timed out (counted as caught)::{HUNG}\n")

    def test_any_other_status_fails_as_it_is(self):
        # 4: the unmutated baseline failed; 1: usage; 70: an internal error.
        self.outcome(caught=[CAUGHT])
        for status in (1, 4, 70):
            with self.subTest(status=status):
                result = self.verdict(status, self.out)
                self.assertEqual(result.returncode, status)
                self.assertIn(f"cargo mutants exited {status}", result.stderr)

    def test_wrong_arguments_print_the_usage(self):
        for args in ((), ("0",), ("x", self.out), ("0", self.out, "extra")):
            with self.subTest(args=args):
                result = self.verdict(*args)
                self.assertEqual((result.returncode, result.stderr), (2, "usage: scripts/mutants_verdict.py STATUS MUTANTS_OUT\n"))

    def test_help(self):
        result = self.verdict("--help")
        self.assertEqual(result.returncode, 0)
        self.assertTrue(result.stdout.startswith("Decide a CI mutation-testing step"))


if __name__ == "__main__":
    unittest.main()
