//! Purpose: how long a frame takes, and what inside it costs the most.
//!
//! The panels are redrawn from scratch on every keypress, so anything the
//! renderer does is done again for each cursor movement. A regression here is
//! not a slow report the user waits once for — it is input lag, which is felt
//! long before it is measured.
//!
//! Three questions, in the order they matter:
//!
//! 1. What does one whole frame cost, at the sizes people actually use?
//! 2. How does that scale with the listing — a directory of ten thousand
//!    entries must not cost a thousand times a directory of ten, because only
//!    a screenful is ever visible.
//! 3. Which overlay is the expensive one?
//!
//! The pure layout functions are measured too. They are called several times
//! per frame and are cheap enough that a benchmark reads as noise — which is
//! exactly the point: it says so, rather than leaving it assumed.

use criterion::{criterion_group, criterion_main};
use funny_file_manager::models::{App, DirEntry, EntryKind, FileSize, Modified, PanelSide};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::fs;
use tempfile::TempDir;

#[path = "ui_bench/directory.rs"]
mod directory;
#[path = "ui_bench/frame.rs"]
mod frame;
#[path = "ui_bench/highlight.rs"]
mod highlight;
#[path = "ui_bench/layout.rs"]
mod layout;

/// Terminal sizes worth caring about: the classic 80×24, a comfortable
/// window, and a full-screen one on a large display.
const SIZES: [(u16, u16); 3] = [(80, 24), (120, 40), (200, 60)];

/// One entry, named so the extension split and the name column both do work.
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

/// An app whose panels each hold `count` entries, cursor near the middle so
/// the scroll offset is not zero.
///
/// The panels are pointed at real directories under a temporary root. The
/// listing itself is synthetic — writing ten thousand files to measure a
/// renderer would time the filesystem instead — but the paths have to exist:
/// the frame prints them, and each row is checked against its directory to
/// decide whether it is hidden. The returned [`TempDir`] is the caller's to
/// hold, since dropping it takes the directories with it.
#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
fn app_with(count: usize) -> (App, TempDir) {
    let root = TempDir::new().expect("benchmark temp dir");
    let left = root.path().join("left");
    let right = root.path().join("right");
    fs::create_dir_all(&left).expect("left panel directory");
    fs::create_dir_all(&right).expect("right panel directory");

    let mut app = App::new().expect("benchmark app");
    let entries: Vec<DirEntry> = (0..count).map(entry).collect();
    app.left_panel = funny_file_manager::models::Panel::from_entries(left, entries.clone());
    app.right_panel = funny_file_manager::models::Panel::from_entries(right, entries);
    let middle = count / 2;
    app.left_panel.select(Some(middle));
    app.right_panel.select(Some(middle));
    app.active_panel = PanelSide::Left;
    (app, root)
}

fn terminal(width: u16, height: u16) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(width, height)).expect("benchmark terminal")
}

/// A tree with something in every content category, so a walk of it has the
/// same shape as a walk of a real project.
///
/// Built rather than borrowed: pointing a benchmark at the repository makes
/// its result a function of how large the repository happens to be that week
/// — and `target/` alone would dominate it.
#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
fn sample_tree() -> TempDir {
    let root = TempDir::new().expect("benchmark tree");
    for (directory, files) in [
        ("src", ["main.rs", "lib.rs", "util.rs"]),
        ("docs", ["guide.md", "notes.txt", "spec.md"]),
        ("assets", ["logo.png", "shot.jpg", "icon.svg"]),
        ("data", ["config.toml", "index.json", "rows.csv"]),
    ] {
        let path = root.path().join(directory);
        fs::create_dir_all(&path).expect("benchmark subdirectory");
        for (index, name) in files.into_iter().enumerate() {
            fs::write(path.join(name), vec![b'x'; 1_024 * (index + 1)]).expect("benchmark file");
        }
    }
    root
}

criterion_group!(
    benches,
    frame::bench_frame,
    frame::bench_listing_size,
    directory::bench_sort_modes,
    frame::bench_overlays,
    frame::bench_information_panel,
    layout::bench_layout,
    layout::bench_clock,
    highlight::bench_highlight,
    directory::bench_stats,
);
criterion_main!(benches);
