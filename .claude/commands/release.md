<!-- Purpose: Documents the Claude command for release builds. -->

Build the release binary and report its size and status.

Steps:

1. Run `cargo build --release -j1 2>&1` to build the optimized release binary.
   (Use `-j1` to avoid OOM on AWS SDK crates.)

2. If the build fails, show the error and fix it (same approach as /build).

3. Locate the binary: `target/release/ffm` (or `target/release/ffm.exe` on Windows).

4. Report binary size: `ls -lh target/release/ffm*`

5. Run a quick smoke test: `./target/release/ffm --help 2>&1 || echo "no --help flag"` to confirm the binary starts.

6. Report: "Release build OK — binary size: X MB" or errors.

Rules:
- Never strip debug symbols manually unless the user asks — let the release profile settings control this.
- Do NOT commit the release binary to git.
- If the release binary is significantly larger than expected, note it but don't change optimization settings without asking.
