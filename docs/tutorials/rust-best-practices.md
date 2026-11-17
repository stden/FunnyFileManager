<!-- Purpose: Teach practical Rust design and verification using FunnyFileManager. -->

# Rust best practices through FunnyFileManager

This tutorial is for contributors who know basic Rust syntax and want to make
safe, maintainable changes to FFM. Follow the examples in order; the short Rust
blocks are standalone and can be checked with `rustdoc --test` as shown below.
The FFM links lead to production code, not simplified copies of its APIs.

## 1. Follow a request through the program

FFM separates user intent from its effects. A key event becomes an `Action`
in [`src/action.rs`](../../src/action.rs); the application dispatches it in
[`src/application/dispatch.rs`](../../src/application/dispatch.rs), and
operations live under [`src/ops/`](../../src/ops/). Rendering reads state.
That separation makes key resolution testable without a terminal or filesystem.
It does **not** mean that every method on an FFM model is free of I/O.

When adding a feature, first identify the owner of the state transition, then
the boundary that reads or changes the outside world. Give each a narrow job.
See the [architecture guide](../knowledge-base/02-architecture/system-architecture.md)
and [module map](../knowledge-base/02-architecture/module-structure.md).

## 2. Make states explicit

Use an enum when alternatives exclude each other; put data on the variant that
owns it. A `match` forces callers to consider every variant. Two independent
facts can still be separate fields or enums: do not merge them merely because
both happen to be state.

```rust
#[derive(Debug, PartialEq, Eq)]
enum Dialog {
    Closed,
    ConfirmDelete { name: String },
}

fn message(dialog: &Dialog) -> &str {
    match dialog {
        Dialog::Closed => "Ready",
        Dialog::ConfirmDelete { name } => name,
    }
}

assert_eq!(message(&Dialog::Closed), "Ready");
assert_eq!(message(&Dialog::ConfirmDelete { name: "draft.txt".into() }), "draft.txt");
```

FFM's [`Overlay`](../../src/models/overlay.rs) represents the active modal;
[`BookmarkSlot`](../../src/models/bookmarks.rs) is a checked newtype. Use a
private field and a fallible constructor to keep an invariant true after
construction. Enforce the same validation for deserialization and other entry
points; a constructor cannot protect code that bypasses it.

```rust
#[derive(Debug, PartialEq, Eq)]
struct Slot(u8);

impl Slot {
    fn new(value: u8) -> Option<Self> {
        (value <= 9).then_some(Self(value))
    }
}

assert_eq!(Slot::new(9), Some(Slot(9)));
assert_eq!(Slot::new(10), None);
```

Read the project's [type-system guide](../knowledge-base/03-rust-best-practices/type-system.md)
for real examples and the trade-offs between enums and newtypes.

## 3. Borrow for access; own what must outlive the call

Accept a borrow when a function only reads a value, and return an owned value
when the result must be kept independently. Prefer `&str` over `&String` and
`&Path` over `&PathBuf` for read-only parameters. These are useful defaults,
not a reason to add complicated lifetimes everywhere. Clone deliberately when
independent ownership is needed; measure before optimizing a harmless clone.

```rust
use std::path::{Path, PathBuf};

fn child_path(directory: &Path, name: &str) -> PathBuf {
    directory.join(name)
}

let directory = Path::new("/tmp");
let child = child_path(directory, "notes.txt");
assert_eq!(child, Path::new("/tmp/notes.txt"));
```

The example only joins paths. It does **not** validate an untrusted name or
confine access to a directory; absolute paths, `..`, symlinks and concurrent
filesystem changes need an explicit policy. Avoid assuming paths are UTF-8:
`Path` and `OsStr` preserve OS-native names, while `display()` is for display,
not a lossless serialization. The MCP server has its own path-boundary rules;
read its implementation before changing filesystem access.

### Iterators and collections

Use an iterator when its stages make the transformation easier to read; use a
loop when branches or error handling are clearer that way. Iterators are lazy:
`filter` alone does no work until a consumer such as `count` or `collect` runs.
Avoid collecting an intermediate `Vec` when a consumer can use the iterator
directly, but choose clarity over speculative allocation savings.

```rust
let names = ["report.txt", ".hidden", "notes.txt"];
let visible = names.iter().filter(|name| !name.starts_with('.')).count();
assert_eq!(visible, 2);
```

The names above are illustrative: FFM also considers platform-specific hidden
attributes. For generic code, choose trait bounds that express what the
function actually needs. Prefer a concrete type when a generic API would add
complexity without a caller that needs it.

## 4. Distinguish absence, failure, and bugs

`Option<T>` means a value may be absent; `Result<T, E>` reports an operation
that can fail. Propagate recoverable failures with `?`, handle them at the
boundary that can decide what to tell the user, and preserve useful context.
Don't convert every error to `None`, silently discard it, or use `unwrap()` in
production. Tests may use `expect` when a failure should stop that test.

```rust
use std::num::ParseIntError;

fn parse_size(text: &str) -> Result<u64, ParseIntError> {
    let size = text.trim().parse::<u64>()?;
    Ok(size)
}

assert_eq!(parse_size(" 42 "), Ok(42));
assert!(parse_size("not a number").is_err());
```

The root application's release profile uses `panic = "abort"` and disables
overflow checks; the separate MCP package has its own profile. Destructors do
not run on abort, and `catch_unwind` cannot recover from an aborting panic.
Do not rely on a panic for routine I/O failures or bounds checks. For numeric
input and sizes, choose `checked_*`, `try_from`, or another explicit policy
instead of relying on debug-only overflow checks.

```rust
fn total_bytes(current: u64, additional: u64) -> Option<u64> {
    current.checked_add(additional)
}

assert_eq!(total_bytes(10, 20), Some(30));
assert_eq!(total_bytes(u64::MAX, 1), None);
```

## 5. Use RAII, and handle cleanup errors intentionally

Own a resource in a guard and restore it when the guard is dropped. FFM's
[terminal guards](../../src/terminal/guards.rs) manage terminal transitions;
[`merge_run_and_cleanup`](../../src/terminal.rs) shows why an explicit
cleanup path also matters when restoration can fail. `Drop` cannot return an
error to the caller, and neither `Drop` nor normal cleanup is guaranteed after
process termination or abort. Keep external effects and their failure paths
visible instead of treating RAII as a promise against every exit.

```rust
use std::cell::Cell;

struct Reset<'a>(&'a Cell<bool>);
impl Drop for Reset<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

let active = Cell::new(false);
{
    active.set(true);
    let _guard = Reset(&active);
    assert!(active.get());
}
assert!(!active.get());
```

## 6. Keep interfaces and unsafe code small

Prefer the least visibility a caller needs: `pub(crate)` for internal APIs,
`pub` for intentional external APIs. Use `foo.rs` plus `foo/`, not `mod.rs`,
and keep source files within the repository's 200-line policy. See
[`lib.rs`](../../src/lib.rs) for the crate's public surface. Document a public
function's behavior, errors and important invariants, rather than restating its
name. Do not make private implementation details public only for a test.

Safe Rust is the default. FFM denies `unsafe_code` unless a reviewed exception
is explicitly allowed; follow the repository's unsafe allowlist and document
why each unsafe operation satisfies its safety requirements. An `unsafe` block
moves proof obligations to the author; it does not disable Rust's other rules.
See the [Rustonomicon](https://doc.rust-lang.org/nomicon/) for the model.

## 7. Test behavior at the boundary that owns it

Start with the smallest failing test that demonstrates the behavior. Unit tests
can exercise private rules; integration tests use public APIs and actual
workflows. Test success, invalid inputs, and meaningful failure paths. Use
isolated temporary files for filesystem tests; avoid changing the process-wide
current directory or environment without the project's shared guards.

FFM keeps test bodies in separate test modules. Compare
[`src/action/editor/tests.rs`](../../src/action/editor/tests.rs) for key
resolution with [`src/popup/editor/tests.rs`](../../src/popup/editor/tests.rs)
for command interpretation. Cover the seam between them with a workflow test
when the interaction matters. A test that copies the implementation's logic
can repeat the same mistake; assert the observable result instead. See the
[testing strategy](../knowledge-base/05-testing/testing-strategy.md).

## 8. Measure before optimizing

First establish correctness, then measure a representative workload in a
release build. Compare the same inputs and record how you measured. FFM's
[benchmarks](../../benches/) and existing performance tests can be starting
points. Avoid filesystem reads or expensive recalculation in rendering paths;
FFM renders cached directory facts. Caching requires an invalidation rule and
tests for refresh and failure, not just a faster benchmark.

Choose synchronization around ownership and work boundaries. Keep locks short,
avoid blocking the UI event loop, and account for cancellation, worker failure
and late results. Read the relevant worker before adding concurrency; threads
and async are tools, not automatic speedups.

## 9. Practice: add a safe operation

Suppose a new key should mark one entry. Work through these steps:

1. Find the current keymap and the `Action`/dispatch owner. Write a failing
   key-resolution test and a state-transition test at their respective owners.
2. Add a typed action. Decide what happens for an empty panel, a parent entry,
   and an already marked item; implement those decisions where state is owned.
3. If the operation needs I/O, return or propagate its failure at the boundary;
   do not add an I/O call to rendering or silently ignore an error.
4. Add a workflow test if it needs to prove input, dispatch and visible state
   work together. Run the focused tests and the relevant checks below.

For a second exercise, trace `BookmarkSlot::new` and `from_usize`: identify the
valid range, find callers and tests, and explain where values above the maximum
are rejected. Both functions take unsigned values; if input is signed text,
identify the earlier parsing or conversion step that must reject a negative.

## 10. Check your change

From the repository root, use the pinned toolchain in
[`rust-toolchain.toml`](../../rust-toolchain.toml):

```sh
rustdoc --test docs/tutorials/rust-best-practices.md
cargo test <test_name>
cargo test
cargo test --features test-helpers
./ffm.py lint
```

Replace `<test_name>` with a real test filter; omit it to run the suite. The
MCP server is a separate Cargo package: run
`cargo test --manifest-path mcp/Cargo.toml` when changing it. The full lint
and test commands are required before a commit; see the
[build and test guide](../knowledge-base/04-development-workflow/build-run-test.md)
for features, tool prerequisites, coverage and the complete gate. Don't edit
stored coverage artifacts unless intentionally refreshing coverage.

## Further reading

- [The Rust Programming Language](https://doc.rust-lang.org/book/): ownership,
  error handling, traits and tests.
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/): designing
  usable public interfaces.
- [Rust standard library: `Path`](https://doc.rust-lang.org/std/path/struct.Path.html)
  and [`Result`](https://doc.rust-lang.org/std/result/): platform-aware paths
  and recoverable errors.
- [Rust Performance Book](https://nnethercote.github.io/perf-book/): measuring
  and investigating performance.
- [FFM best practices](../best-practices.md): project-specific rules.
