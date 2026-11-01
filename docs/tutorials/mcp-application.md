<!-- Purpose: Use every application command through an explicit background FFM session. -->
# FFM application sessions over MCP

The MCP server exposes the application's named actions, keyboard and text
input, mouse handling and dialogs through a separate background FFM process.
The session keeps its own panels, editor, selection, open screen and jobs.
Commands use the same handlers and workers as the terminal application.

Install both binaries:

```text
cargo install --path . --bin ffm --force --locked
cargo install --path mcp --bin ffm-mcp --force --locked
```

Both `ffm` and `ffm-mcp` must be on the MCP client's PATH. An installation
outside PATH can set `FFM_MCP_FFM_BINARY` to its FFM executable. This setting
belongs to the server configuration; tool calls cannot choose an executable.

Enable `FFM_MCP_ALLOW_APPLICATION=1` in the server's environment. The
repository's `.mcp.json` enables this mode and the explicit KeePass export
capability. Application sessions have the server account's full FFM authority,
including remote connections, shell commands and system settings. The original
filesystem tools remain confined to `FFM_MCP_ROOT`. A server started with
`FFM_MCP_READONLY=1` refuses application sessions.

## Start and inspect a session

Call `ffm_session_start` with an optional initial `path`, dimensions, and
`profile`. Dimensions are 20–400 columns and 8–200 rows, defaulting to 120×40.
The default `system` profile loads the user's FFM settings and production host
capabilities. The `isolated` profile uses a private session clipboard and starts
without user settings, provider credentials, system cleanup or process-control
capabilities. Both profiles retain full filesystem and shell authority.

Keep the returned `session_id` for subsequent calls:

| Tool | Operation |
|---|---|
| `ffm_session_catalog` | List every canonical context/action, description and default keys |
| `ffm_session_state` | Read panels, current dialog, rendered screen, job and external process |
| `ffm_session_action` | Invoke a named action in its owning context |
| `ffm_session_input` | Send key chords, pasted text or a mouse event |
| `ffm_session_wait` | Wait up to 10 seconds for application work |
| `ffm_session_resize` | Resize the session and its private external terminal |
| `ffm_session_cancel` | Request cancellation of the running operation |
| `ffm_session_close` | End the background session and release its process |

The catalogue is generated from FFM's key tables, including keyless menu rows.
It includes panels, navigation, selection, filters, viewer/editor commands,
SSH/SFTP/FTP sessions and file operations, settings, histories, processes, WSL,
KeePass, cleanup, Space analysis, Things, and external programs. Discover action
names from this catalogue instead of storing a fixed action count.

## Actions and dialogs

A named action respects the active screen. An editor command cannot run behind
a file dialog. Use `ffm_session_state` to inspect `modal.context` and the
rendered screen, then use input to complete or cancel a dialog. Confirmations
retain the application's normal defaults.

For example, open a file with the panel's `edit` action or F4, send text with
`ffm_session_input`, and call the editor's `save` action. Copy, rename, delete
and remote-file operations use their normal destination and confirmation
dialogs. Wait for the submitted job before sending another application action;
state and cancellation remain available while it runs.

External programs receive a private terminal. While one is active, text and
keys go to that terminal; its bounded output and exit status appear under
`external_process`. Terminal escape bytes never become MCP response framing.
Desktop-association commands report their launch result; desktop availability
is a property of the server machine.

Passive snapshots redact revealed vault secrets and omit authentication
secrets. Use the explicitly enabled `kbdx` tool for structured secret
exports. Session processes disable persistent FFM input diagnostics.

## Verification

Tests compare the catalogue with all canonical key tables and exercise real
file edits, copies, deletion confirmations, modal routing, independent
sessions and process shutdown. CI also builds both binaries on Linux and
Windows and runs actual MCP calls through a background FFM session.
