<!-- Purpose: Documents the MCP server tools and usage contract. -->

# 🔌 FFM MCP Server

Model Context Protocol server for FunnyFileManager — exposes file management operations as AI tools.

## 🏛️ Architecture

Built on the official **[rmcp](https://github.com/modelcontextprotocol/rust-sdk)** SDK (v3.x) with:

- **Stdio transport** — JSON-RPC 2.0 over stdin/stdout (standard MCP stdio transport)
- **`#[tool]` macros** — auto-generated JSON schemas from Rust types via `schemars`
- **`#[tool_router]` + `#[tool_handler]`** — declarative tool dispatch, no manual JSON-RPC parsing
- **`tracing`** — structured logging to stderr (stdout reserved for protocol)
- **Root confinement** — every user-supplied path is canonicalized *and* checked to live under one allowed root
  (`FFM_MCP_ROOT`, default: the process working directory)
- **Typed parameters** — each tool has a dedicated `Deserialize + JsonSchema` struct

## ✅ Best Practices Applied

| Practice                      | Implementation                                                                                                        |
|-------------------------------|-----------------------------------------------------------------------------------------------------------------------|
| Official SDK                  | `rmcp` v3.x with `#[tool]` macros instead of hand-rolled JSON-RPC                                                     |
| Never print to stdout         | All logging via `tracing` directed to stderr                                                                          |
| Root confinement              | `paths::sanitize_path*()` rejects empty/null-byte paths, canonicalizes, and refuses anything outside the allowed root |
| Typed inputs                  | `schemars::JsonSchema` derives generate accurate input schemas                                                        |
| Proper error codes            | Uses `McpError::invalid_params` / `internal_error` with JSON-RPC codes                                                |
| Async runtime                 | `tokio` for rmcp's async transport layer                                                                              |
| Graceful shutdown             | `service.waiting().await` handles clean shutdown                                                                      |
| No `unwrap()` in library code | All errors propagated via `Result` / `?` operator                                                                     |
| Focused dependencies          | MCP/stdio, serialization, KeePass, filesystem, and S3 support only                                                    |

## 🔌 Tools (54)

Core local-filesystem tools (the full list also covers pattern matching, batch
operations, bookmarks, symlinks, permissions, checksums, system info and S3):

| Tool                  | Description                                                                                                                                  |
|-----------------------|----------------------------------------------------------------------------------------------------------------------------------------------|
| `list_directory`      | List files/dirs at path with name, type, size                                                                                                |
| `get_file_info`       | Detailed metadata: size, type (incl. `symlink` + target), permissions, modified time                                                         |
| `view_file`           | Read text file contents (max 10 MB) with offset/limit                                                                                        |
| `kbdx`                | Decrypt a root-confined `.kdbx`/`.kbdx` vault to JSON (default), YAML, or password-only dotenv text; the response contains plaintext secrets |
| `create_file`         | Create new file with content (fails if exists)                                                                                               |
| `write_file`          | Write/overwrite file content                                                                                                                 |
| `create_directory`    | Create directory tree (recursive)                                                                                                            |
| `copy`                | Copy file or directory (recursive)                                                                                                           |
| `move_rename`         | Move/rename with atomic rename fallback to copy+delete                                                                                       |
| `delete`              | Delete file or directory (recursive option); a symlink is unlinked, never followed                                                           |
| `search`              | Case-insensitive name search in directory tree                                                                                               |
| `list_drives`         | List local drives, mount points, and AWS S3 profiles                                                                                         |
| `disk_usage`          | Calculate total size of file or directory tree                                                                                               |
| `tree`                | Show directory tree structure with configurable depth                                                                                        |
| `analyse_directory`   | Total size, per-subdirectory sizes, totals per file kind, largest files                                                                      |
| `find_duplicates`     | Identical-content groups (same-size files hashed) and reclaimable bytes                                                                      |
| `tail_file`           | Last N lines, seeking from the end (capped at 10 MB of tail)                                                                                 |
| `compare_directories` | Only-in-left, only-in-right, same-name-different-size across the trees                                                                       |
| `recent_files`        | Files modified within N days, newest first                                                                                                   |
| `head_file`           | First N lines, stopping once it has them; refuses after 10 MB of reading                                                                     |
| `read_lines`          | A 1-based inclusive line range, reading no further than `to`; refuses after 10 MB of reading                                                 |
| `text_format`         | Line endings from a 256 KB sample, the file's true final newline, byte-order mark, and whether it looks binary                               |
| `file_stats`          | Lines, words, characters, bytes, and the longest line                                                                                        |
| `compare_files`       | Identical or not, both SHA-256 sums, and the first differing byte offset (read from the bytes, also when the sizes differ)                   |
| `largest_files`       | The biggest files under a tree, biggest first, with a `truncated` flag when the walk budget stops early                                      |
| `which`               | Every match for an executable on `PATH`, in `PATH` order; on Unix only a file with an execute bit counts                                     |

### 🔐 KeePass export

`kbdx` takes `path`, an optional `password`, an optional root-confined
`keyfile`, and `format` (`json`, `yaml`, or `env`). At least one credential is
required. JSON is the default and is the best machine-readable representation;
YAML contains the same complete decrypted KeePass model. The dotenv form is
intentionally narrower: it emits current password fields as quoted variables
such as `KDBX_WORK_DEPLOY_PASSWORD`, adding numeric suffixes when names collide.

The tool returns the text directly and never creates an export file. Treat the
MCP response as secret material.

### 🧩 Pattern semantics

`glob` matches the pattern against each entry's path **relative to the search
root**: `**` spans any number of directories, while `*` and `?` never cross a
`/`. So `src/*.rs` matches `src/main.rs` but not `src/deep/lib.rs` or
`build.rs`. `find`, `replace` and `delete_by_pattern` apply the same matcher to
the entry name.

## 🏗️ Build & Run

```bash
cd mcp
cargo build --release      # Build optimized binary
cargo run                  # Run MCP server (stdio mode)
cargo bench --bench tree_walk  # Benchmark generated directory trees
```

To include an existing Windows tree and report its measured entry count and
total byte size, set `FFM_BENCH_REAL_TREE` for the benchmark process:

```powershell
$env:FFM_BENCH_REAL_TREE = "$env:WINDIR\System32\WindowsPowerShell\v1.0"
cargo bench --bench tree_walk
```

The size benchmark reports logical bytes accounted for per second; it reads
filesystem metadata, not file contents. Tree rendering reports entries per
second.

## ⚙️ Configuration

### 🧭 From FFM: F9 → Commands → Install MCP server

Install the server next to `ffm` (`cargo install --path mcp` from the FFM
sources), then use **F9 → Commands → Install MCP server** in FFM. It registers
`ffm-mcp` with Claude Code, Codex, Junie and Kiro, either for all projects
(user level) or for the active panel's folder (project level, with
`FFM_MCP_ROOT` set to that folder), and never rewrites a config it cannot
parse. The files it writes:

| Client | All projects | Current folder |
|---|---|---|
| Claude Code | via `claude mcp add --scope user`, or `add-json` to replace an entry (the user config the CLI uses: `~/.claude.json`, `$CLAUDE_CONFIG_DIR/.claude.json`, or a legacy `.config.json`) | `<folder>/.mcp.json` |
| Codex | `$CODEX_HOME/config.toml`, else `~/.codex/config.toml` | `<folder>/.codex/config.toml` |
| Junie | `~/.junie/mcp/mcp.json` | `<folder>/.junie/mcp/mcp.json` |
| Kiro | `$KIRO_HOME/settings/mcp.json`, else `~/.kiro/settings/mcp.json` | `<folder>/.kiro/settings/mcp.json` |

The keys, the two scopes and the rules for leaving files alone are in
[session tools](../docs/session-tools.md#mcp-server-setup). **F11 → MCP
status** (or `:mcp`) shows each client's state at both scopes.

### 🤖 Claude Desktop / Claude Code by hand

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

### ⚙️ Environment Variables

| Variable       | Default                   | Description                                                                                                       |
|----------------|---------------------------|-------------------------------------------------------------------------------------------------------------------|
| `RUST_LOG`     | `info`                    | Log level filter (trace/debug/info/warn/error)                                                                    |
| `FFM_MCP_ROOT` | process working directory | The only directory tree tools may read or write. Resolved once at startup; every path argument is confined to it. |

## 🔌 Protocol Flow

```
Client                          Server (ffm-mcp)
  │                                │
  ├─ initialize ──────────────────►│
  │◄────────── ServerInfo ─────────┤
  ├─ notifications/initialized ───►│
  │                                │
  ├─ tools/list ──────────────────►│
  │◄────────── 54 tools ──────────┤
  │                                │
  ├─ tools/call {name, args} ────►│
  │◄────────── result/error ──────┤
  │                                │
```

## 🔒 Security

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
- **Size limits**: `view_file` accepts at most a 10 MB file, and `head_file`/`read_lines` stop after 10 MB of reading,
  to prevent memory exhaustion
- **KeePass secrets**: `kbdx` is read-only and writes no export file, but its response deliberately contains plaintext
  secrets. Vaults and responses are each limited to 64 MB and are rejected rather than truncated. An optional key file
  passes through the same root-confinement check as the vault. See [`docs/kdbx-usage.md`](../docs/kdbx-usage.md) for the
  full parameter/format reference and the TUI vault browser this tool complements.
- **Existing-path validation**: read/delete operations require paths to exist
- **New-path validation**: write operations validate the nearest existing ancestor

## 📚 Dependencies

| Crate                | Version | Purpose                                            |
|----------------------|---------|----------------------------------------------------|
| `rmcp`               | 3.x     | Official MCP SDK (server + stdio transport)        |
| `serde`              | 1.x     | Serialization                                      |
| `serde_json`         | 1.x     | JSON handling                                      |
| `keepass`            | 0.13.x  | KeePass `.kdbx` decryption and complete data model |
| `serde_norway`       | 0.9.x   | YAML serialization                                 |
| `schemars`           | 1.x     | JSON Schema generation for tool inputs             |
| `tokio`              | 1.x     | Async runtime                                      |
| `tracing`            | 0.1     | Structured logging                                 |
| `tracing-subscriber` | 0.3     | Log output formatting                              |
