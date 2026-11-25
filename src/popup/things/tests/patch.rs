//! Unit tests: F5 opens patch confirmation in the Things dialog and Enter/Esc resolves it.

use crossterm::event::KeyCode;

use super::{app_with, press};
use crate::models::things::{ThingsCatalog, ThingsDialog, ThingsState};
use crate::models::{App, Overlay};
use crate::things::{Config, Listing, Machine, OsInfo, Status};

fn ready_app() -> App {
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
    let config = Config::from_values(
        Some("https://things.example.com".to_owned()),
        Some("tok".to_owned()),
    )
    .expect("cfg");
    let mut app = app_with(ThingsState::Loading);
    app.set_overlay(Overlay::Things(Box::new(ThingsDialog::with_config(
        ThingsState::Ready(ThingsCatalog::new(Listing::new(vec![machine], None))),
        config,
    ))));
    app
}

#[test]
fn f5_opens_patch_confirmation_and_esc_or_enter_resolves_it() {
    let mut app = ready_app();
    press(&mut app, KeyCode::F(5));
    press(&mut app, KeyCode::Char('q'));
    assert!(matches!(
        &app.overlay,
        Overlay::Things(d) if matches!(d.state, ThingsState::ConfirmPatch { .. })
    ));
    press(&mut app, KeyCode::Esc);
    assert!(matches!(
        &app.overlay,
        Overlay::Things(d) if matches!(d.state, ThingsState::Ready(_))
    ));
    press(&mut app, KeyCode::F(5));
    press(&mut app, KeyCode::Enter);
    assert!(matches!(
        &app.overlay,
        Overlay::Things(d) if matches!(d.state, ThingsState::PatchQueued { .. })
    ));
}
