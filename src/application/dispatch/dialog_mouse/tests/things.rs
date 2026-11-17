//! Purpose: the Close button of the Things catalog dialog, clicked with the
//! pointer, in every state that shows a message instead of a list.

use super::*;
use crate::models::things::{ThingsCatalog, ThingsDialog, ThingsState};
use crate::things::{Listing, Machine};

/// Each state that shows a message, and so a Close button.
fn message_states() -> Vec<(&'static str, ThingsState)> {
    let machine = Machine {
        name: "web-01".to_owned(),
        ..Machine::default()
    };
    let mut filtered = ThingsCatalog::new(Listing::new(vec![machine], None));
    filtered.type_filter('z');
    vec![
        (
            "setup",
            ThingsState::Setup("Set THINGS_URL and THINGS_API_TOKEN to browse Things".to_owned()),
        ),
        (
            "failed read",
            ThingsState::Failed("Things could not be read".to_owned()),
        ),
        ("loading", ThingsState::Loading),
        (
            "no machines listed",
            ThingsState::Ready(ThingsCatalog::new(Listing::new(Vec::new(), None))),
        ),
        ("filter hides all", ThingsState::Ready(filtered)),
    ]
}

#[test]
fn clicking_close_closes_the_things_dialog_in_every_message_state() {
    let dir = TempDir::new().expect("temp");
    for (case, state) in message_states() {
        let mut app = app_at(&dir);
        let mut term = terminal();
        app.set_overlay(Overlay::Things(Box::new(ThingsDialog::new(state))));
        draw(&mut app, &mut term);

        let &(index, left, _right) = app
            .dialog_mouse
            .buttons
            .first()
            .expect("the message drew its Close button");
        assert_eq!(index, 0, "{case}: the only button is Close");
        let buttons_y = app.dialog_mouse.buttons_y;
        dispatch_dialog_mouse(
            &mut term,
            &mut app,
            mouse(MouseEventKind::Down(MouseButton::Left), left, buttons_y),
        );

        assert!(
            matches!(app.overlay, Overlay::None),
            "{case}: a click on Close left the dialog open"
        );
    }
}
