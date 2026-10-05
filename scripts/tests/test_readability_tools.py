"""Check which compilers the readability self-test runs: rustup's, else PATH's."""

import os
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch

from sandbox import REPO

sys.path.insert(0, str(REPO / "scripts"))
import test_readability  # noqa: E402


@unittest.skipIf(os.name == "nt", "the fake programs are POSIX shell scripts")
class ToolPathTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="ffm-readability-test-")).resolve()
        self.addCleanup(shutil.rmtree, self.root, ignore_errors=True)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        patcher = patch.dict(os.environ, {"PATH": str(self.bin)})
        patcher.start()
        self.addCleanup(patcher.stop)

    def program(self, name, script="exit 0"):
        path = self.bin / name
        path.write_text(f"#!/bin/sh\n{script}\n", encoding="utf-8")
        path.chmod(0o755)
        return path

    def tool_path(self, name):
        return test_readability.tool_path(name, test_readability.child_environment())

    def test_rustup_names_the_toolchain_that_rust_toolchain_toml_selects(self):
        self.program("clippy-driver")
        self.program("rustup", 'echo "/toolchains/pinned/bin/$2"')
        self.assertEqual(self.tool_path("clippy-driver"), Path("/toolchains/pinned/bin/clippy-driver"))

    def test_without_rustup_the_compilers_on_path_are_used(self):
        driver = self.program("clippy-driver")
        compiler = self.program("rustc")
        self.assertEqual(self.tool_path("clippy-driver"), driver)
        self.assertEqual(self.tool_path("rustc"), compiler)

    def test_neither_rustup_nor_the_tool_names_both(self):
        with self.assertRaises(RuntimeError) as raised:
            self.tool_path("clippy-driver")
        self.assertIn("clippy-driver", str(raised.exception))
        self.assertIn("rustup", str(raised.exception))


if __name__ == "__main__":
    unittest.main()
