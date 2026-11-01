<!-- Purpose: Documents the Claude command for GitHub and CI work. -->

Check the latest GitHub Actions pipeline status and fix all failing steps.

Steps:

1. Run `gh run list --limit 5` to see recent workflow runs.

2. Find the latest run. If it succeeded, report "All checks passed" and stop.

3. If the latest run failed, run `gh run view <run-id> --log-failed` to get the failure logs.

4. Analyze the failure logs and identify the root cause:
   - Compilation errors → fix the code
   - Test failures → fix the failing tests or the code they test
   - Linting/clippy warnings → fix the warnings
   - Formatting issues → run `cargo fmt`
   - Dependency issues → update Cargo.toml (or the tracked mcp/Cargo.lock). The application's Cargo.lock is not tracked; a run's exact dependency set is its `cargo-lock` artifact: `gh run download <run-id> -n cargo-lock` into a checkout of the same commit reproduces it.

5. After fixing, run `cargo test` locally to verify the fix works.

6. Stage the fixes with `git add -u`, review untracked files and add relevant ones. Commit with a message like "fix: resolve CI failure — <brief description>" using the [+]/[-]/[!] markers for changed functions. Then run `git pull --rebase` (stop on conflicts) and `git push`.

7. Run `gh run list --limit 1` to confirm a new run was triggered. Run `gh run watch` to monitor it until it completes. If it fails again, repeat from step 3.

Rules:
- Do NOT skip or disable failing checks. Fix the underlying issue.
- Do NOT modify CI workflow files (.github/workflows/) unless the workflow itself is broken.
- If the failure is flaky (passes locally but fails in CI), investigate environment differences before changing code.
