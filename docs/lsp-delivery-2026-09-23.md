<!-- Purpose: Preserve the LSP delivery, verification evidence, and maintenance handoff in Git. -->

# Editor LSP delivery: 23 September 2026

The editor LSP implementation and automatic installation are merged into `main`.
The user guide is [editor-lsp.md](editor-lsp.md); it documents configuration,
shortcuts, supported servers, installation recipes and troubleshooting.

## Delivery references

| Item | Reference |
|---|---|
| Feature branch | `feature/lsp-autoinstall` |
| Feature commit | `99a8e5e` — automatic installation and inventory |
| Portability fix | `bf87bab` — installer process ownership |
| Review fixes | `0c57f62` — installation validation and document identity |
| Tested PR head | `5e95a835a7965cd43335d7b5a45477a2eaf4ed38` |
| Pull request | [PR #9](https://github.com/stden/FunnyFileManager/pull/9), squash merged |
| Main delivery commit | `fbce01fd664357400b9f3f459a665ecf8ae710c1` |
| PR CI | [35858114526](https://github.com/stden/FunnyFileManager/actions/runs/35858114526) — passed |
| Main CI | [35859687682](https://github.com/stden/FunnyFileManager/actions/runs/35859687682) — passed on the exact delivery commit |

The existing `feature/editor-lsp` changes were already represented in `main`;
`git cherry` found no unapplied patches from that branch. Other-machine updates
to documentation, menu commands, search and S3 were preserved during integration.
No force push was used. The repository permits squash/rebase PR merges.

## Available behavior

| Feature | Result and verification |
|---|---|
| Diagnostics | Severity, range and message; deterministic tests and real Java/Python smoke |
| Completion | Ctrl+Space, popup and edits; stale replies rejected; real smoke passed |
| Hover | Ctrl+K, plain text/Markdown/code fences; real smoke passed |
| Definition | Ctrl+], single/multiple targets and cross-file navigation; real smoke passed |
| Document synchronization | Open/change/save/close, versions, negotiated full/incremental changes; tested |
| Unicode positions | UTF-8/editor offsets to UTF-16 positions; Unicode and multiline tests |
| Protocol | Content-Length framing, partial reads, request IDs, JSON-RPC errors and cancellation; tested |
| Lifecycle | Background initialization, bounded shutdown and owned process-tree cleanup; tested |
| Automatic installation | Trusted-workspace approval, background job, cancellation, atomic cache publication |
| Inventory | F9 → Commands → Language servers (LSP), or the `lsp` command |

The inventory lists languages, extensions, enabled state, availability, configured
commands, resolved executables, arguments and recipes. It does not start servers
or install packages. `lsp` is retained in durable command history.

The catalog includes Python, Java, JavaScript/JSX/TypeScript/TSX, HTML, CSS, JSON,
YAML and Bash. Rust uses the configured rust-analyzer executable or PATH.
Additional servers can be configured with a recipe; there is no automatic recipe
generation for arbitrary languages. Java and Python were also installed into the
acceptance machine's user-local FFM cache. Those installations are machine-local,
not repository artifacts.

## Configuration and ownership

- Windows settings: `%APPDATA%\ffm\settings.yaml`, under the `lsp` key. (Delivered
  as `settings.json`; the YAML switch reads that file until the first save.)
- Cache: `ffm/lsp/<recipe hash>` beside settings; Python uses an isolated venv,
  npm a private prefix, and Java separate configuration/index data per workspace.
- Catalog: `src/config/lsp/catalog.json`; omitted fields retain catalog defaults.
- `enabled: false` disables a server; `install: null` disables its installation.
- Existing configured executables take precedence; only NotFound triggers installation.
- Java requires JDK 21+ and curl 8.4+; JAVA_HOME takes precedence over PATH.
- Python requires Python 3 with venv/pip; npm recipes require Node.js and npm.
- FFM does not install runtimes or modify global PATH.
- Restart with Ctrl+Alt+R after correcting configuration or an installation failure.

```text
Editor: buffer, revision, caret, diagnostics and popup
  ↕ EditorLspController on the UI thread
  ↕ bounded channels
LspManager worker: trust, lifecycle, document updates and requests
  ├─ installer thread: package-manager process tree and cache publication
  └─ LspClient: JSON-RPC IDs, capabilities and document versions
       ↕ stdio reader/writer tasks; stderr drained separately
       rust-analyzer / pylsp / JDT LS / another configured server
```

No additional async runtime was introduced. Protocol/process code lives in
`src/lsp/`; installation code in `src/lsp/install/`; editor effects in
`src/ops/editor_lsp.rs`. Rendering reads state and does not perform installation.
Typing updates are batched after 120 ms; explicit requests flush pending changes.

The worker preserves canonical OS-native document paths across edits while trust,
installation and initialization are pending. This fixes Windows short-path aliases
that previously caused Java diagnostics to be ignored and avoids corrupting
non-UTF-8 Unix filenames through display formatting.

## Review resolutions

All 18 findings on PR #9 were addressed before merge. Besides document identity,
the fixes cover executable permission checks, rejecting directories posing as
Java launcher jars, refreshing Node/Java discovery when reading cached receipts,
and matching inventory workspace/relative-command resolution to actual startup.

Installer tests now synchronize with lock contention and with a running child
before cancellation. They check resource release, retry the same failed recipe,
reject a traversal member in a real tar stream, and reject excessive output even
when the producer exits quickly. Curl 8.4+ is required to enforce download bounds
for responses without a Content-Length. Assertions use typed command variants
and verify inventory headings; documentation paths include their `src/` prefix.

## Verification evidence

| Check | Observed result |
|---|---|
| Local `cargo test` before final merge | 4,021 passed, 10 ignored |
| Local `cargo test` after merge, before saving this report | 4,025 passed, 10 ignored |
| Local shared `./ffm.sh lint` | Passed: feature combinations, MCP, Colorer, architecture and readability guards |
| PR Linux coverage suite | 4,718 passed, 10 skipped; 98.04% line coverage |
| Merged-main Linux coverage suite | 4,722 passed, 10 skipped; 98.02% line coverage |
| Local Windows coverage suite | 4,600 passed, 10 skipped; 96.9066% line coverage |
| Local cleanup coverage | 299/312 lines, 95.8333%; unchanged |
| Real Python smoke | python-lsp-server 1.15.0; passed, final run 41.84 s |
| Real Java smoke | JDT LS 1.61.0 with Java 21; passed, final run 24.47 s |
| Remaining Java/Python LSP processes after smoke | None observed |
| `git diff --check` | Passed |

The real smokes used temporary projects and verified installation, initialization,
diagnostics, completion, hover, local definition resolution, diagnostics after an
edit, and shutdown. Their durations include installation/download work and are not
interactive latency thresholds. Deterministic tests use fake package managers and
a tiny stdio peer; normal tests do not download language servers.

Windows nextest reported one inherited-pipe leak in the existing
`ops::shell::tests::test_run_shell_command_times_out_instead_of_hanging` test.
Its isolated rerun passed without the signal. The LSP cleanup/cancellation tests
passed; the observation was not hidden or treated as a new LSP failure.
Coverage thresholds were not reduced.

Reproduce the canonical checks with:

```sh
cargo fmt --all -- --check
cargo test
./ffm.py lint
cargo nextest run --features test-helpers
git diff --check
```

Optional real acceptance tests (require their runtimes and network for installation):

```sh
cargo test --lib real_rust_analyzer_supports_the_complete_editor_protocol -- --ignored --nocapture
cargo test --lib real_python_package -- --ignored --nocapture
cargo test --lib real_java_package -- --ignored --nocapture
```

The CI workflow `.github/workflows/tests.yml` is the canonical coverage command
and exclusion configuration. An empty Cargo rustc-wrapper causes cargo-llvm-cov
to attempt an invalid executable path. CI uses `/usr/bin/env`; the Windows local
run used a native argument-preserving compiler pass-through. Git Bash's env.exe
did not preserve quoted rustc arguments in that local coverage run. The temporary
wrapper and generated logs/reports stayed under ignored `target/`.

## Confirmed limits

One active editor/server session; explicit completion triggers; UTF-16 positions.
No snippet expansion, completion-item resolve, formatting, rename, code actions
or server-requested edits. Unversioned diagnostics cannot always be identified as
stale. Other npm catalog servers share the tested installation adapter but were
not individually exercised with a real server in this delivery. Runtime upgrades,
global PATH changes and automatic server upgrades are outside this feature.

See the user guide for trust behavior, bounded timeouts, manual retry, cache repair,
logging and instructions for adding another server. Debug logs do not contain
complete source files or captured server stderr text.
