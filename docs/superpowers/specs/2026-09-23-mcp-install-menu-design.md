<!-- Purpose: Design for the F9 Commands row that registers ffm-mcp with Claude Code, Codex, Junie and Kiro, and for the one client table it shares with the :mcp status screen. -->

# Install MCP server from the menu: design

_2026-09-23. Status: implemented on `feat/mcp-install-menu`. This file is the
design as planned; §12 lists where the code departs from it. The behavior is
documented in `docs/session-tools.md` ("MCP server setup")._

## 1. What the user asked for

"Add to menu function to install FFM as MCP Server to Claude Code / Codex /
Junie / Kiro."

The user settled these points before design:

- A Far-style dialog with one checkbox per client (Claude Code, Codex, Junie,
  Kiro). Each row says whether the client was detected and whether ffm-mcp is
  already registered.
- A scope switch: **All projects** (user level) or **Current folder** (the
  active panel's folder, project level). The default is All projects.

## 2. Facts established

- `ffm-mcp` confines itself to `$FFM_MCP_ROOT`, or else to its working
  directory (`mcp/src/paths.rs`, `ROOT_ENV`). A user-level entry therefore
  follows whichever project the client starts it in, as long as the client
  starts it in that project.
- `install-mcp.sh` is the only installer today. It has three defects that
  this design does not copy:
  - It writes Junie to `.junie/mcp.json`. The documented path is
    `.junie/mcp/mcp.json`.
  - It runs `claude mcp add` without `--scope`, which means local scope
    (this project, private). It is not user scope.
  - It recreates any config file that does not parse.

  A Python port of the script is being written elsewhere. This branch does
  not touch the script (see §10).
- The `:mcp` screen (`src/ops/viewer.rs`, `mcp_configs`) has its own client
  table. Several entries in it are wrong:
  - Codex points at `~/.codex/mcp_config.json`, which Codex does not read.
    Codex reads `$CODEX_HOME/config.toml`.
  - Junie has the same wrong path as the script.
  - The "VS Code/Cursor" row reads `.mcp.json`. That is Claude Code's project
    file: VS Code reads `.vscode/mcp.json` and Cursor reads
    `.cursor/mcp.json`.
  - The screen looks for the server binary at
    `<cwd>/mcp/target/release/ffm-mcp`.
- The client CLIs:
  - `codex mcp add` has no scope flag and always writes the global file.
  - Claude Code writes `~/.claude.json` itself, and that file also holds the
    sign-in session, so nothing else should write it.
  - Junie and the Kiro IDE have no non-interactive add command. Their
    documented method is editing the JSON file.
- Working directory at user level:
  - Codex starts stdio servers in the session's working directory.
  - A Kiro issue (kirodotdev/Kiro#6525) shows servers started in the Kiro
    install folder.
  - Junie's behavior is unverified.

  So a user-level entry may be confined to the wrong folder in Kiro and
  Junie.
- Dependencies:
  - The root crate has `serde_json`, but without `preserve_order`. Only a
    dev-dependency enables that feature, so tests keep key order while the
    real binary would sort a rewritten user file.
  - There is no TOML crate. `toml_edit` 0.25 (MIT or Apache-2.0) and its
    dependencies are already in the local Cargo registry.

## 3. The menu row

- `Entry::command("Install MCP s&erver", "", MenuCommand::McpInstall)` in
  `COMMANDS` (`src/menu/tables.rs`).
- It goes directly after `Pl&ugin commands` (F11, whose MCP status row
  opens the `:mcp` screen), at index 22.
- Hotkey `e`. It is free on every platform. The other free letters are b,
  j, q, x and z.
- The rows that `lsp.feature` and `path.feature` reach by position (0–4) do
  not move.
- `MenuCommand::McpInstall` has no payload, because the scope is chosen in
  the dialog. It is handled in `ops::menu::execute_tools`, which calls
  `crate::ops::mcp_install::open(app)`.
- Timing label: `Menu/McpInstall`. It contains no path.

## 4. The dialog

### 4.1 Layout

It is drawn with `ui::dialog` (`Row::Label`, `Row::Radio`, `Row::Checkbox`,
`Row::Divider`, `Button`) on `Ground::Prompt`, titled `Install MCP server`.
The drawings below show an 80×24 screen. The frame glyphs are the ones every
`ui::dialog` draws.

All projects, with the server found:

```
  ╔═════════════════════════ Install MCP server ═════════════════════════╗
  ║ Server: /home/user/.cargo/bin/ffm-mcp                                ║
  ╟──────────────────────────────────────────────────────────────────────╢
  ║ Scope: (•) All projects ( ) Current folder                           ║
  ║ Current folder: /srv/project                                         ║
  ╟──────────────────────────────────────────────────────────────────────╢
  ║ [x] Claude Code  detected      ⊘ not configured                      ║
  ║ [x] Codex        detected      ≠ different entry                     ║
  ║ [ ] Junie        not detected  — no config                           ║
  ║ [x] Kiro         detected      ✓ registered                (dimmed)  ║
  ╟──────────────────────────────────────────────────────────────────────╢
  ║ Junie: may start ffm-mcp outside the project; prefer Current folder. ║
  ║ Kiro: may start ffm-mcp outside the project; prefer Current folder.  ║
  ╟──────────────────────────────────────────────────────────────────────╢
  ║                      { Register }  [ Cancel ]                        ║
  ╚══════════════════════════════════════════════════════════════════════╝
```

Current folder, with the server missing:

```
  ╔═════════════════════════ Install MCP server ═════════════════════════╗
  ║ Server: ffm-mcp was not found next to ffm or on PATH.                ║
  ║ Install it from the FFM sources: cargo install --path mcp            ║
  ╟──────────────────────────────────────────────────────────────────────╢
  ║ Scope: ( ) All projects (•) Current folder                           ║
  ║ Current folder: /srv/project                                         ║
  ╟──────────────────────────────────────────────────────────────────────╢
  ║ [ ] Claude Code  detected      — no config                 (dimmed)  ║
  ║ [ ] Codex        detected      ⊘ not configured            (dimmed)  ║
  ║ [ ] Junie        not detected  — no config                 (dimmed)  ║
  ║ [x] Kiro         detected      ✓ registered                (dimmed)  ║
  ╟──────────────────────────────────────────────────────────────────────╢
  ║ Claude Code: asks once before it starts a server from .mcp.json.     ║
  ║ Codex: reads .codex/config.toml only in folders it trusts.           ║
  ╟──────────────────────────────────────────────────────────────────────╢
  ║                      [ Register ]  { Cancel }      (Register dimmed) ║
  ╚══════════════════════════════════════════════════════════════════════╝
```

Layout rules:

- **Height.** Twelve body rows, or thirteen when the server is missing, plus
  `CHROME_ROWS` (6) is at most 19. That fits 24 rows.
- **Width.** The widest row is 68 cells, within the 72 an 80-column screen
  leaves inside the chrome.
- **Paths.** They are drawn through `path_display::label` and clipped at the
  dialog's width. The report (§4.5) carries them in full.
- **Row label.** `format!("{name:<12} {presence:<13} {state}")`, where `state`
  is `ConfigState::label()`. The `:mcp` screen uses the same function (§6),
  so the two screens use the same words.
- **Notes.** The rows under the clients are the table's notes for the
  selected scope (§5.1), prefixed with the client's name. They are
  static per scope, so the dialog does not change height when a box is
  ticked.
- **Folder not available.** If the active panel is not showing a local
  folder that exists, for example a NetBox panel, an S3 path or a folder
  that has gone:
  - The folder line reads `Current folder: the active panel is not a local
    folder`.
  - The radio stays on All projects.

### 4.2 Row states and default marks

"Presence" is `detected` or `not detected` (§5.3). "State" is the config's
state for the selected scope (§5.4).

| State (`ConfigState`)                          | Label                                                | Enabled     | Default mark                 |
| ---------------------------------------------- | ---------------------------------------------------- | ----------- | ---------------------------- |
| `NoConfig` (file absent)                       | `— no config`                                        | yes         | checked if detected          |
| `NotConfigured` (file parses, no `ffm-mcp`)    | `⊘ not configured`                                   | yes         | checked if detected          |
| `DifferentEntry` (other command, args or root) | `≠ different entry`                                  | yes         | checked (it will be updated) |
| `Registered`                                   | `✓ registered`                                       | no (dimmed) | checked (shown as done)      |
| `Blocked(Unparsable)`                          | `✗ config does not parse`                            | no          | unchecked                    |
| `Blocked(UnexpectedLayout)`                    | `✗ unexpected config layout`                         | no          | unchecked                    |
| `Blocked(Unreadable)`                          | `✗ config unreadable`                                | no          | unchecked                    |
| `Blocked(NeedsClaudeCli)`                      | `✗ needs the claude CLI`                             | no          | unchecked                    |
| `Blocked(HomeVarNotFolder(v))`                 | `✗ $CODEX_HOME is not a folder` (the variable named) | no          | unchecked                    |
| `Blocked(NoHomeFolder)`                        | `✗ no home folder`                                   | no          | unchecked                    |

- **Server missing.** Every row is disabled. Rows keep their mark: registered
  rows show `[x]` and all others show `[ ]`. `Register` is disabled.
- **Not-detected clients stay enabled.** A file-based registration still
  works without the client installed, which is useful for preparing a
  project for teammates. Those rows are unchecked by default.
- **Registered rows are dimmed and checked.** An unchecked "registered" row
  would read as "will be removed". This dialog never removes anything (§10).

### 4.3 Keys and mouse

Keys, as the Add to archive dialog (`src/popup/archive_prompt.rs`) handles
them:

| Key                       | Effect                                                                                                                                                                                                                                                   |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Tab, Down / Shift+Tab, Up | Next / previous control in the focus order: the scope radio, then enabled client rows in table order, then `Register` (when enabled), then `Cancel`. Disabled rows are skipped.                                                                          |
| Left / Right on the radio | Switch scope. Calls `ops::mcp_install::switch_scope`, which surveys again for the new scope and resets every mark to that scope's defaults. Does nothing when the folder is not available.                                                               |
| Space on a client row     | Toggle its mark.                                                                                                                                                                                                                                         |
| Enter                     | On `Cancel` it cancels. Anywhere else it registers the checked rows (the default button). With the server missing it closes the dialog. With nothing checked the dialog stays open and the status line says `Nothing to register: check a client first`. |
| Esc, F10                  | Close without changes.                                                                                                                                                                                                                                   |
| Mouse                     | `Register` / `Cancel` through `dialog_button_key` (`application/dispatch/dialog_mouse/button_keys.rs`). A click sets the focus to that button and sends Enter, as `archive_button_key` does.                                                             |

Focus starts on the scope radio.

### 4.4 The scope switch

|                             | All projects (`Scope::AllProjects`)                                                             | Current folder (`Scope::CurrentFolder`)                                                                                                                                                    |
| --------------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Files                       | User-level (§5.5)                                                                               | `<folder>/…` (§5.5)                                                                                                                                                                        |
| `FFM_MCP_ROOT` in the entry | Must be absent. It is removed if present, because a fixed root would contradict "all projects". | Set to the absolute folder path. This follows the research recommendation for Kiro and Junie. The entry is machine-specific in either case, because `command` is already an absolute path. |
| Notes shown                 | Junie and Kiro: working-directory warning                                                       | Claude Code: approval prompt. Codex: trusted folders only.                                                                                                                                 |

### 4.5 Result reporting

`Register` closes the dialog and opens
`Viewer::report("MCP server registration", lines)`, the same viewer `:mcp`
uses. It also sets the status line. Example report:

```
ffm-mcp: /home/user/.cargo/bin/ffm-mcp
Scope: all projects (user level)

Claude Code  added      ran: claude mcp add --scope user --transport stdio ffm-mcp -- /home/user/.cargo/bin/ffm-mcp
Codex        updated    /home/user/.codex/config.toml
Kiro         unchanged  /home/user/.kiro/settings/mcp.json
Junie        failed     /home/user/.junie/mcp/mcp.json does not parse (expected value at line 3 column 1); left untouched. Fix or remove it, then register again.

Notes:
  Kiro: may start ffm-mcp outside the project; prefer Current folder.
  Restart running clients, or reload their MCP servers, to pick up the change.
```

- **Status line on success.**
  `Status::CommandOutput("ffm-mcp: 1 added, 1 updated, 1 unchanged")`.
- **Status line on failure.** Any failure gives
  `Status::Error("ffm-mcp: Junie failed; see the report")`, naming every
  client that failed.
- **Runs synchronously, after saying so.** Enter only marks the dialog
  `Registering` and puts what is about to run in place of the notes
  (`Registering ffm-mcp: waiting for claude (up to 30 s a call)…`, or
  `Registering ffm-mcp…`); the status line would be hidden under the dialog
  on an 80×24 screen. The event loop draws that frame, then registers on its
  own thread. Only Claude Code at All projects
  starts a process: at most two `claude` calls, each bounded to 30 s, and
  normally about a second. A background job would be more machinery than
  this needs.
- **A stop stops the whole tree.** On Unix the CLI leads a process group of
  its own and the group is killed at the time limit; on Windows it runs in
  a kill-on-close job. An npm or Volta shim is the direct child, and killing
  only it left the real CLI running, able to write its config after the
  report said it did not finish.

## 5. The shared client table and the mechanics

### 5.1 One table (`src/mcp_clients.rs`)

```rust
pub struct Client {
    pub id: ClientId,
    pub name: &'static str,
    /// Looked up on PATH: any one found means "detected", and the first found
    /// is the program run when `user_level` is `ClaudeCli`.
    pub programs: &'static [&'static str],
    /// The variable that moves the client's home folder, if it has one.
    pub home_var: Option<&'static str>,
    /// The client's home folder under the user's home when `home_var` is
    /// unset; "" is the user's home itself.
    pub home_dir: &'static str,
    /// The user-level config, relative to the client's home folder.
    pub user_config: &'static str,
    /// The project-level config, relative to the folder.
    pub project_config: &'static str,
    pub format: ConfigFormat,
    /// How a user-level registration is written. Project level is always
    /// `Mechanism::EditFile`.
    pub user_level: Mechanism,
    pub user_note: Option<&'static str>,
    pub project_note: Option<&'static str>,
}
```

| `id`         | `name`      | `programs`         | `home_var`          | `home_dir` | `user_config`       | `project_config`          | `format` | `user_level` |
| ------------ | ----------- | ------------------ | ------------------- | ---------- | ------------------- | ------------------------- | -------- | ------------ |
| `ClaudeCode` | Claude Code | `claude`           | `CLAUDE_CONFIG_DIR` | `""`       | `.claude.json`      | `.mcp.json`               | Json     | `ClaudeCli`  |
| `Codex`      | Codex       | `codex`            | `CODEX_HOME`        | `.codex`   | `config.toml`       | `.codex/config.toml`      | Toml     | `EditFile`   |
| `Junie`      | Junie       | `junie`            | none                | `.junie`   | `mcp/mcp.json`      | `.junie/mcp/mcp.json`     | Json     | `EditFile`   |
| `Kiro`       | Kiro        | `kiro`, `kiro-cli` | `KIRO_HOME`         | `.kiro`    | `settings/mcp.json` | `.kiro/settings/mcp.json` | Json     | `EditFile`   |

Notes carried by the table:

- Junie and Kiro, All projects: "may start ffm-mcp outside the project;
  prefer Current folder."
- Claude Code, Current folder: "asks once before it starts a server from
  .mcp.json."
- Codex, Current folder: "reads .codex/config.toml only in folders it
  trusts."

How the table is used:

- Relative paths are written with `/` in the table and joined one component
  at a time, so Windows shows `\`.
- `CLIENTS: [Client; 4]` is the only list of clients in the crate. The
  dialog, the installer and the `:mcp` screen all iterate it.
- Adding a client that uses one of these formats is one new row. Gemini, for
  example, would use Json with `~/.gemini/settings.json` and
  `.gemini/settings.json`.

### 5.2 Resolved paths

| Client      | All projects                                                     | Current folder                  |
| ----------- | ---------------------------------------------------------------- | ------------------------------- |
| Claude Code | where the CLI looks (below); FFM only reads it                   | `<dir>/.mcp.json`               |
| Codex       | `$CODEX_HOME/config.toml`, else `~/.codex/config.toml`           | `<dir>/.codex/config.toml`      |
| Junie       | `~/.junie/mcp/mcp.json`                                          | `<dir>/.junie/mcp/mcp.json`     |
| Kiro        | `$KIRO_HOME/settings/mcp.json`, else `~/.kiro/settings/mcp.json` | `<dir>/.kiro/settings/mcp.json` |

- **Home folder.** On Unix it is `HOME`. On Windows it is `USERPROFILE`,
  then `HOME`: that is what Node's `os.homedir()` (Claude) and Rust's
  `dirs::home_dir()` (Codex) use. The Windows locations for Junie and Kiro
  are unverified and use the same rule.
- **Override variable that is not a folder.** If a `home_var` is set but does
  not name an existing folder, the user-level row is
  `Blocked(HomeVarNotFolder(var))`. Codex refuses to start in that state, and
  FFM does not guess what the others do.
- **No home folder and no override.** User-level rows are
  `Blocked(NoHomeFolder)`. A home that is set but is not an existing folder
  counts as none: registering never creates a home folder, and the `claude`
  CLI is never started in a folder that is not there.
- **Claude Code's user config** is chosen as the CLI chooses it (read from
  the installed `claude`): a legacy `.config.json` in Claude's own folder
  (`$CLAUDE_CONFIG_DIR`, else `~/.claude`) wins whenever it exists;
  otherwise `.claude.json` in `$CLAUDE_CONFIG_DIR`, else in the home, named
  `.claude-custom-oauth.json` while `CLAUDE_CODE_CUSTOM_OAUTH_URL` is set.
- **A folder whose config is the user-level one.** When the Current folder
  path of Codex, Junie or Kiro resolves to the same file as its All projects
  path — the panel is on the home folder, or `CODEX_HOME` / `KIRO_HOME`
  points into the folder — the row is `Blocked(SameAsUserConfig)`
  (`✗ same file as All projects`). Writing it would put `FFM_MCP_ROOT=<home>`
  into the user-level config and confine every project to the home. Paths
  are compared after resolving the longest existing ancestor, so links and
  files not created yet compare correctly.
- **Windows spelling.** The panel's folder is canonical, so on Windows it is
  `\\?\C:\…`. `ProjectDir` stores the ordinary spelling (`C:\…`,
  `\\server\share\…`), through the same `path_display` helper the panel
  labels use, so no verbatim prefix reaches `FFM_MCP_ROOT` or a config path.

### 5.3 Detection (`Presence`)

A client is `Detected` if any of these holds:

- one of its `programs` is on PATH;
- its user-level config file exists;
- its `home_dir` is not `""` and the client's home folder exists.

In practice:

- Claude Code is detected by `claude` on PATH or by `~/.claude.json`.
- Codex, Junie and Kiro are also detected by `~/.codex/`, `~/.junie/` or
  `~/.kiro/` (or the override folder).

PATH lookup (`find_program`):

- Each PATH directory is tried with each suffix in `Host::program_suffixes`.
  On Unix the list is `[""]`, and a candidate must be a regular file with an
  execute bit. On Windows the list is PATHEXT (default
  `.COM;.EXE;.BAT;.CMD`, lower-cased), so the npm shims `claude.cmd` and
  `codex.cmd` are found.
- Parsing PATHEXT is a pure function, tested on every platform. Only reading
  the variable is `cfg(windows)`.
- The same helper finds the server (§5.6).

### 5.4 Reading a config (`ConfigState`)

This runs at open time, on every scope switch, and again right before each
write. Order of checks:

1. **File absent:** `NoConfig`.
2. **Read fails:** `Blocked(Unreadable(msg))`. Examples: permission denied,
   the path is a folder, the bytes are not UTF-8. Only a regular file of at
   most 64 MiB is read (`fileload::read_regular`, the viewer's guard): a
   FIFO named `.mcp.json` would otherwise block the dialog forever, and a
   link to `/dev/zero` would fill the memory.
3. **Parse:**
   - JSON uses `serde_json::Value`. An empty or whitespace-only file counts
     as `{}`. A leading UTF-8 BOM is accepted and written back.
   - TOML uses `toml_edit::DocumentMut`.
   - A parse error gives `Blocked(Unparsable(parser message with line and
     column))`.
4. **Shape:** any of the following gives `Blocked(UnexpectedLayout(key))`:
   - the JSON root is not an object;
   - `mcpServers` (JSON) or `mcp_servers` (TOML) is present but is not an
     object or table (inline tables and dotted keys count as tables);
   - `ffm-mcp` is present but is not an object or table.
5. **The `ffm-mcp` entry:**
   - Absent: `NotConfigured`.
   - Present and matching the desired entry (below): `Registered`.
   - Present and not matching: `DifferentEntry`.
   - Present and matching, but switched off (`"disabled": true` or
     `enabled = false`, in either format): `Disabled`
     (`✓ registered (disabled)`). It is dimmed like `Registered`, and
     registering leaves it off: that is its owner's choice.
   - Present and the server not found: `Unverified`
     (`• entry present, server not found`), because there is nothing to
     compare it with. When its `command` is an absolute path with nothing
     there, the ops layer makes it `ProgramMissing`
     (`≠ entry runs a missing program`). A bare name is left `Unverified`:
     the client's PATH is not FFM's to see.
6. **Claude Code at All projects only:** if the state is not `Registered` and
   `claude` is not on PATH, the result is `Blocked(NeedsClaudeCli)`.

An entry matches (is `Registered`) when all of these hold. Every other key in
the entry and in its `env` is ignored:

- `command` is the server path, compared as a string;
- `args` is absent or empty;
- `env.FFM_MCP_ROOT` is absent at All projects, or equals the folder at
  Current folder;
- nothing starts it another way: no `url`, no `cwd`, and no `type` other
  than `stdio`. A `url` is not this server, and a `cwd` pins it to one
  folder, so such an entry is `DifferentEntry`.

For Claude Code at All projects the object read is the top-level
`mcpServers` in `.claude.json`. Local-scope entries under `projects[<dir>]`
are not read (§10).

### 5.5 Writing, per client and scope

| Client      | Scope          | How                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Written entry                                                                                                                                      |
| ----------- | -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Claude Code | All projects   | `claude mcp add --scope user --transport stdio ffm-mcp -- <server>`, working directory = Claude's home. When the state is `DifferentEntry`, `claude mcp remove --scope user ffm-mcp` runs first, because `add` refuses a name that already exists, and then `claude mcp add-json --scope user ffm-mcp '<entry>'` with the old entry merged exactly as a file edit would merge it (rule 2 below), so its `env` and every key Claude knows survive; `add` would keep only the command. | Written by claude: `{"type":"stdio","command":"<server>","args":[],"env":{}}`, or the merged entry                                                 |
| Claude Code | Current folder | FFM edits `<dir>/.mcp.json`. This is a documented, shared file format, so no CLI is needed.                                                                                                                                                                                                                                                                                                                                                                                          | `{"command":"<server>","args":[],"env":{"FFM_MCP_ROOT":"<dir>"}}`. `type` is optional and defaults to stdio. A `type` already in the file is kept. |
| Codex       | both           | FFM edits the TOML file with `toml_edit`.                                                                                                                                                                                                                                                                                                                                                                                                                                            | `[mcp_servers.ffm-mcp]` with `command = "<server>"`, `args = []`, plus `[mcp_servers.ffm-mcp.env]` `FFM_MCP_ROOT = "<dir>"` at Current folder      |
| Junie, Kiro | both           | FFM edits the JSON file.                                                                                                                                                                                                                                                                                                                                                                                                                                                             | `{"command":"<server>","args":[],"env":{}}`, with `FFM_MCP_ROOT` at Current folder. No `type` key.                                                 |

**Codex writes the file at both scopes.** The alternative was running
`codex mcp add` at All projects. The file edit won for three reasons:

- Current folder has no CLI route, so FFM needs a TOML editor anyway.
- With one route, both Codex scopes get the same checks: no write unless the
  file parses, and nothing else in the file changes.
- No process means no timeout, and the tests need no runner.

The cost is that Codex's own config validation does not run. Only the three
documented keys are written, which keeps that risk small.

Rules every file edit follows (`ops/mcp_install/config_file.rs`):

1. **Read it again and check it.** Fetch the state again just before the
   write. `Registered` reports `Unchanged` and writes nothing. `Blocked(_)`
   reports `Failed` and writes nothing.
2. **Merge.** Absent containers are created: `mcpServers` / `[mcp_servers]`
   (TOML as an implicit table, so the file gains only
   `[mcp_servers.ffm-mcp]`), and the entry itself. On the entry:
   - `command` is set;
   - `args` is set to `[]`;
   - `env.FFM_MCP_ROOT` is set at Current folder and removed at All projects;
   - `url`, `cwd` and a `type` other than `stdio` are removed;
   - every other key in the file, in the entry and in `env` is left alone.
3. **Serialize.**
   - JSON: only the entry is rebuilt. Every other value is carried as the
     raw text it was read in (`serde_json` `raw_value`, in an ordered
     `indexmap`), because a `Value` round trip is not exact: `1e3` came back
     `1000.0` and a 24-digit integer lost its tail. Key order is kept, the
     indentation is the file's own (copied from its first indented key, two
     spaces when there is none), CRLF line ends stay CRLF, and a trailing
     newline is added.
   - TOML: `DocumentMut::to_string()`, which keeps comments and formatting.
4. **Write atomically.** Steps in order:
   - If the file exists, write through `fs::canonicalize`, so a symlinked
     dotfile keeps its link and its target is replaced.
   - Otherwise create the parent folders with `create_dir_all`.
   - Then `file_write::ensure_writable`, `file_write::temp_beside` and
     `file_write::fill` (keeps the file's permissions, writes, fsyncs).
   - Right before the rename the file is read once more and compared with
     what was read in rule 1. If the client saved it meanwhile (Kiro and
     Junie write theirs from the IDE), nothing is renamed and the report
     says `it changed while being written, and was left as it now is;
     register again`.
   - Then the rename.
   - A new file gets the temp file's `0600`.
   - Nothing is written until the new text exists in full, so a failure
     leaves the old file as it was.
5. **No `.bak` file.** A backup beside `.mcp.json` would land in the user's
   repository. The design relies on the rules above instead: no write unless
   the file parses, one entry changed, an atomic rename, and every written
   path named in the report.

Rules for the Claude CLI (`ops/mcp_install/claude.rs`):

- `CliCall { program, args, cwd, timeout: CLI_TIMEOUT }` goes through
  `Host::runner`, with `CLI_TIMEOUT = 30 s`.
- `program` is the absolute path from `find_program`. It is never a bare
  name, because `Command` does not apply PATHEXT.
- stdin is null. stdout and stderr are captured, up to the existing 8 MiB
  limit. The client never takes over the terminal.
- Before running anything, `.claude.json` must parse, or be absent. This is
  step 5.4.3 above. The CLI's own behavior on a corrupt file is unverified,
  so the CLI is never run against one.
- After `add` exits 0, FFM reads `.claude.json` again and expects
  `Registered`. Anything else reports `Failed(NotVerified)`.
- Rust refuses to pass arguments to a `.cmd`/`.bat` file when it cannot
  escape them (`InvalidInput`). That is reported as a CLI failure. The native
  Claude installer ships `claude.exe`, so this should be rare.

### 5.6 Locating the server (`ServerState`)

Lookup order, stopping at the first hit:

1. `Host::exe_dir` (the parent of `current_exe()`) joined with `ffm-mcp`,
   trying each program suffix. This is where `cargo install --path mcp`
   leaves it next to `ffm`.
2. `find_program(host, "ffm-mcp")` on PATH.

Result:

- The path is used exactly as found and is never canonicalized, so no
  `\\?\` prefix reaches a config file.
- Found and valid Unicode: `Found(ServerBinary)`.
- Found but not Unicode: `NotUnicode(path)`, shown like a missing server,
  with the path named. JSON and TOML cannot carry such a path.
- Otherwise: `Missing`. The dialog shows the two lines drawn in §4.1.
  Nothing is built.

### 5.7 Failure handling

| Situation                                                                 | Found at                                                   | Dialog                                            | Register                                                                              | Report                                                                                                                  |
| ------------------------------------------------------------------------- | ---------------------------------------------------------- | ------------------------------------------------- | ------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Config does not parse                                                     | survey and again before writing                            | dimmed `✗ config does not parse`                  | skipped                                                                               | `<path> does not parse (<message>); left untouched. Fix or remove it, then register again.`                             |
| Config has an unexpected layout                                           | same                                                       | dimmed `✗ unexpected config layout`               | skipped                                                                               | `<path>: <key> is not an object; left untouched`                                                                        |
| Config unreadable                                                         | same                                                       | dimmed `✗ config unreadable`                      | skipped                                                                               | `cannot read <path>: <error>`                                                                                           |
| Cannot write (permission denied, read-only filesystem, parent is a file)  | Register                                                   | —                                                 | `ensure_writable`, `create_dir_all`, temp file or rename fails; the file is untouched | `cannot write <path>: <error>`                                                                                          |
| `claude` not on PATH (Claude Code, All projects)                          | survey                                                     | dimmed `✗ needs the claude CLI`                   | —                                                                                     | —                                                                                                                       |
| `claude` gone by the time Register runs                                   | Register (`NotFound`, and the program is no longer a file) | —                                                 | failed                                                                                | `claude CLI not found`                                                                                                  |
| Any other `NotFound` from the run (a working or temp folder that is gone) | Register                                                   | —                                                 | failed                                                                                | the error, plus the command to run by hand                                                                              |
| `claude` runs past 30 s                                                   | Register (`TimedOut`)                                      | —                                                 | child killed, failed                                                                  | `claude did not finish within 30 s. Run by hand: <add command>`                                                         |
| `claude` exits non-zero                                                   | Register                                                   | —                                                 | failed                                                                                | last drawable stderr line (`console::first_drawable`), plus the command to run by hand                                  |
| `remove` failed                                                           | Register                                                   | —                                                 | failed                                                                                | the reason. `Run by hand: claude mcp remove --scope user ffm-mcp && claude mcp add-json --scope user ffm-mcp '<entry>'` |
| `remove` succeeded, `add-json` failed or was not verified                 | Register                                                   | —                                                 | failed                                                                                | `the old entry was removed but the new one was not added: <reason>. Run by hand: <add-json command>`                    |
| `add` exited 0 but the entry is missing or differs                        | Register (read again)                                      | —                                                 | failed                                                                                | `claude reported success but <path> has no matching ffm-mcp entry`                                                      |
| Override variable is not a folder / no home folder                        | survey                                                     | dimmed, as §4.2                                   | skipped                                                                               | —                                                                                                                       |
| Server missing or not Unicode                                             | open                                                       | §4.1 second drawing                               | Enter closes                                                                          | —                                                                                                                       |
| Active panel is not a local folder                                        | open                                                       | folder line explains; radio fixed on All projects | —                                                                                     | —                                                                                                                       |
| Nothing checked                                                           | Register                                                   | stays open                                        | status `Nothing to register: check a client first`                                    | —                                                                                                                       |

One client's failure never stops the others. Rows are processed in table
order, and each gets one outcome.

## 6. The `:mcp` status screen

`open_mcp_status` keeps its owner (`src/ops/viewer.rs`) and its pinned
signature. The changes:

- **Client table.** `mcp_configs` and `mcp_config_lines` are deleted. The
  screen calls the installer's `survey` for both scopes and prints one line
  per client with `ConfigState::label()`:

  ```
  FFM MCP Server — 41 AI Tools

  Server: /home/user/.cargo/bin/ffm-mcp
  Folder: /srv/project

  Supported IDEs:                 All projects             Current folder
    Claude Code     detected      ✓ registered             — no config
    Codex           detected      ⊘ not configured         — no config
    Junie           not detected  — no config              — no config
    Kiro            detected      ✗ config does not parse  ✓ registered

  Commands:
    F9 Commands > Install MCP server   Register ffm-mcp with these clients
    :mcp                               Show this screen

  Tools:
    …
  ```

- **Sources.** The folder is the active panel's folder. It is no longer
  `std::env::current_dir()`. The home folder and PATH come from
  `app.mcp_host` (§7.3), no longer from `HOME` / `USERPROFILE` read on the
  spot. The `Server:` line uses §5.6. When the server is missing, the line
  says so and gives the install command.
- **Gemini and VS Code/Cursor are dropped.** FFM cannot register with either.
  The VS Code/Cursor row read Claude Code's `.mcp.json`, so it was wrong
  anyway. Gemini can come back as a full client with one table row (§5.1).
  This is the one visible reduction. The tests that pinned those names are
  updated (§8.1).
- **Stale commands removed.** `./install-mcp.sh` and the
  `mcp/target/release` path no longer appear on the screen.

## 7. Types, modules and seams

### 7.1 Domain types (`src/mcp_clients.rs`)

This module is pure: no `std::fs`, `std::process` or `std::env`. It holds the
table, the types, and the text-to-text JSON and TOML logic.

```rust
pub const SERVER_NAME: &str = "ffm-mcp";
/// `mcp/src/paths.rs::ROOT_ENV`. The TUI may not compile that file
/// (tests/module_boundaries/mcp.rs), so a unit test reads it as text and
/// checks the two still agree.
pub const ROOT_ENV: &str = "FFM_MCP_ROOT";

pub enum ClientId { ClaudeCode, Codex, Junie, Kiro }
pub enum Scope { #[default] AllProjects, CurrentFolder }
pub enum ConfigFormat { Json, Toml }
pub enum Mechanism { EditFile, ClaudeCli }
pub enum Presence { Detected, NotDetected }
pub enum ConfigState { NoConfig, NotConfigured, Registered, DifferentEntry, Blocked(Blocker) }
pub enum Blocker {
    Unparsable(String), UnexpectedLayout(&'static str), Unreadable(String),
    NeedsClaudeCli, HomeVarNotFolder(&'static str), NoHomeFolder,
}
pub enum ServerState { Found(ServerBinary), Missing, NotUnicode(PathBuf) }

/// Absolute path of an existing ffm-mcp, valid Unicode. Built only by the locator.
pub struct ServerBinary(PathBuf);
/// Absolute path of an existing local folder, valid Unicode.
pub struct ProjectDir(PathBuf);
/// What a registration writes: `root` is `Some` exactly at `Scope::CurrentFolder`.
pub struct ServerEntry { pub command: ServerBinary, pub root: Option<ProjectDir> }

pub struct ClientStatus { pub client: ClientId, pub presence: Presence, pub config: PathBuf, pub state: ConfigState }
pub struct Survey { pub server: ServerState, pub folder: Option<ProjectDir>, pub scope: Scope, pub clients: Vec<ClientStatus> }

pub struct Client { /* §5.1 */ }
pub const CLIENTS: [Client; 4] = [ /* §5.1 */ ];

// Text in, text out, per format (children json.rs and codex_toml.rs):
pub fn state(format: ConfigFormat, text: &str, desired: Option<&ServerEntry>) -> ConfigState;
pub fn upsert(format: ConfigFormat, text: &str, desired: &ServerEntry) -> Result<String, Blocker>;

/// What the installer needs from the machine. `Default` is inert: no home,
/// empty PATH, no executable folder, and a runner that always returns
/// `Unsupported`.
#[derive(Clone)]
pub struct Host {
    pub home: Option<PathBuf>,
    /// Only the variables the table names (`home_var`), by name.
    pub env: BTreeMap<&'static str, PathBuf>,
    pub search_path: Vec<PathBuf>,
    pub program_suffixes: Vec<String>,
    pub exe_dir: Option<PathBuf>,
    pub runner: Arc<dyn Runner>,
}
pub trait Runner: Send + Sync {
    fn run(&self, call: &CliCall) -> io::Result<Captured>;
}
pub struct CliCall { pub program: PathBuf, pub args: Vec<String>, pub cwd: PathBuf, pub timeout: Duration }
pub struct Captured { pub exit: Exit, pub stdout: String, pub stderr: String }
pub enum Exit { Success, Failure(Option<i32>) }
```

Outcomes live in `ops/mcp_install/report.rs`, because they are the results
of effects:

- `Outcome { Added, Updated, Unchanged, Failed(Failure) }`
- `Failure { Blocked(Blocker), Io { path, message }, CliMissing, CliTimedOut,
  CliFailed { exit, message }, RemovedNotAdded { message }, NotVerified }`

Style rules for these types:

- Enums replace every flag. Examples: `Mark { Checked, Unchecked }` rather
  than `checked: bool`, and `Presence` rather than `detected: bool`.
- Newtypes carry the paths that have invariants.
- Where the widget takes a `bool` (`Row::Checkbox { checked }`), the
  conversion happens in the `ui` code.

### 7.2 Where each piece lives

The layout follows the AGENTS.md ownership map and
`tests/module_boundaries`:

| File                                                       | Owns                                                                                                                                                                                                                                                                           |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `src/mcp_clients.rs` (+ `json.rs`, `codex_toml.rs`, tests) | §7.1. A root domain module like `packages` / `path_env`, declared `pub mod mcp_clients;` in `lib.rs`.                                                                                                                                                                          |
| `src/ops/mcp_install.rs`                                   | `open(app)`, `switch_scope(app, Direction)` and `register(app)`: the entry points the menu and popup call                                                                                                                                                                      |
| `src/ops/mcp_install/survey.rs`                            | `survey(host, scope, folder)`, `locate_server`, `find_program`, presence and state reading (the only `std::fs` reads)                                                                                                                                                          |
| `src/ops/mcp_install/config_file.rs`                       | The atomic, symlink-following write from §5.5                                                                                                                                                                                                                                  |
| `src/ops/mcp_install/claude.rs`                            | The `claude` remove/add calls and the check after them                                                                                                                                                                                                                         |
| `src/ops/mcp_install/report.rs`                            | `Outcome`, `Failure`, report lines, status line                                                                                                                                                                                                                                |
| `src/ops/mcp_install/system.rs`                            | `system_host()`, which reads `HOME`/`USERPROFILE`, the table's `home_var`s, `PATH`, `PATHEXT` and `current_exe()`. Also `SystemRunner`, which calls `ops::menu::process::capture_with_timeout`.                                                                                |
| `src/models/mcp_install.rs`                                | `McpInstallDialog { scope, folder, server, rows, field }`, `ClientRow { client, presence, state, mark }`, `Mark`, `McpInstallField { Scope, Client(ClientId), Register, Cancel }`, the focus order (`crate::cycle::step`, `crate::search::Direction`), `toggle`, default marks |
| `src/popup/mcp_install.rs`                                 | §4.3 keys, routed from `process_commands_dialog` in `src/popup.rs`                                                                                                                                                                                                             |
| `src/ui/mcp_install.rs`                                    | §4.1 drawing, routed from `ui/overlays.rs` (846 lines, gains 1)                                                                                                                                                                                                                |
| `src/test_support/mcp_install.rs`                          | `FakeMachine` (tempdir home, folder, PATH dir and executable dir), `ClaudeEmulator` runner (§8)                                                                                                                                                                                |

Edits in existing files:

- `menu/command.rs` and `menu/tables.rs`: the new variant and row.
- `ops/menu.rs`: the arm in `execute_tools`.
- `models/overlay.rs`: `Overlay::McpInstall(Box<McpInstallDialog>)`.
- `models/app.rs`: the `mcp_host` field.
- `popup.rs`, `ui/overlays.rs`, `dialog_mouse/button_keys.rs`: routing and
  buttons.
- `bootstrap.rs`: installs the system host.
- `ops/viewer.rs`: §6.
- `ops/menu/process.rs`:
  - `capture_with_timeout` becomes `pub(crate)`.
  - Its `let _ = child.kill()` / `let _ = child.wait()` go. Their errors are
    added to the `TimedOut` message instead of being thrown away, as the new
    lint set requires.
- `Cargo.toml`: §7.4.

The layer rules still hold:

- `models` and `ui` refer only to `crate::mcp_clients` types.
- `ui` does no I/O.
- `std::process` appears only under `ops`.
- `popup` calls `ops`, never `handlers`.
- The TUI still compiles nothing from `mcp/` except `tool_names.rs`.

`application/timing.rs` needs no arm: its `_ => "Dialog"` fallback covers
the new overlay.

### 7.3 Test seams

- **Where the host lives.** `App` gains `pub mcp_host: crate::mcp_clients::Host`.
  `App::with_panels` sets the inert `Host::default()`, so a test's `App`
  cannot reach a real home folder, PATH or program. `Host` is `Clone` with
  its runner in an `Arc`, so it survives `run_key_workflow`'s `App` clone.
  `ClipboardSlot` deliberately loses its backend on clone; `Host` does not
  copy that.
- **Who installs the real host.** Only `bootstrap`, and only for live input.
  It sets `app.mcp_host = ops::mcp_install::system_host()` next to the
  `program_pause` switch. A replayed recording keeps the inert host, so
  replaying a session never edits a config. `test-helpers` changes no runtime
  behavior. Tests assign the public field.
- **How a test builds a machine.** Every seam is a `Host` field: `home`,
  `env`, `search_path`, `program_suffixes`, `exe_dir` and `runner`. A test
  builds a `FakeMachine` from tempdirs. A detected CLI is an empty file with
  mode `0755` in the fake PATH dir. The server is such a file in the fake
  executable dir.
- **Passing the host.** Ops functions take `&Host`. `open`, `switch_scope`
  and `register` read `app.mcp_host` once and pass it down.

### 7.4 Dependencies

- **`toml_edit = { version = "0.25", default-features = false, features =
  ["parse", "display"] }`.** It pays for itself: it reads Codex status at
  both scopes, it writes Current folder where no CLI route exists, and it
  keeps comments intact. Scanning the text by hand would be fragile against
  inline tables and dotted keys.
- **`serde_json = { version = "1.0.151", features = ["preserve_order"] }`.**
  This makes the shipped binary keep key order, as tests already do through
  the dev-dependency's feature. The tests see no change in behavior.

## 8. Test plan

### 8.1 Unit tests

**`mcp_clients`** (strings in, strings out):

- *Table.* Four rows. Ids, names and programs are unique. Paths are relative
  and use `/`. Every `name: note` line fits the 72 cells an 80-column
  dialog has inside its chrome.
- *Registered comparison.* Absent `args` counts as empty. `FFM_MCP_ROOT` must
  be absent at All projects and equal at Current folder. Extra env keys are
  ignored.
- *JSON `state`/`upsert`:*
  - empty, whitespace-only and BOM files;
  - root not an object;
  - `mcpServers` a string;
  - entry an array;
  - other servers and top-level keys preserved, in order;
  - extra entry keys (`type`, `disabled`, `autoApprove`) and extra env keys
    preserved;
  - `FFM_MCP_ROOT` removed at All projects;
  - `upsert(upsert(x)) == upsert(x)`.
- *TOML, the same matrix plus:*
  - comments and blank lines survive;
  - `mcp_servers` written as an inline table;
  - dotted keys;
  - `[mcp_servers]` present but empty;
  - Windows paths with backslashes round-trip;
  - `mcp_servers = 3` gives `UnexpectedLayout`.
- *Parse errors.* The message carries the line and column.
- *`ROOT_ENV` agreement.* The test reads `mcp/src/paths.rs` as text and
  checks it still declares `"FFM_MCP_ROOT"`.

**`ops::mcp_install`** (tempdirs, `FakeMachine`, `ClaudeEmulator`):

- *`find_program`:* PATHEXT suffixes; the Unix execute bit (a non-executable
  file is not found); the first directory wins; an empty PATH.
- *`locate_server`:* the executable folder beats PATH; a missing server; a
  non-Unicode path (Unix only).
- *Presence:* each of the three signals on its own; overrides used instead of
  the home folder.
- *`survey`:* one test per §4.2 state per format. This includes
  `HomeVarNotFolder`, `NoHomeFolder`, and `NeedsClaudeCli` both with and
  without an existing registration.
- *`register`, file clients:*
  - `Added` / `Updated` / `Unchanged`, where `Unchanged` leaves the file's
    bytes and mtime as they were;
  - a symlinked config keeps its link and the target changes;
  - a new file gets its parent folders created;
  - the parent is a regular file gives `Failed(Io)` and nothing else
    changes;
  - a read-only target gives `Failed(Io)`. This case skips as root, the way
    the `file_write` tests do.
- *`register`, Claude CLI (the emulator records every `CliCall`):*
  - absent: one `add` with the exact arguments from §5.5 and cwd = home;
  - `DifferentEntry`: `remove`, then `add`;
  - `Registered`: no call;
  - unparsable `.claude.json`: no call;
  - `TimedOut`, `NotFound`, non-zero exit (stderr tail reported);
  - `remove` ok but `add` failed gives `RemovedNotAdded` with the add
    command;
  - exit 0 with no write gives `NotVerified`.
- *Isolation.* One client failing still registers the rest.
- *`report`:* lines and status for all-success, mixed, and all-failed runs.
- *`system_host`:* the returned values have the right shape (`exe_dir` is
  `Some`, `search_path` equals `env::split_paths(PATH)`). Nothing else is
  touched.
- *`SystemRunner`:* runs `sh -c 'echo out; echo err >&2; exit 3'` (Unix) and
  a sleep past a 100 ms timeout. It never runs `claude` or `codex`.

**`models::mcp_install`:**

- The focus order skips disabled rows and a disabled `Register`.
- `toggle` works only on client rows.
- Default marks follow §4.2.
- Switching scope resets the marks.

**`popup::mcp_install`:**

- Each key in §4.3.
- Enter on `Cancel`, and Enter with nothing checked.
- Esc and F10.

**`ui::mcp_install` (80×24):**

- The first drawing's text appears, including `(•) All projects`,
  `[x] Claude Code`, `{ Register }` and the two notes.
- A disabled row is drawn with the dimmed style.
- The missing-server drawing appears.
- `dialog_mouse` records the frame.

**Existing tests that change:**

- `menu/tests/tests_rows.rs`: 24 Commands rows, plus WSL on Windows.
- `tests_hotkeys.rs` still passes, because `e` is unique.
- `dialog_mouse/tests/pointer.rs`: the screen grows to 80×31 and the stale
  "22 rows" comment is fixed. With 28 entries, "Go to root" would otherwise
  fall off the bottom.
- `ops/viewer/tests.rs`:
  - The server is found in the fake executable dir.
  - The `.mcp.json` "not configured" test now points at Claude Code's
    Current folder column.
  - The header count still comes from `TOOL_NAMES`.
- `tests/coverage_gaps_test.rs` (lines 51, 54) and
  `tests/coverage_boost_test.rs` (line 83): they stop requiring Gemini and
  VS Code/Cursor, and require the four clients plus `Supported IDEs:` and
  `Commands:`.
- `tests/public_api_coverage_test.rs`: every new `pub`/`pub(crate) fn` is
  named in a test, and `UNEXERCISED` does not grow.

### 8.2 BDD: `tests/features/mcp_install.feature`

The steps live in `tests/bdd/mcp_install.rs`, declared from `tests/bdd.rs`
with `#[path]`, the way `tests/tui_bdd.rs` pulls in `tests/tui_bdd/`. The
scenarios drive the real loop through `run_key_workflow`:

- All projects: `F9 Right Right e Enter`.
- Current folder: `F9 Right Right e Right Enter`.

Then they assert on the fake machine's files and on the emulator's recorded
calls.

```gherkin
# Purpose: BDD scenarios for F9 Commands > Install MCP server.
Feature: Install the MCP server for AI clients
  As a user of AI coding clients
  I want FFM to register ffm-mcp with them
  So that they can use FFM's file tools without hand-editing their configs

  Background:
    Given a fake machine with Claude Code, Codex, Junie and Kiro detected
    And ffm-mcp next to ffm

  Scenario: Registering for all projects writes the user-level configs
    When I register the MCP server for all projects
    Then the claude CLI ran "mcp add --scope user --transport stdio ffm-mcp --" with the server
    And the user-level Codex, Junie and Kiro configs register ffm-mcp without FFM_MCP_ROOT

  Scenario: Registering for the current folder roots every entry there
    When I register the MCP server for the current folder
    Then the folder's .mcp.json, .codex/config.toml, .junie/mcp/mcp.json and .kiro/settings/mcp.json register ffm-mcp with FFM_MCP_ROOT set to the folder
    And the claude CLI was not run

  Scenario: A config that does not parse is left untouched
    Given the user-level Kiro config contains "{ broken"
    When I register the MCP server for all projects
    Then the user-level Kiro config still contains exactly "{ broken"
    And the user-level Junie config registers ffm-mcp without FFM_MCP_ROOT

  Scenario: Other servers and keys survive, and a second run changes nothing
    Given the user-level Junie config holds another server "other" and a key "theme"
    When I register the MCP server for all projects
    And I register the MCP server for all projects
    Then the user-level Junie config still holds the server "other" and the key "theme"
    And the user-level Junie config was written once

  Scenario: An entry for another binary is updated in place
    Given the user-level Codex config registers ffm-mcp at "/old/ffm-mcp" with startup_timeout_sec 20
    When I register the MCP server for all projects
    Then the user-level Codex config registers ffm-mcp at the server with startup_timeout_sec 20

  Scenario: A missing ffm-mcp is explained and nothing is written
    Given no ffm-mcp next to ffm or on PATH
    When I register the MCP server for all projects
    Then no config file was created or changed
    And the claude CLI was not run

  Scenario: A claude CLI that times out fails only Claude Code
    Given the claude CLI times out
    When I register the MCP server for all projects
    Then the user-level Codex, Junie and Kiro configs register ffm-mcp without FFM_MCP_ROOT
```

These scenarios run under `./ffm.sh test`, which passes `--features
test-helpers,s3`.

### 8.3 TUI BDD: `tests/tui_features/mcp_install.feature`

This part runs under the gate, which uses `test-helpers,keepass`. A new step,
`Given a TUI workspace with a fake MCP machine`, calls `Session::start_app`
with an `App` whose `mcp_host` is a `FakeMachine`. The scenario:

1. `F9 Right Right e` at 80×24. The screen shows `Install MCP server`,
   `(•) All projects`, `[x] Claude Code` and `{ Register }`.
2. `Right`. The screen shows `(•) Current folder` and the Codex trust note.
3. `Enter`. The screen shows `MCP server registration` and `added`.
4. `Esc`. The screen shows the panels again.

### 8.4 What tests never touch

- **No real machine.** No test reads or writes the real home folder or its
  configs, and none runs `claude` or `codex`. The inert default `Host` makes
  this true by construction, and a unit test on
  `App::with_panels(...).mcp_host` locks it in.
- **The only real process.** It is `SystemRunner`'s own test, which runs
  `sh` only.
- **No environment edits.** No test sets `HOME`, `PATH`, `CODEX_HOME` or any
  other variable. Every value comes in through `Host`.
- **The `:mcp` tests get safer.** Before this change they read the real
  `HOME` through `open_mcp_status`. With the inert host they no longer do.

### 8.5 Gate

- `./ffm.sh lint`, which runs fmt and clippy pedantic+nursery with `-D
  warnings`.
- `cargo test --features test-helpers`, with the `python` → `python3` PATH
  shim for the LSP tests.
- `./ffm.sh test`, for the `tests/features` BDD.
- Coverage of at least 98% lines through `cargo llvm-cov nextest --features
  test-helpers`, with the usual ignore regex.
- Windows cross-clippy: `--target x86_64-pc-windows-gnu --all-targets
  --features test-helpers -- -D warnings`.

The new code follows the stricter lint set that is landing on another
branch:

- no `unwrap`, `expect`, `panic`, `todo` or `unreachable`;
- no slice indexing and no string slicing;
- no `as`;
- no `print!` or `eprintln!`;
- no `let _ =` on must-use values;
- every `#[allow]`/`#[expect]` has a `reason`.

`src/test_support/mcp_install.rs` compiles into the library under
`test-helpers`, so the same rules apply to it, for example
`lock().unwrap_or_else(PoisonError::into_inner)`.

Size limits: files stay under 1000 lines, functions under 30, and nesting at
most 4.

## 9. Docs to update

- **`docs/knowledge-base/07-core-features/main-menu.md`.** Add the new row to
  the Commands extras paragraph (owners `mcp_clients` / `ops/mcp_install`). Add
  a table row: "Install MCP server | Registers ffm-mcp with Claude Code, Codex,
  Junie and Kiro, for all projects or the active panel's folder; see session
  tools."
- **`docs/knowledge-base/09-far-manager-compatibility/hotkeys-and-parity.md`.**
  The F11 row says its MCP status screen lists the four clients at both
  scopes.
  Add one line under the Commands-menu notes: `Install MCP server` is an FFM
  extra with no Far counterpart.
- **`docs/knowledge-base/06-ui-and-interaction/overlays-and-modals.md`.** Add
  **McpInstall** (F9 Commands > Install MCP server) to the overlay list.
- **`docs/knowledge-base/02-architecture/module-structure.md`.** Add rows for
  `src/mcp_clients.rs` and `src/ops/mcp_install.rs`. Update the
  `src/ops/viewer.rs` row to say the status screen uses the shared client
  table.
- **`docs/session-tools.md`.** Add a new "MCP server setup" section: the
  dialog, the keys, the two scopes, the exact file and command per client
  (§5.5), the rules for leaving files untouched, and the Kiro, Junie, Codex
  and Claude Code notes.
- **`mcp/MCP.md`.** The Configuration section points to the menu row as the
  way to register, and lists the §5.2 paths.
- **The `:mcp` screen text** (§6).

## 10. Out of scope

- **Removing a registration.** It was not asked for. The dialog never removes
  anything, which is why registered rows are shown checked and dimmed.
- **More clients.** Gemini, VS Code, Cursor and Windsurf are each one table
  row later. Claude Code's local scope (`projects[<dir>]` in `.claude.json`),
  which the old script wrote and which overrides user scope in that folder,
  is left to the docs note.
- **Merging the four existing PATH lookups** (`packages.rs`,
  `ops/archive/add.rs`, `lsp/install/launch.rs`, `complete/execs.rs`) into
  `find_program`. The Rule of Three says generalize, but they differ in
  meaning: find all, runnable checks, completion. That is a refactor of its
  own.
- **`install-mcp.sh` and its Python port.** Not touched. Its wrong Junie
  path, its local-scope `claude mcp add` and its recreation of unparsable
  files are reported to whoever owns the port.
- **Setting Codex project trust, or running registration in the
  background.**

## 11. Acceptance

- The F9 Commands row `Install MCP server` (hotkey `e`, directly after
  `Plugin commands`) opens the §4.1 dialog. The default scope is All
  projects, and the rows show the §4.2 states.
- Register writes exactly the §5.5 entries at the §5.2 paths for the checked
  rows. It never writes an unparsable file. It keeps every other key. Running
  it a second time reports `unchanged` and writes nothing.
- A missing ffm-mcp is explained in the dialog, with
  `cargo install --path mcp`, and nothing is written.
- The report viewer and status line follow §4.5.
- `:mcp` lists the four clients for both scopes from the same table. The
  Codex path is `config.toml`, and the Junie path is `.junie/mcp/mcp.json`.
- The gate in §8.5 is green, and no test touches a real config or runs a real
  client CLI.

## 12. Where the implementation departs from this plan

- **The host field.** `App::with_panels` was already at the 30-line limit, so
  the host does not sit on `App` directly. It shares a small struct with the
  other switch only live input turns on: `app.live.mcp_host` beside
  `app.live.program_pause` (`models::LiveInput`). Bootstrap sets both in the
  same `Input::Live` branch; every other `App` keeps both off.
- **The radio.** It is drawn in the dialog widget's existing style, with no
  gap between options (`(•) All projects( ) Current folder`), the same as the
  Add to archive and Copy dialogs, rather than in a style of its own.
- **`NeedsClaudeCli`.** It replaces only a state that would change the file
  (`NoConfig`, `NotConfigured`, `DifferentEntry`). A `.claude.json` that does
  not parse keeps that reason, which is the more urgent one to fix.
- **A file where a folder should be.** When a folder on the way to a config
  is a regular file, the config reads as "no config"; Register then fails
  with the write error (§5.7, "Cannot write"), and nothing changes.
- **The `ROOT_ENV` check.** It lives in `tests/module_boundaries/mcp.rs`, the
  test that already reads the MCP crate's sources as text, instead of a unit
  test under `src/`.
- **Nothing checked.** The status line uses `Status::Error`.

## 13. After review

An adversarial review of the first implementation found the defects below;
each is fixed as the sections above now describe, and each has a
regression test.

- Current folder at the home folder wrote the user-level configs of Codex,
  Junie and Kiro (§5.2, `SameAsUserConfig`).
- Replacing Claude Code's entry dropped its `env` and other keys
  (§5.5, `add-json`).
- With the server missing, any entry read as registered (§5.4,
  `Unverified`, `ProgramMissing`).
- A disabled entry, or one with a `url`, `cwd` or HTTP `type`, read as
  registered (§5.4, `Disabled`; §5.5 rule 2).
- The advice after a failed `remove` named only the `add` (§5.7).
- Any `NotFound` read as a missing `claude`; a missing home folder reached
  the CLI (§5.2, §5.7).
- JSON numbers and line ends were rewritten (§5.5 rule 3).
- A FIFO or `/dev/zero` config blocked or exhausted the survey (§5.4).
- Claude Code's user config was not always the file the CLI uses (§5.2).
- A timeout killed only the shim, not the CLI (§4.5).
- Windows put `\\?\` into `FFM_MCP_ROOT` (§5.2).
- Registering froze the screen without a word (§4.5).
- A save between the last read and the rename was lost (§5.5 rule 4).
- `&` in a path was drawn as a hotkey: paths and notes use `Row::Plain`,
  which draws text as it is.
