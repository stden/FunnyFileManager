//! Purpose: Sort modes, drive entries, virtual paths, panels and the app.

use criterion::Criterion;
use funny_file_manager::models::{App, DriveEntry, Panel, SortMode};
#[cfg(feature = "s3")]
use funny_file_manager::s3::VirtualPath;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use tempfile::TempDir;

pub fn bench_sort_mode(c: &mut Criterion) {
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

pub fn bench_drive_display(c: &mut Criterion) {
    let local = DriveEntry::Local(PathBuf::from("/"));
    let s3 = DriveEntry::S3Profile("default".to_owned());

    c.bench_function("drive_entry_display_local", |b| {
        b.iter(|| black_box(&local).display());
    });

    c.bench_function("drive_entry_display_s3", |b| {
        b.iter(|| black_box(&s3).display());
    });
}

#[cfg(feature = "s3")]
pub fn bench_virtual_path(c: &mut Criterion) {
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

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
pub fn bench_panel_basics(c: &mut Criterion) {
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

pub fn bench_app_basics(c: &mut Criterion) {
    c.bench_function("app_get_drives", |b| {
        b.iter(App::get_drives);
    });

    c.bench_function("app_get_all_drives", |b| {
        b.iter(App::get_all_drives);
    });
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
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
