//! Purpose: directory work — re-reading a listing per sort order, and measuring a tree.

use super::{entry, sample_tree};
use criterion::{BenchmarkId, Criterion};
use funny_file_manager::models::SortMode;
use std::fs;
use std::hint::black_box;
use tempfile::TempDir;

/// Re-reading a directory, once per sort order.
///
/// This is what Ctrl+R costs, and what entering a directory costs. Real files,
/// because the work is `read_dir` and an `lstat` each as much as it is the
/// comparison — measuring the sort alone would measure the half that is
/// already fast.
///
/// The three orders are measured against each other: `Name` and `Extension`
/// build a key per entry and cache it, `Size` compares numbers and falls back
/// to the name. If one of them is much worse than the others on a large
/// directory, this is where it shows.
pub fn bench_sort_modes(c: &mut Criterion) {
    // Ten thousand real files: enough for the ordering to dominate, few enough
    // that creating them once does not dominate the benchmark's own runtime.
    const FILES: usize = 10_000;

    let mut group = c.benchmark_group("reload_by_sort");

    let dir = TempDir::new().expect("sort benchmark dir");
    for index in 0..FILES {
        let name = entry(index).name;
        fs::write(dir.path().join(name), b"x").expect("seed sort benchmark");
    }

    for mode in [SortMode::Name, SortMode::Extension, SortMode::Size] {
        let mut panel =
            funny_file_manager::models::Panel::new(dir.path().to_path_buf()).expect("panel");
        panel.sort_mode = mode;
        group.bench_function(BenchmarkId::from_parameter(mode.label()), |b| {
            b.iter(|| panel.load_entries().expect("reload"));
        });
    }

    group.finish();
}

/// Measuring a directory, which Ctrl+L does on demand.
pub fn bench_stats(c: &mut Criterion) {
    let tree = sample_tree();
    c.bench_function("stats/analyse", |b| {
        b.iter(|| funny_file_manager::stats::analyse(black_box(tree.path())));
    });
}
