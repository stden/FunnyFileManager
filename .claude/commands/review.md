<!-- Purpose: Documents the Claude command for code review. -->

Review recent code changes for quality, correctness, and Rust best practices.

Steps:

1. Run `git diff origin/main...HEAD` to see all changes since the last push.
   If the diff is empty, run `git diff HEAD~1` to review the last commit.

2. For each changed file, check for:

   **Correctness:**
   - Off-by-one errors in index arithmetic
   - Unwrap/expect on values that could reasonably be None/Err
   - Integer overflow in size calculations
   - Race conditions (unlikely in single-threaded TUI, but check Arc/Mutex usage)

   **Rust idioms:**
   - Use `?` instead of `unwrap()` in functions returning `Result`
   - Use `if let` / `while let` instead of `match` for single-arm patterns
   - Use iterators instead of manual index loops where readable
   - Avoid unnecessary `.clone()` — check if a borrow suffices
   - Use `PathBuf` methods instead of string concatenation for paths

   **Performance:**
   - Unnecessary allocations in hot loops (`.to_string()` inside loops)
   - Sorting the same data multiple times
   - Reading a file just to get its length

   **Testing:**
   - New public functions with no test coverage
   - Tests that only test the happy path (no edge cases)

3. For each issue found:
   - State the file and line number.
   - Explain the problem concisely.
   - Suggest a concrete fix.
   - Mark as: [BUG], [STYLE], [PERF], or [TEST].

4. If no issues are found, report "Review clean ✓".

Rules:
- Report issues, do NOT auto-fix unless the user asks.
- Be direct — a review comment should be actionable, not vague.
- Do not flag style issues that `cargo fmt` and `cargo clippy` already catch.
