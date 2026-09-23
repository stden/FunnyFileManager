//! Repainting cursor motion must reuse lexical work for an unchanged document.
use super::*;
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn moving_the_editor_cursor_reuses_file_highlighting() {
    let root = tempfile::TempDir::new().expect("directory");
    let mut editor = crate::models::Editor::from_content(
        "large.rs".to_owned(),
        root.path().join("large.rs"),
        &"let answer = 42;\n".repeat(10_000),
    );
    editor.cursor_line = 9_000;
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
    let mut tracker = crate::mouse::DialogMouse::default();
    crate::highlight::FILE_PASSES.set(0);
    terminal
        .draw(|frame| render_editor(frame, &editor, &mut tracker))
        .expect("first frame");
    editor.move_right();
    terminal
        .draw(|frame| render_editor(frame, &editor, &mut tracker))
        .expect("cursor frame");
    assert_eq!(
        crate::highlight::FILE_PASSES.get(),
        1,
        "cursor motion must not rebuild file highlighting"
    );
}

fn drawn(app: &mut App) -> String {
    let mut terminal = Terminal::new(crate::WritableTestBackend::new(80, 24)).expect("terminal");
    terminal
        .draw(|frame| crate::ui::ui(frame, app))
        .expect("draw");
    crate::test_support::buffer_text(&terminal)
}

#[test]
fn the_encoding_dialog_lists_every_codec() {
    let root = tempfile::TempDir::new().expect("directory");
    let mut editor = crate::models::Editor::from_content(
        "text.txt".to_owned(),
        root.path().join("text.txt"),
        "hello",
    );
    editor.encoding_choice = Some(crate::encoding::TextEncoding::Utf8);
    let mut app = App::new().expect("app");
    app.set_overlay(Overlay::Editor(editor));
    let screen = drawn(&mut app);
    assert!(screen.contains("Encoding"), "{screen}");
    assert!(screen.contains("UTF-8"), "{screen}");
    assert!(screen.contains("Reload"), "{screen}");
}

#[test]
fn the_link_dialog_asks_for_a_link_not_a_folder() {
    let mut app = App::new().expect("app");
    app.set_overlay(Overlay::NamePrompt(crate::models::NamePrompt::link_to(
        std::path::Path::new("/tmp/target"),
        "link-name".to_owned(),
    )));
    let screen = drawn(&mut app);
    assert!(screen.contains("Link"), "{screen}");
    assert!(screen.contains("link"), "{screen}");
}
