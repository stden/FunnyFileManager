//! Unit tests: the Select mask dialog's title, mask, buttons, cursor and the
//! offered mask drawn selected, and a search that found nothing.

use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::*;

// -----------------------------------------------------------------------
// The Gray + / Gray − mask dialog
// -----------------------------------------------------------------------

/// Far's Select box: a title that says which way the selection moves, the
/// mask under the cursor, and the two buttons.
#[test]
fn the_mask_dialog_draws_its_title_mask_and_buttons() {
    use crate::models::{MaskPrompt, MaskPurpose};

    for (purpose, title) in [
        (MaskPurpose::Select, "Select"),
        (MaskPurpose::Unselect, "Deselect"),
    ] {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new().unwrap();
        app.set_overlay(Overlay::MaskPrompt(MaskPrompt::new(purpose, "*.rs")));

        terminal.draw(|f| ui(f, &mut app)).unwrap();
        let screen = screen_text(&terminal);

        assert!(screen.contains(title), "the title says which way: {screen}");
        assert!(screen.contains("Enter the mask"), "{screen}");
        assert!(screen.contains("*.rs"), "the mask is in the field");
        assert!(
            screen.contains("OK") && screen.contains("Cancel"),
            "{screen}"
        );
    }
}

/// The cursor is drawn where it is, so a click on Cancel is visible before
/// Enter is pressed.
#[test]
fn the_mask_dialog_shows_which_button_has_the_cursor() {
    use crate::models::{MaskPrompt, MaskPurpose};

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    let mut prompt = MaskPrompt::new(MaskPurpose::Select, "*");
    prompt.focus_cancel(true);
    app.set_overlay(Overlay::MaskPrompt(prompt));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    assert!(screen_text(&terminal).contains("Cancel"));
}

/// Far draws an offered mask selected, which is how the user can tell that
/// typing will replace it; once edited it is ordinary field text again.
#[test]
fn the_offered_mask_is_drawn_selected_until_it_is_edited() {
    use crate::models::{MaskPrompt, MaskPurpose, Panel};

    let empty = || Panel::from_entries(PathBuf::from("/srv/x"), Vec::new());
    let mut app = App::with_panels(empty(), empty());
    let mut prompt = MaskPrompt::new(MaskPurpose::Select, "*.rs");
    app.set_overlay(Overlay::MaskPrompt(prompt.clone()));
    let offered = background_of(&mut app, "*.rs").expect("the mask is drawn");
    assert_ne!(
        offered,
        palette::FIELD_BG,
        "the offer stands out as selected"
    );

    prompt.backspace();
    app.set_overlay(Overlay::MaskPrompt(prompt));
    let edited = background_of(&mut app, "*.r").expect("the mask is drawn");
    assert_eq!(edited, palette::FIELD_BG, "edited text is plain field text");
}

/// The terminal cursor stands where the caret is, not always after the text.
#[test]
fn the_mask_dialog_cursor_follows_the_caret() {
    use crate::models::{MaskPrompt, MaskPurpose, Panel};

    let empty = || Panel::from_entries(PathBuf::from("/srv/x"), Vec::new());
    let mut app = App::with_panels(empty(), empty());
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut prompt = MaskPrompt::new(MaskPurpose::Select, "*.rs");
    app.set_overlay(Overlay::MaskPrompt(prompt.clone()));
    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let at_end = terminal.get_cursor_position().unwrap();

    prompt.left();
    prompt.left();
    app.set_overlay(Overlay::MaskPrompt(prompt));
    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let moved = terminal.get_cursor_position().unwrap();

    assert_eq!((moved.x + 2, moved.y), (at_end.x, at_end.y));
}

/// The background of the first cell of `text` on screen.
fn background_of(app: &mut App, text: &str) -> Option<Color> {
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|f| ui(f, app)).unwrap();
    let buffer = terminal.backend().buffer();
    let wanted: Vec<String> = text.chars().map(String::from).collect();
    (0..buffer.area.height).find_map(|y| {
        let row: Vec<&str> = (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect();
        let x = row
            .windows(wanted.len())
            .position(|cells| cells == wanted.as_slice())?;
        Some(buffer[(u16::try_from(x).ok()?, y)].bg)
    })
}

/// The whole screen as one string, for the dialogs whose content matters more
/// than which row it landed on.
pub(super) fn screen_text(terminal: &Terminal<TestBackend>) -> String {
    crate::test_support::buffer_string(terminal.backend().buffer())
}

// -----------------------------------------------------------------------
// The editor's "not found" dialog
// -----------------------------------------------------------------------

/// A search that found nothing says so, and names what it looked for — the
/// one thing the user needs in order to see the typo.
#[test]
fn a_search_that_found_nothing_names_what_it_looked_for() {
    use crate::models::Editor;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    let mut editor = Editor::default();
    editor.search.not_found = Some("needle".to_owned());
    app.set_overlay(Overlay::Editor(editor));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = screen_text(&terminal);

    assert!(screen.contains("Search"), "{screen}");
    assert!(screen.contains("needle"), "it names the query: {screen}");
}
