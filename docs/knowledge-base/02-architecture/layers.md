<!-- Purpose: Defines the layers of the crate, the one dependency rule, the upward uses still allowed, and the architecture test that enforces them. -->

# Layers and dependency rule

FunnyFileManager is one Rust crate made of modules. Every top-level module
belongs to exactly one of seven layers, and one rule decides what may use what:

> A module may use modules in its own layer or in a layer below it. It never
> uses a module in a layer above it.

The architecture test `module_layers` (`tests/module_boundaries/module_layers.rs`)
checks the rule. It runs with `cargo test --test module_boundaries_test`, which
the quality gate runs. The module-to-layer table, the upward uses still allowed
and the modules on a loop live in `tests/module_boundaries/module_layers/map.rs`.

The test reads the source through the syn-based resolver that the other
architecture contracts use. Comments, string literals, use groups, renames and
crate-root aliases are resolved before the layers are compared. Module-level
items marked `#[cfg(test)]` and test files are skipped. Associated functions
marked `#[cfg(test)]` inside `impl` blocks are still read.

## The layers, bottom to top

1. **foundation**: values and pure helpers, with no files, processes, terminals,
   clock or application state. Modules: `casefold`, `console`, `cycle`,
   `encoding`, `lineedit`, `mask`, `path_display`, `scroll`, `space_verdict`.
2. **adapters**: everything that touches the outside world: files, processes,
   terminals, the network, the operating system, the clock and configuration on
   disk. Modules: `associations`, `clipboard`, `clock`, `config`, `description`,
   `diagnostics`, `directory`, `diskfree`, `fileload`, `file_write`, `git`,
   `highlight`, `history`, `i18n`, `images`, `keepass`, `lsp`, `mcp_clients`,
   `packages`, `panel_session`, `performance`, `platform`, `process_table`,
   `progress`, `recording`, `remote`, `s3`, `sessions`, `stats`, `terminal`,
   `things`, `token_usage`, `usermenu`, `utils`, `windows`, `wsl`.
3. **state**: the application's state and the rules that change it. Modules:
   `command`, `complete`, `copy`, `kdbx`, `models`, `path_env`, `quicksearch`,
   `search`, `status`.
4. **presentation**: drawing the state. Modules: `image_render`, `mouse`,
   `prompt`, `ui`.
5. **operations**: carrying out a request on the state. Modules: `editor`, `ops`.
6. **interaction**: turning keys and menu choices into requests. Modules:
   `action`, `keymap`, `menu`, `popup`.
7. **composition**: startup, the event loop and the compatibility facade.
   Modules: `application`, `automation`, `bootstrap`, `handlers`. The crate
   root declares and re-exports the modules; `main.rs` also holds the entry
   point. Neither is in a layer.

`test_support` is test-only and is not checked.

A request flows down the layers: a key press is resolved in `interaction`,
`operations` carries it out through the `state` rules and the `adapters`, and
`presentation` draws the result.

## What the test enforces

- Every module is in exactly one layer. A module listed twice, a module in no
  layer, and a layer entry that names no module in `src/` all fail.
- No module uses a higher layer, except the pairs listed in `UPWARD_DEBT`.
- Every listed pair must still be an upward use. A pair that is gone fails the
  test until it is removed.
- The modules on a loop with modules of their own layer must be exactly
  `CYCLIC_MODULES`. Breaking a loop means removing its names from the list; a
  new loop fails the test until the list is changed on purpose.

Adding a pair to `UPWARD_DEBT` is a visible edit that the test accepts, so it is
review's job to refuse it unless the move is impossible. "Changing the layout"
says what to do instead.

## Where the code stands

The resolver finds 1288 upward uses in 37 module pairs, from 28 modules. Most of
them point at `models`:

- `models`: 978 uses in 17 pairs (76%);
- `menu`: 153 uses in 4 pairs; `action`: 87 uses in 3 pairs;
- `status`: 17 uses in 3 pairs; `command`: 24 uses in 1 pair; `prompt`: 13 uses
  in 2 pairs;
- `complete`, `copy`, `kdbx`, `keymap`, `mouse`, `application` and `ui` are each
  the target of at most 5 uses.

Eighteen modules sit on a loop with modules of their own layer:

- adapters: `config`, `fileload`, `history`, `recording`, `terminal`, `utils`;
- state: `command`, `complete`, `models`, `search`, `status`;
- operations: `editor` and `ops`, which call each other (`ops` calls
  `editor_for`, and `editor` calls `ops::vault`);
- interaction: `action`, `menu`;
- composition: `application`, `automation`, `bootstrap`.

The upward uses come from four causes, listed by how much they account for.

1. **Values, ports and application state live in `models`.** Adapters use plain
   values such as `FileSize`, `DirEntry`, `RemotePath`, `Session`, `Protocol`,
   `Auth`, `Attributes`, `EntryKind`, `Pid`, `Signal`, `Niceness` and
   `WslOutput`, all defined in `models`. The adapter `process_table` implements
   the ports `ProcessReader` and `NetProber`, which `models` defines so that the
   process table fills them in, as `layers/models.rs` states. `windows/report.rs`
   takes `&mut App` and opens a viewer on it. Fix: move the values and the ports
   to foundation, and move the code that acts on `App` to `ops` or presentation.
   Keep `models` for application state.
2. **The input vocabulary is used below `interaction`.** `models` uses `menu` (67
   uses) and `keymap`; `ops` uses `action` (58) and `menu` (58); `ui` uses
   `action` (25) and `menu` (26); `diagnostics` uses `action`. Fix: move the
   typed `Action` values and the menu entries down to `state`.
3. **Messages and small command types are used below their layer.** `status` is
   used by `diagnostics`, `mcp_clients` and `windows`; `command` by `history`;
   `complete` by `lsp`; `copy` by `progress`. Fix: move the message and value
   types down.
4. **Single references.** `models` uses `application` (`CLIPBOARD_CLEAR`, through
   the crate root's alias, which `layers.rs` already lists as a known inversion),
   and it uses `mouse`, `prompt` and `keymap`. `keepass` calls
   `kdbx::secret::zeroize_string`, a helper that only clears a string and belongs
   in foundation. `terminal` calls into `ui`; `packages` uses `prompt`; `config`
   uses the `PanelView` value and two legacy-format helpers from `models`. Fix
   each where it lives.

The moves already made: `clock` and `progress` to adapters (they read the clock
or the file system); `lineedit` to foundation (it is a pure line buffer);
`path_env` to state; `image_render` to presentation; `editor` to operations (its
work on `App` and `ops` is operation work).

The moves not made, and why:

- `complete` stays in state. Part of its code is adapter work (process and file
  access in `fish.rs`, `execs.rs` and `paths.rs`), and `matcher` is pure. It
  needs a split before the pure part can move down.
- `mcp_clients` stays in adapters. Its `server_lines` function uses
  `status::PathMessage`; once that function moves to presentation, the table is
  pure enough for foundation.
- `i18n` stays in adapters. It uses `config::Lang`, so `Lang` has to move to
  foundation first.
- `performance` is the one module whose layer the review left open.

## Changing the layout

- A new module goes into a layer in `map.rs`. The test fails until it does.
- When a debt pair is fixed, remove the upward use and its `UPWARD_DEBT` entry in
  the same change. The test fails if the entry stays after the use is gone.
- When a loop is broken, remove its module from `CYCLIC_MODULES`. The test fails
  if the list does not match the real loops.
- Do not add a `UPWARD_DEBT` entry to make a new upward use pass. Move the code
  instead. The test accepts such an entry, so this is a rule for review, not
  something the test enforces.

The Rust tests under `tests/module_boundaries/layers` keep finer rules for `ui`,
`models`, `action` and `keymap`. This test is coarser and covers every module.
