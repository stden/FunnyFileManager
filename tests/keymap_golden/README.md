# The golden key tables

What every key does on every screen, one table per context, as the design
spec's golden regression oracle
(`docs/superpowers/specs/2026-09-24-ffm-yml-keymap-design.md`, §9.1)
prescribes. `tests/keymap_golden_test.rs` replays every table against the
live resolvers on every `cargo test`; a difference fails the test.

**A change here must be deliberate.** A changed row is a change to what a
key does for the user. It is made by regenerating the tables, reviewing the
diff, saying what changed in the commit message (spec §10) and, when a
context's `lenient` count moved, updating the count pinned beside its reason
in `tests/keymap_golden_test.rs`. Never edit a table by hand: the replay
checks the order and the completeness of the rows as well as their content.

## Regenerating

```sh
cargo test --test keymap_golden_test regenerate_the_golden_tables -- --ignored
git diff --stat tests/keymap_golden/
```

The generator writes every table from the domain and the resolvers; the
output is deterministic, so an unchanged resolver gives an unchanged file.

## Format

One line per key, no header, three tab-separated columns:

```
chord<TAB>class<TAB>result
```

- **chord**: the key in `KeyChord`'s canonical text — modifiers in the order
  Ctrl, Alt, Shift, then Far's key name (`Ctrl+Shift+F3`, `Alt++`, `K`,
  `Shift+Tab`, `Space`). The last rows name keys no chord can name, as
  crossterm names their codes (`F0`, `F25`, `Null`, `CapsLock`,
  `Media(Play)`, `Modifier(LeftShift)`, …) behind the same modifier prefix;
  none of those texts parses as a chord, which is how a reader tells them
  apart.
- **class**, assigned mechanically in this order:
  - `fallback`: what the context does with a key no binding names — nothing,
    a fall-through, a refusal, or a command of a family (`CommandLineChar`,
    `QuickSearch`, `Insert`, `Search`, `Type`, `Extend`, `PidJump`) or the
    `_ =>` arm's own value (`Ignore`, `QuickSearchClear`);
  - `lenient`: a named command that a strict subset of the chord's modifiers
    also yields — `Shift+F5` copies because `F5` does (spec §2.3); a subset
    is read back through the terminal's event, so `K` is a subset of
    `Ctrl+Shift+K`;
  - `bound`: any other named command.
- **result**: the command in its debug form; `nothing` when the context
  swallows the key; `-> X` when the key goes on to the parent context, which
  answers `X`; `refuses X` when a NetBox panel refuses the panels action `X`
  that would act on the entries hidden beneath it.

Rows are sorted by key — characters by code point, then F1 to F24, then the
named keys — and within a key by modifiers: none, Shift, Alt, Alt+Shift,
Ctrl, Ctrl+Shift, Ctrl+Alt, Ctrl+Alt+Shift. The keys no chord names follow,
in the order the generator lists them. Every table has the same 900 rows:
804 chords and twelve unbindable codes with eight modifier sets each.

## Tables

The fourteen contexts of spec §4.1, named as `ffm.yml` names them, and one
state of the viewer:

| Table                     | Screen                                                                                                                                                                                                              | Layered over                                            |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| `panels`                  | file panels and the Ctrl+O console                                                                                                                                                                                  | —                                                       |
| `tree_panel`              | the Ctrl+T tree                                                                                                                                                                                                     | `panels` (`-> X` is the file panel's action)            |
| `netbox_panel`            | the NetBox sessions list; the remote host reads the same keys and hands the five commands that need or make a session — Shift+F4's `NewSession` and the four row commands of F3, F4, F5 and F8 — back to be refused | `panels`, by the action's scope (`-> X` or `refuses X`) |
| `quick_view`              | the `viewer` context as the Ctrl+Q quick-view panel reads it: the scroll commands are its, everything else, the viewer's closing keys included, is the panels'                                                      | `panels`                                                |
| `viewer`                  | the F3 viewer                                                                                                                                                                                                       | — (every key is consumed)                               |
| `editor`                  | the F4 editor                                                                                                                                                                                                       | —                                                       |
| `command_line`            | the `:` command line                                                                                                                                                                                                | —                                                       |
| `command_line_search`     | its Ctrl+R history search                                                                                                                                                                                           | —                                                       |
| `command_line_completion` | its open completion list                                                                                                                                                                                            | `command_line`                                          |
| `task_list`               | the Ctrl+W task list                                                                                                                                                                                                | —                                                       |
| `task_list_search`        | the task list's `/` bar: F3 is its own key, Up, Down, PgUp and PgDn close the bar and go on to the list, and every other key is the bar's fixed editing, recorded as `nothing`                                      | `task_list`                                             |
| `wsl_manager`             | the Ctrl+Alt+W WSL manager                                                                                                                                                                                          | —                                                       |
| `language_servers`        | Options → Language servers, list and view                                                                                                                                                                           | —                                                       |
| `path_editor`             | Commands → Edit PATH                                                                                                                                                                                                | —                                                       |
| `vault`                   | the KeePass vault                                                                                                                                                                                                   | —                                                       |

Until a context switches to the keymap (step 5 of the spec's migration
plan) its table describes the old resolver, and the replay proves the
generator against it. Step 5 points the replay at the keymap's defaults:
every `bound` row must resolve to the same command, every `lenient` and
`fallback` row to the fallback, and the tree's refusal by scope (spec §6.3)
is the one named change.

While contexts switch one by one, `tests/keymap_golden_test/defaults.rs`
holds the whole default keymap to the tables: every `bound` row is a default
key of its context with the same command (the tree's four `NeedsFilePanel`
rows excepted), and every default key stands in a `bound` or a `lenient` row
with its command. A context's switch can therefore trust the defaults it
reads.
