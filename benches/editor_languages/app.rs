//! Purpose: Editor fixtures shared by the large-file frame and completion benches.

use super::sources::Sample;
use funny_file_manager::{
    models::{App, Editor, Overlay, Panel},
    ui::ui,
};
use ratatui::{backend::TestBackend, Terminal};
use std::hint::black_box;
use tempfile::TempDir;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Top,
    End,
}

impl Place {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::End => "end",
        }
    }
}

pub type Group<'a> = criterion::BenchmarkGroup<'a, criterion::measurement::WallTime>;

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture must stop the run"
)]
pub fn base(root: &TempDir) -> App {
    let mut app = App::new().expect("app");
    let path = root.path().to_path_buf();
    app.left_panel = Panel::new(path.clone()).expect("panel");
    app.right_panel = Panel::new(path).expect("panel");
    app
}

pub fn with_editor(root: &TempDir, base: &App, sample: &Sample, place: Place) -> App {
    let mut app = base.clone();
    let mut editor = Editor::from_content(
        sample.file.to_owned(),
        root.path().join(sample.file),
        &sample.source,
    );
    place_cursor(&mut editor, place);
    app.set_overlay(Overlay::Editor(editor));
    app
}

pub fn prepared(
    root: &TempDir,
    base: &App,
    sample: &Sample,
    place: Place,
) -> (App, Terminal<TestBackend>) {
    (with_editor(root, base, sample, place), terminal())
}

fn place_cursor(editor: &mut Editor, place: Place) {
    if place == Place::Top {
        return;
    }
    editor.cursor_line = editor.lines.len().saturating_sub(1);
    editor.cursor_col = editor.lines.last().map_or(0, |line| line.chars().count());
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed redraw must stop the run"
)]
pub fn nudge(app: &mut App) {
    let editor = app.get_editor_mut().expect("editor");
    if editor.cursor_col == 0 {
        editor.move_right();
    } else {
        editor.move_left();
    }
}

pub fn redraw(app: &mut App, terminal: &mut Terminal<TestBackend>) {
    terminal
        .draw(|frame| ui(frame, black_box(app)))
        .expect("redraw");
    black_box(terminal.backend().buffer().area());
}

pub fn terminal() -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(120, 40)).expect("terminal")
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed directory must stop the run"
)]
pub fn root() -> TempDir {
    TempDir::new().expect("directory")
}
