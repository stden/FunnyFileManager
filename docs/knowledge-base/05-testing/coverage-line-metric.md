<!-- Purpose: Defines the source-line metric and records the LLVM generic-instantiation regression. -->

# Coverage counts physical source lines

The gate requires every executable production source line to run. Rust uses
the native LLVM LCOV `DA:<line>,<count>` records: each source filename and line
appears once, and a positive execution count covers that line. Python keeps
coverage.py's native JSON statement counts. Both require exactly 100%; empty,
malformed or duplicate line records fail.

LLVM's JSON file summaries describe grouped function coverage. Their merge
takes the maximum number of covered lines from each generic instantiation,
then adds the function groups. This can differ from the union of physical
source lines that ran. See LLVM's [instantiation summary merge](https://github.com/llvm/llvm-project/blob/main/llvm/tools/llvm-cov/CoverageSummaryInfo.cpp#L105-L127)
and [line-count merge](https://github.com/llvm/llvm-project/blob/main/llvm/tools/llvm-cov/CoverageSummaryInfo.h#L78-L81).

LCOV's individual `DA` records come from the file's line-coverage iterator.
Its `LF` and `LH` footer fields reuse the grouped summary, so those footers
are validated as numeric metadata and do not decide the gate. LLVM's
[LCOV exporter](https://github.com/llvm/llvm-project/blob/main/llvm/tools/llvm-cov/CoverageExporterLcov.cpp#L227-L246)
shows both paths.

The fixtures in `scripts/tests/fixtures/coverage/` preserve an independently
compiled Rust repro from LLVM 23.1.1. `choose::<u8>(true)` executes one arm;
`choose::<u16>(false)` executes the other. Every source line runs across the
two instantiations:

| Fixture            | Grouped JSON/LF/LH counts | Native DA counts | Gate          |
| ------------------ | ------------------------- | ---------------- | ------------- |
| `complementary.rs` | 8/9                       | 9/9              | Pass          |
| `unexecuted.rs`    | 10/12                     | 11/12            | Fail: line 13 |

The second fixture adds a genuine unexecuted line, which remains zero in the
native trace. The stored LCOV fixtures keep the original counters and normalize
only their `SF` paths to `/audit/`. `scripts/tests/test_coverage_lcov.py` checks
both outcomes, including the exact missing line. This corrects the metric
without running unused generic callbacks or changing production behavior.

The Rust collector preserves `rust-*.json` for diagnostics and exports
`rust-*.lcov` from the same profiles after each configuration's final tests.
MCP first runs its complete suite, including its Cucumber harness, then runs
the ignored `application_sessions` target with `--no-clean`. The gate uses
that combined profile. No tests are repeated to produce the LCOV export.
