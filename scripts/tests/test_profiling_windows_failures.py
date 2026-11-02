"""Purpose: Test error handling and VSDiagnostics discovery in profile_windows."""

from pathlib import Path
import sys
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))
sys.path.insert(0, str(REPO / "scripts/tests"))

import profile_windows  # noqa: E402
from test_profiling_windows import WindowsProfilingTests, completed  # noqa: E402


class WindowsProfilingFailureTests(WindowsProfilingTests):
    def test_host_query_failure_stops_before_building(self):
        self.host.statuses["rustc"] = 79
        self.host.stderrs["rustc"] = "host query failed\n"
        rc, err = self.invoke()
        self.assertEqual(rc, 79)
        self.assertIn("host query failed", err)
        self.assertEqual(self.captures(), [])

    def test_failed_rebuild_never_starts_collector(self):
        self.host.statuses["cargo:build"] = 101
        rc, _ = self.invoke()
        self.assertEqual(rc, 101)
        self.assertEqual(self.captures(), [])
        self.assertEqual(self.host.popened, [])

    def test_missing_built_executable_never_starts_collector(self):
        self.host.write_exe = False
        rc, err = self.invoke()
        self.assertEqual(rc, 1)
        self.assertIn("Built executable is missing", err)
        self.assertEqual(self.captures(), [])

    def test_git_revision_failure_keeps_snapshot_without_starting_collector(self):
        self.host.statuses["git:rev-parse"] = 128
        self.host.stderrs["git:rev-parse"] = "no git repo\n"
        rc, err = self.invoke()
        self.assertEqual(rc, 128)
        self.assertIn("no git repo", err)
        capture, = self.captures()
        self.assertTrue((capture / "ffm.exe").is_file())
        self.assertFalse((capture / "profile.diagsession").exists())
        self.assertEqual(self.host.popened, [])

    def test_collector_start_failure_never_launches_ffm(self):
        self.host.statuses["vsdiag:start"] = 77
        self.host.stderrs["vsdiag:start"] = "collector denied\n"
        rc, err = self.invoke()
        self.assertEqual(rc, 77)
        self.assertIn("collector denied", err)
        self.assertEqual(self.host.popened, [])
        capture, = self.captures()
        self.assertTrue((capture / "ffm.exe").is_file())
        self.assertFalse((capture / "profile.diagsession").exists())

    def test_nonzero_ffm_exit_still_stops_collector_session(self):
        self.host.ffm_rc = 23
        rc, _ = self.invoke()
        self.assertEqual(rc, 23)
        capture, = self.captures()
        self.assertTrue((capture / "profile.diagsession").is_file())

    def test_missing_diagsession_after_zero_exit_reports_failure(self):
        self.host.write_diagsession = False
        self.host.stderrs["vsdiag:stop"] = "collector failed silently\n"
        rc, err = self.invoke()
        self.assertEqual(rc, 1)
        self.assertIn("profile.diagsession", err)


class VsDiagnosticsDiscoveryTests(unittest.TestCase):
    def test_skips_buildtools_without_cpu_agent_and_discovers_full_vs(self):
        with tempfile.TemporaryDirectory(prefix="ffm-vsfind-") as root:
            base = Path(root)
            bt = base / "Microsoft Visual Studio/2022/BuildTools"
            bt_exe = bt / "Team Tools/DiagnosticsHub/Collector/VSDiagnostics.exe"
            bt_exe.parent.mkdir(parents=True)
            bt_exe.write_bytes(b"bare-collector")

            vs = base / "Microsoft Visual Studio/2022/Community"
            exe = vs / "Team Tools/DiagnosticsHub/Collector/VSDiagnostics.exe"
            cfg = exe.parent / "AgentConfigs/CpuUsageBase.json"
            cfg.parent.mkdir(parents=True)
            exe.write_bytes(b"full-collector")
            cfg.write_text("{}", encoding="utf-8")

            found_vswhere = profile_windows.find_vsdiagnostics(
                {},
                lambda name: "/fake/vswhere" if name == "vswhere" else None,
                lambda _args, env=None, capture=True: completed(0, stdout=f"{bt}\n{vs}\n"),
            )
            self.assertEqual(Path(found_vswhere), exe)

            found_glob = profile_windows.find_vsdiagnostics(
                {"PROGRAMFILES": str(base)},
                lambda _name: None,
                lambda _args, env=None, capture=True: completed(1),
            )
            self.assertEqual(Path(found_glob), exe)


if __name__ == "__main__":
    unittest.main()
