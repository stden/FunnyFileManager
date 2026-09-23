<!-- Purpose: Explain editor language-server setup, behavior, and extension points. -->

# Language servers in the built-in editor

Opening a supported source file with F4 asks whether to trust the workspace. Language
servers can execute project build scripts and macros. The default choice keeps
LSP disabled; select **Trust** with Down and confirm with Enter only for a project
you trust. Approval lasts until FFM exits and applies to that workspace root.
Approval also permits downloading a missing server using its configured installation
recipe. Installation and startup run in the background. Editing and saving still
work if a runtime, network connection or server is unavailable.
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

Use **F9 → Commands → Language servers (LSP)**, or type `lsp` in FFM's command
line, to inspect the catalog. The scrolling report shows enabled languages, file
extensions, availability, configured commands, resolved executable paths, each
launch argument and installation recipes. Java's displayed data directory belongs
to the active panel's project. This is an availability report, not a process list;
opening it never installs or starts a server. Close it with Esc.

![LSP inventory rendered with example installation paths](images/lsp-inventory.png)

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

Catalog entries are merged with settings field by field. For example,
`"python": {"enabled": false}` disables Python without replacing its other
defaults. `"install": null` disables automatic installation for one server.

| Language | Server | Installation prerequisite |
|---|---|---|
| Rust | rust-analyzer | Install on PATH, e.g. `rustup component add rust-analyzer` |
| Python | python-lsp-server (pylsp) | Python 3 with venv and pip |
| Java | Eclipse JDT LS 1.61.0 | Java 21+ and curl; JAVA_HOME takes precedence for Java |
| JavaScript / JSX / TypeScript / TSX | typescript-language-server + TypeScript 6 | Node.js and npm |
| HTML / CSS / JSON | vscode-langservers-extracted | Node.js and npm |
| YAML | yaml-language-server | Node.js and npm |
| Bash | bash-language-server | Node.js and npm |

Python and Java have optional real acceptance tests. Other catalog entries use
the same tested npm installation path; their runtime behavior depends on the server.
FFM does not install language runtimes or change global PATH. It first tries the
configured executable. Only a missing executable triggers automatic installation.
Packages go into `ffm/lsp/<recipe hash>` beside settings, not into the project or
global package directories. Identical recipes reuse an installation. Python uses
a private venv; npm uses a private prefix. Java's HTTPS archive is pinned and
checked against its SHA-256 before extraction. Each workspace has separate JDT
configuration and index data. Package-manager installation scripts can execute;
use only recipes and projects you trust.

Additional stdio servers can be added under their standard LSP language ID. For
example, this installs a configured Go server into the managed directory:

```json
"go": {
  "enabled": true,
  "command": "gopls",
  "args": [],
  "extensions": ["go"],
  "root_markers": ["go.mod", "go.work", ".git"],
  "install": {
    "kind": "command",
    "steps": [{
      "command": "go",
      "args": ["install", "golang.org/x/tools/gopls@latest"],
      "env": {"GOBIN": "{install_dir}/bin"}
    }],
    "executable": "bin/gopls.exe"
  }
}
```

Use `bin/gopls` on Unix and install Go separately. The example requires the Go
version required by the chosen gopls release. Custom steps receive literal argument
arrays and run in a staging directory; `{install_dir}` is expanded in commands,
arguments and environment values. The executable must be a relative path inside
the installation. Install tools must produce relocatable artifacts. Servers must
support UTF-16 positions and standard document/feature messages.

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

During installation the header shows `LSP installing`. Closing the editor or
quitting cancels the installer and terminates its process tree. Failed partial
installations are removed; Ctrl+Alt+R retries explicitly, without an automatic
restart loop. A corrupt cached receipt is reported instead of overwritten: close
FFM, remove only that server's cache directory shown by `lsp`, and reopen the file.
Installations wait on a per-recipe lock when another FFM instance is installing.
Commands have a ten-minute timeout; archive downloads have a five-minute timeout.
Python needs its venv/pip components; npm must be installed beside Node.js. For
Java, set JAVA_HOME to a JDK 21+ if PATH still selects an older Java. Package
manager failures show a concise status without exposing captured package output.

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

The worker owns a cancellable installation job in its `Preparing` state and keeps
the latest document snapshot while installation runs. A dedicated installer thread
owns package-manager process trees. A temporary staging directory becomes a cache
entry only after installation and entry-point validation succeed. Atomic rename,
a portable file lock and a launch receipt make reuse independent of editor widgets.
No installer or recipe parsing runs in rendering code.

The pure protocol client is independent of widgets and of rust-analyzer. Add
language defaults in `config/lsp/catalog.json`; installation adapters live under
`lsp/install/`; generic server support belongs under
`src/lsp/`, editor effects under `ops/editor_lsp.rs`, and presentation in the
editor model/input/rendering modules.

Deterministic protocol tests use memory streams; lifecycle/controller tests run
a tiny Python stdio peer in a temporary directory. They require Python but do not
require rust-analyzer. The optional real smoke test creates a temporary Cargo
workspace and is excluded from ordinary test runs:

```sh
cargo test --lib real_rust_analyzer_supports_the_complete_editor_protocol -- --ignored --nocapture
cargo test --lib real_python_package -- --ignored --nocapture
cargo test --lib real_java_package -- --ignored --nocapture
```
