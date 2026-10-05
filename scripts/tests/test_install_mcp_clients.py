"""Check how install_mcp.py registers ffm-mcp: through Claude's and Codex's own
CLIs, and in the JSON configs of the other utilities.
"""

import json
import unittest

from install_mcp_case import InstallerTestCase


class ClientCliTests(InstallerTestCase):
    def setUp(self):
        super().setUp()
        self.binary = self.make_binary().as_posix()

    def test_claude_and_codex_replace_their_entry_through_their_cli(self):
        result = self.install("--no-build")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.sandbox.args("claude"), [
            ["mcp", "remove", "ffm-mcp"],
            ["mcp", "add", "--transport", "stdio", "ffm-mcp", "--", self.binary],
        ])
        self.assertEqual(self.sandbox.args("codex"), [
            ["mcp", "remove", "ffm-mcp"],
            ["mcp", "add", "ffm-mcp", "--", self.binary],
        ])

    def test_ffm_mcp_root_reaches_codex_only_when_set(self):
        cases = {"/data/root": ["--env", "FFM_MCP_ROOT=/data/root"], "": []}
        for value, env_option in cases.items():
            with self.subTest(FFM_MCP_ROOT=value):
                self.install("--no-build", env={"FFM_MCP_ROOT": value})
                add = ["mcp", "add", "ffm-mcp", *env_option, "--", self.binary]
                self.assertEqual(self.sandbox.args("codex")[-1], add)

    def test_missing_clis_are_skipped(self):
        self.sandbox.use_real("claude")
        self.sandbox.use_real("codex")
        # Only the sandbox's bin: a claude or codex installed in /usr/bin stays unseen.
        result = self.install("--no-build", env={"PATH": str(self.sandbox.bin)})
        self.assertEqual(result.returncode, 0, result.stderr)
        lines = result.stdout.splitlines()
        self.assertEqual(lines[2:6], [
            "Claude Code",
            "  ⊘ claude CLI not found, skipping",
            "Codex",
            "  ⊘ codex CLI not found, skipping",
        ])
        self.assertTrue((self.repo / ".gemini/settings.json").exists())

    def test_a_failed_remove_is_ignored_and_its_stderr_hidden(self):
        for cli in ("claude", "codex"):
            self.sandbox.fake(cli, effect="""
                if args[1] == "remove":
                    sys.stderr.write("No MCP server named ffm-mcp\\n")
                    sys.exit(1)
            """)
        result = self.install("--no-build")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, "")
        self.assertEqual([args[1] for args in self.sandbox.args("claude")], ["remove", "add"])
        self.assertEqual([args[1] for args in self.sandbox.args("codex")], ["remove", "add"])

    def check_not_executable(self, cli):
        # A CLI file without execute permission is still found, so the install
        # stops with 126 instead of skipping the utility.
        stub = self.sandbox.fake(cli)
        stub.chmod(0o644)
        # Only the sandbox's bin: a real claude or codex stays unseen.
        result = self.install("--no-build", env={"PATH": str(self.sandbox.bin)})
        self.assert_stopped_at(cli, result, 126, f"{stub}: Permission denied\n")

    def test_a_claude_without_execute_permission_stops_with_126(self):
        self.check_not_executable("claude")

    def test_a_codex_without_execute_permission_stops_with_126(self):
        self.check_not_executable("codex")

    def test_a_cli_whose_interpreter_is_missing_stops_with_127_at_add(self):
        stub = self.sandbox.bin / "claude"
        stub.write_text("#!/nonexistent/interpreter\n", encoding="utf-8")
        result = self.install("--no-build")
        # The failed remove says nothing; the add reports why.
        self.assert_stopped_at("claude", result, 127, f"{stub}: No such file or directory\n")
        self.assertEqual(self.sandbox.calls("codex"), [])

    def check_refused_by_the_kernel(self, first_line):
        # The install stops with 126 and nothing in the file is run.
        stub = self.sandbox.bin / "claude"
        ran = self.sandbox.root / "ran"
        stub.write_bytes(first_line + f"\ntouch '{ran}'\n".encode())
        result = self.install("--no-build")
        self.assert_stopped_at("claude", result, 126, f"{stub}: Exec format error\n")
        self.assertFalse(ran.exists(), "a line of the CLI file was run")

    def test_a_binary_cli_the_kernel_cannot_run_stops_with_126(self):
        # An ELF header the kernel rejects, like a native claude built for another machine.
        self.check_refused_by_the_kernel(b"\x7fELF\x02\x01\x01 not a real header")

    def test_a_cli_script_without_a_shebang_stops_with_126(self):
        self.check_refused_by_the_kernel(b'echo "a script without a #! line"')

    def check_failed_add(self, cli):
        self.sandbox.fake(cli, effect='if args[1] == "add": sys.exit(3)')
        self.assert_stopped_at(cli, self.install("--no-build"), 3, "")

    def test_a_failed_claude_add_stops_with_its_status(self):
        self.check_failed_add("claude")

    def test_a_failed_codex_add_stops_with_its_status(self):
        self.check_failed_add("codex")


class JsonConfigTests(InstallerTestCase):
    def setUp(self):
        super().setUp()
        self.binary = self.make_binary()
        self.windsurf = self.sandbox.home / ".codeium/windsurf/mcp_config.json"

    def test_windsurf_is_skipped_without_codeium(self):
        result = self.install("--no-build")
        self.assertIn("Windsurf\n  ⊘ not detected, skipping\n", result.stdout)
        self.assertFalse(self.windsurf.exists())
        self.assertFalse((self.sandbox.home / ".codeium").exists())

    def test_windsurf_is_skipped_when_codeium_cannot_be_checked(self):
        # Any error while checking ~/.codeium, such as EACCES under a HOME the
        # user cannot search, means not detected. A directory name longer
        # than NAME_MAX fails that check with ENAMETOOLONG, for root as well.
        home = self.sandbox.root / ("h" * 300)
        result = self.install("--no-build", env={"HOME": str(home)})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Windsurf\n  ⊘ not detected, skipping\n", result.stdout)
        self.assertTrue((self.repo / ".mcp.json").exists())

    def test_windsurf_is_configured_when_codeium_exists(self):
        (self.sandbox.home / ".codeium").mkdir()
        result = self.install("--no-build")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(f"Windsurf\n  ✓ Windsurf → {self.windsurf}\n", result.stdout)
        config = json.loads(self.windsurf.read_text(encoding="utf-8"))
        self.assertEqual(config, {"mcpServers": {"ffm-mcp": self.entry(self.binary)}})

    def test_an_upsert_keeps_everything_else_and_replaces_a_stale_entry(self):
        existing = {
            "theme": "dark",
            "mcpServers": {"other": {"command": "other-server"}, "ffm-mcp": {"command": "/old/checkout"}},
        }
        self.sandbox.write(".gemini/settings.json", json.dumps(existing))
        result = self.install("--no-build")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.read_json(".gemini/settings.json"), {
            "theme": "dark",
            "mcpServers": {"other": {"command": "other-server"}, "ffm-mcp": self.entry(self.binary)},
        })

    def test_an_unparsable_config_starts_over(self):
        self.sandbox.write(".cursor/mcp.json", "{ not json")
        result = self.install("--no-build")
        self.assertEqual(result.returncode, 0, result.stderr)
        expected = {"mcpServers": {"ffm-mcp": self.entry(self.binary)}}
        self.assertEqual(self.read_json(".cursor/mcp.json"), expected)


if __name__ == "__main__":
    unittest.main()
