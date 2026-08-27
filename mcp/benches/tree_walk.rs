//! Purpose: Benchmarks production directory-tree readers on realistic fixtures.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ffm_mcp::tree_walk::{directory_size, render_directory_tree};
use std::env;
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;

enum FixtureRoot {
    Generated(TempDir),
    Existing(PathBuf),
}

struct TreeFixture {
    root: FixtureRoot,
    entries: u64,
    bytes: u64,
}

impl TreeFixture {
    fn path(&self) -> &Path {
        match &self.root {
            FixtureRoot::Generated(temp) => temp.path(),
            FixtureRoot::Existing(path) => path,
        }
    }

    fn generated(temp: TempDir, entries: u64) -> Self {
        let bytes = directory_size(temp.path());
        Self {
            root: FixtureRoot::Generated(temp),
            entries,
            bytes,
        }
    }

    fn existing(path: PathBuf) -> Self {
        assert!(path.is_dir(), "real-tree benchmark path is not a directory");
        let entries = count_entries(&path);
        let bytes = directory_size(&path);
        Self {
            root: FixtureRoot::Existing(path),
            entries,
            bytes,
        }
    }
}

fn count_entries(root: &Path) -> u64 {
    let mut count = 0_u64;
    let mut pending = vec![root.to_path_buf()];

    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            count += 1;
            if entry
                .file_type()
                .is_ok_and(|file_type| file_type.is_dir() && !file_type.is_symlink())
            {
                pending.push(entry.path());
            }
        }
    }

    count
}

fn write_files(path: &Path, count: usize) -> u64 {
    for index in 0..count {
        fs::write(path.join(format!("file_{index:04}.bin")), [b'x'; 128])
            .expect("seed benchmark file");
    }
    count as u64
}

fn seed_balanced_level(path: &Path, depth: usize, fanout: usize, files: usize) -> u64 {
    let mut entries = write_files(path, files);
    if depth == 0 {
        return entries;
    }

    for index in 0..fanout {
        let child = path.join(format!("dir_{index:02}"));
        fs::create_dir(&child).expect("seed benchmark directory");
        entries += 1 + seed_balanced_level(&child, depth - 1, fanout, files);
    }
    entries
}

fn balanced_fixture() -> TreeFixture {
    let temp = TempDir::new().expect("benchmark temp dir");
    let entries = seed_balanced_level(temp.path(), 4, 4, 8);
    TreeFixture::generated(temp, entries)
}

fn wide_fixture() -> TreeFixture {
    let temp = TempDir::new().expect("benchmark temp dir");
    let mut entries = write_files(temp.path(), 2_048);
    for index in 0..32 {
        let child = temp.path().join(format!("dir_{index:02}"));
        fs::create_dir(&child).expect("seed benchmark directory");
        entries += 1 + write_files(&child, 4);
    }
    TreeFixture::generated(temp, entries)
}

fn fixtures() -> Vec<(String, TreeFixture)> {
    let mut fixtures = vec![
        ("balanced".to_owned(), balanced_fixture()),
        ("wide".to_owned(), wide_fixture()),
    ];
    if let Some(path) = env::var_os("FFM_BENCH_REAL_TREE").map(PathBuf::from) {
        fixtures.push(("real_windows".to_owned(), TreeFixture::existing(path)));
    }

    for (shape, fixture) in &fixtures {
        eprintln!(
            "tree fixture {shape}: {} entries, {} bytes at {}",
            fixture.entries,
            fixture.bytes,
            fixture.path().display()
        );
    }
    fixtures
}

fn bench_directory_tree_readers(c: &mut Criterion) {
    let fixtures = fixtures();

    let mut size_group = c.benchmark_group("directory_size");
    size_group.sample_size(30);
    size_group.measurement_time(Duration::from_secs(5));
    for (shape, fixture) in &fixtures {
        // Logical bytes accounted for per second; file contents are not read.
        size_group.throughput(Throughput::Bytes(fixture.bytes.max(1)));
        size_group.bench_with_input(BenchmarkId::from_parameter(shape), fixture, |b, fixture| {
            b.iter(|| black_box(directory_size(black_box(fixture.path()))));
        });
    }
    size_group.finish();

    let mut tree_group = c.benchmark_group("render_directory_tree");
    tree_group.sample_size(30);
    tree_group.measurement_time(Duration::from_secs(5));
    for (shape, fixture) in &fixtures {
        tree_group.throughput(Throughput::Elements(fixture.entries));
        tree_group.bench_with_input(BenchmarkId::from_parameter(shape), fixture, |b, fixture| {
            b.iter(|| black_box(render_directory_tree(black_box(fixture.path()), usize::MAX)));
        });
    }
    tree_group.finish();
}

criterion_group!(benches, bench_directory_tree_readers);
criterion_main!(benches);
