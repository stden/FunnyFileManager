<!-- Purpose: Lists the commands used to build, run, and test the project. -->

# 🏗️ Build, Run, Test, and Coverage

## 💬 Essential Commands

| Command | Purpose |
|---------|---------|
| `cargo run` (or `cargo run --bin ffm`) | Run locally using the incremental development profile |
| `cargo build` | Build the development binary for fast iteration |
| `cargo build --release` | Optimized release binary |
| `cargo test` | Run unit + basic tests |
| `cargo test --features test-helpers` | Include helper-gated integration tests |
| `cargo test -- --ignored` | Run tests marked `#[ignore]` |
| `cargo fmt` | Format all code |
| `cargo clippy -- -D warnings` | Lint (must be clean) |
| `./scripts/lint.py` / `./ffm.py lint` | Shared local/CI format, spelling (`typos`, `cargo install typos-cli`) and Clippy gate for TUI, MCP, and Colorer |
| `./ffm.py lint spelling` | Spell check only; project words live in `typos.toml` |
| `./ffm.py lint self-test` | Prove the spell check rejects a misspelling; run the developer scripts' unit tests (`scripts/tests`); compile positive/negative readability fixtures and compare behavior; prove the manifest lint policy on positive/negative fixtures; requires Python |
| `cargo build --manifest-path mcp/Cargo.toml` | Build the standalone MCP server with its development profile |
| `cargo test --manifest-path mcp/Cargo.toml` | Run the MCP package's separate test suite |
| `cargo tarpaulin --out Html --output-dir coverage` | Generate HTML coverage report |

The shared lint gate checks formatting in the root and MCP packages, then
runs Clippy with `--all-targets -- -D warnings` for the root's default features,
all features, and `--no-default-features --features test-helpers`. It also
checks every target in MCP and Colorer. This covers helper-gated tests, feature
interactions, and the built-in highlighter without the optional Colorer adapter.
CI calls the same script, then runs `self-test` and architecture contracts as
separate checks. The self-test can also run directly as
`python scripts/test_readability.py` (the compilers, without Cargo) and
`python scripts/test_lint_policy.py` (see the lint policy below).

## 🛡️ Lint Policy

Each crate declares its own `[lints]` tables (`Cargo.toml`, `mcp/Cargo.toml`,
`colorer-sys/Cargo.toml`); the root `clippy.toml` holds the shared thresholds
and test allowances. On top of Clippy's `all`, `pedantic` and `nursery` groups,
every crate enables these reliability families at `warn`, which the gate's
`-D warnings` makes errors:

| Family | Lints | What it stops |
|---|---|---|
| Rust guards | `unused_lifetimes`, `single_use_lifetimes`, `redundant_lifetimes`, `explicit_outlives_requirements`, `unused_macro_rules`, `meta_variable_misuse`, `unused_import_braces`, `unused_extern_crates`, `macro_use_extern_crate`, `trivial_numeric_casts`, `unit_bindings`, `keyword_idents_2024`, `non_ascii_idents`, `unsafe_attr_outside_unsafe` | Nothing trips them today; they keep it that way |
| Rust idioms | `let_underscore_drop`, `trivial_casts`, `unnameable_types`, `unused_qualifications`, `redundant_imports`, `elided_lifetimes_in_paths` | A destructor hidden by `let _ =`, a cast a coercion would do, an unnameable `pub` type, a path or import that says nothing, a hidden lifetime (`Formatter<'_>`) |
| Production panics | `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `unreachable`, `exit`, `indexing_slicing`, `string_slice`, `as_conversions` | A crash, or a silently wrong number, where an error belongs |
| Lost errors | `map_err_ignore`, `let_underscore_must_use`, `unused_result_ok` | An error dropped without a word |
| TUI and determinism | `print_stdout`, `print_stderr`, `dbg_macro`, `use_debug`, `iter_over_hash_type`, `filetype_is_file` | Output that tears the TUI screen (or corrupts MCP's stdout protocol), `{:?}` shown to a user, hash-map order, `is_file()` misreading devices and pipes |
| Unsafe hygiene | `undocumented_unsafe_blocks` (`deny`), `multiple_unsafe_ops_per_block`, `unnecessary_safety_comment`, `unnecessary_safety_doc`, `allow_attributes_without_reason` | One unsafe operation per documented block; no stale SAFETY notes; every `#[allow]` and `#[expect]` says why |
| Guards | `mem_forget`, `rc_buffer`, `rc_mutex`, `clone_on_ref_ptr`, `mutex_atomic`, `mutex_integer`, `lossy_float_literal`, `float_cmp_const`, `empty_drop`, `verbose_file_reads`, `str_to_string`, `string_add`, `infinite_loop`, `large_include_file`, `get_unwrap`, `try_err`, `rest_pat_in_fully_bound_structs`, `mod_module_files`, `needless_raw_strings`, `unnecessary_self_imports`, `redundant_type_annotations`, `deref_by_slicing` | Leaks, shared-state and float traps, wasted allocations; `name.rs` over `name/mod.rs` |
| Cargo | `wildcard_dependencies`, `negative_feature_names`, `redundant_feature_names` | `*` versions, `no-`/`with-`/`use-` feature names |

`mcp/` forbids `unsafe_code`, which already rejects every unsafe block and
unsafe attribute, so it leaves out `undocumented_unsafe_blocks`,
`multiple_unsafe_ops_per_block` and `unsafe_attr_outside_unsafe`.
Two lints are deliberately not in the set. Clippy removed
`clippy::string_to_string`; the pedantic `implicit_clone` covers it.
`clippy::unwrap_in_result` has no test exemption, so it would reject the
`.expect()` in a `#[test]` returning `Result` that `allow-expect-in-tests`
admits, and in production it fires only where `unwrap_used`/`expect_used`
already do.

- **Fix the code** and keep its behaviour: handle or return the error, use
  `get()`/`first()`, slice on char boundaries, convert with `TryFrom`, write an
  explicit `drop()`, use `to_owned()`.
- **Suppress only code that is right as written** (a proven invariant, a cast a
  `const fn` requires, the CLI's `--help` on stdout) with
  `#[expect(clippy::<lint>, reason = "<why this is correct>")]` on the smallest
  item. Never a module- or crate-wide `allow`, never one without a reason.
- **Tests** may unwrap, expect, index, panic, print and `dbg!`: `clippy.toml`
  sets `allow-{unwrap,expect,panic,indexing-slicing,print,dbg}-in-tests`, so
  the panic family judges production code only.

`scripts/test_lint_policy.py` (run by `./ffm.py lint self-test`) fails when a
manifest drifts from this policy, and proves each family live: throwaway
crates under `target/` copy each manifest's `[lints]` tables, Clippy must
reject one negative fixture per family (`scripts/tests/fixtures/lint_policy/`)
with exactly its codes, and must accept test code that uses the allowances.

## ⚙️ Important Environment Details

- `rust-toolchain.toml` selects stable Rust with rustfmt and Clippy for all
  packages, matching CI. The manifests retain Rust 1.95 as the minimum version.
- The developer scripts (`ffm.py`, `install_mcp.py`,
  `create_desktop_shortcut.py`, `scripts/*.py`) use only the Python standard
  library and need Python 3.9 or newer, the `python3` that macOS's Command Line
  Tools ship; `scripts/tests/test_python_version.py` parses each one with the
  3.9 grammar. `ffm.py`, `create_desktop_shortcut.py` and the scripts in
  `scripts/` find programs on `PATH` as bash does, expanding a leading `~` in
  an entry (`scripts/shell_path.py`).
- Cargo's job count is intentionally unset, so local builds use every logical
  CPU by default. Override it with `cargo build -j N` or `CARGO_BUILD_JOBS` on a
  memory-constrained machine.
- This project leaves compiler caching opt-in. `.cargo/config.toml` sets an
  empty `rustc-wrapper`, which switches off a wrapper forced by a user or
  managed Cargo configuration: a shared sccache server that stalls keeps every
  build waiting for minutes with no `rustc` running. Developers enable a cache
  per shell with `RUSTC_WRAPPER=sccache`, which outranks the file. Do not set
  `SCCACHE_NO_DAEMON`: a foreground server inherits Cargo's output pipes and the
  build never finishes. `cargo llvm-cov` cannot run an empty wrapper, so
  coverage runs set `CARGO_BUILD_RUSTC_WRAPPER=/usr/bin/env`, as CI does;
  `ffm.cmd coverage` uses Git for Windows' `env.exe` the same way.
  `scripts/cargo_preflight.py` (run by `./ffm.py` and `scripts/lint.py`) asks
  the selected wrapper for `rustc -vV` and fails within
  `FFM_WRAPPER_PROBE_SECS` (default 20) seconds instead of hanging; it times
  Cargo itself, so it needs no GNU `timeout`. Cargo
  incremental compilation remains enabled for both development and test
  profiles, unless a user config sets `build.incremental = false`, which
  overrides every profile.
- Plain `cargo build` and `cargo run` compile project code without optimization
  or LTO, with incremental compilation and 256 codegen units. Debug information
  is disabled and stripped; dependencies use optimization level 2. These
  settings avoid full application optimization and cross-crate linking work
  during routine edits. Enable debug information explicitly if a debugger
  needs source-level symbols.
- `cargo build --release` keeps optimization level 3, fat LTO, a single
  codegen unit, stripped symbols, and aborting panics for production binaries.
  It trades build time for runtime optimization. The root crate also provides
  `cargo build --profile release-fast`, which uses thin LTO and 16 codegen units
  for an intermediate option.
- The root and `mcp/` packages use aligned development and test profiles. MCP
  retains Cargo's release defaults. Both packages, including `colorer-sys`,
  share the root `target/` directory through `.cargo/config.toml`; dependencies
  can reuse artifacts when versions, features, and profiles match. Their
  lockfiles stay independent. The application's `Cargo.lock` is not tracked:
  the first `cargo` command generates it with the newest versions the
  manifest allows, so two fresh checkouts can resolve different versions and
  commands for the root crate cannot pass `--locked`. `mcp/Cargo.lock` is
  tracked. Root-crate commands do not build or test MCP automatically.
- Windows uses `lld-link`; Linux uses `mold`. These linker settings live in
  `.cargo/config.toml` and apply without changing the commands below.
- Many integration and UI tests are gated behind the `test-helpers` feature.
- BDD tests (`tests/bdd.rs` and `tests/bdd_ru.rs`) use `harness = false` (Cucumber-based) and require both `test-helpers` and `s3` features.

## 🧪 Coverage Expectations

- High coverage is expected on new logic.
- Some files are intentionally excluded from coverage targets (`src/main.rs`, parts of `src/editor.rs`) because they are difficult or impossible to test meaningfully in an automated way.
- CI enforces 98% measured line coverage for the TUI coverage job and 75% for
  the MCP package. Exclusions and feature sets are explicit in
  `.github/workflows/tests.yml`; these are not whole-repository coverage claims.

## 🧪 Mutation Testing

The project uses `cargo-mutants`, configured via `.cargo/mutants.toml` (the
only path the tool reads; it selects `cargo-nextest` as the test runner).

- Tests frequently contain comments like `// Kill: replace X with Y`.
- Running mutation testing is encouraged on changed code.
- The goal is very few surviving mutants on core logic.
- CI runs `cargo mutants --in-diff` on every pull request for the application
  and `mcp/`; a missed mutant in changed code fails that job. Evidence is the
  `mutants.out` artifact, not a test named after a mutant.

## ✅ Recommended Daily Workflow

```bash
cargo fmt -- --check
cargo test
cargo test --features test-helpers
./ffm.py lint
```

For deeper validation:
- Run mutation testing on modified modules.
- Generate coverage report periodically.

**Even better:** Install the pre-commit hook (see git-and-commits.md). It will run `cargo fmt` automatically on every `git commit`.

## 📝 Notes for AI Agents

When making code changes, always verify with at least:
- `cargo fmt`
- `./ffm.py lint`
- Relevant tests (preferably with `--features test-helpers`)

If the pre-commit hook is installed in the environment, formatting will be applied automatically before the commit lands.
