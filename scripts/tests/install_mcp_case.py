"""The sandbox checkout, the fake Cargo and the expected configs that the tests
of install_mcp.py share.
"""

import json
import shutil

from sandbox import SandboxTestCase
from script_files import INSTALL_MCP


SCRIPT = "install_mcp.py"
METADATA = ["metadata", "--no-deps", "--format-version=1"]
# Kills the fake with SIGKILL, as the OOM killer ends a build.
KILL = "import signal; os.kill(os.getpid(), signal.SIGKILL)"
CLI_TITLES = {"claude": "Claude Code", "codex": "Codex"}
# A fake `claude mcp add` that marks itself started, then sleeps until Ctrl-C
# runs `on_sigint`.
CTRL_C_ADD = """
if args[1] == "add":
    import signal, time
    def on_sigint(signal_number, frame):
        {on_sigint}
    signal.signal(signal.SIGINT, on_sigint)
    open({started!r}, "w").close()
    time.sleep(30)
"""
JSON_CONFIGS = {
    "Gemini": ".gemini/settings.json",
    "Junie": ".junie/mcp.json",
    "Kiro": ".kiro/settings/mcp.json",
    "Cursor": ".cursor/mcp.json",
}


class InstallerTestCase(SandboxTestCase):
    def setUp(self):
        super().setUp()
        self.repo = self.sandbox.repo
        # The installer and the module that it imports.
        for relative in INSTALL_MCP:
            self.sandbox.copy(relative)
        (self.repo / "mcp").mkdir()
        self.target = self.repo / "target"
        self.fake_cargo(self.target)
        self.sandbox.fake("claude")
        self.sandbox.fake("codex")

    def fake_cargo(self, target, build_status=0, build_effect=""):
        """Cargo whose metadata names `target`; every other call runs `build_effect`
        and exits `build_status`."""
        metadata = json.dumps({"target_directory": str(target)})
        self.sandbox.fake("cargo", status=build_status, effect=f"""
            if args[0] == "metadata":
                print({metadata!r})
                sys.exit(0)
            {build_effect}
        """)

    def make_binary(self, target=None, mode=0o755):
        binary = (target or self.target) / "release" / "ffm-mcp"
        binary.parent.mkdir(parents=True, exist_ok=True)
        binary.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        binary.chmod(mode)
        return binary

    def install(self, *args, env=None):
        return self.sandbox.run(SCRIPT, *args, env=env)

    def read_json(self, relative):
        return json.loads((self.repo / relative).read_text(encoding="utf-8"))

    def entry(self, binary):
        return {"command": binary.as_posix(), "args": [], "env": {}}

    def registered(self, relative):
        return self.read_json(relative)["mcpServers"]["ffm-mcp"]

    def assert_no_config_written(self):
        for relative in [*JSON_CONFIGS.values(), ".mcp.json"]:
            self.assertFalse((self.repo / relative).exists(), relative)

    def assert_stopped_at(self, cli, result, status, stderr):
        """The install got as far as `cli`, then stopped with `status`, having written no config."""
        self.assertEqual((result.returncode, result.stderr), (status, stderr))
        self.assertTrue(result.stdout.endswith(f"{CLI_TITLES[cli]}\n"), result.stdout)
        self.assert_no_config_written()

    def copy_installer(self, checkout):
        """Copy the installer and the module that it imports into another checkout."""
        for relative in INSTALL_MCP:
            (checkout / relative).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(self.repo / relative, checkout / relative)
