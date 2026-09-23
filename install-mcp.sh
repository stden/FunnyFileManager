#!/bin/bash
# Purpose: Installs or refreshes MCP server integration for every detected AI utility.
#
# One source of truth: the CONFIGS table below. Each row is "Name|path|detect".
#   detect empty     — project-scoped config, always (re)written
#   detect non-empty — a command or path whose presence means the utility is
#                      installed; skipped otherwise
#
# The entry is UPSERTED on every run: an existing ffm-mcp entry is replaced,
# never skipped, so a config carried over from another machine (or an old
# checkout path) self-heals instead of keeping a stale absolute path.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MCP_DIR="$SCRIPT_DIR/mcp"
# Ask Cargo instead of assuming each package has its own target directory.
# This also honors CARGO_TARGET_DIR and developer Cargo configuration.
BINARY="$(cd "$MCP_DIR" && cargo metadata --no-deps --format-version=1 | python3 -c '
import json, os, pathlib, sys
target = pathlib.Path(json.load(sys.stdin)["target_directory"])
name = "ffm-mcp.exe" if os.name == "nt" else "ffm-mcp"
print((target / "release" / name).as_posix())
')"

echo "=== FFM MCP Server Installer ==="
echo ""

if [ "${1:-}" != "--no-build" ]; then
    echo "Building release binary..."
    (cd "$MCP_DIR" && cargo build --release -j1)
    echo "Built: $BINARY"
    echo ""
fi
if [ ! -x "$BINARY" ]; then
    echo "error: $BINARY does not exist; run without --no-build first" >&2
    exit 1
fi

# Insert or replace the ffm-mcp entry, preserving everything else in the file.
# A file that does not exist or does not parse is (re)created from scratch.
upsert_config() {
    local name="$1" config_file="$2"
    mkdir -p "$(dirname "$config_file")"
    BINARY="$BINARY" CONFIG_FILE="$config_file" python3 - <<'PY'
import json, os

path = os.environ["CONFIG_FILE"]
try:
    with open(path) as f:
        config = json.load(f)
except (OSError, ValueError):
    config = {}

config.setdefault("mcpServers", {})["ffm-mcp"] = {
    "command": os.environ["BINARY"],
    "args": [],
    "env": {},
}
with open(path, "w") as f:
    json.dump(config, f, indent=2)
    f.write("\n")
PY
    echo "  ✓ $name → $config_file"
}

# ── Claude Code registers through its own CLI ───────────────────
echo "Claude Code"
if command -v claude &>/dev/null; then
    claude mcp remove ffm-mcp 2>/dev/null || true
    claude mcp add --transport stdio ffm-mcp -- "$BINARY"
    echo "  ✓ Claude Code (claude mcp)"
else
    echo "  ⊘ claude CLI not found, skipping"
fi

# ── Everything else is table-driven ─────────────────────────────
# Codex uses the shared ~/.codex/config.toml and should be configured through
# its CLI. The older ~/.codex/mcp_config.json file is not read by Codex.
echo "Codex"
if command -v codex &>/dev/null; then
    codex mcp remove ffm-mcp 2>/dev/null || true
    if [ -n "${FFM_MCP_ROOT:-}" ]; then
        codex mcp add ffm-mcp --env "FFM_MCP_ROOT=$FFM_MCP_ROOT" -- "$BINARY"
    else
        codex mcp add ffm-mcp -- "$BINARY"
    fi
    echo "  ✓ Codex (codex mcp)"
else
    echo "  ⊘ codex CLI not found, skipping"
fi

# Remaining JSON-configured integrations are table-driven.
CONFIGS="
Gemini|$SCRIPT_DIR/.gemini/settings.json|
Junie|$SCRIPT_DIR/.junie/mcp.json|
Kiro|$SCRIPT_DIR/.kiro/settings/mcp.json|
Cursor|$SCRIPT_DIR/.cursor/mcp.json|
Windsurf|$HOME/.codeium/windsurf/mcp_config.json|$HOME/.codeium
"

while IFS='|' read -r name config detect; do
    [ -z "$name" ] && continue
    echo "$name"
    if [ -n "$detect" ] && ! command -v "$detect" &>/dev/null && [ ! -e "$detect" ]; then
        echo "  ⊘ not detected, skipping"
        continue
    fi
    upsert_config "$name" "$config"
done <<< "$CONFIGS"

# ── Shared project .mcp.json (VS Code, Cursor, Windsurf, etc.) ──
# Repository-local artifacts use a portable relative command. An explicit
# external target directory needs its resolved absolute command instead.
echo "Shared .mcp.json"
BINARY="$BINARY" SCRIPT_DIR="$SCRIPT_DIR" python3 - <<'PY'
import json, os, pathlib

root = pathlib.Path(os.environ["SCRIPT_DIR"]).resolve()
binary = pathlib.Path(os.environ["BINARY"]).resolve()
try:
    command = binary.relative_to(root).as_posix()
except ValueError:
    command = binary.as_posix()
config = {"mcpServers": {"ffm-mcp": {
    "type": "stdio", "command": command, "args": [], "env": {}
}}}
with (root / ".mcp.json").open("w", encoding="utf-8") as output:
    json.dump(config, output, indent=2)
    output.write("\n")
PY
echo "  ✓ Shared config (.mcp.json)"

echo ""
echo "=== Done! ffm-mcp registered for every detected utility ==="
echo "Binary: $BINARY"
