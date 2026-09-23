//! Full editor redraw after horizontal cursor motion in an unchanged file.
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use funny_file_manager::{
    models::{App, Editor, Overlay, Panel},
    ui::ui,
};
use ratatui::{backend::TestBackend, Terminal};
use std::{hint::black_box, time::Duration};
use tempfile::TempDir;

fn editor_fixture(root: &TempDir, lines: usize) -> App {
    let mut app = App::new().expect("app");
    app.left_panel = Panel::new(root.path().to_path_buf()).expect("panel");
    app.right_panel = Panel::new(root.path().to_path_buf()).expect("panel");
    let mut editor = Editor::from_content(
        "large.rs".to_owned(),
        root.path().join("large.rs"),
        &"let answer = 42; // source\n".repeat(lines),
    );
    editor.cursor_line = lines - 1;
    app.set_overlay(Overlay::Editor(editor));
    app
}

fn cursor_redraw(c: &mut Criterion) {
    let root = TempDir::new().expect("directory");
    let mut group = c.benchmark_group("editor_cursor/cached_redraw");
    for lines in [100, 10_000, 100_000] {
        let mut app = editor_fixture(&root, lines);
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("terminal");
        terminal
            .draw(|frame| ui(frame, &mut app))
            .expect("warm lexical cache");
        group.bench_with_input(BenchmarkId::from_parameter(lines), &lines, |b, _| {
            b.iter(|| {
                move_and_draw(&mut app, &mut terminal);
                black_box(terminal.backend().buffer());
            });
        });
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(30)
        .warm_up_time(Duration::from_secs(1)).measurement_time(Duration::from_secs(2));
    targets = cursor_redraw
}
criterion_main!(benches);

fn move_and_draw(app: &mut App, terminal: &mut Terminal<TestBackend>) {
    let editor = app.get_editor_mut().expect("editor");
    if editor.cursor_col == 0 {
        editor.move_right();
    } else {
        editor.move_left();
    }
    terminal
        .draw(|frame| ui(frame, black_box(app)))
        .expect("redraw");
}
