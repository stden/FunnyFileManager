//! Purpose: what the long operations cost, and where the cost actually is.
//!
//! Copying, moving and deleting are the only things this program does that a
//! user waits for. They also run on the event loop — nothing is drawn and no
//! key is read until they return — so their duration is not "a slow operation"
//! but "a frozen terminal", and it is worth knowing which shapes of work are
//! slow rather than guessing.
//!
//! Every group asks the same question in two shapes, because they have
//! different bottlenecks and the same total size tells you nothing about
//! which one you are in:
//!
//! - **Many small files.** One `open`, one `close` and a directory entry each;
//!   the time is syscalls, not bytes.
//! - **One large file.** A single stream; the time is bytes, not syscalls.
//!
//! Setup is kept out of the measurement with `iter_batched`: building a tree
//! of a thousand files takes far longer than copying it, and timing the two
//! together would measure the harness.

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use funny_file_manager::utils::{copy_dir_all, unique_destination};
use std::ffi::OsStr;
use std::fs;
use std::hint::black_box;
use std::path::Path;
use tempfile::TempDir;

/// Files in the "many small" shape. Enough for the per-file cost to dominate,
/// few enough that building the tree once per iteration stays bearable.
const SMALL_FILES: usize = 500;

/// Bytes in each of them — a source file's order of magnitude.
const SMALL_SIZE: usize = 4 * 1_024;

/// The "one large" shape. Sixteen megabytes is a photograph or a short video,
/// and large enough that the copy is bandwidth rather than bookkeeping.
const LARGE_SIZE: usize = 16 * 1_024 * 1_024;

/// A directory of `count` files of `size` bytes each.
fn many_small(root: &Path, count: usize, size: usize) {
    fs::create_dir_all(root).expect("tree root");
    let payload = vec![b'x'; size];
    for index in 0..count {
        fs::write(root.join(format!("file-{index:05}.dat")), &payload).expect("tree file");
    }
}

/// A directory holding one file of `size` bytes.
fn one_large(root: &Path, size: usize) {
    fs::create_dir_all(root).expect("tree root");
    fs::write(root.join("payload.bin"), vec![b'x'; size]).expect("large file");
}

/// A tree `depth` levels deep with `per_level` files at each.
///
/// Its own shape because a deep tree costs a `read_dir` and a `create_dir` per
/// level on top of the files, and a project checkout is mostly this.
fn deep_tree(root: &Path, depth: usize, per_level: usize) {
    let mut path = root.to_path_buf();
    for level in 0..depth {
        path = path.join(format!("level-{level}"));
        fs::create_dir_all(&path).expect("nested directory");
        for index in 0..per_level {
            fs::write(path.join(format!("file-{index}.txt")), b"payload").expect("nested file");
        }
    }
}

/// Copying, in each shape.
fn bench_copy(c: &mut Criterion) {
    let mut group = c.benchmark_group("copy");
    group.sample_size(20);

    for shape in ["many_small", "one_large", "deep_tree"] {
        let source = source_tree(shape);
        group.bench_function(shape, |b| {
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
fn bench_move(c: &mut Criterion) {
    let mut group = c.benchmark_group("move");
    group.sample_size(20);

    group.bench_function("rename_same_filesystem", |b| {
        b.iter_batched(
            || source_tree("many_small"),
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
            || source_tree("many_small"),
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
fn bench_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete");
    group.sample_size(20);

    group.bench_function("many_small", |b| {
        b.iter_batched(
            || source_tree("many_small"),
            |root| {
                fs::remove_dir_all(black_box(root.path().join("tree"))).expect("delete");
                root
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function("one_large", |b| {
        b.iter_batched(
            || source_tree("one_large"),
            |root| {
                fs::remove_dir_all(black_box(root.path().join("tree"))).expect("delete");
                root
            },
            BatchSize::PerIteration,
        );
    });

    group.finish();
}

/// Finding a free name when the obvious one is taken.
///
/// `unique_destination` counts upwards from the wanted name, and every step
/// asks the filesystem whether that one exists too. A directory already
/// holding a run of `report_1.txt`, `report_2.txt`, … therefore costs a
/// `stat` per existing copy, and the run grows by one each time a file is
/// copied into it. This measures how that curve actually bends.
fn bench_unique_destination_collisions(c: &mut Criterion) {
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
fn bench_editor_open(c: &mut Criterion) {
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
fn bench_directory_load(c: &mut Criterion) {
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

criterion_group!(
    benches,
    bench_copy,
    bench_move,
    bench_delete,
    bench_unique_destination_collisions,
    bench_editor_open,
    bench_directory_load,
);
criterion_main!(benches);

fn source_tree(shape: &str) -> TempDir {
    let root = TempDir::new().expect("source tree");
    let path = root.path().join("tree");
    match shape {
        "many_small" => many_small(&path, SMALL_FILES, SMALL_SIZE),
        "one_large" => one_large(&path, LARGE_SIZE),
        "deep_tree" => deep_tree(&path, 10, 20),
        _ => panic!("unknown benchmark shape"),
    }
    root
}
