<!-- Purpose: Documents the Claude command for running and improving tests. -->

Run all tests, measure code coverage, and improve it to 100%.

Steps:

1. Run `cargo test` to verify all tests pass. If any test fails, fix it before proceeding. Do NOT continue to coverage until all tests are green.

2. Run `./ffm.py coverage` to measure code coverage: CI's gate (`scripts/coverage.py`). It prints llvm-cov's table and every uncovered line per file, then the exact share against the 98% floor.

3. If coverage is already 100%:
   - Run mutation tests: `cargo test --test mutation_tests` and `cargo test mutation_killer_tests`
   - If all mutation tests pass, report success.
   - If any mutants survive, add new tests to kill them.

4. If coverage is below 100%:
   - Identify uncovered lines from the "Uncovered Lines" list. Focus on `src/` files; test files are not measured.
   - Add new tests to cover the missing lines. Add tests to the most appropriate existing test file or module.
   - Re-run `./ffm.py coverage` to verify coverage increased.
   - Repeat until 100% coverage is reached or all remaining uncovered lines are unreachable (e.g. `#[cfg]` guards for other platforms).

Rules:
- Do NOT modify production code just to increase coverage. Only add tests.
- Keep tests simple and focused — one behavior per test.
- Use `tempfile::TempDir` for filesystem isolation.
- Name tests descriptively: `test_<function>_<scenario>`.
