# Editor and session replay

The built-in F4 editor supports Shift+arrow stream selection, Ctrl+C / Ctrl+Insert copy,
Ctrl+V / Shift+Insert paste, and Ctrl+A to select the complete document, including its
final newline. Typing or pasting replaces the selection. F2 saves.

Clipboard and terminal paste events insert whole text blocks. When a terminal
sends pasted text as individual keys, the editor groups already queued text
before drawing, up to 16,384 events per batch. It does not wait for more input;
shortcuts retain their order, and editor subdialogs keep their own input handling.
Large multiline insertions move the remaining document once per batch.

F8 opens the encoding dialog. Arrow keys select UTF-8, UTF-8 with BOM,
UTF-16 LE/BE, Windows-1251, OEM 866, or Windows-1252. **Use for saving** (Enter)
changes the output encoding while keeping the current Unicode text. **Reload**
(R) decodes the original file again with the selected encoding; it refuses to
discard unsaved edits. Mouse clicks select encodings and activate buttons.

Unicode BOMs take precedence during opening. Valid UTF-8 is detected directly;
other input initially uses Windows-1251. Legacy code pages cannot be identified
unambiguously from arbitrary bytes: use Reload for OEM 866 or Windows-1252.
Saving preserves the selected encoding and original line endings. Characters
that cannot be represented cause a save error before the original file is
changed. Malformed Unicode is rejected rather than silently replaced.

`.env` and related dotenv files use their own syntax highlighter. Editor
highlighting caches lexical spans across cursor movements. Editing text or
changing its path invalidates that cache.

## Launching a selected program

Enter or a double-click launches an executable entry in the active panel's
directory. On Windows this includes `.cmd` and `.bat`; PowerShell scripts use
PowerShell and MSI packages use `msiexec`. Native argument passing preserves
spaces and shell metacharacters in file names. The TUI suspends while the program
uses the terminal, then restores its screen and refreshes the panels.

## Recording and replay

Recording is opt-in. A recording holds every character typed into the session,
including anything pasted into a prompt, so a plain launch writes nothing — the
same rule the diagnostics log follows. Ask for one with `--record`, and the
startup message prints the path:

```text
ffm --record                 # auto-named, under the temporary directory
ffm --record session.py      # a durable location of your own
python session.py
python session.py path/to/ffm
ffm --no-record              # accepted, and already the default
```

The auto-named path is `ffm-recordings-<uid>` under the system temporary
directory, created 0700: the temporary directory is shared by every local user,
and a recording is not something to leave where they can reach it. A session
file that cannot be created disables recording and reports why; it never stops
FFM from starting.

Recording files are created exclusively: an existing file is never overwritten.
Python 3.9 or newer is sufficient; the launcher uses only the standard library.
The Python file contains JSON comment records, so typed text is data rather than
generated Python expressions. Replay launches FFM in the recorded working
directory and feeds events through the same application loop.

Records include key codes, modifiers, mouse events, resizes, paste payloads,
inter-event delays, and native clipboard read results. Clipboard replay uses the
recorded text rather than the current system clipboard. The terminal must be
large enough for every recorded frame; replay uses the original frame sizes for
rendering and mouse coordinates. An incomplete recording reports an error if
input ends before the application quits.

Live batching records the original events. Replay preserves their recorded
timing and does not combine them.

Recordings contain entered and pasted text, including any secrets typed during
the session. They replay actual operations against the current filesystem;
restore the original fixture and retain the required programs, profiles, and
services when reproducing a session. They are not filesystem snapshots.
Input handled by a launched external program is outside FFM's event stream.
Background service results and timing still depend on those services.

## Verification

`cargo test --test encoding_conversion_test` covers file conversion, BOMs, line
endings, explicit reload, and refusal of lossy saves or discarded edits.
Library tests cover selection, caching, launch boundaries, event serialization,
clipboard replay, viewport resize, and a recorded directory creation.
`python scripts/test_recording.py` executes the generated Python launcher with
an inspected subprocess boundary, checking Unicode, arguments and working directory.

TDD regressions were observed failing before implementation for clipboard
replay and replay resize. Earlier editor regressions cover conversion without
reinterpretation and repeated cursor highlighting work.

For background Rust diagnostics, completion, hover and definitions, see
[editor language-server setup and shortcuts](editor-lsp.md).
