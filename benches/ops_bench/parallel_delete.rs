//! Purpose: Benchmark parallel directory deletion on branching and deep trees
//! against `std::fs::remove_dir_all` and with active streaming progress.

use super::delete_progress::{confirmed_app, NoopPainter};
use super::{branching_project_tree, source_tree, Shape};
use criterion::{measurement::WallTime, BatchSize, BenchmarkGroup, Criterion};
use funny_file_manager::ops::{handle_delete, handle_delete_with_cancel};
use funny_file_manager::progress;
use std::fs;
use std::hint::black_box;
use std::time::Duration;

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
fn bench_std_vs_parallel(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("std_remove_dir_all_branching", |b| {
        b.iter_batched(
            branching_project_tree,
            |root| {
                let source = root.path().join("tree");
                fs::remove_dir_all(black_box(&source)).expect("std remove_dir_all");
                assert!(!source.exists(), "remove_dir_all left its source tree");
                black_box(root)
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function("ffm_handle_delete_branching", |b| {
        b.iter_batched(
            || confirmed_app(branching_project_tree()),
            |(root, mut app, source)| {
                handle_delete(black_box(&mut app)).expect("parallel delete");
                assert!(!source.exists(), "delete left its source tree");
                black_box((root, app))
            },
            BatchSize::PerIteration,
        );
    });
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
fn bench_deep_and_active_dialog(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("ffm_handle_delete_deep_tree", |b| {
        b.iter_batched(
            || confirmed_app(source_tree(Shape::DeepTree)),
            |(root, mut app, source)| {
                handle_delete(black_box(&mut app)).expect("deep delete");
                assert!(!source.exists(), "deep delete left its source tree");
                black_box((root, app))
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function("ffm_delete_active_dialog_and_cancel", |b| {
        b.iter_batched(
            || confirmed_app(branching_project_tree()),
            |(root, mut app, source)| {
                let _installed = progress::install_with(Box::new(NoopPainter), Duration::ZERO);
                handle_delete_with_cancel(black_box(&mut app), &mut |shown| {
                    black_box(shown);
                    black_box(false)
                })
                .expect("active dialog delete");
                assert!(!source.exists(), "active dialog delete left its source");
                black_box((root, app))
            },
            BatchSize::PerIteration,
        );
    });
}

pub fn bench_parallel_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_delete");
    group.sample_size(20);
    let _installed = progress::install(Box::new(NoopPainter));
    bench_std_vs_parallel(&mut group);
    bench_deep_and_active_dialog(&mut group);
    group.finish();
}
