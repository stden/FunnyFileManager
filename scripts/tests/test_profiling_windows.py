"""Purpose: Test the Windows CPU profiling launcher without real Cargo or Visual Studio."""

import io
from contextlib import redirect_stdout
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))


def completed(returncode=0, stdout="", stderr=""):
    return subprocess.CompletedProcess(args=[], returncode=returncode, stdout=stdout, stderr=stderr)


class FakeHost:
    """In-memory command, process, and tool lookup seam for profile_windows."""

    def __init__(self, root):
        self.root = Path(root)
        self.calls = []
        self.popened = []
        self.tools = {
            "cargo": "/fake/cargo",
            "rustc": "/fake/rustc",
            "git": "/fake/git",
        }
        self.vsdiag = str(self.root / "Collector/VSDiagnostics.exe")
        self.host_tuple = "x86_64-pc-windows-msvc"
        self.statuses = {}
        self.stderrs = {}
        self.ffm_rc = 0
        self.write_pdb = True
        self.write_exe = True
        self.write_diagsession = True

    def setup_vsdiag(self, with_config=True):
        exe = Path(self.vsdiag)
        exe.parent.mkdir(parents=True, exist_ok=True)
        exe.write_bytes(b"vsdiag")
        if with_config:
            cfg = exe.parent / "AgentConfigs/CpuUsageBase.json"
            cfg.parent.mkdir(parents=True, exist_ok=True)
            cfg.write_text("{}", encoding="utf-8")

    def which(self, name):
        return self.tools.get(name)

    def run(self, args, env=None, capture=True):
        del capture
        cmd = [str(part) for part in args]
        self.calls.append((cmd, dict(env) if env is not None else None))
        key = self._key(cmd)
        rc = self.statuses.get(key, 0)
        if rc != 0:
            return completed(rc, stderr=self.stderrs.get(key, ""))
        return completed(0, stdout=self._effect(cmd))

    def _key(self, cmd):
        prog = Path(cmd[0]).stem.lower()
        if prog == "vsdiagnostics":
            return f"vsdiag:{cmd[1]}"
        if prog == "cargo" and cmd[1:2] == ["build"]:
            return "cargo:build"
        if prog == "git" and cmd[1:2] == ["rev-parse"]:
            return "git:rev-parse"
        return prog

    def _effect(self, cmd):
        handlers = {
            "rustc": self._rustc_effect,
            "cargo": self._cargo_effect,
            "git": self._git_effect,
            "vsdiagnostics": self._vsdiag_effect,
        }
        handler = handlers.get(Path(cmd[0]).stem.lower())
        return handler(cmd) if handler else ""

    def _rustc_effect(self, cmd):
        return f"{self.host_tuple}\n" if cmd[1:] == ["--print", "host-tuple"] else "rustc 1.99.1-fake\n"

    def _cargo_effect(self, cmd):
        if cmd[1:] == ["--version"]:
            return "cargo 1.99.1-fake\n"
        self._emit_artifacts(cmd)
        return ""

    def _git_effect(self, cmd):
        return "deadbeef1234\n" if cmd[1:] == ["rev-parse", "HEAD"] else " M src/example.rs\n"

    def _vsdiag_effect(self, cmd):
        if cmd[1] == "stop" and self.write_diagsession:
            out = next(arg.split(":", 1)[1] for arg in cmd[2:] if arg.startswith("/output:"))
            Path(out).write_bytes(b"diagsession-bytes")
        return ""

    def _emit_artifacts(self, cmd):
        target_dir = Path(cmd[cmd.index("--target-dir") + 1])
        triple = cmd[cmd.index("--target") + 1]
        release = target_dir / triple / "release"
        release.mkdir(parents=True, exist_ok=True)
        if self.write_exe:
            (release / "ffm.exe").write_bytes(b"ffm-exe-bytes")
        if self.write_pdb:
            (release / "ffm.pdb").write_bytes(b"ffm-pdb-bytes")

    def launch_ffm(self, cmd, cwd):
        self.popened.append(([str(part) for part in cmd], str(cwd)))
        return _FakeChild(pid=4242, rc=self.ffm_rc)


class _FakeChild:
    def __init__(self, pid, rc):
        self.pid = pid
        self._rc = rc

    def poll(self):
        return None

    def wait(self):
        return self._rc


class WindowsProfilingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="ffm-winprof-test-")
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name) / "repo"
        self.repo.mkdir()
        self.host = FakeHost(self.temp.name)
        self.host.setup_vsdiag()

    def invoke(self, *argv, env=None):
        import profile_windows

        merged = {"FFM_VSDIAGNOSTICS": self.host.vsdiag}
        if env:
            merged.update(env)
        with redirect_stdout(io.StringIO()):
            return profile_windows.run_profile(
                list(argv),
                repo=self.repo,
                environ=merged,
                which=self.host.which,
                runner=self.host.run,
                spawner=self.host.launch_ffm,
            )

    def captures(self):
        profiles = self.repo / "target/profiles"
        return sorted(profiles.glob("ffm-*")) if profiles.is_dir() else []

    def test_missing_tools_stop_before_building_or_creating_a_capture(self):
        for tool in ("cargo", "rustc", "git"):
            saved = self.host.tools.pop(tool)
            try:
                rc, err = self.invoke()
            finally:
                self.host.tools[tool] = saved
            self.assertEqual(rc, 1)
            self.assertIn(f"Required command is missing: {tool}", err)
            self.assertEqual(self.host.calls, [])
            self.assertEqual(self.captures(), [])
        rc, err = self.invoke(env={"FFM_VSDIAGNOSTICS": ""})
        self.assertEqual(rc, 1)
        self.assertIn("Required command is missing: VSDiagnostics", err)
        self.assertEqual(self.captures(), [])

    def test_build_preserves_exe_and_pdb_and_records_diagsession(self):
        rc, err = self.invoke("--no-record")
        self.assertEqual(rc, 0, err)
        capture, = self.captures()
        self.assertEqual((capture / "ffm.exe").read_bytes(), b"ffm-exe-bytes")
        self.assertEqual((capture / "ffm.pdb").read_bytes(), b"ffm-pdb-bytes")
        self.assertEqual((capture / "profile.diagsession").read_bytes(), b"diagsession-bytes")
        meta = (capture / "metadata.txt").read_text(encoding="utf-8")
        self.assertIn("deadbeef1234", meta)
        self.assertIn("VSDiagnostics", meta)
        build_cmd, build_env = next(c for c in self.host.calls if c[0][1:2] == ["build"])
        self.assertIn("--release", build_cmd)
        self.assertEqual(build_env["CARGO_PROFILE_RELEASE_DEBUG"], "1")
        self.assertEqual(build_env["CARGO_PROFILE_RELEASE_STRIP"], "none")
        self.assertEqual(self.host.popened, [([str(capture / "ffm.exe"), "--no-record"], str(self.repo))])


if __name__ == "__main__":
    unittest.main()
