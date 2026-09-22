<!-- Purpose: Documents the Claude command for scaffolding new features. -->

Scaffold a new feature following the project's architecture patterns.

Usage: /scaffold <feature name> — e.g. `/scaffold file-permissions-display`

Steps:

1. Read `CLAUDE.md` to understand the project structure and architecture.

2. Identify which modules the feature touches:
   - New data → `src/models.rs`
   - New UI element → `src/ui.rs`
   - New keyboard handler → `src/lib.rs` (event loop) + `src/handlers.rs`
   - New file operation → `src/utils.rs` or `src/handlers.rs`

3. Add the data model first (structs, enums, fields on `App` or `Panel`).

4. Add the handler function in `src/handlers.rs` with a clear, testable interface.

5. Wire up the keyboard event in `src/lib.rs` event loop.

6. Add UI rendering in `src/ui.rs` if the feature needs visual output.

7. Add at least 2 tests:
   - One happy-path test
   - One edge-case test (empty input, None selection, etc.)

8. Run `/check` to verify everything compiles and passes.

9. Report what was scaffolded and what still needs to be implemented (leave TODO comments for complex parts).

Rules:
- Follow existing naming conventions — look at how similar features are named.
- New fields on `App` must have sensible defaults in `App::new()`.
- Every new `pub fn` in handlers needs at least one test.
- Do NOT implement the full feature — scaffold the structure and let the user fill in logic.
