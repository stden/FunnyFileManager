"""Exercise installer artifact discovery without building or contacting AI tools."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


class InstallerArtifactTests(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="ffm-installer-")
        self.addCleanup(self.scratch.cleanup)
        self.base = Path(self.scratch.name).resolve()
        self.repo = self.base / "checkout with spaces"
        (self.repo / "mcp").mkdir(parents=True)
        source = Path(__file__).resolve().parents[2] / "install-mcp.sh"
        shutil.copyfile(source, self.repo / "install-mcp.sh")
        self.commands = self.base / "commands"
        self.commands.mkdir()
        self.metadata = self.base / "metadata.json"
        self.home = self.base / "home"
        self.home.mkdir()
        self.write_command("cargo", """#!/bin/sh
if [ "$1" != "metadata" ]; then
    echo "unexpected Cargo build in a no-build test" >&2
    exit 19
fi
cat "$FFM_TEST_METADATA"
""")
        # These stubs guarantee a test never registers with installed clients.
        for name in ["claude", "codex"]:
            self.write_command(name, "#!/bin/sh\nexit 0\n")
        self.write_command("python3", '#!/bin/sh\nexec "$FFM_TEST_PYTHON" "$@"\n')

    def write_command(self, name, content):
        command = self.commands / name
        command.write_text(content, encoding="utf-8", newline="\n")
        command.chmod(0o755)

    def run_installer(self, target, create_binary=True):
        name = "ffm-mcp.exe" if os.name == "nt" else "ffm-mcp"
        binary = target / "release" / name
        if create_binary:
            binary.parent.mkdir(parents=True)
            binary.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            binary.chmod(0o755)
        self.metadata.write_text(
            json.dumps({"target_directory": str(target)}), encoding="utf-8"
        )
        environment = dict(os.environ)
        environment.update({
            "PATH": str(self.commands) + os.pathsep + environment.get("PATH", ""),
            "HOME": str(self.home),
            "FFM_TEST_METADATA": str(self.metadata),
            "FFM_TEST_PYTHON": sys.executable,
            "CARGO_TARGET_DIR": str(target),
        })
        bash = environment.get("FFM_TEST_BASH", "bash")
        if os.name == "nt" and "FFM_TEST_BASH" not in environment:
            git_bash = Path(environment.get("ProgramFiles", "C:/Program Files")) / "Git/bin/bash.exe"
            if git_bash.is_file():
                bash = str(git_bash)
        result = subprocess.run(
            [bash, str(self.repo / "install-mcp.sh"), "--no-build"],
            cwd=self.repo, env=environment, capture_output=True, text=True,
            encoding="utf-8", timeout=30,
        )
        return result, binary

    def check_registered(self, binary):
        local = json.loads((self.repo / ".gemini/settings.json").read_text())
        configured = local["mcpServers"]["ffm-mcp"]["command"]
        self.assertEqual(Path(configured).resolve(), binary.resolve())
        return json.loads((self.repo / ".mcp.json").read_text())[
            "mcpServers"]["ffm-mcp"]["command"]

    def test_shared_target_registers_a_repo_relative_command(self):
        result, binary = self.run_installer(self.repo / "target")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(
            self.check_registered(binary), binary.relative_to(self.repo).as_posix()
        )

    def test_external_target_registers_the_actual_binary(self):
        result, binary = self.run_installer(self.base / "external artifacts")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.check_registered(binary), binary.as_posix())

    def test_missing_binary_stops_before_writing_client_configs(self):
        result, _ = self.run_installer(self.repo / "target", create_binary=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not exist", result.stderr)
        self.assertFalse((self.repo / ".mcp.json").exists())
        self.assertFalse((self.repo / ".gemini/settings.json").exists())


if __name__ == "__main__":
    unittest.main()
