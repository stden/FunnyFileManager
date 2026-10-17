use ratatui::Terminal;

use crate::models::{App, FindFile, FindFileField, Overlay};
use crate::test_support::{buffer_text, WritableTestBackend};

#[test]
fn the_find_file_dialog_is_titled_find_file_not_make_folder() {
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    let mut app = App::new().expect("app");
    app.set_overlay(Overlay::FindFile(Box::default()));
    terminal.draw(|f| crate::ui::ui(f, &mut app)).expect("draw");
    let screen = buffer_text(&terminal);
    assert!(screen.contains("Find file"), "{screen}");
    assert!(screen.contains("File masks"), "{screen}");
    assert!(screen.contains("Containing text"), "{screen}");
    assert!(
        !screen.contains("Create the folder"),
        "must not be the F7 dialog:\n{screen}"
    );
}

#[test]
fn a_focused_find_button_does_not_place_the_text_caret() {
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    let mut app = App::new().expect("app");
    let mut dialog = FindFile::default();
    dialog.tab(true);
    dialog.tab(true);
    assert_eq!(dialog.field(), FindFileField::Find);
    app.set_overlay(Overlay::FindFile(Box::new(dialog)));
    terminal.draw(|f| crate::ui::ui(f, &mut app)).expect("draw");
    let screen = buffer_text(&terminal);
    assert!(screen.contains("Find file"), "{screen}");
}
