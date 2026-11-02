"""The JSON configs that install_mcp.py writes the ffm-mcp entry into: one per
utility that reads a project or a home config, and the shared .mcp.json.
"""

import json
from pathlib import Path


# The checkout root: the directory of install_mcp.py, just above scripts/.
ROOT = Path(__file__).resolve().parents[1]
SERVER = "ffm-mcp"


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
