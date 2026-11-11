//! Purpose: Name collisions, opening a file in the editor and loading a directory.

use super::many_small;
use criterion::{BenchmarkId, Criterion};
use funny_file_manager::utils::unique_destination;
use std::ffi::OsStr;
use std::fs;
use std::hint::black_box;
use std::path::Path;
use tempfile::TempDir;

/// Finding a free name when the obvious one is taken.
///
/// `unique_destination` counts upwards from the wanted name, and every step
/// asks the filesystem whether that one exists too. A directory already
/// holding a run of `report_1.txt`, `report_2.txt`, … therefore costs a
/// `stat` per existing copy, and the run grows by one each time a file is
/// copied into it. This measures how that curve actually bends.
#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
pub fn bench_unique_destination_collisions(c: &mut Criterion) {
    let mut group = c.benchmark_group("unique_destination");

    for taken in [0_usize, 100, 500, 2_000] {
        let dir = TempDir::new().expect("collision dir");
        fs::write(dir.path().join("report.txt"), b"x").expect("seed");
        for index in 1..=taken {
            fs::write(dir.path().join(format!("report_{index}.txt")), b"x").expect("seed");
        }

        group.bench_with_input(BenchmarkId::from_parameter(taken), &taken, |b, _| {
            b.iter(|| {
                unique_destination(black_box(dir.path()), black_box(OsStr::new("report.txt")))
                    .expect("free name")
            });
        });
    }

    group.finish();
}

/// Opening a file in the editor, which highlights the whole thing up front.
///
/// The editor shows forty lines and re-highlights them each frame, but the
/// first frame pays for every line in the file. This is the wait between
/// pressing F4 and seeing anything.
pub fn bench_editor_open(c: &mut Criterion) {
    let mut group = c.benchmark_group("editor_open");

    for lines in [100_usize, 1_000, 10_000] {
        let source: Vec<String> = std::iter::repeat_n(
            "pub fn handle(name: &str) -> Result<u32, Error> { Ok(name.len() as u32) } // note"
                .to_owned(),
            lines,
        )
        .collect();

        group.bench_with_input(BenchmarkId::from_parameter(lines), &lines, |b, _| {
            b.iter(|| {
                highlight_first_screen(&source);
            });
        });
    }

    group.finish();
}

/// Build the editor highlighter and render the first screenful.
fn highlight_first_screen(source: &[String]) {
    let mut highlighter = funny_file_manager::highlight::Highlighter::for_file(
        black_box(Path::new("open.rs")),
        black_box(source),
    );
    for line in source.iter().take(40) {
        black_box(highlighter.line(line));
    }
}

/// Reading a directory into a panel, which is what entering one costs.
///
/// One `lstat` per entry, and a second `metadata` for every symlink. The
/// panel cannot draw until this returns, so it is the delay on every Enter.
#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
pub fn bench_directory_load(c: &mut Criterion) {
    use funny_file_manager::models::Panel;

    let mut group = c.benchmark_group("directory_load");
    group.sample_size(20);

    for count in [100_usize, 1_000, 10_000] {
        let dir = TempDir::new().expect("listing dir");
        many_small(dir.path(), count, 1);

        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| Panel::new(black_box(dir.path().to_path_buf())).expect("panel"));
        });
    }

    group.finish();
}
