//! Standalone queue benchmark; no real Git repositories or filesystem reads.
//!
//! Run from the repository root:
//! `rustc --edition=2021 -O -A dead_code scripts/bench_git_queue.rs -o /tmp/ffm-bench-git-queue`
//! `/tmp/ffm-bench-git-queue`

use std::path::PathBuf;
use std::time::Instant;

#[path = "../src/models/file_name.rs"]
mod file_name;
mod models {
    pub use crate::file_name::FileName;
}
#[path = "../src/application/git_jobs/pool.rs"]
mod pool;

fn measure(size: usize, panels: usize) {
    let parent = PathBuf::from("/synthetic");
    let names: Vec<_> = (0..size)
        .map(|i| models::FileName::new(format!("repo-{i:05}")))
        .collect();
    let mut samples = Vec::new();
    for _ in 0..5 {
        let left = Some((parent.clone(), names.clone()));
        let right = (panels == 2).then(|| (parent.clone(), names.clone()));
        let pool = pool::Pool::new();
        let started = Instant::now();
        let demand = pool.update_queue([&parent, &parent], [left, right]);
        samples.push(started.elapsed().as_secs_f64() * 1_000.0);
        assert_eq!(std::hint::black_box(demand), size);
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "{size:>5} rows, {panels} panel(s): median {:>9.3} ms",
        samples[2]
    );
}

fn measure_small_batch(size: usize, batch_size: usize, duplicate: bool) {
    let parent = PathBuf::from("/synthetic");
    let names: Vec<_> = (0..size)
        .map(|i| models::FileName::new(format!("repo-{i:05}")))
        .collect();
    let candidates: Vec<_> = (0..batch_size)
        .map(|i| {
            if duplicate {
                models::FileName::new(format!("repo-{:05}", size - batch_size + i))
            } else {
                models::FileName::new(format!("new-{i:05}"))
            }
        })
        .collect();
    let mut samples = Vec::new();
    for _ in 0..5 {
        let pool = pool::Pool::new();
        pool.update_queue(
            [&parent, &parent],
            [Some((parent.clone(), names.clone())), None],
        );
        let batch = Some((parent.clone(), candidates.clone()));
        let started = Instant::now();
        let demand = pool.update_queue([&parent, &parent], [batch, None]);
        samples.push(started.elapsed().as_secs_f64() * 1_000.0);
        assert_eq!(
            std::hint::black_box(demand),
            size + if duplicate { 0 } else { batch_size }
        );
    }
    samples.sort_by(f64::total_cmp);
    let case = if duplicate { "duplicate" } else { "unique" };
    println!(
        "{size:>5} queued, {batch_size} {case:>9}: median {:>9.3} ms",
        samples[2]
    );
}

fn main() {
    for size in [100, 1_000, 3_000, 10_000] {
        for panels in [1, 2] {
            measure(size, panels);
        }
    }
    for size in [0, 1, 1_000, 10_000] {
        measure_small_batch(size, 1, false);
        if size > 0 {
            measure_small_batch(size, 1, true);
        }
    }
    for batch_size in [2, 8, 9, 12, 13, 16, 32] {
        measure_small_batch(10_000, batch_size, false);
        measure_small_batch(10_000, batch_size, true);
    }
}
