//! The Apply command dialog's OK and Cancel answer to the mouse like the
//! keyboard: a click names the button before Enter.

use std::fs;

use super::*;

fn open_with(app: &mut App, command: &str) {
    crate::ops::apply::open(app);
    for ch in command.chars() {
        crate::popup::process_popup_input(app, KeyCode::Char(ch).into());
    }
}

fn on_cancel(app: &App) -> bool {
    let Overlay::ApplyCommand(prompt) = &app.overlay else {
        panic!("the apply dialog is open");
    };
    prompt.on_cancel()
}

#[test]
fn the_apply_dialog_buttons_name_the_button_they_land_on() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    crate::ops::apply::open(&mut app);

    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    assert!(!on_cancel(&app), "a click on OK stays on OK");
    assert_eq!(code(&mut app, 1), KeyCode::Enter);
    assert!(on_cancel(&app), "and a click on Cancel moves there");
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    assert!(!on_cancel(&app), "a click on OK after Cancel goes back");
}

/// End to end: a click on OK runs the typed command, a click on Cancel runs
/// nothing — even when the keyboard focus was left on the other button.
#[test]
fn clicking_ok_runs_the_command_and_clicking_cancel_does_not() {
    let dir = TempDir::new().expect("temp");
    fs::write(dir.path().join("a.txt"), "alpha\n").expect("a.txt");
    let mut app = app_at(&dir);
    let mut term = terminal();
    assert!(app.get_active_panel().select_named("a.txt"));

    open_with(&mut app, "cp !.! !.bak");
    crate::popup::process_popup_input(&mut app, KeyCode::Tab.into());
    assert!(on_cancel(&app), "the keyboard focus is on Cancel");
    click_text(&mut term, &mut app, "OK");
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(
        fs::read_to_string(dir.path().join("a.bak")).expect("a.bak"),
        "alpha\n"
    );

    open_with(&mut app, "touch made-by-a-click");
    click_text(&mut term, &mut app, "Cancel");
    assert!(matches!(app.overlay, Overlay::None));
    assert!(!dir.path().join("made-by-a-click").exists());
}
