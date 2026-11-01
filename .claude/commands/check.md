<!-- Purpose: Documents the Claude command for the full quality gate. -->

Run all quality checks in sequence: fmt → clippy → build → test.

This is the full pre-commit quality gate.

Steps:

1. `cargo fmt --check 2>&1`
   - If dirty: run `cargo fmt` and note which files were reformatted.

2. `cargo clippy --all-targets -- -D warnings 2>&1`
   - If warnings: fix them all (see /clippy for guidance). Re-run until clean.

3. `cargo build -j1 2>&1`
   - If errors: fix them. Re-run until clean.

4. `cargo test --features test-helpers 2>&1`
   - If failures: fix the failing tests or the code they test. Re-run until all pass.

5. Report final status for each step:
   ```
   fmt:    OK
   clippy: OK
   build:  OK
   tests:  OK (N passed)
   ```
   Or list failures.

Rules:
- Do not skip a step because it passes — run all four in order.
- Do not commit if any step fails.
- This command is the definition of "ready to commit".
