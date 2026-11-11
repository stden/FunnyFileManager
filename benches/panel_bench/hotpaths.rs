//! Purpose: End-to-end hot-path benchmarks for panel mark-and-frame, editor
//! case-insensitive search, tree quick search, tokenizer keyword lookup, and
//! cold `$PATH` executable scanning.

use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};

use criterion::{BenchmarkId, Criterion};
use funny_file_manager::complete::execs;
use funny_file_manager::highlight::Highlighter;
use funny_file_manager::models::{App, FolderTree, Panel};
use funny_file_manager::search::{Caret, Needle, Search};
use funny_file_manager::ui::ui;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tempfile::TempDir;

use super::entry;

pub fn bench_hotpaths(c: &mut Criterion) {
    bench_mark_and_frame(c);
    bench_editor_search(c);
    bench_tree_search(c);
    bench_tokenizer(c);
    bench_path_scan(c);
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or draw must stop the run"
)]
fn bench_mark_and_frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("hotpaths/mark_and_frame");
    for size in [1_000_usize, 10_000, 50_000] {
        let root = TempDir::new().expect("temp dir");
        let entries: Vec<_> = (0..size).map(entry).collect();
        let left = Panel::from_entries(root.path().to_path_buf(), entries.clone());
        let right = Panel::from_entries(root.path().to_path_buf(), Vec::new());
        let mut app = App::with_panels(left, right);
        let mut term = Terminal::new(TestBackend::new(120, 40)).expect("terminal");
        group.bench_function(BenchmarkId::from_parameter(size), |b| {
            b.iter(|| {
                toggle_and_advance(&mut app.left_panel, size.saturating_sub(1));
                term.draw(|f| ui(f, black_box(&mut app))).expect("draw");
            });
        });
    }
    group.finish();
}

fn toggle_and_advance(panel: &mut Panel, last: usize) {
    panel.toggle_mark_selected();
    panel.next();
    if panel.selected() == Some(last) {
        panel.select(Some(0));
    }
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: needle is non-empty"
)]
fn bench_editor_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("hotpaths/editor_search_ignore");
    let query = Search::new(Needle::new("FFM_BENCH_NEEDLE").expect("needle"));
    let ascii: Vec<String> = (0..50_000)
        .map(|i| format!("00{i:05} INFO routine background activity in module handler"))
        .collect();
    let unicode: Vec<String> = (0..50_000)
        .map(|i| format!("00{i:05} — Ελληνικά Русский 日本語 Straße routine line"))
        .collect();
    group.bench_function("ascii_50k", |b| {
        b.iter(|| black_box(query.find(black_box(&ascii), Caret::new(0, 0))));
    });
    group.bench_function("unicode_50k", |b| {
        b.iter(|| black_box(query.find(black_box(&unicode), Caret::new(0, 0))));
    });
    group.finish();
}

fn bench_tree_search(c: &mut Criterion) {
    let mut tree = FolderTree::rooted_at(PathBuf::from("/workspace"));
    tree.expand(&mut |dir| {
        (0..5_000)
            .map(|i| dir.join(format!("subfolder-{i:04}")))
            .collect()
    });
    c.bench_function("hotpaths/tree_find_first_5k", |b| {
        b.iter(|| black_box(tree.find_first(black_box("zzz_absent"))));
    });
}

fn bench_tokenizer(c: &mut Criterion) {
    let sample = "pub fn compute_metric(values: &[usize], threshold: usize) -> Option<usize> { let mut total = 0; for item in values { if *item > threshold { total += *item; } } Some(total) }";
    let lines: Vec<String> = std::iter::repeat_n(sample.to_owned(), 1_000).collect();
    c.bench_function("hotpaths/highlight_rust_1k", |b| {
        b.iter(|| {
            let mut hl = Highlighter::for_file(Path::new("bench.rs"), &lines);
            for line in &lines {
                black_box(hl.line(line));
            }
        });
    });
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: fixture directory must exist"
)]
fn bench_path_scan(c: &mut Criterion) {
    let dir = TempDir::new().expect("temp dir");
    for i in 0..500 {
        fs::write(dir.path().join(format!("tool-{i:03}.exe")), b"MZ").expect("write");
    }
    let path_var = execs::join_paths([dir.path()]).expect("PATH");
    c.bench_function("hotpaths/path_scan_500", |b| {
        b.iter(|| black_box(execs::scan(Some(black_box(&path_var)))));
    });
}
