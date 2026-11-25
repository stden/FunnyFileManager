//! Unit tests: the Things catalog dialog's mouse buttons for messages and patch confirm.

use super::*;
use crate::models::things::{ThingsCatalog, ThingsDialog, ThingsState};
use crate::things::{Listing, Machine, OsInfo, Status};

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

fn confirm_patch_dialog() -> ThingsDialog {
    let machine = Machine {
        id: "vm-1".to_owned(),
        name: "web-01".to_owned(),
        hostname: "web-01.example.com".to_owned(),
        status: Status::Online,
        os_info: Some(OsInfo {
            name: "Red Hat Enterprise Linux 9.8".to_owned(),
            version: String::new(),
        }),
        ..Machine::default()
    };
    let catalog = ThingsCatalog::new(Listing::new(vec![machine.clone()], None));
    let plan = crate::things::build_plan(&[machine], crate::things::PatchScope::All, true, "p");
    let cfg = crate::things::Config::from_values(
        Some("https://things.example.com".to_owned()),
        Some("tok".to_owned()),
    )
    .expect("cfg");
    ThingsDialog::with_config(ThingsState::ConfirmPatch { catalog, plan }, cfg)
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

#[test]
fn clicking_confirm_patch_buttons_confirms_or_cancels() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let mut term = terminal();
    app.set_overlay(Overlay::Things(Box::new(confirm_patch_dialog())));
    draw(&mut app, &mut term);

    let &(_, patch_x, _) = app.dialog_mouse.buttons.first().expect("Patch button");
    let buttons_y = app.dialog_mouse.buttons_y;
    dispatch_dialog_mouse(
        &mut term,
        &mut app,
        mouse(MouseEventKind::Down(MouseButton::Left), patch_x, buttons_y),
    );
    assert!(matches!(
        &app.overlay,
        Overlay::Things(d) if matches!(d.state, ThingsState::PatchQueued { .. })
    ));
}
