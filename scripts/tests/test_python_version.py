"""Check that every developer script parses as the oldest Python it supports."""

import ast
import unittest

from sandbox import REPO


# The python3 that macOS's Command Line Tools ship.
OLDEST_PYTHON = (3, 9)
SCRIPTS = sorted([*REPO.glob("*.py"), *REPO.glob("scripts/*.py")])


class PythonVersionTests(unittest.TestCase):
    def test_every_script_parses_as_the_oldest_supported_python(self):
        # A newer construct, such as match (3.10), fails before the script
        # can even print its help.
        self.assertIn(REPO / "scripts/k9s_eks.py", SCRIPTS)
        for script in SCRIPTS:
            with self.subTest(script=str(script.relative_to(REPO))):
                source = script.read_text(encoding="utf-8")
                ast.parse(source, filename=str(script), feature_version=OLDEST_PYTHON)


if __name__ == "__main__":
    unittest.main()
