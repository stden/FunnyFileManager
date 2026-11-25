//! Purpose: Size formatting, unique destinations and recursive directory copy.

use criterion::{BenchmarkId, Criterion};
use funny_file_manager::models::FileSize;
use funny_file_manager::utils::{copy_dir_all, unique_destination};
use std::ffi::OsStr;
use std::fs;
use std::hint::black_box;
use tempfile::TempDir;

pub fn bench_format_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_size");

    for size in [0_u64, 999, 1_024, 1_536, 1_048_576, 1_073_741_824] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| FileSize::from_bytes(black_box(size)).to_string());
        });
    }

    group.finish();
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
pub fn bench_unique_destination(c: &mut Criterion) {
    let temp = TempDir::new().expect("benchmark temp dir");
    let base = temp.path();
    let free_name = OsStr::new("report.txt");

    for suffix in 0..50 {
        let name = if suffix == 0 {
            "report.txt".to_owned()
        } else {
            format!("report_{suffix}.txt")
        };
        fs::write(base.join(name), b"x").expect("seed benchmark files");
    }

    c.bench_function("unique_destination_with_conflicts", |b| {
        b.iter(|| unique_destination(black_box(base), black_box(free_name)).expect("unique path"));
    });
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
pub fn bench_copy_dir_all(c: &mut Criterion) {
    let temp = TempDir::new().expect("benchmark temp dir");
    let src = temp.path().join("src");
    fs::create_dir_all(src.join("nested/deeper")).expect("create benchmark source tree");

    for index in 0..20 {
        fs::write(src.join(format!("file_{index}.txt")), b"payload").expect("seed source file");
    }
    for index in 0..10 {
        fs::write(
            src.join("nested").join(format!("nested_{index}.txt")),
            b"payload",
        )
        .expect("seed nested source file");
    }
    fs::write(src.join("nested/deeper/final.txt"), b"payload").expect("seed deep source file");

    c.bench_function("copy_dir_all_small_tree", |b| {
        b.iter(|| {
            let dst = temp.path().join(format!("dst_{}", rand_suffix()));
            copy_dir_all(black_box(&src), black_box(&dst)).expect("copy tree");
            fs::remove_dir_all(&dst).expect("cleanup copied tree");
        });
    });
}

fn rand_suffix() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}
