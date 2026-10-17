//! Purpose: Editor cost on large Java and Python files.
//!
//! Highlighting walks the buffer from the first line, so a frame with the caret
//! at the bottom colours the whole file. A completion request copies that same
//! text out before any language server sees it; no server is running here.

use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;

#[path = "editor_languages/app.rs"]
mod app;
#[path = "editor_languages/completion.rs"]
mod completion;
#[path = "editor_languages/frames.rs"]
mod frames;
#[path = "editor_languages/highlight.rs"]
mod highlight;
#[path = "editor_languages/sources.rs"]
mod sources;

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(400))
        .measurement_time(Duration::from_secs(2));
    targets = highlight::bench,
        frames::bench_cold,
        frames::bench_warm,
        frames::bench_type,
        completion::bench_completion
}
criterion_main!(benches);
