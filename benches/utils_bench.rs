//! Purpose: Benchmarks utility and model hot paths to watch for performance regressions.
//!
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use funny_file_manager::models::FileSize;
use funny_file_manager::models::{App, DriveEntry, Panel, SortMode};
#[cfg(feature = "s3")]
use funny_file_manager::s3::VirtualPath;
use funny_file_manager::utils::{copy_dir_all, unique_destination};
use std::ffi::OsStr;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use tempfile::TempDir;

fn bench_format_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_size");

    for size in [0_u64, 999, 1_024, 1_536, 1_048_576, 1_073_741_824] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| FileSize::from_bytes(black_box(size)).to_string());
        });
    }

    group.finish();
}

fn bench_unique_destination(c: &mut Criterion) {
    let temp = TempDir::new().expect("benchmark temp dir");
    let base = temp.path();
    let free_name = OsStr::new("report.txt");

    for suffix in 0..50 {
        let name = if suffix == 0 {
            "report.txt".to_string()
        } else {
            format!("report_{suffix}.txt")
        };
        fs::write(base.join(name), b"x").expect("seed benchmark files");
    }

    c.bench_function("unique_destination_with_conflicts", |b| {
        b.iter(|| unique_destination(black_box(base), black_box(free_name)).expect("unique path"));
    });
}

fn bench_copy_dir_all(c: &mut Criterion) {
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

fn bench_sort_mode(c: &mut Criterion) {
    c.bench_function("sort_mode_next", |b| {
        let mut mode = SortMode::Name;
        b.iter(|| {
            mode = black_box(mode).next();
            black_box(mode)
        });
    });

    c.bench_function("sort_mode_label", |b| {
        b.iter(|| black_box(SortMode::Extension).label());
    });
}

fn bench_drive_display(c: &mut Criterion) {
    let local = DriveEntry::Local(PathBuf::from("/"));
    let s3 = DriveEntry::S3Profile("default".to_string());

    c.bench_function("drive_entry_display_local", |b| {
        b.iter(|| black_box(&local).display());
    });

    c.bench_function("drive_entry_display_s3", |b| {
        b.iter(|| black_box(&s3).display());
    });
}

#[cfg(feature = "s3")]
fn bench_virtual_path(c: &mut Criterion) {
    let local = "/tmp/example";
    let s3_bucket = "s3://bucket-name/prefix/deeper";
    let s3_profile = "s3://prod@bucket-name/prefix/deeper";

    let parsed = VirtualPath::parse(s3_profile);

    c.bench_function("virtual_path_parse_local", |b| {
        b.iter(|| VirtualPath::parse(black_box(local)));
    });

    c.bench_function("virtual_path_parse_s3_bucket", |b| {
        b.iter(|| VirtualPath::parse(black_box(s3_bucket)));
    });

    c.bench_function("virtual_path_parse_s3_profile", |b| {
        b.iter(|| VirtualPath::parse(black_box(s3_profile)));
    });

    c.bench_function("virtual_path_display", |b| {
        b.iter(|| black_box(&parsed).display());
    });
}

fn bench_panel_basics(c: &mut Criterion) {
    let temp = panel_fixture();

    let mut group = c.benchmark_group("panel_basic_ops");

    group.bench_function("panel_new", |b| {
        b.iter(|| Panel::new(black_box(temp.path().to_path_buf())).expect("panel new"));
    });

    let mut panel_for_load = Panel::new(temp.path().to_path_buf()).expect("panel");
    group.bench_function("panel_load_entries", |b| {
        b.iter(|| panel_for_load.load_entries().expect("load entries"));
    });

    let mut panel_for_match = Panel::new(temp.path().to_path_buf()).expect("panel");
    group.bench_function("panel_find_first_matching", |b| {
        b.iter(|| panel_for_match.find_first_matching(black_box("del")));
    });

    let mut panel_for_next = Panel::new(temp.path().to_path_buf()).expect("panel");
    let _ = panel_for_next.find_first_matching("a");
    group.bench_function("panel_find_next_matching", |b| {
        b.iter(|| panel_for_next.find_next_matching(black_box("a")));
    });

    let mut panel_for_marks = Panel::new(temp.path().to_path_buf()).expect("panel");
    assert!(panel_for_marks.select_named("alpha.txt"));
    group.bench_function("panel_toggle_mark_selected", |b| {
        b.iter(|| panel_for_marks.toggle_mark_selected());
    });

    let mut panel_for_cycle = Panel::new(temp.path().to_path_buf()).expect("panel");
    group.bench_function("panel_cycle_sort_mode", |b| {
        b.iter(|| panel_for_cycle.cycle_sort_mode());
    });

    group.finish();
}

fn bench_app_basics(c: &mut Criterion) {
    c.bench_function("app_get_drives", |b| {
        b.iter(App::get_drives);
    });

    c.bench_function("app_get_all_drives", |b| {
        b.iter(App::get_all_drives);
    });
}

fn rand_suffix() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

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

fn panel_fixture() -> TempDir {
    let temp = TempDir::new().expect("benchmark temp dir");
    for name in [
        "alpha.txt",
        "beta.rs",
        "gamma.md",
        "delta.txt",
        "epsilon.log",
    ] {
        fs::write(temp.path().join(name), b"payload").expect("seed panel file");
    }
    fs::create_dir_all(temp.path().join("folder_a")).expect("seed panel dir");
    fs::create_dir_all(temp.path().join("folder_b")).expect("seed panel dir");

    temp
}
