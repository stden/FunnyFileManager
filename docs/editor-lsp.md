<!-- Purpose: Explain editor language-server setup, behavior, and extension points. -->

# Language servers in the built-in editor

Opening a Rust `.rs` file with F4 asks whether to trust the workspace. Language
servers can execute project build scripts and macros. The default choice keeps
LSP disabled; select **Trust** with Down and confirm with Enter only for a project
you trust. Approval lasts until FFM exits and applies to that workspace root.
After approval, `rust-analyzer` starts in the background.
Install it on PATH, for example with `rustup component add rust-analyzer` when
using rustup. Editing and saving still work if the server is unavailable.
The editor header reports pending trust, startup, readiness, failure, and diagnostic counts.
Errors and warnings underline the affected text; placing the cursor there shows
the message in the header. Information and hints are also displayed.

![Built-in editor hover popup, captured from the TUI renderer](images/editor-lsp.png)

| Editor key | Action |
|---|---|
| Ctrl+Space | Request completions |
| Ctrl+K | Show hover information |
| Ctrl+] | Go to definition |
| Ctrl+Alt+R | Restart the language server or reopen the trust prompt |
| Up / Down | Select a popup result or scroll hover text |
| Enter / Esc | Accept / dismiss the popup |

Completions show labels, kinds, details, and supplied documentation. The editor
applies text edits and additional edits atomically. Unsupported snippets produce
a notice; their placeholder syntax is never inserted into the file. Hover shows
plain text and Markdown, including code fences, without a web renderer.
Multiple definitions open a selection list. Save with F2 before following a
definition into another file when the current buffer has unsaved changes.

## Configuration

The existing `ffm/settings.json` supports an `lsp` object. Its Windows location is
`%APPDATA%\ffm\settings.json`; on Unix it is under the application's normal XDG
configuration directory. Existing settings files retain their defaults.

```json
{
  "lsp": {
    "rust": {
      "enabled": true,
      "command": "rust-analyzer",
      "args": [],
      "extensions": ["rs"],
      "root_markers": ["Cargo.toml", ".git"]
    }
  }
}
```

An explicit command is used as configured; otherwise the default executable is
resolved through PATH. Arguments are passed directly to the executable, not
through a shell. Use an absolute executable path for an installation outside
PATH. Set `enabled` to `false` to disable it. Restart FFM after changing settings.
The nearest ancestor containing a configured root marker is the workspace root;
without a marker, the source file's parent directory is used.

Rust is the tested language. Other stdio servers can be configured under their
LSP language ID with their own command, extensions, and root markers. Servers
must support UTF-16 positions and standard document/feature messages.

## Behavior and troubleshooting

Document changes are batched after 120 ms of inactivity. An explicit feature
request flushes pending changes first. Incremental synchronization is preferred
when advertised; full synchronization and disabled synchronization are honored.
F2 sends `didSave` if the server requests it. Rust-analyzer's Cargo diagnostics
describe saved files, so save after fixing a Cargo error to trigger a fresh check.

An unavailable server usually means its command cannot be found. Verify
`rust-analyzer --version` from the environment used to launch FFM. After installing
it or correcting PATH, restart the server with Ctrl+Alt+R. A failed server is not
restarted automatically. A server still indexing may reject a request as content
modified (`-32801`); request the information again when indexing settles.

Opt-in FFM debug logging records request methods/IDs, lifecycle changes and error
codes. Source text is not logged. Stderr is drained separately and its byte counts
are logged with contents redacted, because a server may print source text there.

Current limits: one active editor/server session, explicit completion triggers,
UTF-16 positions, no snippets, no completion-item resolve requests, no formatting,
rename, code actions or edits requested by the server. Unversioned diagnostics
cannot be conclusively associated with an earlier document version.

## Developer architecture

```text
Editor (buffer, revision, caret, diagnostics, popup)
  ↕ application::lsp_jobs::EditorLspController (UI thread)
  ↕ bounded command/event channels
LspManager worker (server lifecycle, synchronized snapshots, requests)
  ↕ LspClient (initialize, capabilities, JSON-RPC IDs, document versions)
  ↕ separate stdin writer / stdout reader / stderr drain threads
Language server (stdio Content-Length frames)
```

The controller creates snapshots only when synchronization is needed. Process
startup, filesystem root discovery, framing and pipe I/O run off the UI thread.
Rendering reads model state and performs no server operations. Replies carry the
document identity, revision and request position; stale results are discarded.
Cancellation removes superseded request IDs and sends `$/cancelRequest`.

Local debug-build measurements for the small acceptance workspace: background
startup handoff 0.26 ms, diagnostics after 2.42 s, completion 42.15 ms, an edit
handoff 0.006 ms, shutdown 40.14 ms. With a busy fake server, 1,000 editor edits
and controller pumps took 6.13 ms; controller startup with LSP disabled took
0.361 ms. These are observations, not portable performance thresholds.

Closing the editor sends `didClose` and shuts down its server. Application exit
joins the worker: `shutdown` has a bounded response wait, then `exit` has a bounded
process wait. Remaining processes are terminated and reaped; Windows job objects
and Unix process groups include descendants. No async runtime is required.
On Unix, exit polling retains the child PID until its group is terminated and
reaped, preventing a reused PID from receiving cleanup signals. A full writer
queue retains messages for retry in order, with a bounded backlog.

The pure protocol client is independent of widgets and of rust-analyzer. Add
language defaults in `config/lsp.rs`; generic server support belongs under
`src/lsp/`, editor effects under `ops/editor_lsp.rs`, and presentation in the
editor model/input/rendering modules.

Deterministic protocol tests use memory streams; lifecycle/controller tests run
a tiny Python stdio peer in a temporary directory. They require Python but do not
require rust-analyzer. The optional real smoke test creates a temporary Cargo
workspace and is excluded from ordinary test runs:

```sh
cargo test --lib real_rust_analyzer_supports_the_complete_editor_protocol -- --ignored --nocapture
```
