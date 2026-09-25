//! Unit tests: the Add to archive dialog draws its title, the name, the
//! format radio and the checkbox, and records where it landed for the mouse.

use std::path::Path;

use ratatui::Terminal;

use crate::models::{App, ArchiveField, ArchiveFormat, ArchivePrompt, Overlay};
use crate::test_support::{buffer_text, WritableTestBackend};
use crate::ui::render_overlays;

fn prompt() -> ArchivePrompt {
    ArchivePrompt::new(
        vec!["notes.txt".to_owned()],
        "notes",
        Path::new("/other"),
        vec![ArchiveFormat::TarGz, ArchiveFormat::Tar, ArchiveFormat::Zip],
    )
}

fn draw(prompt: ArchivePrompt) -> (String, App) {
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::ArchivePrompt(Box::new(prompt)));
    terminal
        .draw(|f| render_overlays(f, &mut app))
        .expect("draw");
    (buffer_text(&terminal), app)
}

#[test]
fn the_dialog_shows_every_control() {
    let (screen, app) = draw(prompt());
    assert!(screen.contains("Add to archive"), "{screen}");
    assert!(
        screen.contains("Add \"notes.txt\" to the archive:"),
        "{screen}"
    );
    let archive = Path::new("/other").join("notes.tar.gz");
    assert!(
        screen.contains(archive.to_string_lossy().as_ref()),
        "{screen}"
    );
    assert!(screen.contains("(•) tar.gz ( ) tar ( ) zip"), "{screen}");
    assert!(
        screen.contains("[ ] Delete files after archiving"),
        "{screen}"
    );
    assert!(screen.contains("{ Add }"), "the default button: {screen}");
    assert!(screen.contains("[ Cancel ]"), "{screen}");
    assert!(!screen.contains("Coming in this release"), "{screen}");
    assert!(app.dialog_mouse.area.is_some(), "the frame was tracked");
}

/// The radio and the checkbox draw what the state says, and a machine
/// without `zip` does not show it.
#[test]
fn the_controls_follow_the_state() {
    let mut prompt = ArchivePrompt::new(
        vec!["a".to_owned(), "b".to_owned()],
        "work",
        Path::new("/other"),
        vec![ArchiveFormat::TarGz, ArchiveFormat::Tar],
    );
    prompt.field = ArchiveField::Format;
    prompt.cycle(true);
    prompt.delete_after = true;
    let (screen, _) = draw(prompt);
    assert!(screen.contains("Add 2 items to the archive:"), "{screen}");
    assert!(screen.contains("( ) tar.gz (•) tar"), "{screen}");
    assert!(!screen.contains("zip"), "{screen}");
    assert!(
        screen.contains("[x] Delete files after archiving"),
        "{screen}"
    );
}
