#!/usr/bin/env python3
"""Install or refresh the ffm-mcp server in every detected AI utility.

Builds mcp/ in release mode, registers the binary with Claude Code and Codex
through their own CLIs, writes it into the Gemini, Junie, Kiro and Cursor
project configs and, when Windsurf is installed, into its config, then
rewrites the shared .mcp.json.

The entry is upserted on every run: an existing ffm-mcp entry is replaced,
never skipped, so a config carried over from another machine (or an old
checkout path) self-heals instead of keeping a stale absolute path.
"""

import argparse
import errno
import json
import os
from pathlib import Path
import shlex
import shutil
import signal
import subprocess
import sys


ROOT = Path(__file__).resolve().parent
MCP_DIR = ROOT / "mcp"
SERVER = "ffm-mcp"


def run_step(command, *, check=True, **options):
    """Run one step of the install to the end and return its CompletedProcess.

    - Ctrl-C reaches the step as well, and the step decides what it means:
      the installer waits for it, and raises KeyboardInterrupt only if the
      step died of SIGINT.
    - A step that cannot start gets status 127 when a file it needs is
      missing (the program, its #! interpreter, its working directory), 126
      otherwise. The reason is printed on stderr unless the step's stderr is
      redirected.
    - With check, a step that fails raises CalledProcessError.
    """
    handler = signal.getsignal(signal.SIGINT)
    if handler is not signal.SIG_IGN:
        # Wait through Ctrl-C. A new program never inherits this handler, so
        # the step starts with the default Ctrl-C. SIG_IGN is inherited, so an
        # ignored Ctrl-C is left alone and stays ignored in the step too.
        signal.signal(signal.SIGINT, lambda signal_number, frame: None)
    try:
        result = subprocess.run(command, **options)
    except OSError as error:
        if options.get("stderr") is None:
            print(f"{error.filename}: {error.strerror}", file=sys.stderr)
        result = subprocess.CompletedProcess(command, 127 if error.errno == errno.ENOENT else 126)
    finally:
        signal.signal(signal.SIGINT, handler)
    if result.returncode == -signal.SIGINT:
        raise KeyboardInterrupt
    if check:
        result.check_returncode()
    return result


def run_in_mcp_dir(command, **options):
    """Run a Cargo step in mcp/, with PWD naming it too for tools that read PWD."""
    environment = {**os.environ, "PWD": str(MCP_DIR)}
    return run_step(command, cwd=MCP_DIR, env=environment, **options)


def release_binary():
    """The release binary's path, in POSIX form.

    Asks Cargo instead of assuming each package has its own target directory.
    This also honors CARGO_TARGET_DIR and developer Cargo configuration.
    """
    metadata = run_in_mcp_dir(
        ["cargo", "metadata", "--no-deps", "--format-version=1"], stdout=subprocess.PIPE
    ).stdout
    target = Path(json.loads(metadata)["target_directory"])
    name = "ffm-mcp.exe" if os.name == "nt" else "ffm-mcp"
    return (target / "release" / name).as_posix()


def build(binary):
    print("Building release binary...")
    run_in_mcp_dir(["cargo", "build", "--release", "-j1"])
    print(f"Built: {binary}")
    print()


def find_cli(cli):
    """The CLI's path: an executable on PATH, else a file there without execute
    permission, which then fails to start instead of being skipped."""
    return shutil.which(cli) or shutil.which(cli, mode=os.F_OK)


def register_with_cli(title, cli, add_arguments):
    """Replace the ffm-mcp entry through a utility's own `mcp` subcommand."""
    print(title)
    path = find_cli(cli)
    if not path:
        print(f"  ⊘ {cli} CLI not found, skipping")
        return
    # The remove turns the add into a replace. It fails when there is no
    # entry yet, so its status and its stderr are ignored.
    run_step([path, "mcp", "remove", SERVER], stderr=subprocess.DEVNULL, check=False)
    run_step([path, "mcp", "add", *add_arguments])
    print(f"  ✓ {title} ({cli} mcp)")


def json_configs():
    """(name, config file, detect) rows for the JSON-configured utilities.

    detect None: a project-scoped config, always (re)written.
    detect a path: whose presence means the utility is installed; the row is
    skipped otherwise.
    """
    home = Path.home()
    return [
        ("Gemini", ROOT / ".gemini" / "settings.json", None),
        ("Junie", ROOT / ".junie" / "mcp.json", None),
        ("Kiro", ROOT / ".kiro" / "settings" / "mcp.json", None),
        ("Cursor", ROOT / ".cursor" / "mcp.json", None),
        ("Windsurf", home / ".codeium" / "windsurf" / "mcp_config.json", home / ".codeium"),
    ]


def write_json(path, config):
    """Write a config the one way this installer writes them: indent 2, final newline."""
    path.write_text(json.dumps(config, indent=2) + "\n", encoding="utf-8")


def upsert_config(path, binary):
    """Insert or replace the ffm-mcp entry, preserving everything else in the file.

    A file that does not exist or does not parse is (re)created from scratch.
    """
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        config = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        config = {}
    config.setdefault("mcpServers", {})[SERVER] = {"command": binary, "args": [], "env": {}}
    write_json(path, config)


def write_shared_config(binary):
    """Rewrite the shared project .mcp.json (VS Code, Cursor, Windsurf, etc.).

    Repository-local artifacts use a portable relative command. An explicit
    external target directory needs its resolved absolute command instead.
    """
    resolved = Path(binary).resolve()
    try:
        command = resolved.relative_to(ROOT).as_posix()
    except ValueError:
        command = resolved.as_posix()
    server = {"type": "stdio", "command": command, "args": [], "env": {}}
    write_json(ROOT / ".mcp.json", {"mcpServers": {SERVER: server}})


def main(argv=None):
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
        allow_abbrev=False,
    )
    parser.add_argument(
        "--no-build", action="store_true", help="skip the build; register the binary already built"
    )
    arguments = parser.parse_args(argv)
    # UTF-8 whatever the locale, for the ✓ and ⊘ marks. A path that is not
    # UTF-8 goes out as its own bytes.
    sys.stdout.reconfigure(encoding="utf-8", errors="surrogateescape", line_buffering=True)
    sys.stderr.reconfigure(encoding="utf-8", errors="surrogateescape")

    binary = release_binary()
    print("=== FFM MCP Server Installer ===")
    print()
    if not arguments.no_build:
        build(binary)
    if not os.access(binary, os.X_OK):
        print(f"error: {binary} does not exist; run without --no-build first", file=sys.stderr)
        return 1

    register_with_cli("Claude Code", "claude", ["--transport", "stdio", SERVER, "--", binary])
    # Codex keeps its servers in the shared ~/.codex/config.toml and should be
    # configured through its CLI; it does not read ~/.codex/mcp_config.json.
    mcp_root = os.environ.get("FFM_MCP_ROOT")
    codex_env = ["--env", f"FFM_MCP_ROOT={mcp_root}"] if mcp_root else []
    register_with_cli("Codex", "codex", [SERVER, *codex_env, "--", binary])

    for name, config_file, detect in json_configs():
        print(name)
        # os.path.exists: an error such as EACCES means not detected, where
        # Path.exists would raise it.
        if detect and not os.path.exists(detect):
            print("  ⊘ not detected, skipping")
            continue
        upsert_config(config_file, binary)
        print(f"  ✓ {name} → {config_file}")

    print("Shared .mcp.json")
    write_shared_config(binary)
    print("  ✓ Shared config (.mcp.json)")

    print()
    print("=== Done! ffm-mcp registered for every detected utility ===")
    print(f"Binary: {binary}")
    return 0


def exit_status(error):
    """The installer's status after a failed step: the step's own, or 128 + N
    after signal N, which is then named on stderr with the step."""
    if error.returncode >= 0:
        return error.returncode
    signal_number = -error.returncode
    command = shlex.join(map(str, error.cmd))
    print(f"{signal.strsignal(signal_number)}  {command}", file=sys.stderr)
    return 128 + signal_number


if __name__ == "__main__":
    try:
        sys.exit(main())
    except subprocess.CalledProcessError as error:
        sys.exit(exit_status(error))
    except KeyboardInterrupt:
        # End by SIGINT, without a traceback, so that a calling script sees
        # the interrupt and stops as well.
        signal.signal(signal.SIGINT, signal.SIG_DFL)
        os.kill(os.getpid(), signal.SIGINT)
