//! Completion latency: full requests, directory scaling, and isolated ranking.
//! Fixtures and PATH cache warm-up stay outside measured iterations.

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use funny_file_manager::complete::{complete_with, execs, matcher, CompletionContext};
use std::{fs, hint::black_box, time::Duration};
use tempfile::TempDir;

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
fn bench_commands(c: &mut Criterion) {
    let root = TempDir::new().expect("completion directory");
    let bookmarks = [Some(root.path().join("workspace")), None];
    let context = CompletionContext {
        cwd: root.path(),
        bookmarks: &bookmarks,
    };
    black_box(execs::executables());
    let mut group = c.benchmark_group("autocomplete/commands_warm_path");
    for input in ["", "c", "srt", "sort e", "bookmark go ", "mark a"] {
        assert!(complete_with(input, &context).is_some(), "{input}");
        group.bench_with_input(BenchmarkId::from_parameter(input), &input, |b, input| {
            b.iter(|| black_box(complete_with(black_box(input), black_box(&context))));
        });
    }
    group.finish();
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
fn directory_fixture(count: usize) -> TempDir {
    let root = TempDir::new().expect("path completion fixture");
    for index in 0..count {
        fs::create_dir(root.path().join(format!("project folder-{index:05}")))
            .expect("candidate directory");
    }
    root
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
fn bench_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("autocomplete/paths_builtin");
    group.sample_size(20);
    for count in [10, 100, 1_000] {
        let root = directory_fixture(count);
        let context = CompletionContext::for_cwd(root.path());
        // Whitespace uses the production built-in-only path, without spawning fish.
        let input = "cd project f";
        assert_eq!(
            complete_with(input, &context)
                .expect("paths")
                .candidates
                .len(),
            count
        );
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| black_box(complete_with(black_box(input), black_box(&context))));
        });
    }
    group.finish();
}

fn bench_ranking(c: &mut Criterion) {
    let mut group = c.benchmark_group("autocomplete/ranking");
    for count in [100, 1_000, 10_000] {
        let candidates: Vec<String> = (0..count)
            .map(|index| format!("project-source-{index:05}.rs"))
            .collect();
        for query in ["project", "psr", "missing"] {
            group.bench_with_input(BenchmarkId::new(query, count), &query, |b, query| {
                b.iter_batched(
                    || candidates.clone(),
                    |pool| black_box(matcher::rank(black_box(query), black_box(pool))),
                    BatchSize::SmallInput,
                );
            });
        }
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    targets = bench_commands, bench_paths, bench_ranking
}
criterion_main!(benches);
