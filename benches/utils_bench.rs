//! Purpose: Benchmarks utility and model hot paths to watch for performance regressions.
//!

use criterion::{criterion_group, criterion_main};

#[path = "utils_bench/models.rs"]
mod models;
#[path = "utils_bench/utils.rs"]
mod utils;
#[cfg(feature = "s3")]
use models::bench_virtual_path;
use models::{bench_app_basics, bench_drive_display, bench_panel_basics, bench_sort_mode};
use utils::{bench_copy_dir_all, bench_format_size, bench_unique_destination};

#[cfg(feature = "s3")]
criterion_group!(
    benches,
    bench_format_size,
    bench_unique_destination,
    bench_copy_dir_all,
    bench_sort_mode,
    bench_drive_display,
    bench_virtual_path,
    bench_panel_basics,
    bench_app_basics
);

#[cfg(not(feature = "s3"))]
criterion_group!(
    benches,
    bench_format_size,
    bench_unique_destination,
    bench_copy_dir_all,
    bench_sort_mode,
    bench_drive_display,
    bench_panel_basics,
    bench_app_basics
);

criterion_main!(benches);
