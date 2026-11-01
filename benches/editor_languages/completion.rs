//! Purpose: In-process cost of asking for completions on a large editor buffer.
//!
//! Ctrl+Space copies the whole buffer into the language-server request. Accepting
//! a suggestion joins that text, applies the edit, and splits it back into lines.
//! This measures those two steps. It does not start jdtls or a Python server.

use super::app::{base, root, with_editor, Place};
use super::sources::samples;
use criterion::{BenchmarkId, Criterion};
use funny_file_manager::models::App;
use std::hint::black_box;

pub fn bench_completion(c: &mut Criterion) {
    measure(c, "editor_languages/completion_snapshot", snapshot);
    measure(c, "editor_languages/completion_rewrite", rewrite);
}

fn measure(c: &mut Criterion, name: &str, step: fn(&App) -> usize) {
    let root = root();
    let base = base(&root);
    let mut group = c.benchmark_group(name);
    for sample in samples() {
        let app = with_editor(&root, &base, &sample, Place::End);
        let id = BenchmarkId::new(sample.name, sample.line_count());
        group.bench_with_input(id, &app, |b, app| b.iter(|| black_box(step(app))));
    }
    group.finish();
}

fn snapshot(app: &App) -> usize {
    text(app).len()
}

fn rewrite(app: &App) -> usize {
    let mut body = text(app);
    body.push('x');
    let lines: Vec<String> = body.split('\n').map(str::to_owned).collect();
    black_box(lines).len()
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed editor read must stop the run"
)]
fn text(app: &App) -> String {
    app.get_editor().expect("editor").content()
}
