"""Keep interactive profiling captures usable after subsequent release builds."""

from pathlib import Path
import unittest

from sandbox import SandboxTestCase


NATIVE_TARGET = "x86_64-unknown-linux-gnu"


class ProfilingLaunchTests(SandboxTestCase):
    def setUp(self):
        super().setUp()
        sandbox = self.sandbox
        for script in ("x.sh", "sx.sh"):
            sandbox.copy(script)
        self.template = sandbox.fake(
            "profiled-ffm", path=sandbox.root / "built-ffm",
            effect='sys.exit(int(os.environ.get("FFM_TEST_STATUS", "0")))',
        )
        self.binary = sandbox.repo / "target" / NATIVE_TARGET / "release/ffm"
        sandbox.fake("git", effect='''
            print("revision-for-this-capture" if args == ["rev-parse", "HEAD"]
                  else " M src/example.rs")
        ''')
        sandbox.fake("rustc", effect=f'''
            print({NATIVE_TARGET!r} if args == ["--print", "host-tuple"]
                  else "rustc test-compiler")
        ''')
        sandbox.fake("cargo", effect=f'''
            from pathlib import Path
            import shutil
            if args == ["--version"]:
                print("cargo test-builder")
                sys.exit(0)
            target = Path(args[args.index("--target-dir") + 1]) if "--target-dir" in args else Path("target")
            triple = args[args.index("--target") + 1] if "--target" in args else os.environ.get("CARGO_BUILD_TARGET")
            if triple:
                target = target / triple
            binary = target / "release/ffm"
            binary.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2({str(self.template)!r}, binary)
        ''')
        sandbox.fake("flamegraph", effect='''
            from pathlib import Path
            import subprocess
            record = args[args.index("--cmd") + 1].split()
            command = args[args.index("--") + 1:]
            result = subprocess.run(["perf", *record, "--", *command], check=False)
            if result.returncode:
                sys.exit(1)
            Path(args[args.index("--output") + 1]).write_text("captured SVG")
        ''')
        sandbox.fake("perf", effect='''
            from pathlib import Path
            import subprocess
            Path(args[args.index("-o") + 1]).write_text("raw CPU samples")
            result = subprocess.run(args[args.index("--") + 1:], check=False)
            sys.exit(result.returncode)
        ''')

    def launch(self, *args, env=None):
        return self.sandbox.execute(
            [self.sandbox.repo / "x.sh", *args], cwd=self.sandbox.home, env=env,
        )

    def captures(self):
        return sorted((self.sandbox.repo / "target/profiles").glob("ffm-*"))

    def test_an_existing_release_is_rebuilt_with_symbols_before_capture(self):
        self.binary.parent.mkdir(parents=True)
        self.binary.write_text("#!/bin/sh\nexit 90\n")
        self.binary.chmod(0o755)
        result = self.launch()
        self.assertEqual(result.returncode, 0, result.stderr)
        builds = [call for call in self.sandbox.calls("cargo") if call.args[0] == "build"]
        self.assertEqual(len(builds), 1)
        self.assertEqual(builds[0].args, [
            "build", "--release", "--bin", "ffm", "--target", NATIVE_TARGET, "--target-dir",
            str(self.sandbox.repo / "target"),
        ])
        for name, value in {
            "CARGO_PROFILE_RELEASE_DEBUG": "1",
            "CARGO_PROFILE_RELEASE_STRIP": "none",
            "CARGO_PROFILE_RELEASE_SPLIT_DEBUGINFO": "off",
        }.items():
            self.assertEqual(builds[0].env.get(name), value, name)
        self.assertEqual(self.binary.read_bytes(), self.template.read_bytes())

    def test_capture_preserves_the_executed_binary_and_build_identity(self):
        result = self.launch()
        self.assertEqual(result.returncode, 0, result.stderr)
        capture, = self.captures()
        executable = capture / "ffm"
        self.assertTrue(executable.is_file(), "the capture must retain its executable")
        self.assertEqual(executable.read_bytes(), self.template.read_bytes())
        flamegraph, = self.sandbox.calls("flamegraph")
        self.assertEqual((Path(flamegraph.cwd) / flamegraph.args[-1]).resolve(), executable)
        self.assertEqual(len(self.sandbox.calls("profiled-ffm")), 1)
        for artifact in ("perf.data", "flamegraph.svg"):
            self.assertTrue((capture / artifact).is_file(), artifact)
        metadata = (capture / "metadata.txt").read_text()
        for identity in ("revision-for-this-capture", " M src/example.rs",
                         "rustc test-compiler", "cargo test-builder"):
            self.assertIn(identity, metadata)
        self.binary.parent.mkdir(parents=True, exist_ok=True)
        self.binary.write_text("replacement release build")
        self.assertEqual(executable.read_bytes(), self.template.read_bytes())

    def test_an_inherited_target_cannot_reuse_a_stale_default_executable(self):
        stale = self.sandbox.write("target/release/ffm", "#!/bin/sh\nexit 0\n")
        stale.chmod(0o755)
        result = self.launch(env={"CARGO_BUILD_TARGET": "wrong-target"})
        self.assertEqual(result.returncode, 0, result.stderr)
        capture, = self.captures()
        self.assertEqual((capture / "ffm").read_bytes(), self.template.read_bytes())
        builds = [call for call in self.sandbox.calls("cargo") if call.args[0] == "build"]
        self.assertEqual(builds[0].args[builds[0].args.index("--target") + 1], NATIVE_TARGET)
        self.assertEqual(self.binary.read_bytes(), self.template.read_bytes())
        self.assertIn("rustc test-compiler", (capture / "metadata.txt").read_text())

    def test_recording_keeps_arguments_and_disables_debug_server_network(self):
        arguments = ["--", "two words", "--file=with space", ""]
        result = self.launch(*arguments, env={"DEBUGINFOD_URLS": "https://example.invalid"})
        self.assertEqual(result.returncode, 0, result.stderr)
        call, = self.sandbox.calls("profiled-ffm")
        self.assertEqual(call.args, arguments)
        self.assertEqual(call.cwd, str(self.sandbox.repo))
        self.assertEqual(call.env.get("DEBUGINFOD_URLS"), "")
        flamegraph, = self.sandbox.calls("flamegraph")
        self.assertIn("--no-inline", flamegraph.args)
        perf, = self.sandbox.calls("perf")
        self.assertEqual(perf.args[:8], [
            "record", "-e", "cpu-clock:u", "-F", "99",
            "--call-graph", "dwarf,16384", "-o",
        ])
        capture, = self.captures()
        self.assertEqual((Path(perf.cwd) / perf.args[8]).resolve(), capture / "perf.data")

    def test_repeated_runs_keep_distinct_executable_snapshots(self):
        for _ in range(2):
            result = self.launch()
            self.assertEqual(result.returncode, 0, result.stderr)
        captures = self.captures()
        self.assertEqual(len(captures), 2)
        self.assertTrue(all((capture / "ffm").is_file() for capture in captures))

    def test_a_failed_rebuild_never_profiles_the_existing_executable(self):
        self.binary.parent.mkdir(parents=True)
        self.binary.write_text("#!/bin/sh\nexit 0\n")
        self.binary.chmod(0o755)
        self.sandbox.fake("cargo", status=101)
        result = self.launch()
        self.assertEqual(result.returncode, 101)
        self.assertEqual(self.sandbox.calls("flamegraph"), [])
        self.assertEqual(self.sandbox.calls("perf"), [])

    def test_a_failed_profiled_process_keeps_raw_data_without_a_flamegraph(self):
        result = self.launch(env={"FFM_TEST_STATUS": "23"})
        self.assertEqual(result.returncode, 1)
        capture, = self.captures()
        self.assertTrue((capture / "ffm").is_file())
        self.assertTrue((capture / "perf.data").is_file())
        self.assertFalse((capture / "flamegraph.svg").exists())


if __name__ == "__main__":
    unittest.main()
