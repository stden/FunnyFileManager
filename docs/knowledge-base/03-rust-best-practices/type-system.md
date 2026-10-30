<!-- Purpose: Documents the enum and newtype patterns used to model state safely. -->

# 🧩 Rust Type System Best Practices

This project places extremely high value on using Rust's type system to produce robust, maintainable code.

**Core Principle**: "Make illegal states unrepresentable."

## 🧩 1. Prefer Enums Over Multiple Booleans/Options for State

Using many `bool` and `Option<T>` fields for modal state or application modes leads to invalid combinations that are only caught at runtime.

**Preferred approach** — Single `enum` representing the current state:

```rust
pub enum Overlay {
    None,
    Help,
    Viewer(Viewer),
    CommandLine(CommandLine),
    DriveSelection { side: PanelSide, selected_index: usize },
    DeleteConfirm(Vec<PathBuf>),
    // ... other mutually exclusive states
}
```

This pattern is already used successfully in the codebase with the `Overlay`
enum. The `CommandLine` state keeps its editable buffer and completion menu
together rather than scattering modal state across `App`.

## 🧩 2. Use Newtypes Instead of Primitive Types

Raw primitives (`u32`, `u64`, `usize`, `String`) lose semantic meaning and allow accidental misuse.

**Example** (already implemented):

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BookmarkSlot(u8);

impl BookmarkSlot {
    pub const MAX: u8 = 9;
    pub const COUNT: usize = 10;

    pub const fn new(n: u8) -> Result<Self, &'static str> { ... }
    pub const fn from_usize(n: usize) -> Result<Self, &'static str> { ... }
}
```

Other recommended newtypes:
- `FileSize(u64)`
- `UserId(u64)`, `OrderId(u64)`, etc.
- `Email(String)` with validation in constructor

## 💾 3. Make Illegal States Unrepresentable

Instead of a struct with many optional fields that can be combined in invalid ways:

```rust
// Bad - many invalid combinations possible
struct Order {
    status: String,
    tracking_number: Option<String>,
    cancellation_reason: Option<String>,
    paid_at: Option<DateTime>,
}
```

Use an enum with data that only exists in valid combinations:

```rust
enum Order {
    Pending { items: Vec<Item>, customer: Customer },
    Paid { items: Vec<Item>, customer: Customer, paid_at: DateTime, payment_id: PaymentId },
    Shipped { items: Vec<Item>, customer: Customer, tracking: TrackingInfo, shipped_at: DateTime },
    Cancelled { items: Vec<Item>, customer: Customer, reason: String, cancelled_at: DateTime },
}
```

## 🧩 4. Exhaustive Matching

Rust's exhaustive `match` is a superpower. Adding a new variant to an enum forces you to update all handling sites. This catches bugs at compile time.

## 🛡️ 5. Validate at the Boundaries ("Parse, Don't Validate")

- Validate and transform raw input (`&str`, `u64`, etc.) into rich domain types as early as possible.
- Once you have a `BookmarkSlot` or `Email`, you should be able to trust it within the domain logic.

## 🔗 Current Good Examples in the Codebase

**State machines (enums instead of bool/Option pairs):**

- `Overlay` — which modal dialog is open
- `ViewMode` — panels or console transcript, *orthogonal* to `Overlay`; the two
  were one enum until that made them mutually exclusive
- `Command` / `ShellCommand` — command-line parsing; `Command::Empty` is its own
  variant so `Shell` cannot carry a blank line
- `EntryKind` — replaced `is_dir` + `is_symlink`, whose reading needed two
  paragraphs of prose and once shipped a bug
- `EditState` — replaced the editor's `modified` + `confirm_exit`
- `Lifecycle` — running or quitting; it replaced a `should_quit` flag that
  sat *beside* a `Flow` enum, so the same question had two answers
- `Recall` — command-line history browsing vs. reverse search
- `QuickSearch` — `Active` never holds an empty query
- `Persistence` — a history that writes to a file, or deliberately nowhere
- `ConsoleLine`, `DriveEntry`, `SortMode`, `PanelSide`, `VirtualPath`, `Status`
- `Auth` — a session logs in by password, key file or agent; the password
  variant carries nothing, because no password is ever stored
- `ActionScope` — whether an action reaches for the active panel's entries or
  the program at large, which is what a NetBox panel refuses or lets through
- `Schedule` — when the task list samples next: `Due`, `At(Instant)`,
  `Waiting` with the `Request` that asked, `Obsolete` when a process control
  requires a fresh sample after the pending one, `Held`, or `Stopped` once the
  worker is gone; it replaced what would otherwise be a deadline, an
  in-flight flag, a held flag and a "worker lost" flag that had to agree
- `Modal` — the one thing open inside the task list, so a bar, a menu and a
  kill confirmation can never be open at once
- The task list's switches — `Arrangement` (flat or tree), `CommandView`,
  `KernelThreads` (hidden by default), `UserScope` — and `Sort`, a `SortKey`
  with a `SortOrder` relative to the key's natural `SortDirection`, where a
  pair of flags would let "descending" mean opposite things for CPU and PID

**Newtypes (a named unit instead of a bare primitive):**

- `SessionName`, `HostName`, `Port`, `UserName`, `KeyFile`, `RemotePath` —
  one checking constructor each, which `serde` also goes through, so a value
  from the dialog, `sessions.json` or an imported `.netbox` file is refused
  by the same rule; the refusal (`InvalidField`) names the field and the
  reason, never the value
- `RowCursor` — a list cursor that every move clamps to its list

- `BookmarkSlot` (0..=9) and `Bookmarks` (one entry per slot, so indexing is
  total and needs no bounds check)
- `FileSize` — bytes, with the megabyte conversion and the human-readable
  rendering on the type
- `FileName` — a bare entry name, never a path
- `Cells` (screen columns), `CharColumn` (characters), `Caret` (bytes) — three
  units the program used to measure in one `usize`
- `ScrollTop` and `ScrollBack` — offsets anchored to opposite ends;
  `ScrollLeft` — the viewer's sideways pan, in cells
- `RingIndex` — a position in the history ring
- `DurableLine` — a command line proven safe to persist
- `FailedEntry` — a failure's path and reason, kept apart
- `FileFormat` — the EOL style and final newline a file is written back with
- `Pid` — 1..=`i32::MAX`, so `kill(0)`, `kill(-1)` and a group kill cannot
  be written; `ProcessKey` — a `Pid` plus its `Start` (`Known(StartTime)` or
  `Unknown`), so a PID reused by a newcomer is a different process to the
  cursor, the marks and a kill
- `Percent` (hundredths, `ratio` rounds half up and never divides by zero),
  `MemoryBytes` (checked KiB and page conversions) and `Load` — the process
  viewer's units, all integer because release builds do not check overflow
- `Niceness` (−20..=19, stepped by a `NiceStep`) and `SettableClass` — a
  Windows priority class that cannot be Realtime, beside the `PriorityClass`
  that reports one
- `MenuCursor` (always less than its menu's length) and `PidQuery` (ASCII
  digits, at most ten, never above `i32::MAX` as a `Pid`)

**Constructor gates ("parse, don't validate"):** `ShellCommand::new`,
`DurableLine::classify`, `Menu::new`, `BookmarkSlot::new`, `Pid::new`,
`Niceness::try_from` — each returns
`Option`/`Result` and is the only way in, so the invalid state has no
representation for a later `if` to check for.

**Private fields carrying an invariant:** `Bookmarks` (one slot per
`BookmarkSlot`), `Menu` (never empty, selection always on a row), `Panel.state`
(the highlight always names a row — `Panel::select` clamps, and
`Panel::from_entries` exists because a struct literal was the way around it),
`LineEditor` (the caret is always on a `char` boundary). Each of these replaced
a check written out by hand at every call site.

## 📝 References

- Rust API Guidelines: C-CUSTOM-TYPE, C-NEWTYPE
- "Make Illegal States Unrepresentable" (widely adopted pattern)
- The Rust Book – Enums and Advanced Types
- Clippy lints: `struct_excessive_bools`, `fn_params_excessive_bools`

Follow these patterns when designing new features or refactoring existing code.
