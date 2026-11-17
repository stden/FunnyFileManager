"""Purpose: Verify lint's Python children can print Unicode under Windows pipe encodings."""

import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
LAUNCHER = """from pathlib import Path
import sys
from lint_steps import run_step, use_utf8_output
use_utf8_output()
sys.exit(run_step([sys.executable, '-c', "print(chr(0x2713) + ' child output')"],
                  cwd=Path(__file__).resolve().parent))
"""


def child_output(encoding):
    with tempfile.TemporaryDirectory() as temporary:
        directory = Path(temporary)
        for module in ("lint_steps.py", "shell_path.py"):
            shutil.copyfile(ROOT / "scripts" / module, directory / module)
        launcher = directory / "parent.py"
        launcher.write_text(LAUNCHER, encoding="utf-8")
        environment = {**os.environ, "PYTHONUTF8": "0", "PYTHONIOENCODING": encoding}
        return subprocess.run(
            [sys.executable, str(launcher)], cwd=directory, env=environment,
            capture_output=True, encoding="utf-8", check=False,
        )


class ChildEncodingTests(unittest.TestCase):
    def test_python_steps_inherit_utf8_under_legacy_output_encodings(self):
        for encoding in ("cp1252", "cp866", "latin-1", "ascii"):
            with self.subTest(encoding=encoding):
                result = child_output(encoding)
                self.assertEqual((result.returncode, result.stderr), (0, ""))
                self.assertEqual(result.stdout.splitlines(), ["✓ child output"])


if __name__ == "__main__":
    unittest.main()
