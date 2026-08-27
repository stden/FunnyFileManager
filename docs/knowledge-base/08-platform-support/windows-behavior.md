<!-- Purpose: Documents Windows-specific runtime behavior and compatibility notes. -->

# Windows Platform Support

FunnyFileManager has specific handling for Windows differences.

## Drive Handling

- On Windows, `App::get_drives()` uses the Win32 `GetLogicalDrives` API.
- Drive letters are presented as `C:\`, `D:\`, etc.
- On Unix, only `/` is offered as the root "drive".

## Console / Shell Commands

- Unknown commands in the `:` command line are executed via `cmd /C <command>` (with `CREATE_NO_WINDOW` flag).
- This avoids flashing console windows.

## Editor Launching

- When launching an external editor, the `editor.rs` module uses `CommandExt::creation_flags(CREATE_NO_WINDOW)` on Windows to avoid console spam.

## Windows-Specific Diagnostics

- `:winmsg` / `:winerrors` — shows recent Windows system messages (via `open_windows_system_messages`).
- `:winperf` / `:wincheck` — basic performance and health checks.

These commands populate the internal `Viewer` overlay with collected output.

## Path Handling

- The code uses `PathBuf` and `canonicalize` everywhere, which behaves correctly on Windows.
- UNC paths and long paths are not specially handled yet.

## Testing

- There is a dedicated `tests/windows_behavior_test.rs`.
- Many Windows-specific behaviors are also covered in the general integration tests.

## Limitations

- No special support for Windows 10/11 "terminal" vs legacy console differences beyond basic flags.
- S3 integration works identically on Windows and Unix.

See `src/windows.rs` for the implementation of Windows-specific commands.