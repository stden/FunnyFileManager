"""Exercise all Rust coverage configurations without starting a real Cargo build."""

import json
from pathlib import Path
import re
import unittest

from sandbox import SandboxTestCase


RUNNER = """
import json
import os
from pathlib import Path
import subprocess
import sys
sys.path.insert(0, str(Path(__file__).resolve().parent / "scripts"))
from coverage_rust import run
before = dict(os.environ)
try:
    reports = run(Path("reports"))
except subprocess.CalledProcessError as problem:
    sys.exit(problem.returncode)
except OSError as problem:
    sys.exit(str(problem))
assert dict(os.environ) == before, "coverage backend changed the caller environment"
print("REPORTS=" + json.dumps([str(path) for path in reports]))
"""

FAKE_CARGO = """
    import json
    if "--output-path" in args:
        path = args[args.index("--output-path") + 1]
        with open(path, "w", encoding="utf-8") as report:
            if "--lcov" in args:
                report.write("SF:/fixture/main.rs\\nDA:1,0\\nend_of_record\\n")
            else:
                json.dump({"data": [{"files": [], "totals": {"lines": {"count": 1, "covered": 0}}}]}, report)
"""


def test_commands(sandbox):
    return [call for call in sandbox.calls("cargo") if call.args[:2] == ["llvm-cov", "test"]]


class RustCoverageTests(SandboxTestCase):
    def setUp(self):
        super().setUp()
        for relative in ("scripts/lint_steps.py", "scripts/shell_path.py"):
            self.sandbox.copy(relative)
        self.sandbox.copy("scripts/coverage_rust.py")
        self.sandbox.write("run_coverage.py", RUNNER)
        for directory in ("", "mcp/", "colorer-sys/"):
            self.sandbox.write(directory + "Cargo.toml", "[package]\nname = 'fixture'\n")
        self.sandbox.fake("cargo", effect=FAKE_CARGO)

    def run_backend(self, env=None):
        return self.sandbox.run("run_coverage.py", env=env)

    def test_all_packages_and_root_fallback_export_independent_reports(self):
        result = self.run_backend()
        self.assertEqual(result.returncode, 0, result.stderr)
        names = ["rust-root-all.lcov", "rust-root-fallback.lcov", "rust-mcp-all.lcov", "rust-colorer-sys-all.lcov"]
        reports_line = next(line for line in result.stdout.splitlines() if line.startswith("REPORTS="))
        reports = json.loads(reports_line.removeprefix("REPORTS="))
        self.assertEqual([Path(path).name for path in reports], names)
        calls = [call for call in test_commands(self.sandbox) if "--test" not in call.args]
        self.assertEqual(len(calls), 4)
        for call, name, manifest in zip(calls, names, ("Cargo.toml", "Cargo.toml", "mcp/Cargo.toml", "colorer-sys/Cargo.toml")):
            with self.subTest(report=name):
                self.assertEqual(call.args[:2], ["llvm-cov", "test"])
                self.assertEqual(call.args[call.args.index("--manifest-path") + 1], str(self.sandbox.repo / manifest))
                self.assertEqual(call.cwd, str(self.sandbox.repo))
                for flag in ("--include-build-script", "--no-cfg-coverage", "--no-fail-fast", "--json", "--skip-functions"):
                    self.assertIn(flag, call.args)
                self.assertNotIn("--no-clean", call.args)
                self.assertNotIn("--ignore-run-fail", call.args)
                diagnostic = self.sandbox.repo / "reports" / name.replace(".lcov", ".json")
                self.assertEqual(json.loads(diagnostic.read_text())["data"][0]["totals"]["lines"]["covered"], 0)
        self.assertIn("--all-features", calls[0].args)
        self.assertIn("--no-default-features", calls[1].args)
        self.assertEqual(calls[1].args[calls[1].args.index("--features") + 1], "test-helpers")
        self.assertNotIn("--all-features", calls[1].args)
        self.assertTrue(all("--all-features" in call.args for call in calls[2:]))

    def test_report_filter_keeps_every_production_entrypoint_and_native_source(self):
        result = self.run_backend()
        self.assertEqual(result.returncode, 0, result.stderr)
        call = self.sandbox.calls("cargo")[0]
        ignored = re.compile(call.args[call.args.index("--ignore-filename-regex") + 1])
        production = (
            "src/main.rs", "src/bootstrap.rs", "src/bootstrap/at_exit.rs", "src/editor.rs",
            "src/clipboard/system.rs", "src/terminal/typeahead.rs", "src/highlight/colorer.rs",
            "src/highlight/colorer/parse.rs", "src/s3.rs", "mcp/src/main.rs",
            "colorer-sys/build.rs", "colorer-sys/build_logic.rs", "colorer-sys/src/lib.rs",
        )
        for path in production:
            with self.subTest(path=path):
                self.assertIsNone(ignored.search("/checkout/" + path))
                self.assertIsNone(ignored.search("C:\\checkout\\" + path.replace("/", "\\")))
        for path in ("tests/x.rs", "src/tests.rs", "src/tests_calls.rs", "src/calls_tests.rs", "src/x/tests/y.rs", "src/test_support.rs", "src/test_support/x.rs", "benches/x.rs", "examples/x.rs", "mcp/bench_mcp.rs", "mcp/bench_mcp/run.rs"):
            with self.subTest(path=path):
                self.assertIsNotNone(ignored.search("/checkout/" + path))
                self.assertIsNotNone(ignored.search("C:\\checkout\\" + path.replace("/", "\\")))

    def test_failed_tests_stop_before_later_configurations(self):
        self.sandbox.fake("cargo", status=43, stderr="fixture Rust test failed\n")
        result = self.run_backend()
        self.assertEqual(result.returncode, 43, result.stderr)
        self.assertIn("fixture Rust test failed", result.stderr)
        self.assertEqual(len(self.sandbox.calls("cargo")), 1)

    def test_missing_llvm_cov_is_reported_without_a_fallback(self):
        self.sandbox.fake("cargo", status=101, stderr="no such command: llvm-cov\n")
        result = self.run_backend()
        self.assertEqual(result.returncode, 101, result.stderr)
        self.assertIn("no such command: llvm-cov", result.stderr)
        self.assertEqual(len(self.sandbox.calls("cargo")), 1)

    def test_empty_wrappers_use_env_without_mutating_the_caller(self):
        for extra in ({}, {"RUSTC_WRAPPER": ""}, {"CARGO_BUILD_RUSTC_WRAPPER": ""}):
            with self.subTest(environment=extra):
                self.sandbox.log.unlink(missing_ok=True)
                result = self.run_backend(env=extra)
                self.assertEqual(result.returncode, 0, result.stderr)
                for call in self.sandbox.calls("cargo"):
                    self.assertEqual(call.env.get("CARGO_BUILD_RUSTC_WRAPPER"), "/usr/bin/env")
                    self.assertNotIn("RUSTC_WRAPPER", call.env)

    def test_nonempty_caller_wrappers_are_preserved(self):
        for extra in ({"RUSTC_WRAPPER": "sccache"}, {"CARGO_BUILD_RUSTC_WRAPPER": "/opt/wrapper"}):
            with self.subTest(environment=extra):
                self.sandbox.log.unlink(missing_ok=True)
                result = self.run_backend(env=extra)
                self.assertEqual(result.returncode, 0, result.stderr)
                for call in self.sandbox.calls("cargo"):
                    self.assertEqual({name: call.env.get(name) for name in extra}, extra)

    def test_missing_env_fails_before_cargo(self):
        result = self.run_backend(env={"PATH": str(self.sandbox.bin)})
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("pass-through compiler wrapper", result.stderr)
        self.assertEqual(self.sandbox.calls("cargo"), [])

    def test_each_call_uses_private_coverage_artifacts(self):
        result = self.run_backend(env={"RUSTC_WRAPPER": "existing-wrapper"})
        self.assertEqual(result.returncode, 0, result.stderr)
        for call in self.sandbox.calls("cargo"):
            self.assertEqual(call.env.get("CARGO_LLVM_COV_TARGET_DIR"), str(self.sandbox.repo / "reports/target"))
            self.assertEqual(call.env.get("FFM_TEST_SFTP_SERVER"), "1")

    def test_explicit_coverage_artifact_directory_is_preserved(self):
        target = str(self.sandbox.root / "caller-coverage")
        result = self.run_backend(env={"CARGO_LLVM_COV_TARGET_DIR": target, "FFM_TEST_SFTP_SERVER": "0"})
        self.assertEqual(result.returncode, 0, result.stderr)
        for call in self.sandbox.calls("cargo"):
            self.assertEqual(call.env.get("CARGO_LLVM_COV_TARGET_DIR"), target)
            self.assertEqual(call.env.get("FFM_TEST_SFTP_SERVER"), "0")

    def test_test_source_filter_works_with_extended_posix_regex(self):
        result = self.run_backend()
        self.assertEqual(result.returncode, 0, result.stderr)
        args = self.sandbox.calls("cargo")[0].args
        pattern = args[args.index("--ignore-filename-regex") + 1]
        paths = "/checkout/src/tests.rs\n/checkout/src/main.rs\n/checkout/examples/snapshot.rs\n"
        matched = self.sandbox.execute(["grep", "-E", pattern], input=paths)
        self.assertEqual(matched.stdout.splitlines(), ["/checkout/src/tests.rs", "/checkout/examples/snapshot.rs"])

    def test_windows_compiles_a_native_pass_through_wrapper(self):
        self.sandbox.fake("rustc", effect='open(args[args.index("-o") + 1], "wb").close()')
        self.sandbox.write("windows_wrapper.py", '''
from pathlib import Path
import sys
from unittest.mock import patch
sys.path.insert(0, str(Path(__file__).resolve().parent / "scripts"))
import coverage_rust
directory = Path("reports").resolve()
directory.mkdir()
with patch("coverage_rust.os.name", "nt"):
    environment = coverage_rust.compiler_environment(directory)
wrapper = Path(environment["CARGO_BUILD_RUSTC_WRAPPER"])
assert wrapper == directory / "rustc-wrapper.exe"
assert wrapper.is_file()
assert (directory / "rustc-wrapper.rs").is_file()
assert "RUSTC_WRAPPER" not in environment
assert environment["FFM_APPLICATION_TEST_BINARY"] == str(directory / "target/debug/ffm.exe")
''')
        result = self.sandbox.run("windows_wrapper.py", env={"RUSTC_WRAPPER": ""})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(self.sandbox.calls("rustc")), 1)


if __name__ == "__main__":
    unittest.main()
