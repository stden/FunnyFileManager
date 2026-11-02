<!-- Purpose: Documents the Claude command for dependency maintenance. -->

Audit and update project dependencies.

Steps:

1. Run `cargo outdated 2>&1` to list outdated dependencies.
   If `cargo-outdated` is not installed, run `cargo install cargo-outdated` first.

2. Run `cargo audit 2>&1` to check for known security vulnerabilities.
   If `cargo-audit` is not installed, run `cargo install cargo-audit` first.

3. For each outdated dependency:
   - Check the changelog/release notes for breaking changes.
   - If it is a minor/patch update with no breaking changes: update it in Cargo.toml.
   - If it is a major update: note it but do NOT auto-update — ask the user first.

4. For each security advisory:
   - Show the CVE/advisory ID and affected version range.
   - Update to the patched version if available.
   - If no patch is available, add it to a tracking comment in Cargo.toml.

5. After updates: run `cargo build -j1` and `cargo test` to confirm nothing broke.

6. Report:
   - N dependencies updated
   - N security advisories found/fixed
   - List of major updates that need manual review

Rules:
- Never downgrade a dependency.
- Never update a dependency just because a newer version exists — only update if there's a reason (security, bug fix, needed feature).
- Always verify tests pass after updating.
