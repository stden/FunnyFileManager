# Editor cursor redraw measurements

Run `cargo bench --bench editor_cursor_bench -- --noplot`.
If another FFM instance holds a build output open on Windows, use a separate
Cargo target directory, for example `--target-dir target/verification`.

The benchmark warms the lexical cache, then alternates horizontal cursor motion
at the last line and renders the entire UI into a 120×40 `TestBackend`. Fixtures
and the initial highlighting pass are outside measurement. It uses 30 samples,
a one-second warm-up and a two-second measurement target.

Windows, Rust 1.98.1, optimized bench profile, 2026-09-07:

| Document lines | Central estimate | Criterion interval |
| -------------: | ---------------: | -----------------: |
|            100 |           628 µs |         600–657 µs |
|         10,000 |          1.02 ms |       0.94–1.11 ms |
|        100,000 |          2.11 ms |       1.95–2.32 ms |

Previously every editor frame reconstructed the file highlighter and tokenized
from the beginning through the visible window. The regression test observed two
file passes for two cursor-only frames; the cache reduces that to one.

These measurements include in-memory terminal drawing, not physical console
latency. They establish a baseline rather than a before/after speedup ratio.
Cache validation still compares the source text with its snapshot, and edits
invalidate lexical state. Cursor redraw therefore retains work proportional to
document size, plus rendering the visible cells. The cache also uses memory for
the source snapshot and token ranges.
