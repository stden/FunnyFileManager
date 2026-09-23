"""Run install_mcp.py in a sandbox: fake Cargo, Claude and Codex, a throwaway checkout."""

import importlib.util
import json
import os
import shutil
import signal
import subprocess
import sys
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from sandbox import REPO, SandboxTestCase


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
        self.sandbox.copy(SCRIPT)
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


class SignalTests(InstallerTestCase):
    """A step killed by signal N ends the install with 128 + N and is named on stderr."""

    def assert_killed(self, result):
        self.assertEqual(result.returncode, 128 + 9)
        self.assertTrue(result.stderr.startswith("Killed  "), result.stderr)
        self.assert_no_config_written()

    def test_a_killed_build(self):
        self.fake_cargo(self.target, build_effect=KILL)
        result = self.install()
        self.assert_killed(result)
        self.assertEqual(result.stderr, "Killed  cargo build --release -j1\n")

    def test_a_killed_add(self):
        binary = self.make_binary()
        self.sandbox.fake("claude", effect=f'if args[1] == "add": {KILL}')
        result = self.install("--no-build")
        self.assert_killed(result)
        claude = self.sandbox.bin / "claude"
        self.assertIn(f"{claude} mcp add --transport stdio ffm-mcp -- '{binary}'", result.stderr)

    def test_a_killed_cargo_metadata(self):
        self.sandbox.fake("cargo", effect=KILL)
        result = self.install("--no-build")
        self.assert_killed(result)
        self.assertEqual(result.stdout, "")


class CtrlCTests(InstallerTestCase):
    """Ctrl-C reaches the running step as well, and the step decides what it means."""

    def setUp(self):
        super().setUp()
        self.make_binary()
        self.started = self.sandbox.root / "started"

    def press_ctrl_c_during_add(self, on_sigint):
        """Run the installer; once `claude mcp add` has started, send SIGINT
        to the installer's whole process group, as a terminal does."""
        effect = CTRL_C_ADD.format(on_sigint=on_sigint, started=str(self.started))
        self.sandbox.fake("claude", effect=effect)
        process = subprocess.Popen(
            [sys.executable, self.repo / SCRIPT, "--no-build"],
            cwd=self.repo, env=self.sandbox.env(), start_new_session=True,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8",
        )
        with process:
            deadline = time.monotonic() + 30
            while not self.started.exists():
                self.assertIsNone(process.poll(), "the installer ended before `claude mcp add` started")
                self.assertLess(time.monotonic(), deadline, "`claude mcp add` never started")
                time.sleep(0.02)
            os.killpg(process.pid, signal.SIGINT)
            stdout, stderr = process.communicate(timeout=60)
        return subprocess.CompletedProcess(process.args, process.returncode, stdout, stderr)

    def test_the_install_waits_for_a_step_that_cleans_up_first(self):
        finished = self.sandbox.root / "finished"
        result = self.press_ctrl_c_during_add(
            f'time.sleep(1); open({str(finished)!r}, "w").close(); sys.exit(130)'
        )
        self.assertTrue(finished.exists(), "the step was killed before it had cleaned up")
        self.assert_stopped_at("claude", result, 130, "")

    def test_a_step_killed_by_ctrl_c_ends_the_install_by_sigint_too(self):
        result = self.press_ctrl_c_during_add(
            "signal.signal(signal.SIGINT, signal.SIG_DFL); os.kill(os.getpid(), signal.SIGINT)"
        )
        # Killed by SIGINT, so a calling script sees the interrupt; no traceback.
        self.assert_stopped_at("claude", result, -signal.SIGINT, "")

    def test_the_install_goes_on_when_a_step_exits_0_on_ctrl_c(self):
        result = self.press_ctrl_c_during_add("sys.exit(0)")
        self.assertEqual((result.returncode, result.stderr), (0, ""))
        self.assertTrue((self.repo / ".mcp.json").exists())

    def test_a_step_starts_with_the_default_ctrl_c_unless_it_was_ignored(self):
        self.sandbox.fake("claude", effect="""
            import signal
            if args[1] == "add":
                print("Ctrl-C ignored:", signal.getsignal(signal.SIGINT) is signal.SIG_IGN)
        """)
        for ignored in (False, True):
            with self.subTest(ignored=ignored):
                # The installer inherits this test's disposition.
                disposition = signal.SIG_IGN if ignored else signal.default_int_handler
                handler = signal.signal(signal.SIGINT, disposition)
                try:
                    result = self.install("--no-build")
                finally:
                    signal.signal(signal.SIGINT, handler)
                self.assertIn(f"Ctrl-C ignored: {ignored}\n", result.stdout)


class EncodingTests(InstallerTestCase):
    def test_paths_that_are_not_utf8_are_printed_as_their_own_bytes(self):
        # Outside UTF-8 mode, a UTF-8 locale gives Python a strict stdout.
        checkout = self.sandbox.root / os.fsdecode(b"check\xffout")
        home = self.sandbox.root / os.fsdecode(b"h\xffome")
        (checkout / "mcp").mkdir(parents=True)
        (home / ".codeium").mkdir(parents=True)
        shutil.copy2(self.repo / SCRIPT, checkout / SCRIPT)
        self.fake_cargo(checkout / "target")
        binary = self.make_binary(checkout / "target")
        environment = self.sandbox.env({"PYTHONUTF8": "0", "LC_ALL": "C.UTF-8", "HOME": str(home)})
        # Bytes, not text: the output is deliberately not UTF-8.
        result = subprocess.run(
            [sys.executable, checkout / SCRIPT, "--no-build"],
            cwd=checkout, env=environment, capture_output=True, timeout=60, check=False,
        )
        self.assertEqual((result.returncode, result.stderr), (0, b""))
        lines = result.stdout.splitlines()
        gemini = checkout / ".gemini/settings.json"
        windsurf = home / ".codeium/windsurf/mcp_config.json"
        self.assertIn("  ✓ Gemini → ".encode() + os.fsencode(gemini), lines)
        self.assertIn("  ✓ Windsurf → ".encode() + os.fsencode(windsurf), lines)
        self.assertEqual(lines[-1], b"Binary: " + os.fsencode(binary))
        # JSON escapes the stray byte.
        config = json.loads(gemini.read_text(encoding="utf-8"))
        self.assertEqual(config["mcpServers"]["ffm-mcp"], self.entry(binary))

    def test_a_config_is_read_as_utf8_under_an_ascii_locale(self):
        # With UTF-8 mode off, LC_ALL=C makes the locale's encoding ASCII. A
        # read in that encoding would fail on this config and drop the other
        # server along with it.
        self.make_binary()
        other = {"command": "café ✓"}
        config = json.dumps({"mcpServers": {"other": other}}, ensure_ascii=False)
        self.sandbox.write(".cursor/mcp.json", config)
        result = self.install("--no-build", env={"PYTHONUTF8": "0", "LC_ALL": "C"})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.read_json(".cursor/mcp.json")["mcpServers"]["other"], other)


class ArgumentTests(InstallerTestCase):
    def test_an_unknown_argument_is_rejected_before_cargo_runs(self):
        self.make_binary()
        result = self.install("--no-build", "--bogus")
        self.assertEqual(result.returncode, 2)
        self.assertIn("unrecognized arguments: --bogus", result.stderr)
        self.assertEqual(self.sandbox.calls(), [])
        self.assert_no_config_written()

    def test_an_abbreviation_of_no_build_is_rejected(self):
        # argparse would otherwise read `--no` as --no-build and skip the build.
        self.make_binary()
        for argument in ("--no", "--no-b"):
            with self.subTest(argument=argument):
                result = self.install(argument)
                self.assertEqual(result.returncode, 2)
                self.assertIn(f"unrecognized arguments: {argument}", result.stderr)
                self.assertEqual(self.sandbox.calls(), [])

    def test_help_describes_the_installer_without_running_it(self):
        result = self.install("--help")
        self.assertEqual(result.returncode, 0)
        self.assertIn("Install or refresh the ffm-mcp server", result.stdout)
        self.assertIn("--no-build", result.stdout)
        # argparse writes the one usage line.
        self.assertEqual(result.stdout.lower().count("usage:"), 1, result.stdout)
        self.assertEqual(self.sandbox.calls(), [])


class EntryPointTests(unittest.TestCase):
    def test_the_installer_runs_as_a_program(self):
        # The other tests start it with sys.executable, which skips both checks.
        script = REPO / SCRIPT
        self.assertEqual(script.read_text(encoding="utf-8").splitlines()[0], "#!/usr/bin/env python3")
        self.assertTrue(os.access(script, os.X_OK), f"{script} is not executable")


if __name__ == "__main__":
    unittest.main()
