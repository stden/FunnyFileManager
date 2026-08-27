#set document(
  title: "FunnyFileManager Architecture",
  author: "FunnyFileManager maintainers",
)

#set page(
  paper: "a4",
  margin: (top: 17mm, bottom: 18mm, left: 18mm, right: 18mm),
  numbering: "1",
)
#set text(size: 10pt)
#set par(justify: true, leading: 0.68em)
#set heading(numbering: "1.")

#let navy = rgb("#102a43")
#let blue = rgb("#247ba0")
#let cyan = rgb("#2a9d8f")
#let pale = rgb("#edf6f9")
#let border = rgb("#c9d7e0")

#show heading.where(level: 1): it => block(
  above: 1.4em,
  below: 0.65em,
)[
  #set text(fill: navy)
  #it
  #line(length: 100%, stroke: blue)
]

#align(center)[
  #text(size: 23pt, weight: "bold", fill: navy)[FunnyFileManager]
  #v(3pt)
  #text(size: 14pt, weight: "medium", fill: blue)[Architecture]
  #v(7pt)
  #emph[A state-driven, dual-panel terminal file manager inspired by Far Manager]
]

#v(10pt)

#block(
  width: 100%,
  fill: pale,
  stroke: border,
  radius: 4pt,
  inset: 10pt,
)[
  *Scope.* This document describes the runtime architecture, state model,
  module ownership, extension boundaries, and test seams of the root TUI crate
  and the separate `ffm-mcp` server crate. The source of truth for module-level
  details is `docs/knowledge-base/02-architecture/module-structure.md`.
]

#outline(title: [Contents], depth: 2)

= Architectural Style

FunnyFileManager uses a thin executable, a library-owned event loop, and a
central `App` state aggregate. Input becomes typed state transitions or effects;
the UI then renders the resulting state. Filesystem and process effects have
canonical owners, while terminal restoration is protected by RAII guards.

The repository contains two runtime programs:

- *TUI application:* the root crate and `ffm` binary. It owns panels, modal
  overlays, command-line interaction, rendering, and terminal lifecycle.
- *MCP server:* the independent `mcp/` crate and `ffm-mcp` binary. It exposes
  root-confined file and S3 tools over stdio and never enters the TUI event loop.

The architecture favors typed state. Enums represent mutually exclusive modes,
and newtypes distinguish domain units such as file sizes, screen cells, caret
positions, scroll offsets, file names, and bookmark slots.

= Runtime Event Flow

#figure(
  block(width: 100%, fill: pale, stroke: border, radius: 4pt, inset: 9pt)[
```text
src/main.rs
    |
    v
bootstrap::run  ---- owns host terminal and loads persisted history
    |
    v
lib::run_app_with_event_reader
    |
    +--> ui::ui(&App) -------------------------------+
    |                                                |
    +--> read KeyEvent                               |
          |                                          |
          +--> Overlay active?                       |
          |      yes -> popup::process_popup_input   |
          |               -> ops/editor/state        |
          |                                          |
          +------ no -> action::resolve              |
                         -> typed Action              |
                         -> apply_action              |
                              -> ops/editor/panel     |
                                                     |
          App + Status + ConsoleLog <----------------+
                         |
                         +--> next frame
```
  ],
  caption: [Modal-first input, typed dispatch, state update, and rendering.],
)

The ordering is an invariant:

1. Render the current `App` state.
2. Read an event through the injected event source.
3. If an `Overlay` is active, send the key only to `popup`; even unsupported
   keys are consumed and never fall through.
4. Otherwise, resolve the key through the pure `action::resolve` DSL and apply
   the resulting `Action` in `lib.rs`.
5. Record outcomes in typed state and render the next frame.

Fallible effects report through `Status`; an ordinary file-operation failure
does not tear down the event loop.

= State Model

`App` is the central aggregate and owns the state needed to render and operate
the application:

#table(
  columns: (24%, 31%, 45%),
  align: (left, left, left),
  inset: 5pt,
  stroke: border,
  fill: (_, row) => if row == 0 { pale } else { none },
  [*Concern*], [*Type/owner*], [*Invariant*],
  [Panels], [`Panel`, `PanelSide`], [Two independent panels; each panel owns and clamps its selection.],
  [Modal state], [`Overlay`], [Exactly one dialog/editor/viewer/command line may be active.],
  [Background view], [`ViewMode`], [`Panels` and `Console` are orthogonal to modal state.],
  [Application exit], [`Lifecycle`], [`Running` or `Quitting` is the sole lifecycle answer.],
  [User outcome], [`Status`], [Paths, slots, errors, and other outcomes remain typed until rendering.],
  [Search], [`QuickSearch`], [An active search cannot contain an empty query.],
  [Commands], [`CommandLine`, `History`, `ConsoleLog`], [Editable input, durable recall, and session transcript are separate concerns.],
)

Representative domain types include `EntryKind`, `FileSize`, `FileName`,
`BookmarkSlot`, `Bookmarks`, `Cells`, `Caret`, `CharColumn`, `ScrollTop`,
`ScrollBack`, `RingIndex`, `ShellCommand`, `DurableLine`, `FailedEntry`, and
`FileFormat`. Their purpose is to make invalid combinations or mixed units
impossible to express.

= Module Ownership

#table(
  columns: (19%, 29%, 52%),
  align: (left, left, left),
  inset: 5pt,
  stroke: border,
  fill: (_, row) => if row == 0 { pale } else { none },
  [*Layer*], [*Primary modules*], [*Responsibility*],
  [Entry/lifecycle], [`main`, `bootstrap`, `terminal`], [Thin binary, production startup, terminal entry/suspension/restoration, and persisted-history loading.],
  [Coordinator], [`lib`], [Event loop, modal-first routing, `Action` effect dispatch, and render cadence.],
  [State], [`models`, `status`, `console`, `history`, `quicksearch`, `scroll`], [Application aggregate, domain invariants, typed outcomes, transcript, recall, and offsets.],
  [Input], [`action`, `popup`, `lineedit`, `complete`, `command`], [Pure main-view keymap, modal keys, editing/completion, and typed command parsing.],
  [Effects], [`ops`, `editor`, `utils`, `fileload`], [Canonical filesystem, shell, viewer, command, editor, and bounded-loading behavior.],
  [Presentation], [`ui`], [Ratatui layout and rendering from state; no operation ownership.],
  [Adapters], [`s3`, `keepass`, `windows`], [Feature-gated or platform-specific integration boundaries.],
  [Compatibility], [`handlers`], [Stable re-exports of `ops` and `popup`; never an implementation owner.],
  [MCP process], [`mcp/src`], [Independent stdio server with its own path guard and filesystem helpers.],
)

== Dependency Rules

#figure(
  block(width: 100%, fill: pale, stroke: border, radius: 4pt, inset: 9pt)[
```text
bootstrap --> lib --> action                 pure key mapping
                  --> popup --> ops          modal transitions and effects
                  --> ops                    main-view effects
                  --> ui                     state-only rendering
                  --> editor                 editor boundary
                  --> models                 shared state

handlers -----------------> ops + popup      compatibility facade only
mcp/ ------------------------------------>   separate process
```
  ],
  caption: [The principal dependency and ownership boundaries.],
)

- New operation implementations belong under `src/ops/`; `handlers` remains a
  facade, and `popup` calls `ops` directly.
- `action` may inspect a `KeyEvent` but performs no terminal or filesystem I/O.
- `ui` reads state and renders it; it does not decide which operation runs.
- `bootstrap` and `terminal` own host-terminal transitions. RAII guards restore
  raw mode, alternate screen, mouse capture, and cursor state after return,
  partial setup failure, external-editor suspension, or unwinding.
- The `test-helpers` feature exposes seams and visibility only. It does not
  select a different production event loop, startup path, or editor behavior.

= Command and Effect Pipeline

Printable main-view input opens `Overlay::CommandLine`; subsequent keys are
modal and stay inside the command-line subsystem.

```text
CommandLine state
    -> LineEditor + completion menu
    -> Command::parse
    -> ops::execute_command
         -> built-in file/navigation operation
         -> bounded shell execution
    -> Status + ConsoleLog + History
```

`Command::parse` is the syntax and validation authority. Completion reuses
command metadata rather than maintaining a second grammar. The console
transcript is session-only; history has an explicit persistence policy and
stores only lines accepted by its durability gate.

= Filesystem and Integration Boundaries

- *Local file operations* live in `ops::file`. Moves try atomic rename first
  and fall back to copy plus delete across filesystems. Destination creation
  uses `unique_destination` and never overwrites an existing entry.
- *Shell execution* lives in `ops::shell` and is bounded/cancellable. Platform
  shell arguments are selected behind the module boundary.
- *S3* is behind the `s3` feature. `VirtualPath` keeps local and S3 address
  spaces distinct at the type level.
- *KeePass* is behind the `keepass` feature and remains an optional backend.
- *MCP* is a separate stdio process. `mcp/src/paths.rs` confines paths to the
  configured root; `fsops` owns atomic replacement and removal behavior; the
  TUI shares only the canonical tool-name list for its MCP status view.

= Test Architecture

The production architecture contains explicit seams instead of alternate test
implementations:

- `run_app_with_event_reader` accepts an injected event source.
- Ratatui's test backend, wrapped where a writable backend is required, makes
  rendered frames deterministic.
- `TerminalOps` abstracts individual terminal transitions for lifecycle tests.
- Temporary directories isolate every filesystem test.
- In-memory fault-injection controls exercise rename fallback, recursive-copy
  failures, and panel-refresh failures without process-wide environment flags.
- Logging tests pass explicit temporary paths and never change the process-wide
  working directory.

= Extension Map

#table(
  columns: (39%, 61%),
  align: (left, left),
  inset: 5pt,
  stroke: border,
  fill: (_, row) => if row == 0 { pale } else { none },
  [*Change*], [*Primary owner*],
  [Main-view hotkey], [`src/action.rs`, then dispatch in `src/lib.rs`],
  [Modal or dialog key behavior], [`src/popup.rs` or a focused `popup/` child],
  [Command syntax or validation], [`src/command.rs`],
  [Editing, completion, or recall], [`lineedit`, `complete`, or `history`],
  [File, shell, viewer, or command effect], [The matching module under `src/ops/`],
  [Application or domain state], [`src/models.rs` or a focused `models/` child],
  [Rendering or layout], [`src/ui.rs` or a focused `ui/` child],
  [Terminal lifecycle], [`src/bootstrap.rs` and `src/terminal.rs`],
  [MCP tool or root-confined behavior], [`mcp/src/`],
)

#v(10pt)

#block(width: 100%, fill: pale, stroke: cyan, radius: 4pt, inset: 10pt)[
  *Architectural invariant:* input routing, state ownership, rendering, and
  effects remain separate. When adding behavior, choose the module that owns
  the responsibility instead of creating a second route around the boundary.
]
