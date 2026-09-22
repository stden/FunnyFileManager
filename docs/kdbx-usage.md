<!-- Purpose: End-user guide to opening and browsing KeePass (.kdbx) vaults in FFM, and to the MCP kbdx export tool. -->

# 🔐 KeePass (.kdbx) Support

FunnyFileManager can open KeePass password databases in two ways:

1. **The TUI vault browser/editor** — an in-app tree for searching, copying,
   editing, deleting, and undoing entry changes.
2. **The MCP `kbdx` tool** — a scriptable export of a whole vault to JSON,
   YAML, or a password-only `.env` file, for AI agents and automation.

Both share the same backend (`keepass` crate, KDBX3/KDBX4) and the same
`.kdbx` / `.kbdx` (case-insensitive) extension match.

## 1. TUI vault browser

### Build

KeePass support is enabled in the default build. To request it explicitly:

```bash
cargo run --features keepass
```

### Opening a vault

Point either panel at a `.kdbx`/`.kbdx` file and press:

| Key | Effect |
|-----|--------|
| `Enter` | Opens the vault browser (checked *before* FFM would otherwise treat an executable-flagged `.kdbx` as a program to run) |
| `F3` | Same vault-open check, from the viewer path |

### Unlocking

- Type the master password (masked as `•` characters), then `Enter` to unlock.
  The one-symbol `◉` button (or `Ctrl+R`) shows/hides the typed value.
- `Esc` cancels and closes the overlay without unlocking.
- A wrong password and a corrupt/unreadable database show the same message,
  `Wrong password or key file` — deliberately merged, since the two are
  indistinguishable from the caller's side.
- **Key-file unlock is not reachable from the TUI today.** The backend
  supports combined password + key-file databases, but there is no keybinding
  or `:` command to supply a key file in the current build. Use the
  [scripting alternative](#scripting-alternative-kdbx_dump) below for
  key-file-protected vaults.

### Browsing

Once unlocked, the overlay starts in **Tree** view and includes empty groups.
`F6` switches between the tree and the column list.

| Key | Effect |
|-----|--------|
| `Up` / `Down` | Move selection one row (no wrap past the first/last row) |
| `PageUp` / `PageDown` | Jump 10 rows |
| Type any character | Filters live by case-insensitive substring in title, username, URL, or group path |
| `Backspace` | Shrinks the filter |
| `Ctrl+R` | Open/close a dialog showing the selected password; `Enter` in it copies the password |
| `Enter` | Copy the selected entry's password to the clipboard |
| `Ctrl+U` | Copy the selected entry's username to the clipboard |
| `F4` | Open the selected entry for editing when the vault was opened in edit mode |
| `F8` | Ask for confirmation, then delete the selected entry |
| `Ctrl+Z` | Undo the latest saved edit or deletion |
| `Esc` | Close the overlay and remember the key for this FFM process |
| `Ctrl+L` | Lock the vault, forget cached keys, and close the overlay |
| `Ctrl+C` | Lock the vault, then quit FFM |

Moving the selection or editing the filter closes the password dialog and
zeroizes its plaintext value, so a stale password cannot remain attached to
another entry.

Copying tries the native OS clipboard first and falls back to an OSC 52
terminal escape sequence over SSH/headless sessions; the status bar reports
which route was used, so you know when to check the terminal instead of the
system clipboard.

### Editing and deletion

Open the database with `F4`, then press `F4` on an entry. The password is
visible in the edit dialog and `Ctrl+C` copies it. `Tab` / `Shift+Tab` moves
between fields, `Ctrl+Z` undoes the latest field change, and `F2` saves.
Leaving a changed form asks whether to save, discard, or continue editing.

Every save or `F8` deletion writes an encrypted `.bak` before replacing the
database. `Ctrl+Z` in the browser restores the latest backup and refreshes the
tree. Only one persisted change can be undone.

### Known limitation

Key-file unlock is supported by the backend but cannot yet be selected from
the TUI.

Every other key inside the vault overlay is intentionally swallowed and does
nothing, so an unmapped keystroke can never leak through to the file panel.

### Scripting alternative: `kdbx_dump`

For key-file-protected vaults, or scripted/offline dumps, use the standalone
example binary instead of the TUI:

```bash
KDBX_PASSWORD='your-password' \
  cargo run --quiet --example kdbx_dump --features keepass -- /path/to/vault.kdbx
```

Set `KDBX_KEYFILE=/path/to/keyfile` for a combined password + key-file
database. Output goes to stdout only (`group/title  user: ...  pass: ...  url: ...`),
one line per entry — never to a log file.

## 2. MCP `kbdx` tool

For AI agents and automation connected over MCP (see [`mcp/MCP.md`](../mcp/MCP.md)),
the `kbdx` tool exports a whole vault as structured text in one call. It has
no feature flag — the MCP server always depends on the `keepass` crate.

### Parameters

| Field | Type | Notes |
|-------|------|-------|
| `path` | string, required | Must exist, must resolve inside the server's allowed root, extension `.kdbx` or `.kbdx` |
| `password` | string, optional | Master password |
| `keyfile` | string, optional | Must also resolve inside the allowed root |
| `format` | `json` \| `yaml` \| `env`, optional | Defaults to `json` |

At least one of `password` or `keyfile` is required.

### Output formats

- **`json`** (default) — the complete decrypted model: every group, entry,
  and field, including protected/plaintext passwords and custom fields.
- **`yaml`** — the same complete model, structurally identical to `json`.
- **`env`** — deliberately narrower: one line per entry that currently has a
  password field, as `VAR="value"`. Custom fields, URLs, and notes are
  omitted. Variable names are built as
  `KDBX_<GROUP_PATH_AND_TITLE>_PASSWORD` (non-alphanumeric runs collapse to
  `_`, uppercased); a name collision gets a numeric suffix inserted before
  `_PASSWORD` (`KDBX_API_PASSWORD`, then `KDBX_API_2_PASSWORD`). Values are
  shell-escaped and double-quoted, including `$` → `\$`, so sourcing the
  output can't trigger unintended shell expansion.

Example `env` output for a vault with a root entry `GitHub` and two entries
named `API token` under group `Work`:

```env
KDBX_GITHUB_PASSWORD="root-secret"
KDBX_WORK_API_TOKEN_PASSWORD="a\\b\"c\n\${TOKEN}\t"
KDBX_WORK_API_TOKEN_2_PASSWORD="second-secret"
```

### Limits and errors

- Vault size is capped at 64 MB, checked before decryption; larger files are
  rejected outright (never truncated).
- Output size is capped at 64 MB, checked after export; an over-limit result
  is rejected rather than truncated, so a successful response is always whole.
- Wrong extension, non-existent path, wrong password/corrupt database,
  invalid key file, and serialization failure all return distinct,
  credential-free error messages — the supplied password is never echoed.
- `path` and `keyfile` are both root-confined the same way every other MCP
  file tool is; a path outside the allowed root is rejected.

### Security note

The `kbdx` tool is read-only and never writes an export file to disk, but its
response **deliberately contains plaintext secrets**. Treat the MCP response
as secret material — anything reading the response (log, terminal scrollback,
another tool) sees the vault's decrypted contents.
