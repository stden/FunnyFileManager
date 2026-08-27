<!-- Purpose: Documents the Rust design and quality rules followed across the project. -->

# Best Practices

## Testing

- Prefer deterministic tests over interactive flows.
- Use `cargo test --features test-helpers` when you want the full suite, because several integration-style tests are feature-gated.
- Add focused regression tests for real branches or bug fixes before adding broad coverage tests.
- Keep heavy test generation limited to stable, pure helpers such as `utils::format_size`.
- For TUI rendering checks, prefer `ratatui::backend::TestBackend`.
- Avoid tests that depend on network access, terminal state, or machine-specific paths unless the scenario is explicitly platform-gated.

## Code Changes

- Keep `src/main.rs` thin and move behavior into library modules.
- Prefer small, isolated modules under `src/` when adding major features.
- Use `Result` with `?` in application code instead of unchecked `unwrap()`.
- Preserve existing user changes in a dirty worktree; do not revert unrelated edits.
- Add comments only when the code would otherwise be hard to parse quickly.

## Documentation

- Update Markdown docs when commands, workflows, or feature flags change.
- Document feature-gated test commands explicitly so automation uses the right invocation.
- Keep examples copy-pasteable and consistent with the repository layout.

## Verification

- Baseline local validation:
  - `cargo test`
  - `cargo test --features test-helpers`
  - `cargo clippy -- -D warnings`
- Use coverage tooling only when intentionally refreshing coverage artifacts.

## Rust Type System & Modeling (Enums & Newtypes)

This project strongly prefers rich domain types over raw primitives (`bool`, `&str`, `u32/u64/usize`, `Option<T>`) whenever the concept has meaning in the problem domain. This follows the Rust philosophy of "make illegal states unrepresentable."

### Core Principles

- **Prefer enums over multiple booleans or Options for state.**  
  Multiple `bool` / `Option` fields (e.g., `show_help`, `viewer`, `command_line`, `show_drives`, `delete_confirm`) allow illegal combinations. Use a single `enum` instead.

  Example from this codebase:
  ```rust
  pub enum Overlay {
      None,
      Help,
      Viewer(Viewer),
      CommandLine(CommandLine),
      DriveSelection { side: PanelSide, selected_index: usize },
      // ... other mutually exclusive states
  }
  ```

  The `CommandLine` state owns both its editable buffer and its completion
  menu, so the overlay cannot hold one without the other.

- **Use newtypes (tuple structs) for domain values instead of primitives.**  
  `BookmarkSlot(u8)`, `UserId(u64)`, `FileSize(u64)`, `Email(String)` etc. give semantic meaning and allow invariants at construction time.

  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub struct BookmarkSlot(u8);

  impl BookmarkSlot {
      pub const MAX: u8 = 9;
      pub fn new(n: u8) -> Result<Self, &'static str> { ... }
  }
  ```

- **Make illegal states unrepresentable at the type level.**  
  Instead of:
  ```rust
  struct Order { status: String, tracking: Option<String>, cancel_reason: Option<String> }
  ```
  Prefer:
  ```rust
  enum Order {
      Pending { ... },
      Paid { ... },
      Shipped { tracking: Tracking, ... },
      Cancelled { reason: String, ... },
  }
  ```

- **Exhaustive matching is a feature.**  
  Adding a new variant to an enum forces you to handle it everywhere. This catches bugs at compile time.

- **Validate at the boundary ("parse, don't validate").**  
  Turn raw input (`&str`, `u64`) into a validated rich type as early as possible (e.g., in `Command::parse`, constructors for newtypes). After that point, the type guarantees correctness.

### When to Apply in This Codebase

- State machines / UI modes → `Overlay`, `ConnectionState`, etc.
- Identifiers and constrained integers → `BookmarkSlot`, `UserId`, `DriveIndex`
- File system concepts → `FileSize`, `VirtualPath` (already an enum: `Local` / `S3`)
- Commands and operations → `Command` enum (replacing string dispatch)
- Any place where a primitive + comments or runtime checks currently describe the rules.

### Practical Guidelines Used Here

- Derive the useful traits on newtypes/enums (`Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Display` when appropriate).
- Keep the inner value private (or `pub(crate)`) and provide controlled constructors.
- Use `const fn` for constructors and accessors where possible.
- Document invariants in the type's doc comment.
- Combine with `#[non_exhaustive]` on public enums when we expect future variants.

### References (authoritative sources)

- Rust API Guidelines – C-CUSTOM-TYPE & C-NEWTYPE
- "Make Illegal States Unrepresentable" (widely adopted pattern in the Rust community)
- The Rust Book – Enums (Ch. 6) and Advanced Types (newtype pattern)
- Clippy lints: `struct_excessive_bools`, `fn_params_excessive_bools`

Follow these patterns when extending the application (new features, refactoring overlays, commands, or domain objects). The compiler becomes your strongest ally against whole classes of bugs.
