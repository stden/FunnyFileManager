//! OS associations, explicit viewing, deleted folders and menu mnemonic precedence.

use super::BehaviorWorld;
use crate::models::Overlay;
use cucumber::{given, then};

#[given(expr = "the selected document is named {string}")]
fn document(world: &mut BehaviorWorld, name: String) {
    let case = world.case();
    std::fs::write(
        case.root.path().join("left").join(&name),
        "fixture document body",
    )
    .unwrap();
    case.app.left_panel.load_entries().unwrap();
    assert!(case.app.left_panel.select_named(&name));
    case.paint();
}

#[then(expr = "the system association opener receives the literal document {string}")]
fn opened(world: &mut BehaviorWorld, name: String) {
    let case = world.case();
    let expected = case.root.path().join("left").join(name);
    assert_eq!(crate::ops::system_open::take_opened(), Some(expected));
    assert!(matches!(case.app.overlay, Overlay::None));
}

#[given("the left panel is inside a directory that disappears")]
fn disappearing(world: &mut BehaviorWorld) {
    let case = world.case();
    let directory = case.root.path().join("left/gone");
    std::fs::create_dir(&directory).unwrap();
    case.app.left_panel.navigate_to(directory.clone()).unwrap();
    std::fs::remove_dir(directory).unwrap();
}

#[then("refresh restores the nearest existing left directory")]
fn restored(world: &mut BehaviorWorld) {
    let case = world.case();
    assert_eq!(case.app.left_panel.path, case.root.path().join("left"));
    assert!(case.app.left_panel.path.is_dir());
}

#[then(expr = "the active menu heading is {word}")]
fn heading(world: &mut BehaviorWorld, expected: String) {
    let case = world.case();
    let Overlay::Menu(menu) = &case.app.overlay else {
        panic!("the menu must stay open")
    };
    assert_eq!(format!("{:?}", menu.heading), expected);
    assert!(
        case.app.pending_row.is_none(),
        "a heading shortcut must not execute a row"
    );
}
