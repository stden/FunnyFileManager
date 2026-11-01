# Remote control of FunnyFileManager

> **Draft proposal:** This document describes a proposed control channel and
> MCP integration. The features and settings below are not implemented in the
> current version; the text is retained as a design specification.

<!-- Purpose: Contract for the FFM control channel and the ffm-mcp tools that use it. -->

FunnyFileManager (FFM) can expose its diagnostics and accept its input remotely,
through the MCP server `ffm-mcp`. Both programs are off by default.

## Security model

- Control gives the same reach as the person at the keyboard. Keys can reach any
  file FFM can reach, and F8 deletes. Treat an enabled channel as shell access.
- FFM listens on `127.0.0.1` only and never on another address.
- Every request carries a 256-bit random token. It is generated when control
  starts, kept in the discovery file (owner-only on Unix), and nowhere else.
- FFM logs control calls by method name only. Keys and text are never logged.
- `ffm-mcp` HTTP mode requires a bearer token on every request. A non-loopback
  address needs `--allow-remote`. HTTP is plain text: reach another machine
  through an SSH tunnel or a TLS reverse proxy.
- `ffm-mcp` confines its file tools to `FFM_MCP_ROOT`. The control tools cannot
  be confined this way, because keys reach the whole file system. They are
  registered only when `FFM_MCP_CONTROL=1` is set.

## Enabling

| Program | Setting | Effect |
|---|---|---|
| FFM | `FFM_CONTROL=1` | Starts the control channel and writes the discovery file. `0` or unset: off. |
| ffm-mcp | `FFM_MCP_CONTROL=1` | Registers the `ffm_*` control tools (stdio and HTTP). |
| ffm-mcp | `--http <addr>` plus `FFM_MCP_HTTP_TOKEN` | Serves MCP over streamable HTTP at `/mcp` with bearer auth. The token must be at least 32 characters. |
| ffm-mcp | `--allow-remote` | Required for a non-loopback `--http` address. |
| ffm-mcp | `FFM_CONTROL_FILE` | Overrides the discovery file path. |

## Discovery file

Path: `control.json` in FFM's configuration directory. That is the rule in
`src/config/home.rs` `dir()`: `$XDG_CONFIG_HOME/ffm`, else `$HOME/.config/ffm`
on Unix, and `%APPDATA%\ffm` on Windows.

```json
{"version": 1, "pid": 4242, "port": 51234, "token": "<64 lowercase hex>", "started_at": "2026-10-19T12:00:00Z"}
```

Written when control starts. Removed on a normal exit. Overwritten by the next
start. On Unix it is created with mode 0600.

## Wire protocol

- TCP to `127.0.0.1:<port>`. One JSON object per line, UTF-8, terminated by `\n`.
- Requests are answered in order, one response line each.
- Request: `{"id": <integer>, "token": "<token>", "method": "<name>", "params": {...}}`.
  `params` is optional and defaults to `{}`.
- Response: `{"id": <same>, "result": {...}}` or
  `{"id": <same>, "error": {"code": "<code>", "message": "<text>"}}`.
- Error codes: `unauthorized` (token missing or wrong), `bad_request` (not JSON,
  or no method), `unknown_method`, `invalid_params`, `busy` (the event loop did
  not answer within 10 seconds), `failed` (the operation failed; the message says
  why), `unavailable` (no frame has been drawn yet).
- A request line longer than 64 KiB gets `bad_request`, and the connection closes.

## Methods

Requests run on the event loop thread, between events. A long operation such as
a copy delays the answer until it finishes, and the caller may see `busy`.

| Method | Params | Result |
|---|---|---|
| `ping` | none | `{"pid", "version", "lifecycle"}`. `lifecycle` is `running` or `quitting`. |
| `status` | none | `{"lifecycle", "active_panel": "left"|"right", "view": "panels"|"console", "overlay": "<variant name>", "status_text": string|null, "panels_shown": {"left": bool, "right": bool}, "terminal": {"width", "height"}}` |
| `panels` | none | `{"left": P, "right": P}`. `P` is `{"path", "entries", "cursor": {"index", "name"}|null, "marked": [names, at most 500], "marked_count"}`. |
| `screen` | none | `{"width", "height", "lines": [text per row, trailing spaces removed], "cursor": {"x", "y"}|null}`. The last frame drawn. |
| `performance` | none | `{"actions": [{"action", "count", "total_ms", "mean_ms", "max_ms", "render_ms"}]}`, ranked by total wait. Same rows as the performance file. |
| `log` | `{"lines"}` (default 200, max 2000) | `{"path": string|null, "lines": [...]}`. The tail of the FFM log. |
| `keys` | `{"keys": [chord, ...]}` | `{"sent": n}`. Chords use the keymap syntax (`F10`, `Ctrl+Shift+F5`, `Alt+Left`, `Enter`). All chords are parsed first; if any fails, nothing is sent and the result is `invalid_params`. Keys are dispatched in order, exactly as typed keys. The answer comes after a frame has been drawn. |
| `text` | `{"text"}` | `{"sent": n}`. Each character is typed as a key. The answer comes after a frame has been drawn. |
| `navigate` | `{"path"}` | `{"path"}`, the active panel's path after the move. Failure is `failed`. The answer comes after a frame has been drawn. |
| `quit` | none | `{"lifecycle": "quitting"}`. Same as pressing F10. |

## MCP tools

When `FFM_MCP_CONTROL=1`, `ffm-mcp` registers these tools. Each maps one to one
onto the method with the same suffix (`ffm_status` -> `status`):

`ffm_ping`, `ffm_status`, `ffm_panels`, `ffm_screen`, `ffm_performance`,
`ffm_log`, `ffm_keys`, `ffm_text`, `ffm_navigate`, `ffm_quit`.

`ffm_navigate` validates its `path` with the same root confinement as the file
tools (`FFM_MCP_ROOT`) before forwarding it. The other tools take no paths.

When FFM is not running, or its control channel is off, a tool fails with a
message that says how to enable it. Tools read the discovery file on every call,
so an FFM restart is picked up without restarting `ffm-mcp`.

## Testing

- FFM: protocol parsing and dispatch are unit tests. The server is tested over a
  real loopback socket. Each method is tested against a constructed `App`.
- ffm-mcp: the client is tested against a fake control server. The HTTP layer is
  tested for 401 without a token and for 200 with the right one.
- Both: `./ffm.py lint`, and `cargo test` for the crate that changed.
