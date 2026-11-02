<!-- Purpose: Documents the Claude command for building the project. -->

Build the project and report all warnings and errors.

Steps:

1. Run `cargo build -j1 2>&1` to build the project. (Use `-j1` to avoid OOM on AWS SDK crates.)

2. If the build fails:
   - Show the full error output.
   - Identify the root cause (missing import, type mismatch, lifetime issue, etc.).
   - Fix the error in the source code.
   - Re-run `cargo build -j1` to confirm the fix. Repeat until the build succeeds.

3. If the build succeeds but shows warnings:
   - List all warnings with their file and line number.
   - Fix warnings that are clearly wrong (unused variables, dead code, deprecated APIs).
   - Do NOT suppress warnings with `#[allow(...)]` unless the warning is a known false positive.
   - Re-run `cargo build -j1` to confirm no new warnings were introduced.

4. Report: "Build OK — N warnings fixed" or "Build failed: <summary>".

Rules:
- Never use `#[allow(warnings)]` globally.
- Never comment out code just to silence warnings — remove it if unused.
- Treat `cargo build` success with zero warnings as the target state.
