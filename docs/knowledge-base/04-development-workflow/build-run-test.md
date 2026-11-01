<!-- Purpose: Lists the commands used to build, run, and test the project. -->

# 🏗️ Build, Run, Test, and Coverage

## 💬 Essential Commands

| Command                                      | Purpose                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cargo run` (or `cargo run --bin ffm`)       | Run locally using the incremental development profile                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `cargo build`                                | Build the development binary for fast iteration                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `cargo build --release`                      | Optimized release binary                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `cargo test`                                 | Run unit + basic tests                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `cargo test --features test-helpers`         | Include helper-gated integration tests                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `cargo test -- --ignored`                    | Run tests marked `#[ignore]`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `cargo fmt`                                  | Format all code                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `cargo clippy -- -D warnings`                | Lint (must be clean)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `./scripts/lint.py` / `./ffm.py lint`        | Shared local/CI format, spelling (`typos`, `cargo install typos-cli`) and Clippy gate for TUI, MCP, and Colorer                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `./ffm.py lint spelling`                     | Spell check only; project words live in `typos.toml`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `./ffm.py lint markdown`                     | Markdown tables and trailing spaces (`scripts/format_markdown.py`, standard library only). Every tracked `*.md` file is checked, not rewritten: each GFM table's columns are aligned (the delimiter row as wide as its column, with the colons of its alignment kept), trailing spaces outside code are removed, and the file ends with exactly one newline. Fenced code blocks are left alone. `./ffm.py lint fix` rewrites the files instead. In both, a table row whose cell count is not its header's fails the gate, named by file and line (escape a pipe that belongs in a cell as `\|`). The AI-DLC framework's files under `.claude/` and `aidlc/` are not formatted |
| `./ffm.py lint complexity`                   | The Python scripts' complexity budget (`scripts/lint_complexity.py`, standard library only): McCabe cyclomatic complexity at most 10 and blocks nested at most 4 deep per function. The Rust gets the same from Clippy (`clippy.toml`: cognitive complexity 25, nesting 4, 30 lines per function)                                                                                                                                                                                                                                                                                                                                                                             |
| `./ffm.py lint duplication`                  | Copied code (`scripts/lint_duplication.py`, standard library only): no run of 10+ significant lines — indentation, blank lines, comments and bare punctuation left out — may appear twice in the production Rust or the scripts. Tests are left out; a copy that cannot be shared goes in its `ALLOWED` list with the reason                                                                                                                                                                                                                                                                                                                                                  |
| `./ffm.py lint self-test`                    | Prove the spell check rejects a misspelling; run the developer scripts' unit tests (`scripts/tests`) and the Markdown formatter's (`scripts/test_format_markdown.py`); compile positive/negative readability fixtures and compare behavior; prove the manifest lint policy on positive/negative fixtures; requires Python                                                                                                                                                                                                                                                                                                                                                     |
| `cargo build --manifest-path mcp/Cargo.toml` | Build the standalone MCP server with its development profile                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `cargo test --manifest-path mcp/Cargo.toml`  | Run the MCP package's separate test suite                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `./ffm.py coverage`                          | CI's line-coverage gate: the root suite under `cargo llvm-cov nextest`, failing under 98% of the measured lines (`scripts/coverage.py`; needs `cargo-llvm-cov`, `cargo-nextest` and `rustup component add llvm-tools`)                                                                                                                                                                                                                                                                                                                                                                                                                                                        |

The Linux coverage job also enables a real SFTP integration test with
`FFM_TEST_SFTP_SERVER=1`. It needs `ssh-keygen`, `/usr/sbin/sshd` from
OpenSSH Server, an unlocked current account, and the OpenSSH privilege
separation directory `/run/sshd`. The test starts its own sshd on loopback,
uses disposable keys and a temporary SSH home, then stops it; it never changes
the system daemon or the user's SSH configuration. Run the same test locally:

```sh
FFM_TEST_SFTP_SERVER=1 cargo test --lib remote::sftp::tests_integration::real_sftp_round_trip
FFM_TEST_SFTP_SERVER=1 ./ffm.py coverage
```

With the variable unset the test returns without starting a server; when
enabled, missing tools or a server that cannot start fail the test. The CI job
checks the runner's OpenSSH Server installation and explicitly enables this check.

The shared lint gate checks formatting in the root and MCP packages, then
runs Clippy with `--all-targets -- -D warnings` for the root's default features,
all features, and `--no-default-features --features test-helpers`. It also
checks every target in MCP and Colorer. This covers helper-gated tests, feature
interactions, and the built-in highlighter without the optional Colorer adapter.
Its last step runs the architecture contracts (module boundaries, the unsafe
allowlist, file limits). CI's lint job calls the same script, then `self-test`.
The self-test can also run directly as
`python scripts/test_readability.py` (the compilers, without Cargo) and
`python scripts/test_lint_policy.py` (see the lint policy below).

## 🛡️ Lint Policy

Each crate declares its own `[lints]` tables (`Cargo.toml`, `mcp/Cargo.toml`,
`colorer-sys/Cargo.toml`); the root `clippy.toml` holds the shared thresholds
and test allowances. On top of Clippy's `all`, `pedantic` and `nursery` groups,
every crate enables these reliability families at `warn`, which the gate's
`-D warnings` makes errors:

| Family              | Lints                                                                                                                                                                                                                                                                                                                                                                                                                                | What it stops                                                                                                                                                   |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust guards         | `unused_lifetimes`, `single_use_lifetimes`, `redundant_lifetimes`, `explicit_outlives_requirements`, `unused_macro_rules`, `meta_variable_misuse`, `unused_import_braces`, `unused_extern_crates`, `macro_use_extern_crate`, `trivial_numeric_casts`, `unit_bindings`, `keyword_idents_2024`, `non_ascii_idents`, `unsafe_attr_outside_unsafe`                                                                                       | Nothing trips them today; they keep it that way                                                                                                                 |
| Rust idioms         | `let_underscore_drop`, `trivial_casts`, `unnameable_types`, `unused_qualifications`, `redundant_imports`, `elided_lifetimes_in_paths`                                                                                                                                                                                                                                                                                                | A destructor hidden by `let _ =`, a cast a coercion would do, an unnameable `pub` type, a path or import that says nothing, a hidden lifetime (`Formatter<'_>`) |
| Production panics   | `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `unreachable`, `exit`, `indexing_slicing`, `string_slice`, `as_conversions`                                                                                                                                                                                                                                                                                          | A crash, or a silently wrong number, where an error belongs                                                                                                     |
| Lost errors         | `map_err_ignore`, `let_underscore_must_use`, `unused_result_ok`                                                                                                                                                                                                                                                                                                                                                                      | An error dropped without a word                                                                                                                                 |
| TUI and determinism | `print_stdout`, `print_stderr`, `dbg_macro`, `use_debug`, `iter_over_hash_type`, `filetype_is_file`                                                                                                                                                                                                                                                                                                                                  | Output that tears the TUI screen (or corrupts MCP's stdout protocol), `{:?}` shown to a user, hash-map order, `is_file()` misreading devices and pipes          |
| Unsafe hygiene      | `undocumented_unsafe_blocks` (`deny`), `multiple_unsafe_ops_per_block`, `unnecessary_safety_comment`, `unnecessary_safety_doc`, `allow_attributes_without_reason`                                                                                                                                                                                                                                                                    | One unsafe operation per documented block; no stale SAFETY notes; every `#[allow]` and `#[expect]` says why                                                     |
| Guards              | `mem_forget`, `rc_buffer`, `rc_mutex`, `clone_on_ref_ptr`, `mutex_atomic`, `mutex_integer`, `lossy_float_literal`, `float_cmp_const`, `empty_drop`, `verbose_file_reads`, `str_to_string`, `string_add`, `infinite_loop`, `large_include_file`, `get_unwrap`, `try_err`, `rest_pat_in_fully_bound_structs`, `mod_module_files`, `needless_raw_strings`, `unnecessary_self_imports`, `redundant_type_annotations`, `deref_by_slicing` | Leaks, shared-state and float traps, wasted allocations; `name.rs` over `name/mod.rs`                                                                           |
| Cargo               | `wildcard_dependencies`, `negative_feature_names`, `redundant_feature_names`                                                                                                                                                                                                                                                                                                                                                         | `*` versions, `no-`/`with-`/`use-` feature names                                                                                                                |

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

- `rust-toolchain.toml` pins the exact Rust release (1.99.1) with rustfmt and
  Clippy for all packages; rustup installs it on first use, and every CI job
  installs the same file's release with `rustup toolchain install`. Bump it on
  purpose, in its own commit: a new stable brings new Clippy lints, and the
  gate denies every warning. The manifests retain Rust 1.95 as the minimum
  version. A toolchain outside rustup (such as a plain copy on `PATH`) ignores
  the file.
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
  `./ffm.py coverage` sets `CARGO_BUILD_RUSTC_WRAPPER` to `env` when no
  wrapper is set, as CI does for its llvm-cov jobs; `ffm.cmd coverage` uses
  Git for Windows' `env.exe` the same way.
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
  commands for the root crate cannot pass `--locked`. CI resolves it once per
  run instead: its lockfile job runs `cargo generate-lockfile` for the
  application and `colorer-sys`, and every job that builds them downloads
  that `cargo-lock` artifact, so all jobs of a run test one dependency set.
  To reproduce a red run, put its `cargo-lock` artifact into a checkout of the
  same commit. `mcp/Cargo.lock` is tracked. Root-crate commands do not build
  or test MCP automatically.
- Windows uses `lld-link`; Linux uses `mold`. These linker settings live in
  `.cargo/config.toml` and apply without changing the commands below.
- Many integration and UI tests are gated behind the `test-helpers` feature.
- BDD tests (`tests/bdd.rs` and `tests/bdd_ru.rs`) use `harness = false` (Cucumber-based) and require both `test-helpers` and `s3` features.

## 🧪 Coverage Expectations

- High coverage is expected on new logic.
- Some files are intentionally excluded from coverage targets (`src/main.rs`, parts of `src/editor.rs`) because they are difficult or impossible to test meaningfully in an automated way.
- CI enforces 98% measured line coverage for the root crate and 75% for the
  MCP package; these are not whole-repository coverage claims. The root gate
  is defined once, in `scripts/coverage.py` (features, the files left out of
  the measurement and why, the floor), and both CI and `./ffm.py coverage` run
  it. Its last line gives the share cut to two decimals: llvm-cov's own table
  rounds, and printed 97.998% as "98.00%" in a run that failed the gate. The
  MCP gate is its job's `cargo llvm-cov` step in `.github/workflows/tests.yml`.
- `.config/nextest.toml` fails a test that runs for three minutes, so a hung
  test ends the coverage run (and a mutant's test run) instead of stalling it.
  Its default filter leaves out the `harness = false` cucumber suites `bdd`
  and `bdd_ru`, which nextest cannot list, so `cargo nextest run --features
  test-helpers,s3` runs everything else; `cargo test --test bdd --test bdd_ru`
  runs them.

## 🧪 Mutation Testing

The project uses `cargo-mutants`, configured via `.cargo/mutants.toml` (the
only path the tool reads; it selects `cargo-nextest` as the test runner).

- Tests frequently contain comments like `// Kill: replace X with Y`.
- Running mutation testing is encouraged on changed code.
- The goal is very few surviving mutants on core logic.
- CI runs `cargo mutants --in-diff` on every pull request for the application
  and `mcp/`, one mutant at a time over four shards (about 150 mutants fit the
  60-minute step). `scripts/mutants_verdict.py` judges each step: a missed
  mutant in changed code fails, and so does a run in which every mutant timed
  out, since it tested nothing; a timeout among caught mutants passes with a
  warning. Evidence is the `mutants-out-*` artifacts, not a test named after a
  mutant.

## 🚦 Continuous Integration

`.github/workflows/tests.yml` runs on every push to `main`, on pull requests,
weekly (Monday 04:23 UTC) and on demand (`workflow_dispatch`). A pull request
keeps only its newest run; a run on `main` is never cancelled, so a red job
stays red on the commit that broke it.

| Job                       | Runner                                        | What it runs                                                                                                                                                                                           |
| ------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Resolve Cargo.lock        | ubuntu-24.04                                  | `cargo generate-lockfile` for the application and `colorer-sys`; uploads the `cargo-lock` artifact every job below except MCP downloads                                                                |
| Lint                      | ubuntu-24.04                                  | `./ffm.py lint` (fmt, typos, Markdown tables, the Python complexity budget, the duplication check, the Clippy matrix for all three crates, the architecture contracts), then `./ffm.py lint self-test` |
| Tests & Coverage          | ubuntu-24.04                                  | `./ffm.py coverage`: every root test, the TUI BDD scenarios included, under llvm-cov with the 98% gate; uploads `tui-screens-Linux`                                                                    |
| BDD (s3 feature)          | ubuntu-24.04, windows-2025                    | the `s3` unit tests, then the `bdd` and `bdd_ru` suites                                                                                                                                                |
| Windows                   | windows-2025                                  | the whole root suite once (`cargo test --features test-helpers --no-fail-fast`, TUI BDD included), then Clippy on all targets, for the application and for `mcp/`; uploads `tui-screens-Windows`       |
| MCP Server                | ubuntu-24.04                                  | the MCP tests under llvm-cov with the 75% gate; its fmt and Clippy run in Lint                                                                                                                         |
| Colorer feature           | ubuntu-24.04                                  | builds Colorer-library, then Clippy and the `highlight` tests with the `colorer` feature and real schemes                                                                                              |
| Dependencies (cargo-deny) | ubuntu-24.04                                  | `cargo deny --locked` for the application, MCP and `colorer-sys`                                                                                                                                       |
| Mutation testing          | ubuntu-24.04, four shards, pull requests only | `cargo mutants --in-diff` for the application and MCP, judged by `scripts/mutants_verdict.py`                                                                                                          |

Nothing an unchanged commit depends on floats. Each input is pinned in one
place and bumped on purpose, in its own commit:

- Rust: `rust-toolchain.toml`. Jobs install it with `rustup toolchain install`.
- Tools: the `TOOL_*` entries in the workflow's `env` (typos, cargo-deny,
  cargo-llvm-cov, cargo-nextest, cargo-mutants).
- Colorer: `COLORER_LIBRARY_SHA` and `COLORER_SCHEMES_SHA` in the Colorer job.
- Actions: each `uses:` names a commit SHA, with its release in a comment;
  `.github/dependabot.yml` proposes updates weekly.
- Runner images: `ubuntu-24.04` and `windows-2025`, never `-latest`.
- Crates: the application's lockfile is resolved once per run, not tracked;
  the weekly run shows a crate release that breaks the build.

Every job has a `timeout-minutes`, and `.config/nextest.toml` fails a test
after three minutes. Caches (rust-cache) are saved from `main` only; mutation
testing runs on pull requests only, so its cache is saved there, scoped to
that pull request. The keys include the run's `Cargo.lock`, so they follow
dependency updates. The workflow's token is read-only
(`permissions: contents: read`).

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
