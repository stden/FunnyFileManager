"""A command double at a production filename must never cover the real source."""

import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

from sandbox import SandboxTestCase

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import coverage_python  # noqa: E402


class FakeSourceCoverageTests(SandboxTestCase):
    def measure_double(self, kind):
        canonical = self.sandbox.root / "canonical"
        canonical.mkdir()
        (canonical / "ffm.py").write_text("NEVER_ONE = 1\nNEVER_TWO = 2\n", encoding="utf-8")
        measured = self.sandbox.root / "measurement"
        measured.mkdir()
        config_path = measured / "python.ini"
        with patch.object(coverage_python, "ROOT", canonical):
            config = coverage_python.configuration(measured, ["ffm.py"])
        with config_path.open("w", encoding="utf-8") as output:
            config.write(output)
        fake = (self.sandbox.fake("fake-ffm", path=self.sandbox.repo / "ffm.py") if kind == "fake"
                else self.sandbox.write("ffm.py", "import os\nTEST_DOUBLE = True\n"))
        command = [sys.executable, "-m", "coverage"]
        report_path = measured / "python.json"
        operations = [
            [*command, "run", "--rcfile", config_path, fake],
            [*command, "combine", "--rcfile", config_path],
            [*command, "json", "--rcfile", config_path, "-o", report_path, "ffm.py"],
        ]
        for operation in operations:
            result = self.sandbox.execute(operation, cwd=canonical)
            self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(report_path.read_text(encoding="utf-8"))

    def test_replacing_a_production_script_with_a_fake_cannot_credit_the_original(self):
        report = self.measure_double("fake")
        self.assertEqual(report["files"]["ffm.py"]["summary"]["covered_lines"], 0)
        self.assertEqual(report["files"]["ffm.py"]["missing_lines"], [1, 2])

    def test_written_python_doubles_cannot_credit_the_production_source_they_replace(self):
        report = self.measure_double("written")
        self.assertEqual(report["files"]["ffm.py"]["summary"]["covered_lines"], 0)
        self.assertEqual(report["files"]["ffm.py"]["missing_lines"], [1, 2])


if __name__ == "__main__":
    unittest.main()
