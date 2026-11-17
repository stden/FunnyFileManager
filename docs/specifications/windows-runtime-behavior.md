<!-- Purpose: Specifies expected Windows runtime behavior and diagnostics. -->

# 🪟 Windows Runtime Behavior Specification

## 📖 Purpose

Define the Windows-specific runtime behavior that `ffm` must preserve for command execution, system report collection, and drive discovery.

## 🎯 Scope

- Command-line shell execution triggered from `:` command mode
- Windows diagnostic viewers opened by built-in commands such as `winmsg` and `winperf`
- Local drive discovery for drive-picker overlays and startup panel state

## 📐 Requirements

### 💻 Shell Execution

- On Windows, shell-backed commands must be launched through `cmd`.
- The command invocation must include `/D` before `/C` so `cmd` ignores AutoRun registry hooks.
- Shell commands must inherit the active panel path as their working directory.
- Shell commands must run without opening an extra console window.

### 🪟 Python Reports

- Windows reports require Python 3.9 or newer as `python.exe` on `PATH`.
- Report commands pass `-I` to isolate imports from the current directory and Python environment settings.
- Report commands pass `-X utf8` for text that the Rust viewer can decode.
- Embedded Python collectors use native Windows APIs and perform read-only operations.
- Report commands run hidden, without a separate console window.
- The Chocolatey helper uses Python and the native UAC prompt. It waits and returns the installer exit code.

### 🛣️ Drive Discovery

- Windows drive enumeration must use the Win32 logical-drive bitmask rather than probing `A:\` through `Z:\` with filesystem existence checks.
- Each set bit from 0 through 25 must map to drive letters `A:` through `Z:` respectively.
- Returned drive paths must use the canonical `X:\` form.
- Returned drive paths must be ordered by ascending drive letter.

### 🌍 Cross-Platform Boundaries

- Non-Windows targets must continue using the existing POSIX shell path for shell-backed commands.
- Non-Windows targets must not attempt to execute Windows-only report collectors.
- Non-Windows targets must continue returning `/` as the root drive entry.

## 🚫 Non-Goals

- Defining Windows installer behavior
- Defining registry changes outside the child process invocation requirements above
- Defining report content beyond the invocation contract
