<!-- Purpose: Documents the Claude command for mutation testing. -->

Run the full mutation testing suite and kill all surviving mutants.

Steps:

1. Run all tests first to ensure baseline passes:
   `cargo test --features test-helpers 2>&1 | tail -10`
   If any test fails, stop and report — do not proceed with mutation testing on a broken baseline.

2. Run mutation-specific test suites:
   ```
   cargo test --test mutation_tests 2>&1 | tail -20
   cargo test mutation_killer_tests 2>&1 | tail -20
   ```

3. For each failing mutation test:
   - Read the test to understand which mutant it targets.
   - Check if the mutant can actually survive (i.e., the behavior difference is observable).
   - Add a new assertion or test that would fail for the mutant but pass for the correct code.
   - The new assertion must test a *specific, observable behavior difference*, not just duplicate existing checks.

4. Re-run the mutation test suite to confirm all mutants are killed.

5. Report: "All mutants killed ✓" or list surviving mutants with explanation.

Rules:
- Do NOT modify production code to kill mutants — only add tests.
- Each new test should have a comment like `// Kill: replace X with Y in function Z`.
- A good mutation-killing test checks a precise boundary condition or output value.
- Use `assert_eq!` and `assert_ne!` over `assert!(a == b)` — better error messages.
