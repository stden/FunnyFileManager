<!-- Purpose: Documents the MCP server tools and usage contract. -->

# FFM MCP Server

Model Context Protocol server for FunnyFileManager — exposes file management operations as AI tools.

## Architecture

Built on the official **[rmcp](https://github.com/modelcontextprotocol/rust-sdk)** SDK (v1.x) with:

- **Stdio transport** — JSON-RPC 2.0 over stdin/stdout (standard MCP stdio transport)
- **`#[tool]` macros** — auto-generated JSON schemas from Rust types via `schemars`
- **`#[tool_router]` + `#[tool_handler]`** — declarative tool dispatch, no manual JSON-RPC parsing
- **`tracing`** — structured logging to stderr (stdout reserved for protocol)
- **Root confinement** — every user-supplied path is canonicalized *and* checked to live under one allowed root (`FFM_MCP_ROOT`, default: the process working directory)
- **Typed parameters** — each tool has a dedicated `Deserialize + JsonSchema` struct

## Best Practices Applied

| Practice | Implementation |
|----------|----------------|
| Official SDK | `rmcp` v1.x with `#[tool]` macros instead of hand-rolled JSON-RPC |
| Never print to stdout | All logging via `tracing` directed to stderr |
| Root confinement | `paths::sanitize_path*()` rejects empty/null-byte paths, canonicalizes, and refuses anything outside the allowed root |
| Typed inputs | `schemars::JsonSchema` derives generate accurate input schemas |
| Proper error codes | Uses `McpError::invalid_params` / `internal_error` with JSON-RPC codes |
| Async runtime | `tokio` for rmcp's async transport layer |
| Graceful shutdown | `service.waiting().await` handles clean shutdown |
| No `unwrap()` in library code | All errors propagated via `Result` / `?` operator |
| Minimal dependencies | Only rmcp, serde, schemars, tokio, tracing |

## Tools (41)

Core local-filesystem tools (the full list also covers pattern matching, batch
operations, bookmarks, symlinks, permissions, checksums, system info and S3):

| Tool | Description |
|------|-------------|
| `list_directory` | List files/dirs at path with name, type, size |
| `get_file_info` | Detailed metadata: size, type (incl. `symlink` + target), permissions, modified time |
| `view_file` | Read text file contents (max 10 MB) with offset/limit |
| `create_file` | Create new file with content (fails if exists) |
| `write_file` | Write/overwrite file content |
| `create_directory` | Create directory tree (recursive) |
| `copy` | Copy file or directory (recursive) |
| `move_rename` | Move/rename with atomic rename fallback to copy+delete |
| `delete` | Delete file or directory (recursive option); a symlink is unlinked, never followed |
| `search` | Case-insensitive name search in directory tree |
| `list_drives` | List local drives, mount points, and AWS S3 profiles |
| `disk_usage` | Calculate total size of file or directory tree |
| `tree` | Show directory tree structure with configurable depth |

### Pattern semantics

`glob` matches the pattern against each entry's path **relative to the search
root**: `**` spans any number of directories, while `*` and `?` never cross a
`/`. So `src/*.rs` matches `src/main.rs` but not `src/deep/lib.rs` or
`build.rs`. `find`, `replace` and `delete_by_pattern` apply the same matcher to
the entry name.

## Build & Run

```bash
cd mcp
cargo build --release      # Build optimized binary
cargo run                  # Run MCP server (stdio mode)
```

## Configuration

### Claude Desktop / Claude Code

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "ffm": {
      "command": "/path/to/ffm-mcp",
      "env": {
        "FFM_MCP_ROOT": "/path/to/the/only/tree/the/server/may/touch"
      }
    }
  }
}
```

Without `FFM_MCP_ROOT` the server confines itself to whatever directory it was
started in — set it explicitly, since a client-spawned process often inherits
`/`.

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level filter (trace/debug/info/warn/error) |
| `FFM_MCP_ROOT` | process working directory | The only directory tree tools may read or write. Resolved once at startup; every path argument is confined to it. |

## Protocol Flow

```
Client                          Server (ffm-mcp)
  │                                │
  ├─ initialize ──────────────────►│
  │◄────────── ServerInfo ─────────┤
  ├─ notifications/initialized ───►│
  │                                │
  ├─ tools/list ──────────────────►│
  │◄────────── 41 tools ──────────┤
  │                                │
  ├─ tools/call {name, args} ────►│
  │◄────────── result/error ──────┤
  │                                │
```

## Security

Every tool argument arrives from an AI client and is treated as untrusted input.

- **Root confinement**: paths are canonicalized *and* rejected unless they resolve
  under `FFM_MCP_ROOT` (default: the working directory). Canonicalizing `..` is not
  traversal prevention on its own — an absolute path resolves to itself.
- **Null byte rejection**: paths containing `\0` are rejected
- **Symlinks are not followed for destructive or reporting operations**: `delete`
  unlinks the link itself (the target tree survives, dangling links stay
  removable) and `get_file_info` reports the link, not its target
- **Atomic replacement**: `replace` writes a temp file and renames it, so a failed
  write is reported instead of leaving a truncated file
- **Permissions are narrowed, never widened**: clearing the read-only flag restores
  the owner's write bit only — it does not grant group/world access
- **No editor / arbitrary command tool**: the server owns fd 0/1 for JSON-RPC, so a
  spawned child would corrupt the protocol stream; there is no tool that runs a
  caller-named binary
- **One internal subprocess**: `system_info` shells out to `df -h /` (fixed
  argv, no user input) to report free space. Everything else uses `std::fs`.
- **Size limits**: `view_file` enforces 10 MB max to prevent memory exhaustion
- **Existing-path validation**: read/delete operations require paths to exist
- **New-path validation**: write operations validate the nearest existing ancestor

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `rmcp` | 1.x | Official MCP SDK (server + stdio transport) |
| `serde` | 1.x | Serialization |
| `serde_json` | 1.x | JSON handling |
| `schemars` | 1.x | JSON Schema generation for tool inputs |
| `tokio` | 1.x | Async runtime |
| `tracing` | 0.1 | Structured logging |
| `tracing-subscriber` | 0.3 | Log output formatting |
