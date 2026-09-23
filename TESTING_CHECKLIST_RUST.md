<!-- Purpose: Collects the Rust testing checklist used before integration or release. -->

# Testing checklist

The strategy and the seam table are in `docs/knowledge-base/05-testing/testing-strategy.md`. The workflow is in `TESTING_TDD_GUIDE.md`.

## Before a change is finished

- [ ] `./ffm.sh lint` passes (format, Clippy `-D warnings`, the shared feature matrix).
- [ ] `cargo test --features test-helpers` passes for the behavior you touched.
- [ ] New behavior and bug fixes have a test that failed before the fix.
- [ ] Filesystem tests use `tempfile::TempDir` and do not read the checkout or the home directory.
- [ ] Rendering checks use a `TestBackend` buffer when the style or the text is the behavior.
- [ ] Cucumber changes run with `cargo test --test bdd` (and `bdd_ru` when those steps changed). Nextest does not run `harness = false` suites.
- [ ] Optional `s3` or `keepass` behavior was tested with that feature enabled.
- [ ] Coverage exclusions and the 98% / 75% line gates were not weakened to hide a miss.
- [ ] No secrets, personal paths, or machine-specific hosts were added. The private-data lint enforces this.
