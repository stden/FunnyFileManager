<!-- Purpose: Documents the Claude command for guided refactoring. -->

Refactor the specified code to follow Rust best practices and improve clarity.

Usage: /refactor <file or function name>

Steps:

1. Read the specified file or locate the function by name with grep.

2. Identify refactoring opportunities:
   - **Extract**: Repeated logic that appears 3+ times → extract to a named function
   - **Simplify**: Nested `match`/`if let` that can be flattened
   - **Rename**: Variables/functions with unclear names (`x`, `tmp`, `do_thing`)
   - **Reduce allocations**: `to_string()` → `as_str()`, `clone()` where a borrow works
   - **Error handling**: `unwrap()`/`expect()` → propagate with `?`
   - **Iterator chains**: Manual loops → `.map()`, `.filter()`, `.any()`, `.find()`
   - **Dead code**: Remove unused functions, imports, variables

3. For each refactoring:
   - Show the before and after.
   - Explain why the change improves the code.
   - Apply the change.

4. Run `cargo test 2>&1 | tail -10` to confirm tests still pass after refactoring.

5. Run `cargo clippy 2>&1` to confirm no new warnings.

6. Report a summary of changes made.

Rules:
- Do NOT change behavior — refactoring is behavior-preserving by definition.
- Do NOT refactor code that isn't part of the specified scope.
- If a refactoring would require changing tests, that's a sign it might not be behavior-preserving — stop and ask.
- Prefer small, incremental changes over large rewrites.
