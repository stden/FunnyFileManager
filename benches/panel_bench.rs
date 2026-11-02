//! Purpose: the panel work that happens between a key press and the next frame.
//!
//! `benches/ui_bench` already measures drawing a panel and re-reading a
//! directory. What is not measured is what the panel does to itself in
//! between: quick search scans names on every `Alt+letter`, marking rewrites
//! the selection set on every `Insert`, and both run on the event loop, so
//! their cost is added to every keystroke rather than paid once.
//!
//! All of it is linear in the entry count by design. These benchmarks exist to
//! notice when it stops being linear, or when the constant grows — a directory
//! of ten thousand files is ordinary on a build tree, and a scan that allocates
//! per entry is only visible at that size.
//!
//! Entries are built in memory with `Panel::from_entries`, not read from disk:
//! a real listing would add `read_dir` and an `lstat` per entry and drown the
//! thing being measured. `ui_bench` covers the on-disk path separately.

use std::hint::black_box;
use std::path::PathBuf;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use funny_file_manager::models::{DirEntry, EntryKind, FileSize, Modified, Panel};

#[path = "panel_bench/hotpaths.rs"]
mod hotpaths;

/// A small directory, a large one, and one big enough to expose a per-entry
/// allocation that the smaller sizes hide in noise.
const SIZES: [usize; 3] = [100, 1_000, 10_000];

/// Names that share long prefixes, which is what makes quick search interesting:
/// a scan that can reject on the first byte is not the scan users perform.
fn entry(index: usize) -> DirEntry {
    DirEntry {
        name: format!("document-{index:05}.txt"),
        kind: if index.is_multiple_of(7) {
            EntryKind::Directory
        } else {
            EntryKind::File
        },
        size: FileSize::from_bytes(u64::try_from(index).unwrap_or(0) * 1_337),
        modified: Modified::UNKNOWN,
        executable: index.is_multiple_of(11),
        ..DirEntry::default()
    }
}

fn panel(count: usize) -> Panel {
    let entries = (0..count).map(entry).collect();
    Panel::from_entries(PathBuf::from("/srv/x"), entries)
}

/// Quick search, the `Alt+letter` path.
///
/// Three prefixes with very different answers: one that matches the first
/// entry, one that matches deep in the list, and one that matches nothing.
/// They cost the same to scan but differ in how early they stop, so measuring
/// only the first would make a regression in the miss case invisible — and the
/// miss case is the one a user hits while still typing.
fn bench_quick_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("panel/quick_search");

    for size in SIZES {
        let mut p = panel(size);
        let deep = format!("document-{:05}", size.saturating_sub(1));
        let cases = [
            ("hit_first", "document-00000"),
            ("hit_last", deep.as_str()),
            ("miss", "zzzz"),
        ];

        for (label, prefix) in cases {
            group.bench_function(BenchmarkId::new(label, size), |b| {
                b.iter(|| black_box(p.find_first_matching(black_box(prefix))));
            });
        }
    }

    group.finish();
}

/// Repeating a quick search, which is what a second `Alt+letter` with the same
/// prefix does: continue from the cursor and wrap.
///
/// Separate from the first-match case because it starts mid-list and must wrap
/// around, so an off-by-one that turns the wrap into a second full scan doubles
/// the cost without changing any behaviour a test would catch.
///
/// The prefix names exactly one entry. A prefix matching everything would be
/// answered by the very next row and would measure nothing but the call
/// overhead — which is how the first version of this benchmark reported a flat
/// 20 ns at every size.
fn bench_search_next(c: &mut Criterion) {
    let mut group = c.benchmark_group("panel/search_next");

    for size in SIZES {
        let mut p = panel(size);
        let sole = format!("document-{:05}", size / 2);
        group.bench_function(BenchmarkId::from_parameter(size), |b| {
            b.iter(|| black_box(p.find_next_matching(black_box(sole.as_str()))));
        });
    }

    group.finish();
}

/// Marking one entry, the `Insert` / `Space` path.
///
/// One keystroke toggles the entry and advances the cursor. Cheap by
/// construction; measured because the marked set is keyed by name, and a
/// change that made it key by index — or that cloned the name set per toggle —
/// would grow with the listing.
fn bench_mark(c: &mut Criterion) {
    let mut group = c.benchmark_group("panel/mark");

    for size in SIZES {
        let mut p = panel(size);
        group.bench_function(BenchmarkId::from_parameter(size), |b| {
            b.iter(|| black_box(p.toggle_mark_selected()));
        });
    }

    group.finish();
}

/// Building a panel from a listing that is already in memory.
///
/// This is the part of entering a directory that is not I/O: taking the
/// entries, sorting them and placing the cursor. `ui_bench` measures the
/// version that reads the disk; this one isolates the half that a change to
/// sorting or selection actually affects.
fn bench_from_entries(c: &mut Criterion) {
    let mut group = c.benchmark_group("panel/from_entries");

    for size in SIZES {
        let entries: Vec<DirEntry> = (0..size).map(entry).collect();
        group.bench_function(BenchmarkId::from_parameter(size), |b| {
            b.iter(|| {
                black_box(Panel::from_entries(
                    PathBuf::from("/srv/x"),
                    black_box(entries.clone()),
                ))
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_quick_search,
    bench_search_next,
    bench_mark,
    bench_from_entries,
    hotpaths::bench_hotpaths
);
criterion_main!(benches);
