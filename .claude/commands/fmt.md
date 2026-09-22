<!-- Purpose: Documents the Claude command for formatting the repository. -->

Format all Rust source code with cargo fmt.

Steps:

1. Run `cargo fmt --check 2>&1` to see what would change without modifying files.

2. If output is empty (nothing to format), report "Already formatted ✓" and stop.

3. If there are formatting differences:
   - Run `cargo fmt 2>&1` to apply formatting.
   - Run `cargo fmt --check 2>&1` again to confirm all files are now formatted.
   - Run `git diff --stat` to show which files were changed.

4. Report the list of reformatted files.

Rules:
- Never modify rustfmt.toml settings to suppress formatting changes — apply the format as-is.
- Do NOT stage or commit the formatting changes unless the user asks.
- If a file has a `#[rustfmt::skip]` attribute, leave it — it was intentionally excluded.
