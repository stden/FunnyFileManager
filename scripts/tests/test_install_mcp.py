"""Run install_mcp.py in a sandbox: fake Cargo, Claude and Codex, a throwaway checkout.

Where the binary is, the output and the build; the other cases have
test_install_mcp_*.py files of their own, and install_mcp_case.py holds what
they share.
"""

import importlib.util
import json
import subprocess
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from install_mcp_case import JSON_CONFIGS, METADATA, SCRIPT, InstallerTestCase


class ArtifactTests(InstallerTestCase):
    """Where the binary is looked for, and a binary that is missing."""

    def test_shared_target_registers_a_repo_relative_command(self):
        binary = self.make_binary()
        result = self.install("--no-build")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.registered(".gemini/settings.json"), self.entry(binary))
        self.assertEqual(self.registered(".mcp.json")["command"], "target/release/ffm-mcp")

    def test_external_target_registers_the_actual_binary(self):
        external = self.sandbox.root / "external artifacts"
        self.fake_cargo(external)
        binary = self.make_binary(external)
        result = self.install("--no-build")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.registered(".gemini/settings.json"), self.entry(binary))
        self.assertEqual(self.registered(".mcp.json")["command"], binary.as_posix())

    def test_missing_binary_stops_before_writing_client_configs(self):
        result = self.install("--no-build")
        binary = self.target / "release" / "ffm-mcp"
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stderr, f"error: {binary} does not exist; run without --no-build first\n")
        self.assert_no_config_written()
        self.assertEqual(self.sandbox.calls("claude"), [])

    def test_a_binary_without_execute_permission_is_refused(self):
        self.make_binary(mode=0o644)
        result = self.install("--no-build")
        self.assertEqual(result.returncode, 1)
        self.assertIn("does not exist; run without --no-build first", result.stderr)
        self.assert_no_config_written()

    def test_windows_names_the_binary_ffm_mcp_exe(self):
        spec = importlib.util.spec_from_file_location("install_mcp", self.repo / SCRIPT)
        installer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(installer)
        metadata = subprocess.CompletedProcess([], 0, stdout=b'{"target_directory": "/cargo target"}')
        with (
            patch.object(installer.subprocess, "run", return_value=metadata) as run,
            patch.object(installer, "os", SimpleNamespace(name="nt", environ={})),
        ):
            self.assertEqual(installer.release_binary(), "/cargo target/release/ffm-mcp.exe")
        self.assertEqual(run.call_args.args[0], ["cargo", *METADATA])
        self.assertEqual(run.call_args.kwargs["cwd"], self.repo / "mcp")


class OutputTests(InstallerTestCase):
    def test_a_full_run_reports_every_utility_in_order(self):
        binary = self.make_binary()
        self.sandbox.fake("claude", effect='if args[1] == "add": print("Added stdio MCP server")')
        result = self.install("--no-build")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, "")
        configured = [
            line for name, relative in JSON_CONFIGS.items()
            for line in (name, f"  ✓ {name} → {self.repo / relative}")
        ]
        self.assertEqual(result.stdout.splitlines(), [
            "=== FFM MCP Server Installer ===",
            "",
            "Claude Code",
            "Added stdio MCP server",
            "  ✓ Claude Code (claude mcp)",
            "Codex",
            "  ✓ Codex (codex mcp)",
            *configured,
            "Windsurf",
            "  ⊘ not detected, skipping",
            "Shared .mcp.json",
            "  ✓ Shared config (.mcp.json)",
            "",
            "=== Done! ffm-mcp registered for every detected utility ===",
            f"Binary: {binary.as_posix()}",
        ])

    def test_every_project_config_is_written_in_the_same_format(self):
        binary = self.make_binary()
        self.install("--no-build")
        for relative in JSON_CONFIGS.values():
            with self.subTest(config=relative):
                expected = {"mcpServers": {"ffm-mcp": self.entry(binary)}}
                text = (self.repo / relative).read_text(encoding="utf-8")
                self.assertEqual(text, json.dumps(expected, indent=2) + "\n")

    def test_the_shared_config_is_rewritten_in_full(self):
        self.make_binary()
        self.sandbox.write(".mcp.json", '{"mcpServers": {"other": {"command": "x"}}, "extra": 1}\n')
        self.install("--no-build")
        expected = {"mcpServers": {"ffm-mcp": {
            "type": "stdio", "command": "target/release/ffm-mcp", "args": [], "env": {},
        }}}
        text = (self.repo / ".mcp.json").read_text(encoding="utf-8")
        self.assertEqual(text, json.dumps(expected, indent=2) + "\n")


class BuildTests(InstallerTestCase):
    def test_a_build_runs_cargo_in_mcp_before_registering(self):
        binary = self.make_binary()
        result = self.install()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines()[:5], [
            "=== FFM MCP Server Installer ===",
            "",
            "Building release binary...",
            f"Built: {binary.as_posix()}",
            "",
        ])
        self.assertEqual(self.sandbox.args("cargo"), [METADATA, ["build", "--release", "-j1"]])
        calls = self.sandbox.calls("cargo")
        self.assertEqual({call.cwd for call in calls}, {str(self.repo / "mcp")})
        # PWD names mcp/ too, for tools that read it.
        self.assertEqual({call.env.get("PWD") for call in calls}, {str(self.repo / "mcp")})

    def test_no_build_asks_cargo_only_for_metadata(self):
        self.make_binary()
        result = self.install("--no-build")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.sandbox.args("cargo"), [METADATA])
        self.assertNotIn("Building release binary...", result.stdout)

    def test_a_failed_build_exits_with_cargo_status_before_any_config(self):
        self.fake_cargo(self.target, build_status=101)
        result = self.install()
        self.assertEqual(result.returncode, 101)
        self.assertTrue(result.stdout.endswith("Building release binary...\n"), result.stdout)
        self.assert_no_config_written()
        self.assertEqual(self.sandbox.calls("claude"), [])

    def test_failing_cargo_metadata_stops_with_cargo_status_before_any_output(self):
        self.sandbox.fake("cargo", status=101, stderr="error: could not find `Cargo.toml`\n")
        result = self.install("--no-build")
        self.assertEqual(result.returncode, 101)
        self.assertEqual(result.stdout, "")
        self.assertEqual(result.stderr, "error: could not find `Cargo.toml`\n")
        self.assert_no_config_written()

    def test_a_missing_cargo_stops_with_127_and_names_it(self):
        self.make_binary()
        self.sandbox.use_real("cargo")
        # Only the sandbox's bin, so no real Cargo can be found.
        result = self.install("--no-build", env={"PATH": str(self.sandbox.bin)})
        self.assertEqual((result.returncode, result.stderr), (127, "cargo: No such file or directory\n"))
        self.assertEqual(result.stdout, "")
        self.assert_no_config_written()


if __name__ == "__main__":
    unittest.main()
