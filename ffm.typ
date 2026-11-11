// Purpose: Present the TUI and MCP architecture with editable vector diagrams.
#import "@preview/fletcher:0.5.8" as fletcher: diagram, node, edge

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
#let muted = rgb("#52606d")
#let amber = rgb("#d97706")
#let rose = rgb("#e11d48")
#let violet = rgb("#7c3aed")
#let zebra = rgb("#f6f9fc")

// One colour and one emoji per architectural layer, used by the diagrams, the
// module table and the legend alike, so a layer reads the same everywhere.
#let layer(name, icon, ink, wash) = (name: name, icon: icon, ink: rgb(ink), wash: rgb(wash))
#let layers = (
  entry: layer("Entry and lifecycle", "🚀", "#7c3aed", "#f3e8ff"),
  coordinator: layer("Coordinator", "🔁", "#1d4ed8", "#dbeafe"),
  state: layer("State", "🧠", "#0f766e", "#ccfbf1"),
  input: layer("Input", "⌨️", "#c2410c", "#ffedd5"),
  effects: layer("Effects", "⚡", "#b91c1c", "#fee2e2"),
  presentation: layer("Presentation", "🎨", "#be185d", "#fce7f3"),
  adapters: layer("Adapters", "🔌", "#b45309", "#fef3c7"),
  compatibility: layer("Compatibility", "🧱", "#475569", "#f1f5f9"),
  mcp: layer("MCP process", "🤖", "#4338ca", "#e0e7ff"),
)

// Diagram vocabulary: a step (coloured by its layer when it has one), a
// question, the state a step leaves behind, and a small grey note under a
// module name.
#let step(pos, body, kind: none, ..args) = {
  let (ink, wash) = if kind == none { (border, white) } else { (layers.at(kind).ink, layers.at(kind).wash) }
  node(pos, body, shape: fletcher.shapes.rect, fill: wash, stroke: 0.9pt + ink, corner-radius: 3pt, inset: 6pt, ..args)
}
#let question(pos, body, ..args) = node(pos, body, shape: fletcher.shapes.diamond, fill: rgb("#fff7ed"), stroke: 0.9pt + amber, inset: 3pt, ..args)
#let outcome(pos, body, ..args) = node(pos, body, shape: fletcher.shapes.rect, fill: layers.state.wash, stroke: 1.2pt + layers.state.ink, corner-radius: 3pt, inset: 6pt, ..args)
#let note(body) = text(size: 7.5pt, fill: muted, body)
#let chart(..args) = align(center, block(
  width: 100%,
  fill: gradient.linear(pale.lighten(55%), rgb("#f5f3ff").lighten(30%), angle: 30deg),
  stroke: border,
  radius: 4pt,
  inset: 10pt,
  text(size: 8.5pt, diagram(spacing: (12mm, 7mm), edge-stroke: 0.8pt + navy, mark-scale: 80%, ..args)),
))

// A row of coloured chips naming the layers the diagrams paint.
#let legend(..kinds) = align(center, box(inset: (top: 2pt), kinds.pos().map(kind => {
  let it = layers.at(kind)
  box(fill: it.wash, stroke: 0.6pt + it.ink, radius: 6pt, inset: (x: 5pt, y: 2.5pt),
    text(size: 7.5pt, fill: it.ink, weight: "medium")[#it.icon #it.name])
}).join(h(4pt))))

// A tinted callout with an emoji, for the scope note and the invariant.
#let callout(icon, ink, wash, body) = block(
  width: 100%,
  fill: wash,
  stroke: (left: 3pt + ink, rest: 0.6pt + ink.lighten(55%)),
  radius: 4pt,
  inset: 10pt,
)[#text(size: 13pt)[#icon] #h(3pt) #body]

// Tables share a navy header with white text and zebra rows.
#let table-fill = (_, row) => if row == 0 { navy } else if calc.even(row) { zebra } else { none }
#show table.cell.where(y: 0): set text(fill: white, weight: "bold")

#show heading.where(level: 1): it => block(
  above: 1.4em,
  below: 0.65em,
  sticky: true,
)[
  #set text(fill: navy)
  #it
  #line(length: 100%, stroke: 1.6pt + gradient.linear(blue, cyan, violet, rose))
]

#align(center)[
  #text(size: 26pt)[📂] #h(4pt) #text(size: 23pt, weight: "bold", fill: gradient.linear(navy, blue, violet))[FunnyFileManager]
  #v(3pt)
  #text(size: 14pt, weight: "medium", fill: blue)[🏗️ Architecture]
  #v(7pt)
  #emph[A state-driven, dual-panel terminal file manager inspired by Far Manager]
  #v(5pt)
  #legend(..layers.keys())
]

#v(10pt)

#callout("💡", blue, pale)[
  *Scope.* This document describes the runtime architecture, state model,
  module ownership, extension boundaries, and test seams of the root TUI crate
  and the separate `ffm-mcp` server crate. The source of truth for module-level
  details is `docs/knowledge-base/02-architecture/module-structure.md`.
]

#outline(title: [Contents], depth: 2)

= 🏛️ Architectural Style

FunnyFileManager uses a thin executable, a library-owned event loop, and a
central `App` state aggregate. Input becomes typed state transitions or effects;
the UI then renders the resulting state. Filesystem and process effects have
canonical owners, while terminal restoration is protected by RAII guards.

The repository contains two runtime programs:

- 🖥️ *TUI application:* the root crate and `ffm` binary. It owns panels, modal
  overlays, command-line interaction, rendering, and terminal lifecycle.
- 🤖 *MCP server:* the independent `mcp/` crate and `ffm-mcp` binary. It exposes
  root-confined file and S3 tools over stdio and never enters the TUI event loop.

The architecture favors typed state. Enums represent mutually exclusive modes,
and newtypes distinguish domain units such as file sizes, screen cells, caret
positions, scroll offsets, file names, and bookmark slots.

== 🌐 Process Boundaries

#figure(
  chart(
    spacing: (18mm, 10mm),
    step((0, 0), [🖥️ Terminal user], kind: "input", width: 55mm, name: <user>),
    step((1, 0), [🤖 AI client], kind: "mcp", width: 55mm, name: <client>),
    step((0, 1), [🔁 `ffm` \ #note[root crate · interactive TUI]], kind: "coordinator", width: 55mm, name: <tui>),
    step((1, 1), [🤖 `ffm-mcp` \ #note[independent `mcp/` crate]], kind: "mcp", width: 55mm, name: <server>),
    step((0, 2), [⚡ `ops` + host adapters \ #note[file, shell, editor, clipboard effects]], kind: "effects", width: 55mm, name: <effects>),
    step((1, 2), [🛡️ `paths` + `fsops` \ #note[validate paths before file effects]], kind: "mcp", width: 55mm, name: <guard>),
    step((0, 3), [🔌 Host resources \ #note[local files and OS integrations]], kind: "adapters", width: 55mm, name: <host>),
    step((1, 3), [📁 Configured filesystem root \ #note[confined scans, reads, and writes]], kind: "adapters", width: 55mm, name: <root>),
    edge(<user>, <tui>, "-|>", [#note[keys and mouse]]),
    edge(<client>, <server>, "-|>", [#note[MCP over stdio]]),
    edge(<tui>, <effects>, "-|>"),
    edge(<server>, <guard>, "-|>"),
    edge(<effects>, <host>, "-|>"),
    edge(<guard>, <root>, "-|>"),
  ),
  caption: [Two independently started programs, each with its own effect boundary.],
)

#legend("coordinator", "effects", "adapters", "mcp")

The TUI and MCP server have separate entry points, event handling, and
lockfiles. Starting `ffm` starts only the interactive application. The TUI's
MCP status viewer includes the import-free `mcp/src/tool_names.rs` list;
that shared build-time metadata leaves both runtime lifecycles independent.

= 🔄 Runtime Event Flow

#figure(
  chart(
    step((1, 0), [🚀 `src/main.rs`], kind: "entry"),
    edge("-|>"),
    step((1, 1), kind: "entry", [🧰 `bootstrap::run` \ #note[owns the host terminal, loads history]]),
    edge("-|>"),
    step((1, 2), [🔁 `application::run_loop` \ #note[`LoopState` pumps jobs and controls redraw]], kind: "coordinator"),
    edge("-|>"),
    step((1, 3), [🎨 `ui::ui` \ #note[renders `App` when a frame is needed]], kind: "presentation", name: <render>),
    edge("-|>"),
    step((1, 4), [⌨️ read the next event \ #note[injected `read_event`]], kind: "input"),
    edge("-|>"),
    question((1, 5), [`Overlay` \ active?], name: <overlay>),
    edge(<overlay>, <popup>, "-|>", text(fill: layers.state.ink)[✅ yes], label-side: right),
    edge(<overlay>, <panel>, "-|>", text(fill: rose)[❌ no], label-side: left),
    step((0, 6), [🪟 Modal keys \ #note[`popup::process_popup_input` \ every key stays in the dialog]], kind: "input", name: <popup>),
    edge(<popup>, <overlay>, "-|>", bend: 55deg, [#note[menu row: \ re-dispatch its key]], label-side: left),
    question((2, 6), [tree or \ quick-view \ panel?], name: <panel>),
    edge(<panel>, <resolve>, "-|>", [not taken], label-side: left),
    step((2, 7), [🧭 `action::resolve` \ #note[pure key #sym.arrow typed `Action`]], kind: "input", name: <resolve>),
    edge("-|>"),
    step((2, 8), [⚡ `apply_action` \ #note[`application::dispatch` \ effects in `ops`]], kind: "effects", name: <apply>),
    outcome((1, 9), [🧠 `App` · `Status` · `ConsoleLog` · `History`], name: <state>),
    edge(<popup>, <state>, "-|>"),
    edge(<panel>, <state>, "-|>", bend: -30deg, [taken], label-side: left),
    edge(<apply>, <state>, "-|>"),
    edge(<state>, (-0.6, 9), (-0.6, 3), <render>, "-|>", [🎞️ next frame], stroke: 0.9pt + violet, label-pos: 0.12, label-side: left),
  ),
  caption: [Modal-first input, typed dispatch, state update, and rendering.],
)

#legend("entry", "coordinator", "input", "effects", "state", "presentation")

The ordering is an invariant:

1. Pump background work and render `App` when a frame is needed.
2. Read an event through the injected event source.
3. If an `Overlay` is active, send the key only to `popup`; even unsupported
   keys are consumed and never fall through.
4. Otherwise a tree or quick-view panel may take the key; if it does not, the
   pure `action::resolve` maps it to an `Action`, which
   `application::dispatch` applies.
5. Record outcomes in typed state and request the next frame.

An idle poll continues pumping workers. A completion, real input, or a clock
minute change requests a redraw; unchanged polls reuse the last frame.

Fallible effects report through `Status`; an ordinary file-operation failure
does not tear down the event loop.

= 🧠 State Model

#block(sticky: true)[
  `App` is the central aggregate and owns the state needed to render and operate
  the application:
]

#figure(
  chart(
    spacing: (8mm, 14mm),
    outcome((1, 0), [🧠 `App` \ #note[central owned aggregate]], width: 44mm, name: <app>),
    step((0, 1), [🗂️ Panels \ `left_panel: Panel` \ `right_panel: Panel` \ `active_panel: PanelSide` \ #note[selection belongs to each panel]], kind: "state", width: 44mm, name: <panels>),
    step((1, 1), [🪟 `Overlay` \ `None` or one variant \ #note[`Editor`, `Viewer`, `CommandLine`, … \ each carries its own payload]], kind: "state", width: 44mm, name: <overlay>),
    step((2, 1), [🖥️ Independent modes \ `view: ViewMode` \ `lifecycle: Lifecycle` \ #note[Panels / Console \ Running / Quitting]], kind: "state", width: 44mm, name: <modes>),
    edge(<app>, <panels>, "-|>"),
    edge(<app>, <overlay>, "-|>"),
    edge(<app>, <modes>, "-|>"),
  ),
  caption: [Ownership of panels and orthogonal enum state; arrows mean owns.],
)

An editor is one `Overlay` variant with its editor payload. `ViewMode` and
`Lifecycle` remain independent, so opening a dialog does not require a second
background-view or exit flag.

#table(
  columns: (24%, 31%, 45%),
  align: (left, left, left),
  inset: 5pt,
  stroke: border,
  fill: table-fill,
  table.header([*Concern*], [*Type/owner*], [*Invariant*]),
  [🗂️ Panels], [`Panel`, `PanelSide`], [Two independent panels; each panel owns and clamps its selection.],
  [🪟 Modal state], [`Overlay`], [At most one dialog/editor/viewer/command line is active.],
  [🖥️ Background view], [`ViewMode`], [`Panels` and `Console` are orthogonal to modal state.],
  [🚪 Application exit], [`Lifecycle`], [`Running` or `Quitting` is the sole lifecycle answer.],
  [💬 User outcome], [`Status`], [Paths, slots, errors, and other outcomes remain typed until rendering.],
  [🔍 Search], [`QuickSearch`], [An active search cannot contain an empty query.],
  [⌨️ Commands], [`CommandLine`, `History`, `ConsoleLog`], [Editable input, durable recall, and session transcript are separate concerns.],
)

Representative domain types include `EntryKind`, `FileSize`, `FileName`,
`BookmarkSlot`, `Bookmarks`, `Cells`, `Caret`, `CharColumn`, `ScrollTop`,
`ScrollBack`, `RingIndex`, `ShellCommand`, `DurableLine`, `FailedEntry`, and
`FileFormat`. Their purpose is to make invalid combinations or mixed units
impossible to express.

= 🧩 Module Ownership

#table(
  columns: (19%, 29%, 52%),
  align: (left, left, left),
  inset: 5pt,
  stroke: border,
  fill: table-fill,
  table.header([*Layer*], [*Primary modules*], [*Responsibility*]),
  table.cell(fill: layers.entry.wash)[#text(fill: layers.entry.ink, weight: "bold")[#layers.entry.icon Entry/lifecycle]], [`main`, `bootstrap`, `terminal`], [Thin binary, production startup, terminal entry/suspension/restoration, and persisted-history loading.],
  table.cell(fill: layers.coordinator.wash)[#text(fill: layers.coordinator.ink, weight: "bold")[#layers.coordinator.icon Coordinator]], [`application`], [Event loop, modal-first routing, `Action` effect dispatch, and render cadence.],
  table.cell(fill: layers.state.wash)[#text(fill: layers.state.ink, weight: "bold")[#layers.state.icon State]], [`models`, `status`, `console`, `history`, `quicksearch`, `scroll`], [Application aggregate, domain invariants, typed outcomes, transcript, recall, and offsets.],
  table.cell(fill: layers.input.wash)[#text(fill: layers.input.ink, weight: "bold")[#layers.input.icon Input]], [`action`, `popup`, `lineedit`, `complete`, `command`], [Pure main-view keymap, modal keys, editing/completion, and typed command parsing.],
  table.cell(fill: layers.effects.wash)[#text(fill: layers.effects.ink, weight: "bold")[#layers.effects.icon Effects]], [`ops`, `editor`, `utils`, `fileload`], [Canonical filesystem, shell, viewer, command, editor, and bounded-loading behavior.],
  table.cell(fill: layers.presentation.wash)[#text(fill: layers.presentation.ink, weight: "bold")[#layers.presentation.icon Presentation]], [`ui`], [Ratatui layout and rendering from state; no operation ownership.],
  table.cell(fill: layers.adapters.wash)[#text(fill: layers.adapters.ink, weight: "bold")[#layers.adapters.icon Adapters]], [`platform`, `s3`, `keepass`, `wsl`, `lsp`], [OS boundaries, optional storage integrations, WSL commands, and language-server transport.],
  table.cell(fill: layers.compatibility.wash)[#text(fill: layers.compatibility.ink, weight: "bold")[#layers.compatibility.icon Compatibility]], [`handlers`], [Stable re-exports of `ops` and `popup`; never an implementation owner.],
  table.cell(fill: layers.mcp.wash)[#text(fill: layers.mcp.ink, weight: "bold")[#layers.mcp.icon MCP process]], [`mcp/src`], [Independent stdio server with its own path guard and filesystem helpers.],
)

== 🔗 Dependency Rules

#figure(
  chart(
    spacing: (6mm, 11mm),
    step((2.5, 0), [🚀 `bootstrap`], kind: "entry"),
    edge("-|>"),
    step((2.5, 1), [🔁 `application` \ #note[event loop and dispatch]], kind: "coordinator", name: <app>),
    step((0, 2), [⌨️ `action` \ #note[pure key mapping]], kind: "input", name: <action>),
    step((1, 2), [🪟 `popup` \ #note[modal transitions]], kind: "input", name: <popup>),
    step((2, 2), [⚡ `ops` \ #note[effects]], kind: "effects", name: <ops>),
    step((3, 2), [🎨 `ui` \ #note[state-only rendering]], kind: "presentation", name: <ui>),
    step((4, 2), [📝 `editor` \ #note[editor boundary]], kind: "effects", name: <editor>),
    step((5, 2), [🧠 `models` \ #note[shared state]], kind: "state", name: <models>),
    ..(<action>, <popup>, <ops>, <ui>, <editor>, <models>).map(target => edge(<app>, target, "-|>")),
    edge(<popup>, <ops>, "-|>"),
    step((1.5, 3), [🧱 `handlers` \ #note[compatibility facade only]], kind: "compatibility", stroke: (paint: muted, dash: "dashed"), name: <handlers>),
    edge(<handlers>, <popup>, "--|>", stroke: muted),
    edge(<handlers>, <ops>, "--|>", stroke: muted),
    step((4.5, 3), [🤖 `mcp/` \ #note[separate process, no TUI dependency]], kind: "mcp", stroke: (paint: layers.mcp.ink, dash: "dashed")),
  ),
  caption: [The principal dependency and ownership boundaries.],
)

#legend("entry", "coordinator", "input", "effects", "presentation", "state", "compatibility", "mcp")

- New operation implementations belong under `src/ops/`; `handlers` remains a
  facade, and `popup` calls `ops` directly.
- `action` may inspect a `KeyEvent` but performs no terminal or filesystem I/O.
- `ui` reads state and renders it; it does not decide which operation runs.
- `bootstrap` and `terminal` own host-terminal transitions. RAII guards restore
  raw mode, alternate screen, mouse capture, and cursor state after return,
  partial setup failure, external-editor suspension, or unwinding.
- The `test-helpers` feature exposes seams and visibility only. It does not
  select a different production event loop, startup path, or editor behavior.

= ⌨️ Command and Effect Pipeline

#block(sticky: true)[
  Printable main-view input opens `Overlay::CommandLine`; subsequent keys are
  modal and stay inside the command-line subsystem.
]

#figure(
  chart(
    spacing: (7mm, 5mm),
    step((0, 1), [⌨️ `Overlay::` \ `CommandLine`], kind: "input"),
    edge("-|>"),
    step((1, 1), [✏️ `LineEditor` \ #note[with completion menu]], kind: "input"),
    edge("-|>"),
    step((2, 1), [⚡ `ops::execute_command` \ #note[parses with `Command::parse`]], kind: "effects", name: <execute>),
    step((3, 0), [📁 built-in file or \ navigation operation], kind: "effects", name: <builtin>),
    step((3, 2), [🐚 bounded shell \ #note[`ops::shell`]], kind: "effects", name: <shell>),
    edge(<execute>, <builtin>, "-|>"),
    edge(<execute>, <shell>, "-|>"),
    outcome((4, 1), [🧠 `Status` \ `ConsoleLog` \ `History`], name: <outcome>),
    edge(<builtin>, <outcome>, "-|>"),
    edge(<shell>, <outcome>, "-|>"),
  ),
  caption: [From the typed line to its recorded outcome.],
)

`Command::parse` is the syntax and validation authority. Completion reuses
command metadata rather than maintaining a second grammar. The console
transcript is session-only; history has an explicit persistence policy and
stores only lines accepted by its durability gate.

= ⚙️ Background Jobs and Redraw

#block(sticky: true)[
  `application::LoopState` owns the worker controllers. `App` holds domain state
  and typed requests; workers exchange values through channels, and the UI
  thread applies completed results before drawing.
]

#figure(
  chart(
    spacing: (8mm, 13mm),
    step((0, 0), [🧠 `App` requests \ #note[WSL intent, sample schedule, \ or editor snapshot]], kind: "state", width: 43mm, name: <requests>),
    step((1, 0), [🔁 UI-thread controllers \ `WslJobs` \ `TaskJobs` \ `EditorLspController`], kind: "coordinator", width: 48mm, name: <controllers>),
    step((2, 0), [🔌 Background workers \ #note[WSL backend, process reader, \ or LSP transport]], kind: "adapters", width: 43mm, name: <workers>),
    outcome((0, 1), [🧠 Updated `App` \ #note[status, table, or editor results]], width: 43mm, name: <updated>),
    step((1, 1), [⚡ Apply a current result \ #note[request, document, or session \ identity checks]], kind: "effects", width: 48mm, name: <apply>),
    step((2, 1), [📨 Typed completion \ #note[outcome or error]], kind: "state", width: 43mm, name: <reply>),
    step((0, 2), [🎨 Next needed frame \ `ui::ui` \ #note[`Redraw::Needed`]], kind: "presentation", width: 43mm, name: <draw>),
    edge(<requests>, <controllers>, "-|>"),
    edge(<controllers>, <workers>, "-|>"),
    edge(<workers>, <reply>, "-|>"),
    edge(<reply>, <apply>, "-|>"),
    edge(<apply>, <updated>, "-|>"),
    edge(<updated>, <draw>, "-|>"),
  ),
  caption: [Requests cross to a worker; completions return to the UI thread and state.],
)

#legend("state", "coordinator", "adapters", "effects", "presentation")

- *WSL:* one serial worker executes queued requests. Request IDs keep a late
  result from reopening a closed dialog or navigating after close.
- *Task list:* one sample is in flight. A late sample marks the table stale
  at 5 seconds; a stuck reader is abandoned at 10 seconds without waiting for
  its thread to finish.
- *Editor LSP:* snapshots are debounced for 120 milliseconds. Document IDs and
  version stamps keep stale replies out of the current editor.

Local directory scans and file transfers execute synchronously in `ops`.
The progress painter can show a long transfer while that operation runs.

= 🗂️ Filesystem and Integration Boundaries

- 📁 *Local file operations* live in `ops::file`. Moves try atomic rename first
  and fall back to copy plus delete across filesystems. A taken name is
  handled by the F5/F6 dialog's choice; "Overwrite" merges a folder into a
  folder and never lets a file and a folder replace one another
  (`ops::file::merge`).
- 🐚 *Shell execution* lives in `ops::shell` and is bounded/cancellable. Platform
  shell arguments are selected behind the module boundary.
- ☁️ *S3* is behind the `s3` feature. `VirtualPath` keeps local and S3 address
  spaces distinct at the type level.
- 🔐 *KeePass* is behind the `keepass` feature and remains an optional backend.
- 🤖 *MCP* is a separate stdio process. `mcp/src/paths.rs` confines paths to the
  configured root; `fsops` owns atomic replacement and removal behavior; the
  TUI shares only the canonical tool-name list for its MCP status view.

= 🧪 Test Architecture

#block(sticky: true)[
  The production architecture contains explicit seams instead of alternate test
  implementations:
]

#figure(
  chart(
    spacing: (9mm, 12mm),
    step((1, 0), [⌨️ Scripted events \ #note[keys, mouse, and timer ticks]], kind: "input", width: 43mm, name: <events>),
    step((0, 1), [🔌 Isolated adapters \ #note[temporary files, fake backends, \ in-memory fault injection]], kind: "adapters", width: 43mm, name: <fixtures>),
    step((1, 1), [🔁 Production path \ `application` + `ops`], kind: "coordinator", width: 43mm, name: <runtime>),
    step((2, 1), [🎨 Captured frames \ `TestBackend` \ #note[text and style assertions]], kind: "presentation", width: 43mm, name: <frames>),
    outcome((1, 2), [🧠 Resulting `App` \ #note[selection, overlays, outcomes, \ and lifecycle assertions]], width: 43mm, name: <state>),
    edge(<events>, <runtime>, "-|>"),
    edge(<fixtures>, <runtime>, "-|>"),
    edge(<runtime>, <frames>, "-|>"),
    edge(<runtime>, <state>, "-|>"),
  ),
  caption: [Tests inject inputs and adapters, then observe real frames and state transitions.],
)

- 🎛️ `run_app_with_event_reader` accepts an injected event source.
- 🖼️ Ratatui's test backend, wrapped where a writable backend is required, makes
  rendered frames deterministic.
- 🖥️ `TerminalOps` abstracts individual terminal transitions for lifecycle tests.
- 📦 Temporary directories isolate every filesystem test.
- 💥 In-memory fault-injection controls exercise rename fallback, recursive-copy
  failures, and panel-refresh failures without process-wide environment flags.
- 📝 Logging tests pass explicit temporary paths and never change the process-wide
  working directory.

= 🗺️ Extension Map

#block(breakable: false)[
#table(
  columns: (39%, 61%),
  align: (left, left),
  inset: 5pt,
  stroke: border,
  fill: table-fill,
  table.header([*Change*], [*Primary owner*]),
  [⌨️ Main-view hotkey], [`src/action.rs`, then `src/application/dispatch.rs`],
  [🪟 Modal or dialog key behavior], [`src/popup.rs` or a focused `popup/` child],
  [📜 Command syntax or validation], [`src/command.rs`],
  [✏️ Editing, completion, or recall], [`lineedit`, `complete`, or `history`],
  [⚡ File, shell, viewer, or command effect], [The matching module under `src/ops/`],
  [🧠 Application or domain state], [`src/models.rs` or a focused `models/` child],
  [🎨 Rendering or layout], [`src/ui.rs` or a focused `ui/` child],
  [🚀 Terminal lifecycle], [`src/bootstrap.rs` and `src/terminal.rs`],
  [🤖 MCP tool or root-confined behavior], [`mcp/src/`],
)
]

#v(10pt)

#callout("🧭", amber, rgb("#fffbeb"))[
  *Architectural invariant:* input routing, state ownership, rendering, and
  effects remain separate. When adding behavior, choose the module that owns
  the responsibility instead of creating a second route around the boundary.
]
