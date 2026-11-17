"""Purpose: Preserve useful evidence and stop unsafe launches when profiling setup fails."""

import shutil

from profiling_case import ProfilingCase


class ProfilingFailureTests(ProfilingCase):
    def test_missing_tools_stop_before_building_or_creating_a_capture(self):
        for tool in ("bash", "dirname"):
            executable = shutil.which(tool, path="/usr/bin:/bin")
            self.assertIsNotNone(executable, tool)
            (self.sandbox.bin / tool).symlink_to(executable)
        for tool in ("cargo", "rustc", "git", "flamegraph", "perf"):
            with self.subTest(tool=tool):
                executable = self.sandbox.bin / tool
                saved = executable.with_suffix(".saved")
                executable.rename(saved)
                try:
                    result = self.launch(env={"PATH": str(self.sandbox.bin)})
                finally:
                    saved.rename(executable)
                self.assertEqual(result.returncode, 1, result.stderr)
                self.assertIn(f"Required command is missing: {tool}", result.stderr)
                self.assertEqual(self.sandbox.calls(), [])
                self.assertEqual(self.captures(), [])

    def test_a_failed_host_query_never_builds_or_starts_the_profiler(self):
        self.sandbox.fake("rustc", status=79, stderr="host query failed\n")
        result = self.launch()
        self.assertEqual(result.returncode, 79)
        self.assertIn("host query failed", result.stderr)
        self.assertEqual(self.sandbox.calls("cargo"), [])
        self.assertEqual(self.sandbox.calls("flamegraph"), [])
        self.assertEqual(self.captures(), [])

    def test_a_successful_build_without_a_binary_never_starts_recording(self):
        self.sandbox.fake("cargo")
        result = self.launch()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.sandbox.calls("flamegraph"), [])
        self.assertEqual(self.sandbox.calls("perf"), [])
        capture, = self.captures()
        self.assertFalse((capture / "ffm").exists())
        self.assertFalse((capture / "metadata.txt").exists())

    def test_missing_revision_stops_recording_and_keeps_the_binary(self):
        self.sandbox.fake("git", status=128, stderr="revision unavailable\n")
        result = self.launch()
        self.assertEqual(result.returncode, 128)
        self.assertIn("revision unavailable", result.stderr)
        self.assertEqual(self.sandbox.calls("flamegraph"), [])
        capture, = self.captures()
        self.assertEqual((capture / "ffm").read_bytes(), self.template.read_bytes())
        self.assertFalse((capture / "perf.data").exists())

    def test_recording_permission_errors_keep_identity_without_running_ffm(self):
        self.sandbox.fake("perf", status=77, stderr="recording permission denied\n")
        result = self.launch()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("recording permission denied", result.stderr)
        self.assertEqual(self.sandbox.calls("profiled-ffm"), [])
        capture, = self.captures()
        self.assertEqual((capture / "ffm").read_bytes(), self.template.read_bytes())
        self.assertIn("revision-for-this-capture", (capture / "metadata.txt").read_text())
        self.assertFalse((capture / "flamegraph.svg").exists())

    def test_graph_failure_preserves_samples_and_returns_the_failure(self):
        self.sandbox.fake("flamegraph", effect='''
            import subprocess
            record = args[args.index("--cmd") + 1].split()
            command = args[args.index("--") + 1:]
            subprocess.run(["perf", *record, "--", *command], check=True)
            print("cannot render graph", file=sys.stderr)
            sys.exit(42)
        ''')
        result = self.launch()
        self.assertEqual(result.returncode, 42)
        self.assertIn("cannot render graph", result.stderr)
        capture, = self.captures()
        self.assertEqual((capture / "perf.data").read_text(), "raw CPU samples")
        self.assertEqual((capture / "ffm").read_bytes(), self.template.read_bytes())
        self.assertTrue((capture / "metadata.txt").is_file())
        self.assertFalse((capture / "flamegraph.svg").exists())
