//! Purpose: whole-frame benchmarks — terminal sizes, listing length, and each overlay.

use super::{app_with, sample_tree, terminal, SIZES};
use criterion::{BenchmarkId, Criterion};
use funny_file_manager::menu::{Heading, Menu};
use funny_file_manager::models::{Confirm, FileName, NamePrompt, Overlay};
use funny_file_manager::ui::ui;
use std::hint::black_box;
use std::path::PathBuf;

/// The headline number: one whole frame, at the sizes people use.
pub fn bench_frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame");

    for (width, height) in SIZES {
        let (mut app, _root) = app_with(500);
        let mut term = terminal(width, height);
        group.bench_function(
            BenchmarkId::from_parameter(format!("{width}x{height}")),
            |b| {
                b.iter(|| {
                    term.draw(|f| ui(f, black_box(&mut app))).expect("draw");
                });
            },
        );
    }

    group.finish();
}

/// A frame against listings from tiny to enormous.
///
/// Only a screenful is ever drawn, so this should be close to flat. If it is
/// not, something is walking the whole listing once per frame — which is the
/// mistake this benchmark exists to catch.
pub fn bench_listing_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_by_entries");

    for count in [10_usize, 100, 1_000, 10_000, 100_000] {
        let (mut app, _root) = app_with(count);
        let mut term = terminal(120, 40);
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| {
                term.draw(|f| ui(f, black_box(&mut app))).expect("draw");
            });
        });
    }

    group.finish();
}

/// Each overlay, over the same panels, so the difference is the overlay.
pub fn bench_overlays(c: &mut Criterion) {
    let mut group = c.benchmark_group("overlay");

    let overlays: Vec<(&str, Overlay)> = vec![
        ("none", Overlay::None),
        ("help", Overlay::Help),
        ("quit", Overlay::QuitConfirm(Confirm::Yes)),
        (
            "delete",
            Overlay::DeleteConfirm(vec![PathBuf::from("doomed.txt")]),
        ),
        (
            "name",
            Overlay::NamePrompt(NamePrompt::rename(FileName::new("document.txt"))),
        ),
        ("information", Overlay::Information(Box::default())),
        ("menu", Overlay::Menu(Menu::open(Heading::Left))),
    ];

    for (name, overlay) in overlays {
        let (mut app, _root) = app_with(500);
        app.overlay = overlay;
        let mut term = terminal(120, 40);
        group.bench_function(name, |b| {
            b.iter(|| {
                term.draw(|f| ui(f, black_box(&mut app))).expect("draw");
            });
        });
    }

    group.finish();
}

/// The information panel over a real measurement, which is the largest body
/// any dialog has to lay out.
pub fn bench_information_panel(c: &mut Criterion) {
    let tree = sample_tree();
    let stats = funny_file_manager::stats::analyse(tree.path());
    let (mut app, _root) = app_with(500);
    app.overlay = Overlay::Information(Box::new(stats));
    let mut term = terminal(120, 40);

    c.bench_function("overlay/information_full", |b| {
        b.iter(|| {
            term.draw(|f| ui(f, black_box(&mut app))).expect("draw");
        });
    });
}
