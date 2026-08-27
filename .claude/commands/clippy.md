<!-- Purpose: Documents the Claude command for clippy cleanup. -->

Run clippy linter and fix all warnings.

Steps:

1. Run `cargo clippy --all-targets --all-features -- -D warnings 2>&1` to lint the project.

2. If clippy passes with no warnings, report "Clippy clean ✓" and stop.

3. For each warning:
   - Read the affected code.
   - Apply the suggested fix (clippy suggestions are usually correct).
   - Common fixes:
     - `needless_return` → remove `return`
     - `redundant_closure` → replace `|x| f(x)` with `f`
     - `map_unwrap_or` → replace `.map(...).unwrap_or(...)` with `.map_or(..., ...)`
     - `while let Ok(_) = x` → `while x.is_ok()`
     - `len() > 0` → `!is_empty()`
     - `clone()` on `Copy` types → remove clone
     - `unwrap_or_else(|_| ...)` when arg is not expensive → `unwrap_or(...)`

4. After fixing all warnings, re-run clippy to confirm zero warnings.

5. Run `cargo test 2>&1 | tail -5` to confirm fixes didn't break tests.

Rules:
- Do NOT use `#[allow(clippy::...)]` to suppress a warning unless you can explain why the warning is a false positive for this specific case.
- Fix the code, don't silence clippy.
- If a clippy suggestion would make the code harder to read, leave a comment explaining why, and then use `#[allow]` as a last resort.
