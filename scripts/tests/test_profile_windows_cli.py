"""Exercise the profiler's real subprocess adapters with recording command fakes."""

from pathlib import Path
import sys
import unittest

from sandbox import SandboxTestCase

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import profile_windows  # noqa: E402


class WindowsProfilerCommandTests(SandboxTestCase):
    def test_cli_builds_and_collects_a_windows_profile_with_real_process_adapters(self):
        sandbox = self.sandbox
        sandbox.copy("scripts/profile_windows.py")
        executable = sandbox.fake("profiled-ffm", path=sandbox.root / "built-ffm")
        sandbox.fake("rustc", stdout="x86_64-pc-windows-msvc\n")
        sandbox.fake("git", stdout="captured revision\n")
        sandbox.fake("cargo", effect=f'''
            from pathlib import Path
            import shutil
            if args == ["--version"]:
                print("cargo test-builder")
                sys.exit(0)
            binary = Path(args[args.index("--target-dir") + 1]) / args[args.index("--target") + 1] / "release/ffm.exe"
            binary.parent.mkdir(parents=True)
            shutil.copy2({str(executable)!r}, binary)
        ''')
        collector = sandbox.fake("VSDiagnostics", effect='''
            from pathlib import Path
            if args[0] == "stop":
                Path(args[-1].split(":", 1)[1]).write_text("recorded CPU samples")
        ''')
        result = sandbox.run("scripts/profile_windows.py", "--demo", env={"FFM_VSDIAGNOSTICS": str(collector)})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(sandbox.args("profiled-ffm"), [["--demo"]])
        capture, = (sandbox.repo / "target/profiles").iterdir()
        self.assertEqual((capture / "profile.diagsession").read_text(), "recorded CPU samples")
        self.assertTrue((capture / "metadata.txt").is_file())

    def test_cli_reports_missing_collector_before_building(self):
        self.sandbox.copy("scripts/profile_windows.py")
        result = self.sandbox.run("scripts/profile_windows.py", env={"FFM_VSDIAGNOSTICS": ""})
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stderr, "Required command is missing: VSDiagnostics\n")
        self.assertEqual(self.sandbox.calls(), [])

    def test_discovery_returns_none_when_no_visual_studio_installation_exists(self):
        self.assertIsNone(profile_windows.find_vsdiagnostics({}, lambda _name: None, None))


if __name__ == "__main__":
    unittest.main()
