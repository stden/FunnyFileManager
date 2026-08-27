<!-- Purpose: Documents the Claude command for running and improving tests. -->

Run all tests, measure code coverage, and improve it to 100%.

Steps:

1. Run `cargo test` to verify all tests pass. If any test fails, fix it before proceeding. Do NOT continue to coverage until all tests are green.

2. Run `cargo tarpaulin --out json --output-dir target/tarpaulin 2>&1` to measure code coverage. Parse the JSON output from `target/tarpaulin/tarpaulin-report.json` to find uncovered lines.

3. If coverage is already 100%:
   - Run mutation tests: `cargo test --test mutation_tests` and `cargo test mutation_killer_tests`
   - If all mutation tests pass, report success.
   - If any mutants survive, add new tests to kill them.

4. If coverage is below 100%:
   - Identify uncovered lines from the tarpaulin JSON report. Focus on `src/` files, ignore test files.
   - Add new tests to cover the missing lines. Add tests to the most appropriate existing test file or module.
   - Re-run `cargo tarpaulin` to verify coverage increased.
   - Repeat until 100% coverage is reached or all remaining uncovered lines are unreachable (e.g. `#[cfg]` guards for other platforms).

Rules:
- Do NOT modify production code just to increase coverage. Only add tests.
- Keep tests simple and focused — one behavior per test.
- Use `tempfile::TempDir` for filesystem isolation.
- Name tests descriptively: `test_<function>_<scenario>`.
