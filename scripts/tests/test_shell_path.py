"""Check that shell_path finds programs on PATH the way bash does."""

import os
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch

from sandbox import REPO

sys.path.insert(0, str(REPO / "scripts"))
import shell_path  # noqa: E402


@unittest.skipIf(os.name == "nt", "PATH lookup by execute permission is POSIX")
class ShellPathTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="ffm-shell-path-test-")).resolve()
        self.addCleanup(shutil.rmtree, self.root, ignore_errors=True)

    def program(self, relative, mode=0o755):
        """A file at root/relative with `mode`; its path as text."""
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("#!/bin/sh\n", encoding="utf-8")
        path.chmod(mode)
        return str(path)

    def with_path(self, *entries):
        """PATH made of `entries`, and HOME at root/home, while the test runs."""
        environment = {"PATH": os.pathsep.join(entries), "HOME": str(self.root / "home")}
        patcher = patch.dict(os.environ, environment)
        patcher.start()
        self.addCleanup(patcher.stop)

    def test_a_leading_tilde_is_expanded_as_bash_expands_it(self):
        found = self.program("home/tools/cargo")
        self.with_path("~/tools")
        self.assertEqual(shell_path.find("cargo"), found)

    def test_every_match_is_listed_in_path_order(self):
        first, second = self.program("a/ffm"), self.program("b/ffm")
        self.with_path(str(self.root / "b"), str(self.root / "missing"), str(self.root / "a"))
        self.assertEqual(shell_path.find_all("ffm"), [second, first])

    def test_directories_and_files_without_execute_permission_are_skipped(self):
        (self.root / "a/cargo").mkdir(parents=True)
        plain = self.program("b/cargo", mode=0o644)
        runnable = self.program("c/cargo")
        self.with_path(*(str(self.root / name) for name in ("a", "b", "c")))
        self.assertEqual(shell_path.find_all("cargo"), [runnable])
        # os.F_OK: any file, as for a program the shell then fails to start.
        self.assertEqual(shell_path.find_all("cargo", os.F_OK), [plain, runnable])

    def test_relative_and_empty_entries_are_taken_from_the_given_directory(self):
        crate = self.root / "crate"
        in_tools = self.program("crate/tools/cargo")
        self.program("crate/cargo")
        self.with_path("tools", "")
        # An empty entry is ".", as `type -pa` prints it.
        in_crate = os.path.join(crate, ".", "cargo")
        self.assertEqual(shell_path.find_all("cargo", cwd=crate), [in_tools, in_crate])

    def test_nothing_found_is_none(self):
        self.with_path(str(self.root))
        self.assertIsNone(shell_path.find("cargo"))
        self.assertIsNone(shell_path.resolve("cargo"))

    def test_resolve_takes_a_word_with_a_directory_as_it_is(self):
        found = self.program("bin/cargo")
        self.with_path(str(self.root / "bin"))
        for word in ("./cargo", "/nonexistent/cargo", "bin/cargo"):
            with self.subTest(word=word):
                self.assertEqual(shell_path.resolve(word), word)
        self.assertEqual(shell_path.resolve("cargo"), found)


if __name__ == "__main__":
    unittest.main()
