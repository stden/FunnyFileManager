<!-- Purpose: Practical Rust tutorial — patterns used in FFM, with working examples. -->

# 📘 Rust Best Practices: A Practical Tutorial

This tutorial covers the Rust idioms and patterns this codebase relies on
most. Each section shows the *why*, a minimal counter-example, and then the
preferred approach with a real or FFM-inspired example.

Related reading: [`type-system.md`](type-system.md) — enums and newtypes (**read that first**).

---

## 1. Error Handling

### 1.1 Never use `unwrap()` or `expect()` in production code

`unwrap()` turns a recoverable error into an unrecoverable panic.
In a TUI application a single panic kills the whole process and
leaves the terminal in a broken state.

```rust
// ❌ panics at runtime if the file is missing
let text = fs::read_to_string("config.toml").unwrap();

// ✅ propagates the error to the caller with `?`
fn load_config() -> io::Result<String> {
    let text = fs::read_to_string("config.toml")?;
    Ok(text)
}
```

`unwrap()` and `expect()` are allowed **only inside `#[test]`** — there a
panic is the correct failure mode.

### 1.2 Use `?` for propagation, not manual `match`

```rust
// ❌ boilerplate that obscures intent
fn parse_port(s: &str) -> Result<u16, String> {
    match s.parse::<u16>() {
        Ok(n) => Ok(n),
        Err(e) => Err(e.to_string()),
    }
}

// ✅ concise — ? converts the error type automatically (needs From impl)
fn parse_port(s: &str) -> Result<u16, ParseIntError> {
    let n = s.parse::<u16>()?;
    Ok(n)
}
```

### 1.3 Define typed errors with `thiserror`

Use `thiserror::Error` for library / domain errors; keep variants specific so
callers can match on them.

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum SessionError {
    #[error("hostname cannot be empty")]
    EmptyHostname,
    #[error("port {0} is out of range 1–65535")]
    InvalidPort(u16),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}
```

The `#[from]` attribute auto-generates the `From<io::Error>` impl so `?`
on `io::Result` works inside functions that return `Result<_, SessionError>`.

---

## 2. Ownership and Borrowing

### 2.1 Prefer borrowing to cloning

Cloning copies heap data. Pass a reference unless the callee needs ownership.

```rust
// ❌ unnecessary heap allocation
fn print_name(name: String) { println!("{name}"); }
print_name(entry.name.clone());

// ✅ borrow
fn print_name(name: &str) { println!("{name}"); }
print_name(&entry.name);
```

### 2.2 Return owned data from constructors; borrow everywhere else

```rust
impl FileName {
    /// Constructor — owns the string.
    pub fn new(s: impl Into<String>) -> Self { Self(s.into()) }

    /// Accessor — borrows cheaply.
    pub fn as_str(&self) -> &str { &self.0 }
}
```

### 2.3 Use `Cow<str>` when the common path needs no allocation

`Cow::Borrowed` costs nothing; `Cow::Owned` allocates only when necessary.

```rust
use std::borrow::Cow;

fn normalise_path(s: &str) -> Cow<str> {
    if s.contains('\\') {
        Cow::Owned(s.replace('\\', "/"))
    } else {
        Cow::Borrowed(s)   // no allocation for the common case
    }
}
```

### 2.4 Avoid interior mutability unless the design demands it

`RefCell<T>` and `Cell<T>` defer borrow checking to runtime and can panic.
Prefer restructuring the API to pass `&mut self` instead.

---

## 3. Iterators and Functional Patterns

Iterators are zero-cost abstractions — they compile to the same machine code
as a hand-written loop.

### 3.1 Chain instead of mutating a buffer

```rust
// ❌ imperative
let mut names = Vec::new();
for e in entries {
    if e.is_dir() {
        names.push(e.name.clone());
    }
}

// ✅ declarative — no temporary mutation
let names: Vec<&str> = entries
    .iter()
    .filter(|e| e.is_dir())
    .map(|e| e.name.as_str())
    .collect();
```

### 3.2 Use `find` / `position` / `any` for early exit

```rust
// ❌ walks the whole list even after finding the item
let found = entries.iter().filter(|e| e.name == name).count() > 0;

// ✅ stops at the first match
let found = entries.iter().any(|e| e.name == name);

// returning the index
let index = entries.iter().position(|e| e.name == name);
```

### 3.3 Avoid `collect` → iterate again; compose the chain

```rust
// ❌ two passes
let visible: Vec<_> = entries.iter().filter(|e| !e.hidden).collect();
let count = visible.len();

// ✅ one pass
let count = entries.iter().filter(|e| !e.hidden).count();
```

### 3.4 `flat_map` for nested collections

```rust
let all_files: Vec<&Path> = dirs
    .iter()
    .flat_map(|d| d.files.iter().map(|f| f.path.as_path()))
    .collect();
```

---

## 4. Module Organisation (Rust 2018 Layout)

FFM uses the **Rust 2018 module layout**: no `mod.rs` files.

```
src/
  popup.rs          ← declares the module
  popup/
    editor.rs       ← submodule
    viewer.rs       ← submodule
    viewer/
      tests.rs      ← nested tests
```

### 4.1 Visibility rules

| Visibility   | When to use                                                        |
| ------------ | ------------------------------------------------------------------ |
| `pub(crate)` | **Default** — everything not consumed by `tests/`                  |
| `pub(super)` | Helpers only the parent module needs                               |
| `pub`        | Items re-exported from `lib.rs`, or exercised by integration tests |
| *(private)*  | Internal helpers that nothing outside the file needs               |

```rust
// A type used across crates → pub
pub struct Viewer { … }

// A type only used within this crate → pub(crate)
pub(crate) struct ScrollState { … }

// A helper only the parent module calls → pub(super)
pub(super) fn layout(area: Rect) -> [Rect; 3] { … }
```

### 4.2 Keep files under 200 lines

When a file grows beyond ~150 lines, split it:

```
// before: src/popup/editor.rs  (250 lines)
// after:
src/popup/editor.rs       ← public API + dispatch (~80 lines)
src/popup/editor/
  search.rs               ← search modal logic
  keys.rs                 ← key resolution
  tests.rs                ← unit tests
```

The enforcing test is `tests/file_line_limit_test.rs`.

---

## 5. Pattern Matching

### 5.1 Destructure in `let` and function parameters

```rust
// ❌ accessing fields via dotting
let x = point.x;
let y = point.y;

// ✅ destructure once
let Point { x, y } = point;

// in a function parameter
fn render_cursor(&self, Point { x, y }: Point) { … }
```

### 5.2 Use `if let` for single-variant checks

```rust
// ❌ verbose match on one arm
match app.overlay {
    Overlay::Viewer(ref v) => scroll(v),
    _ => {}
}

// ✅ concise
if let Overlay::Viewer(ref v) = app.overlay {
    scroll(v);
}
```

### 5.3 Guards for complex conditions

```rust
match event {
    Event::Key(key) if key.code == KeyCode::Enter && !app.is_busy() => confirm(app),
    Event::Key(key) if key.modifiers.contains(KeyModifiers::CTRL) => handle_ctrl(app, key),
    _ => {}
}
```

### 5.4 Avoid nested Options — use `?` in closures via `.and_then`

```rust
// ❌ nested if-lets
if let Some(overlay) = app.get_overlay() {
    if let Some(viewer) = overlay.as_viewer() {
        scroll(viewer);
    }
}

// ✅ short-circuit with and_then
app.get_overlay()
   .and_then(Overlay::as_viewer)
   .map(scroll);
```

---

## 6. Traits and Generics

### 6.1 Prefer `impl Trait` in argument position for concrete callers

```rust
// ✅ accepts any iterator over &str without a type parameter in the API
fn join_paths(parts: impl Iterator<Item = &str>) -> String {
    parts.collect::<Vec<_>>().join("/")
}
```

Use a named generic (`<T: Trait>`) when the bound appears more than once, or
when the type parameter must appear in the return type.

### 6.2 Use `From` / `Into` for infallible conversions

```rust
impl From<FileSize> for u64 {
    fn from(s: FileSize) -> u64 { s.0 }
}

// callers get .into() for free
let bytes: u64 = file_size.into();
```

### 6.3 Blanket impls with `where` for readability

```rust
// ❌ hard to parse inline
fn print<T: Debug + Display + Clone>(val: T) { … }

// ✅ where clause
fn print<T>(val: T)
where
    T: Debug + Display + Clone,
{ … }
```

### 6.4 Prefer static dispatch; use `dyn Trait` only at boundaries

```rust
// ✅ monomorphised — zero overhead
fn render_item(item: &impl Renderable, frame: &mut Frame) { item.render(frame); }

// dyn is fine at application boundaries where the concrete type is not known at
// compile time (e.g. the backend passed to Terminal::new)
fn make_terminal(backend: Box<dyn Backend>) -> Terminal<Box<dyn Backend>> { … }
```

---

## 7. Testing

### 7.1 Test-Driven Development (TDD)

Write the failing test **first**, then implement the smallest code that makes
it pass.

```rust
// 1. write this first — it must fail to compile or fail at runtime
#[test]
fn bookmark_slot_rejects_values_above_nine() {
    assert!(BookmarkSlot::new(10).is_err());
}

// 2. now implement BookmarkSlot::new
pub const fn new(n: u8) -> Result<Self, &'static str> {
    if n > Self::MAX { Err("slot must be 0–9") } else { Ok(Self(n)) }
}
```

### 7.2 One assertion per logical concept

One test, one fact. When a test fails the name says exactly what broke.

```rust
// ❌ multiple concepts in one test — which one failed?
#[test]
fn bookmark_tests() {
    assert!(BookmarkSlot::new(0).is_ok());
    assert!(BookmarkSlot::new(9).is_ok());
    assert!(BookmarkSlot::new(10).is_err());
    assert!(BookmarkSlot::new(255).is_err());
}

// ✅ one fact per test
#[test] fn slot_zero_is_valid()      { assert!(BookmarkSlot::new(0).is_ok()); }
#[test] fn slot_nine_is_valid()      { assert!(BookmarkSlot::new(9).is_ok()); }
#[test] fn slot_ten_is_rejected()    { assert!(BookmarkSlot::new(10).is_err()); }
#[test] fn slot_255_is_rejected()    { assert!(BookmarkSlot::new(255).is_err()); }
```

### 7.3 Use `TempDir` for filesystem tests

```rust
#[test]
fn viewing_a_utf8_file_decodes_correctly() -> io::Result<()> {
    let dir = tempfile::TempDir::new()?;
    let path = dir.path().join("notes.txt");
    fs::write(&path, "héllo\n")?;

    let bytes = fs::read(&path)?;
    let viewer = Viewer::file(path, &bytes);
    assert_eq!(viewer.lines, vec!["héllo"]);
    Ok(())
}
```

### 7.4 Test the public API, not the implementation

Tests in `tests/` (integration) and `src/**/tests.rs` (unit) should call the
same functions a consumer would. If a test needs to reach into private fields,
the design may need reconsideration.

---

## 8. Clippy and Style

FFM runs `cargo clippy -- -D warnings` (warnings become errors). Common
violations to avoid:

| Lint                                 | Bad                                  | Good                     |
| ------------------------------------ | ------------------------------------ | ------------------------ |
| `needless_pass_by_value`             | `fn f(s: String)` when you only read | `fn f(s: &str)`          |
| `clone_on_ref_ptr`                   | `arc.clone()` just to pass it        | keep the reference       |
| `match_wildcard_for_single_variants` | `_ =>` on a single-variant enum      | name the variant         |
| `struct_excessive_bools`             | 3+ `bool` fields                     | an `enum`                |
| `manual_let_else`                    | `if let … { } else { return; }`      | `let … else { return; }` |
| `redundant_closure`                  | `.map(\|x\| f(x))`                   | `.map(f)`                |

Run the full gate before every commit:

```bash
./ffm.py lint    # typos + clippy + fmt
cargo test       # unit + integration
```

---

## Quick Reference

| Pattern                 | Rule                              |
| ----------------------- | --------------------------------- |
| `unwrap()` / `expect()` | Tests only                        |
| Error propagation       | `?` operator                      |
| Typed errors            | `thiserror::Error` enum           |
| Module files            | `foo.rs` + `foo/`, never `mod.rs` |
| Default visibility      | `pub(crate)`                      |
| File size               | ≤ 200 lines                       |
| New feature             | Failing test first                |
| Bool state              | Use an enum                       |
| Raw primitive           | Newtype with a constructor        |
| Multiple options/bools  | Enum with per-variant data        |
