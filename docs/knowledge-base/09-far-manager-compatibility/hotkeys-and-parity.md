<!-- Purpose: Tracks Far Manager hotkey parity and behavior expectations. -->

# Far Manager Hotkey Compatibility

This document tracks how closely FunnyFileManager matches classic Far Manager keyboard behavior.

## Legend
- ✅ Fully implemented
- ⚠️ Partially implemented or different behavior
- ❌ Not implemented (by design or future work)

## Navigation

| Key          | Far Manager Action          | FFM Status     | Notes |
|--------------|-----------------------------|----------------|-------|
| Up / Down    | Move cursor                 | ✅             |       |
| Home / End   | First / Last entry          | ✅             |       |
| Enter        | Enter directory / execute   | ✅             |       |
| Tab          | Switch panels               | ✅             |       |
| Alt+Letter   | Quick search                | ✅             |       |
| Ctrl+\       | Go to root                  | ✅             |       |
| Backspace    | Go to parent                | ❌             | Not implemented |

## File Operations (F-keys)

| Key       | Far Action                     | FFM Status | Notes |
|-----------|--------------------------------|------------|-------|
| F1        | Help                           | ✅         |       |
| F3        | View                           | ✅         | Internal viewer |
| F4        | Edit                           | ✅         | Built-in editor (Far style); Alt+F4 = external editor |
| Shift+F4  | Create new file                | ✅         |       |
| F5        | Copy                           | ✅         |       |
| F6        | Move / Rename                  | ✅         |       |
| F7        | Mkdir                          | ✅         |       |
| F8        | Delete                         | ✅         | With confirmation dialog |
| F10       | Quit                           | ✅         | Also Q and Ctrl+C |
| Alt+F1/F2 | Drive selection                | ✅         |       |
| Ctrl+O    | Toggle console (hide panels)   | ✅         |       |
| Ctrl+U    | Swap panels                    | ✅         |       |

## Marking

| Key          | Far Action              | FFM Status | Notes |
|--------------|-------------------------|------------|-------|
| Insert / Space | Toggle mark + move next | ✅         |       |
| Ctrl+A       | Select all / attributes | ❌         | Not implemented |

## Command Line

| Command              | Supported | Notes |
|----------------------|-----------|-------|
| `cd`, `pwd`, `sort`  | ✅        |       |
| `bookmark set/go`    | ✅        | Uses `BookmarkSlot` type; the *only* way to reach bookmarks — F2 is unbound and there is no bookmark overlay |
| `mark all`, `unmark` | ✅        |       |
| `mcp`, `winmsg`, etc.| ✅        |       |
| Shell fallback       | ✅        | Any unknown command runs in shell |

## Overlay / Dialog Keys

- Esc — universally closes current overlay (consistent with Far)
- Y/N in delete confirmation — implemented
- Arrow keys in viewer — implemented
- F2 (bookmark overlay) and 0-9 / Ctrl+0-9 slot keys — ❌ never implemented; use `:bookmark set|go <0-9>`

## Known Differences (Intentional)

- F4 opens the built-in editor (no Colorer/syntax highlighting yet); Alt+F4 delegates to the external editor.
- Some advanced Far plugins and menu systems have no equivalent.
- Drive letters on Unix are simulated as `/`.
- S3 buckets appear as additional "drives".

## Far Manager Reference

Upstream documentation can be fetched on demand (see `docs/far-manager/README.md`).

This document is the living parity checklist for the project.