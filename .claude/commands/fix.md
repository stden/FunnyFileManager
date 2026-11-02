<!-- Purpose: Documents the Claude command for focused bug fixing. -->

Fix a specific bug or error. Usage: /fix <description or error message>

Steps:

1. If an error message was provided, find the relevant source location:
   - Parse the file path and line number from the error (e.g. `src/handlers.rs:42`)
   - Read the file around that line for context.

2. Reproduce the bug:
   - If there's a test that triggers it: run `cargo test <test_name> 2>&1`
   - If it's a build error: run `cargo build -j1 2>&1`
   - If it's a runtime bug: identify the code path and trace it manually.

3. Identify the root cause — do not guess. Read the actual code.

4. Apply the minimal fix:
   - Fix the root cause, not the symptom.
   - Do not add workarounds (`unwrap_or_default()` to silence a panic) — fix why the value is None.
   - Do not change unrelated code.

5. Verify the fix:
   - Run `cargo test 2>&1 | tail -20` — all tests must pass.
   - If the bug had no test: add one that would have caught it.

6. Report: what the bug was, what caused it, and what the fix does.

Rules:
- The fix should be the smallest change that correctly addresses the root cause.
- If fixing the bug would require a larger refactor, do the refactor and the fix together but clearly separate them in the commit message.
- If you cannot reproduce the bug, say so — do not guess and patch blindly.
