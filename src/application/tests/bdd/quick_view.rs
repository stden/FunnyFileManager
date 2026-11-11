//! Quick view (`Ctrl+Q`) acceptance steps: passive preview, live cursor follow,
//! binary refusal, and folder summary vs README description.

use super::BehaviorWorld;
use crate::models::PanelKind;
use cucumber::{given, then};

#[given(expr = "a left folder {string} with {int} files")]
fn left_folder_with_files(world: &mut BehaviorWorld, folder: String, count: usize) {
    let case = world.case();
    let dir = case.root.path().join("left").join(&folder);
    std::fs::create_dir_all(&dir).unwrap();
    for index in 0..count {
        std::fs::write(dir.join(format!("item_{index}.txt")), "sample").unwrap();
    }
    case.app.left_panel.load_entries().unwrap();
    case.paint();
}

#[given(expr = "a left folder {string} described by README.md containing {string}")]
fn left_folder_with_readme(world: &mut BehaviorWorld, folder: String, body: String) {
    let case = world.case();
    let dir = case.root.path().join("left").join(&folder);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("README.md"), body).unwrap();
    case.app.left_panel.load_entries().unwrap();
    case.paint();
}

#[then("the right panel is a quick view")]
fn right_is_quick_view(world: &mut BehaviorWorld) {
    assert!(
        matches!(world.case().app.right_panel.kind, PanelKind::QuickView(_)),
        "expected the right panel to be in QuickView mode"
    );
}
