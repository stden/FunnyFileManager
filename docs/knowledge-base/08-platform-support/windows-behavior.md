<!-- Purpose: Documents Windows-specific runtime behavior and compatibility notes. -->

# 🪟 Windows Platform Support

FunnyFileManager has specific handling for Windows differences.

## 🛣️ Drive Handling

- On Windows, `App::get_drives()` uses the Win32 `GetLogicalDrives` API.
- Drive letters are presented as `C:\`, `D:\`, etc.
- On Unix, only `/` is offered as the root "drive".

## 💻 Console / Shell Commands

- Unknown commands in the `:` command line are executed via `cmd /C <command>` (with `CREATE_NO_WINDOW` flag).
- This avoids flashing console windows.
- `codex`, `codex.exe`, and `codex.cmd` typed in FFM open a new interactive
  console with the active panel as its working directory. Arguments and quotes
  are preserved; FFM does not wait for completion. The process uses
  `CREATE_NEW_CONSOLE` without inheriting FFM's standard handles.

## ✏️ Editor Launching

- When launching an external editor, the `editor.rs` module uses `CommandExt::creation_flags(CREATE_NO_WINDOW)` on Windows to avoid console spam.

## 🪟 Windows-Specific Diagnostics

- `:winmsg` / `:winerrors` — shows recent Windows system messages (via `open_windows_system_messages`).
- `:winperf` / `:wincheck` — basic performance and health checks.

These commands populate the internal `Viewer` overlay with collected output.

## WSL manager

Open the manager with `Ctrl+Alt+W`, the bare `wsl` command, or
`F9 → Commands → WSL manager`. WSL must already be installed. The window lists
registered distributions and shows the current operation's busy state or outcome.

| Key | Behavior |
|---|---|
| Up / Down, Home / End | Select a distribution |
| Enter | Open `\\wsl.localhost\<distribution>\` in the active file panel |
| F2 | Refresh distribution names using `wsl.exe --list --quiet` |
| F5 | Update the WSL platform using `wsl.exe --update` |
| F6 | Upgrade installed packages inside the selected distribution |
| Esc | Close the manager |

Platform updates and Linux package updates are separate. Microsoft's
[WSL command reference](https://learn.microsoft.com/en-us/windows/wsl/basic-commands)
describes the platform commands. File access uses normal panels, so existing
browsing, viewing, editing, and file operations apply to the distribution root.
Opening the UNC directory uses the normal synchronous filesystem path and can
block the interface while the share responds.

Package updates run as the distribution's `root` user, with a noninteractive
script that chooses the first supported installed package manager:

| Package manager | Update operation |
|---|---|
| apt-get | Refresh indexes, then `upgrade -y` |
| dnf | `upgrade -y` |
| yum | `update -y` |
| zypper | `--non-interactive update` |
| pacman | `-Syu --noconfirm` |
| apk | Refresh indexes, then `upgrade` |

These follow the package manager's normal installed-package upgrade rules;
they do not perform a distribution release upgrade. An unsupported package
manager produces an explicit error.

Operations run on a background worker. The list remains navigable while busy,
but additional operations are disabled. Esc does not cancel an update: the
result appears in application status when it completes. A completion cannot
reopen a closed manager or navigate a panel after that manager closes.
Quitting ffm is different from Esc: completion reporting ends, and ffm does not
wait for the worker or guarantee that WSL/package processes stop.
Timeouts are 20 seconds for listing, five minutes for WSL updates, and one hour
for distribution package updates. A timeout attempts to stop the Windows WSL
client; Linux descendants may continue. It does not roll back package changes.

Implementation is split between `models/wsl.rs` (state and requests),
`action/wsl.rs` (key language), `popup/wsl.rs` (interpretation), `ops/wsl.rs`
(request and outcome coordination), `application/wsl_jobs.rs` (worker),
`wsl.rs` and `wsl/process.rs` (system adapter), and `ui/wsl.rs` (rendering).

## 🛣️ Path Handling

- The code uses `PathBuf` and `canonicalize` everywhere, which behaves correctly on Windows.
- The WSL manager uses validated distribution names to form UNC roots; normal
  filesystem adapters handle subsequent panel operations.

## 🧪 Testing

- There is a dedicated `tests/windows_behavior_test.rs`.
- Many Windows-specific behaviors are also covered in the general integration tests.
- WSL tests inject backend/process results and temporary directories. They test
  update requests and outcomes without updating the host or Linux packages.

## ⚠️ Limitations

- No special support for Windows 10/11 "terminal" vs legacy console differences beyond basic flags.
- S3 integration works identically on Windows and Unix.

See `src/windows.rs` for the implementation of Windows-specific commands.
## Paths and command directories

Windows canonical paths can include the `\\?\` prefix. Titles, the command prompt,
and path-based status messages display ordinary drive or UNC paths while file
operations retain the original native paths. Device and volume names retain
their special prefixes.

Interactive commands such as `codex` start in the active panel directory.
The Windows console launcher converts canonical directory names to the spelling
understood by `cmd.exe`; otherwise cmd treats the extended prefix as a UNC path
and silently falls back to the Windows directory.

For UNC shares, use a mapped drive before launching a new interactive console:
`cmd.exe` does not accept a UNC path as its working directory.
