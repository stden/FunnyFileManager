//! Purpose: Cold and warm editor frames, and the cost of typing into a large file.

use super::app::{base, prepared, redraw, root, with_editor, Group, Place};
use super::sources::{samples, Sample};
use criterion::{BatchSize, BenchmarkId, Criterion};
use funny_file_manager::models::App;
use tempfile::TempDir;

pub fn bench_cold(c: &mut Criterion) {
    let root = root();
    let base = base(&root);
    let mut group = c.benchmark_group("editor_languages/frame_cold");
    for sample in samples() {
        cold_place(&mut group, &root, &base, &sample, Place::Top);
        cold_place(&mut group, &root, &base, &sample, Place::End);
    }
    group.finish();
}

pub fn bench_warm(c: &mut Criterion) {
    let root = root();
    let base = base(&root);
    let mut group = c.benchmark_group("editor_languages/frame_warm");
    for sample in samples() {
        warm_one(&mut group, &root, &base, &sample);
    }
    group.finish();
}

pub fn bench_type(c: &mut Criterion) {
    let root = root();
    let base = base(&root);
    let mut group = c.benchmark_group("editor_languages/type_at_end");
    for sample in samples() {
        type_one(&mut group, &root, &base, &sample);
    }
    group.finish();
}

fn cold_place(group: &mut Group<'_>, root: &TempDir, base: &App, sample: &Sample, place: Place) {
    let id = BenchmarkId::new(
        format!("{}/{}", sample.name, place.label()),
        sample.line_count(),
    );
    group.bench_function(id, |b| {
        b.iter_batched(
            || prepared(root, base, sample, place),
            |(mut app, mut terminal)| redraw(&mut app, &mut terminal),
            BatchSize::LargeInput,
        );
    });
}

fn warm_one(group: &mut Group<'_>, root: &TempDir, base: &App, sample: &Sample) {
    let mut app = with_editor(root, base, sample, Place::End);
    let mut terminal = super::app::terminal();
    redraw(&mut app, &mut terminal);
    let id = BenchmarkId::new(sample.name, sample.line_count());
    group.bench_function(id, |b| {
        b.iter(|| {
            super::app::nudge(&mut app);
            redraw(&mut app, &mut terminal);
        });
    });
}

fn type_one(group: &mut Group<'_>, root: &TempDir, base: &App, sample: &Sample) {
    let id = BenchmarkId::new(sample.name, sample.line_count());
    group.bench_function(id, |b| {
        b.iter_batched(
            || {
                let (mut app, mut terminal) = prepared(root, base, sample, Place::End);
                redraw(&mut app, &mut terminal);
                (app, terminal)
            },
            |(mut app, mut terminal)| {
                type_char(&mut app);
                redraw(&mut app, &mut terminal);
            },
            BatchSize::LargeInput,
        );
    });
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed edit must stop the run"
)]
fn type_char(app: &mut App) {
    app.get_editor_mut().expect("editor").insert_char('.');
}
