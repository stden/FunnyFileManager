//! Unit tests: the Make folder dialog's whole box, the link group's target
//! field, the Rename box without it, and a dialog taller than the screen.

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::overlay_mask::screen_text;
use super::*;

// -----------------------------------------------------------------------
// The Make folder dialog, in both of its states
// -----------------------------------------------------------------------

/// Far's F7 box: the name field, the rule, and the group under it — the link
/// type, the target it points at, and the checkbox that makes the name field
/// a list.
#[test]
fn the_make_folder_dialog_draws_fars_whole_box() {
    use crate::models::NamePrompt;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = screen_text(&terminal);

    for expected in [
        "Make folder",
        "Create the folder:",
        "Link type:",
        "none",
        "Target:",
        "Process multiple names",
        "OK",
        "Cancel",
    ] {
        assert!(screen.contains(expected), "missing {expected:?}: {screen}");
    }
}

/// With a link chosen the Target field stops being grey and takes the caret,
/// which is the only visible difference between the two states.
#[test]
fn choosing_a_link_lights_up_the_target_field() {
    use crate::models::{NameField, NamePrompt};

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    let mut prompt = NamePrompt::make_folder();
    prompt.focus(NameField::Link);
    prompt.cycle(true);
    prompt.focus(NameField::Target);
    for ch in "/srv/data".chars() {
        prompt.push(ch);
    }
    app.set_overlay(Overlay::NamePrompt(prompt));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = screen_text(&terminal);

    assert!(screen.contains("symbolic link"), "{screen}");
    assert!(
        screen.contains("/srv/data"),
        "the target is shown: {screen}"
    );
}

/// The rename box is the same dialog with the link group left out — an absent
/// control says "not for this key", where a grey one would say "not yet".
#[test]
fn the_rename_box_leaves_the_link_group_out_altogether() {
    use crate::models::{FileName, NamePrompt};

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::NamePrompt(NamePrompt::rename(FileName::new(
        "notes.txt",
    ))));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = screen_text(&terminal);

    assert!(screen.contains("Rename notes.txt to:"), "{screen}");
    assert!(
        !screen.contains("Link type"),
        "no link group here: {screen}"
    );
    assert!(!screen.contains("Process multiple"), "{screen}");
}

/// A dialog taller than the terminal draws what fits rather than panicking or
/// spilling over the panels below it.
#[test]
fn a_dialog_taller_than_the_screen_draws_what_fits() {
    use crate::models::NamePrompt;

    let backend = TestBackend::new(40, 9);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    assert!(screen_text(&terminal).contains("Make folder"));
}
