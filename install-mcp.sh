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
BINARY="$MCP_DIR/target/release/ffm-mcp"

echo "=== FFM MCP Server Installer ==="
echo ""

if [ "${1:-}" != "--no-build" ]; then
    echo "Building release binary..."
    (cd "$MCP_DIR" && cargo build --release -j1)
    echo "Built: $BINARY"
    echo ""
elif [ ! -x "$BINARY" ]; then
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
CONFIGS="
Gemini|$SCRIPT_DIR/.gemini/settings.json|
Junie|$SCRIPT_DIR/.junie/mcp.json|
Kiro|$SCRIPT_DIR/.kiro/settings/mcp.json|
Cursor|$SCRIPT_DIR/.cursor/mcp.json|
Codex|$HOME/.codex/mcp_config.json|codex
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
# Committed to the repo, so the command is RELATIVE to the project root and
# works on every machine without running this script.
echo "Shared .mcp.json"
cat > "$SCRIPT_DIR/.mcp.json" <<'EOF'
{
  "mcpServers": {
    "ffm-mcp": {
      "type": "stdio",
      "command": "mcp/target/release/ffm-mcp",
      "args": [],
      "env": {}
    }
  }
}
EOF
echo "  ✓ Shared config (.mcp.json, relative path)"

echo ""
echo "=== Done! ffm-mcp registered for every detected utility ==="
echo "Binary: $BINARY"
