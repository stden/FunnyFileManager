//! Purpose: Complete content searches over finite files, without persistent app state.
//!
//! Fixtures are outside measurement. Each iteration verifies both the result
//! count and successful completion, so a timeout cannot look like an improvement.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use funny_file_manager::{
    models::{App, FindFile, MenuDialog, Overlay, Panel},
    ops::findfile,
    status::Status,
};
use std::{fs, hint::black_box, time::Duration};
use tempfile::TempDir;

const CORPUS_BYTES: usize = 8 * 1024 * 1024;
const NEEDLE: &str = "FFM_CONTENT_BENCH_NEEDLE";
const CASES: [(&str, &[u8], bool); 4] = [
    ("ascii_absent", b"INFO 12345 background activity\n", false),
    ("ascii_late", b"INFO 12345 background activity\n", true),
    (
        "unicode_absent",
        "INFO Ελληνικά Русский 日本語 Straße\n".as_bytes(),
        false,
    ),
    (
        "invalid_utf8_absent",
        b"INFO \xff invalid \xf0\x9f bytes\n",
        false,
    ),
];

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: an invalid corpus size must stop the run"
)]
fn content_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("content_search");
    for (name, pattern, matching) in CASES {
        let bytes = corpus(pattern, matching);
        let (_root, mut app) = fixture(&bytes);
        let expected = usize::from(matching);
        search(&mut app, expected);
        let byte_count = u64::try_from(bytes.len()).expect("content search corpus size fits u64");
        group.throughput(Throughput::Bytes(byte_count));
        group.bench_function(BenchmarkId::new(name, bytes.len()), |b| {
            b.iter(|| search(black_box(&mut app), expected));
        });
    }
    group.finish();
}

fn corpus(pattern: &[u8], matching: bool) -> Vec<u8> {
    let mut bytes = pattern.repeat(CORPUS_BYTES / pattern.len());
    if matching {
        bytes.extend_from_slice(NEEDLE.as_bytes());
    }
    bytes
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture must stop the run"
)]
fn fixture(bytes: &[u8]) -> (TempDir, App) {
    let root = TempDir::new().expect("content search directory");
    fs::write(root.path().join("corpus.txt"), bytes).expect("content search corpus");
    let panel = Panel::from_entries(root.path().to_owned(), Vec::new());
    let app = App::with_panels(panel.clone(), panel);
    (root, app)
}

#[expect(
    clippy::panic,
    reason = "criterion has no error channel: invalid result dialogs must stop the run"
)]
fn search(app: &mut App, expected: usize) {
    app.set_status(Status::KeyHints);
    let mut dialog = FindFile::default();
    dialog.mask.set_text("*.txt");
    dialog.containing.set_text(NEEDLE);
    app.set_overlay(Overlay::FindFile(Box::new(dialog)));
    findfile::accept(app);
    assert_eq!(app.status, Status::KeyHints, "incomplete content search");
    let Overlay::MenuDialog(dialog) = &app.overlay else {
        panic!("content search did not produce a result dialog");
    };
    let MenuDialog::Choices { rows, .. } = dialog.as_ref() else {
        panic!("content search did not produce choices");
    };
    assert_eq!(rows.len(), expected, "unexpected content search results");
    black_box(rows);
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(300))
        .measurement_time(Duration::from_secs(1));
    targets = content_search
}
criterion_main!(benches);
