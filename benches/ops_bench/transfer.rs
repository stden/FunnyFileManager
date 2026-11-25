//! Purpose: Copy, move and delete over each tree shape.

use super::{source_tree, Shape};
use criterion::{BatchSize, Criterion};
use funny_file_manager::utils::copy_dir_all;
use std::fs;
use std::hint::black_box;
use tempfile::TempDir;

/// Copying, in each shape.
#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
pub fn bench_copy(c: &mut Criterion) {
    let mut group = c.benchmark_group("copy");
    group.sample_size(20);

    for shape in Shape::ALL {
        let source = source_tree(shape);
        group.bench_function(shape.label(), |b| {
            b.iter_batched(
                || TempDir::new().expect("destination"),
                |dst| {
                    copy_dir_all(
                        black_box(&source.path().join("tree")),
                        black_box(&dst.path().join("copy")),
                    )
                    .expect("copy");
                    dst
                },
                BatchSize::PerIteration,
            );
        });
    }

    group.finish();
}

/// Moving, both ways it can happen.
///
/// Within one filesystem a move is a `rename`: the data does not go anywhere,
/// only a directory entry changes, and it is constant-time whatever the size.
/// Across filesystems `rename` fails with `EXDEV` and there is nothing to do
/// but copy and then delete — the full price, twice over. The gap between
/// these two numbers is the whole reason a file manager must not promise that
/// F6 is quick.
#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
pub fn bench_move(c: &mut Criterion) {
    let mut group = c.benchmark_group("move");
    group.sample_size(20);

    group.bench_function("rename_same_filesystem", |b| {
        b.iter_batched(
            || source_tree(Shape::ManySmall),
            |root| {
                fs::rename(
                    black_box(root.path().join("tree")),
                    black_box(root.path().join("moved")),
                )
                .expect("rename");
                root
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function("copy_then_delete", |b| {
        b.iter_batched(
            || source_tree(Shape::ManySmall),
            |root| {
                // What a cross-filesystem move comes down to.
                copy_dir_all(&root.path().join("tree"), &root.path().join("moved"))
                    .expect("copy half");
                fs::remove_dir_all(root.path().join("tree")).expect("delete half");
                root
            },
            BatchSize::PerIteration,
        );
    });

    group.finish();
}

/// Deleting, in each shape.
///
/// A thousand small files cost a thousand `unlink`s; one large file costs one.
/// Size on disk is close to irrelevant, which is the opposite of what a
/// progress bar counting bytes would suggest.
#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
pub fn bench_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete");
    group.sample_size(20);

    group.bench_function(Shape::ManySmall.label(), |b| {
        b.iter_batched(
            || source_tree(Shape::ManySmall),
            |root| {
                fs::remove_dir_all(black_box(root.path().join("tree"))).expect("delete");
                root
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function(Shape::OneLarge.label(), |b| {
        b.iter_batched(
            || source_tree(Shape::OneLarge),
            |root| {
                fs::remove_dir_all(black_box(root.path().join("tree"))).expect("delete");
                root
            },
            BatchSize::PerIteration,
        );
    });

    group.finish();
}
