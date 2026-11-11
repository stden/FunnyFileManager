"""Check complete Python source inventory and subprocess coverage orchestration."""

import importlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import textwrap
import unittest
from unittest.mock import patch


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

CHILD_SOURCE = """import os
CHILD_ONLY = 'executed by a copied launcher'
if os.environ.get('UNRELATED'):
    WRONG_CREDIT = 'must remain uncovered'
"""
CHILD_TEST = """
    import os
    from pathlib import Path
    import shutil
    import subprocess
    import sys
    import unittest

    class ChildTests(unittest.TestCase):
        def test_child(self):
            root = Path(__file__).resolve().parents[1]
            copy = root / 'ffm-script-test-copy' / 'checkout with spaces'
            copy.mkdir(parents=True)
            shutil.copyfile(root / 'ffm.py', copy / 'ffm.py')
            env = {'COVERAGE_PROCESS_CONFIG': os.environ['COVERAGE_PROCESS_CONFIG']}
            subprocess.run([sys.executable, str(copy / 'ffm.py')], cwd=root, env=env, check=True)
            unrelated = root / 'unrelated'
            unrelated.mkdir()
            shutil.copyfile(root / 'ffm.py', unrelated / 'ffm.py')
            env['UNRELATED'] = '1'
            subprocess.run([sys.executable, str(unrelated / 'ffm.py')], cwd=unrelated, env=env, check=True)
"""


class PythonCoverageTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue((SCRIPTS / "coverage_python.py").is_file(), "Python coverage backend is missing")
        self.backend = importlib.import_module("coverage_python")

    def test_inventory_keeps_every_production_python_file(self):
        paths = [
            "ffm.py", "install_mcp.py", "create_desktop_shortcut.py", "src/recording/replay.py",
            "scripts/test_recording.py", "scripts/new_tool.py", "scripts/tests/test_new_tool.py",
            "src/test_support/server.py", "tests/test_cli.py", "examples/demo.py", "benches/time.py",
            "scripts/tests/fixtures/tool.py", "fixtures/helper.py", "src/lsp/tests/server.py",
        ]
        completed = subprocess.CompletedProcess([], 0, stdout="\0".join(paths) + "\0")
        with patch.object(self.backend.subprocess, "run", return_value=completed) as execute:
            files = self.backend.production_files()
        self.assertEqual(files, sorted(paths[:6]))
        self.assertIn("--others", execute.call_args.args[0])
        self.assertEqual(execute.call_args.kwargs["cwd"], self.backend.ROOT)

    def test_configuration_keeps_subprocess_tracking_and_all_statement_exclusions_off(self):
        with tempfile.TemporaryDirectory() as directory:
            config = self.backend.configuration(Path(directory), ["ffm.py", "scripts/lint.py"])
            self.assertEqual(config["run"]["patch"], "subprocess")
            self.assertEqual(config["run"]["relative_files"], "True")
            self.assertEqual(config["run"]["parallel"], "True")
            self.assertEqual(config["run"].get("disable_warnings"), "no-data-collected")
            self.assertEqual(config["report"]["exclude_lines"], "")
            self.assertIn(str(self.backend.ROOT / "ffm.py"), config["run"]["include"])
            self.assertIn("*/ffm-script-test-*/checkout with spaces/scripts/lint.py", config["run"]["include"])
            self.assertNotIn("*/lint.py", config["run"]["include"].splitlines())
            self.assertIn("*/ffm-script-test-*/checkout with spaces", config["paths"]["project"])

    def test_missing_coverage_fails_with_the_install_command(self):
        failed = subprocess.CompletedProcess([], 1, stdout="", stderr="No module named coverage")
        with patch.object(self.backend.subprocess, "run", return_value=failed):
            with self.assertRaisesRegex(SystemExit, "pip install coverage"):
                self.backend.require_coverage()

    def test_old_coverage_fails_before_running_tests(self):
        old = subprocess.CompletedProcess([], 0, stdout="Coverage.py, version 7.9.0\n")
        with patch.object(self.backend.subprocess, "run", return_value=old):
            with self.assertRaisesRegex(SystemExit, "7.10"):
                self.backend.require_coverage()

    def test_collector_runs_the_real_readability_validator(self):
        self.assertIn(("scripts/test_readability.py",), self.backend.SUITES)

    def test_empty_production_inventory_stops_before_running_a_suite(self):
        with (
            patch.object(self.backend, "require_coverage"),
            patch.object(self.backend, "production_files", return_value=[]),
            patch.object(self.backend.subprocess, "run") as execute,
        ):
            with self.assertRaisesRegex(SystemExit, "no production Python"):
                self.backend.run(Path("unused"))
        execute.assert_not_called()

    def test_failed_tests_keep_the_original_command_failure(self):
        failed = subprocess.CalledProcessError(23, [sys.executable, "-m", "coverage", "run"])
        erased = subprocess.CompletedProcess([], 0)
        with tempfile.TemporaryDirectory() as directory:
            with (
                patch.object(self.backend, "require_coverage"),
                patch.object(self.backend, "production_files", return_value=["ffm.py"]),
                patch.object(self.backend.subprocess, "run", side_effect=[erased, failed]) as execute,
            ):
                with self.assertRaises(subprocess.CalledProcessError) as caught:
                    self.backend.run(Path(directory))
        self.assertEqual(caught.exception.returncode, 23)
        self.assertEqual(execute.call_args_list[0].args[0][3], "erase")

    def test_real_child_coverage_keeps_unexecuted_files_without_crediting_unrelated_copies(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "tests").mkdir()
            (root / "scripts").mkdir()
            (root / "ffm.py").write_text(CHILD_SOURCE, encoding="utf-8")
            (root / "scripts/never.py").write_text("NEVER_EXECUTED = 42\n", encoding="utf-8")
            (root / "tests/test_child.py").write_text(textwrap.dedent(CHILD_TEST), encoding="utf-8")
            with (
                patch.object(self.backend, "ROOT", root),
                patch.object(self.backend, "production_files", return_value=["ffm.py", "scripts/never.py"]),
                patch.object(self.backend, "SUITES", [("-m", "unittest", "discover", "-s", "tests")]),
            ):
                report = self.backend.run(root / "measurement")
            measured = json.loads(report.read_text(encoding="utf-8"))
        self.assertEqual(set(measured["files"]), {"ffm.py", "scripts/never.py"})
        self.assertEqual(measured["files"]["ffm.py"]["executed_lines"], [1, 2, 3])
        self.assertEqual(measured["files"]["ffm.py"]["missing_lines"], [4])
        self.assertEqual(measured["files"]["scripts/never.py"]["missing_lines"], [1])
        self.assertEqual(measured["totals"]["num_statements"], 5)
        self.assertEqual(measured["totals"]["covered_lines"], 3)


if __name__ == "__main__":
    unittest.main()
