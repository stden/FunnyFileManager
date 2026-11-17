//! Purpose: Preview, confirmation defaults and deletion results preserve every protected fixture.
use super::{old_file, BehaviorWorld};
use crate::models::{
    temp_cleanup::{TempChoice, TempCleanupState},
    Confirm, Overlay,
};
use cucumber::then;

#[then("the temporary cleanup preview includes only the old file and its bytes")]
fn preview_plan(world: &mut BehaviorWorld) {
    let expected = std::fs::canonicalize(old_file(world)).unwrap();
    let Overlay::TempCleanup(dialog) = &world.case().app.overlay else {
        panic!("temporary cleanup must remain open");
    };
    let TempCleanupState::Preview { plan, focus } = &dialog.state else {
        panic!("a preview must be shown before deleting anything");
    };
    assert_eq!(plan.files.len(), 1);
    assert_eq!(plan.files[0].path, expected);
    assert_eq!(plan.bytes.bytes(), 128);
    assert_eq!(*focus, TempChoice::Cancel);
}

#[then("temporary cleanup confirmation defaults to No")]
fn default_no(world: &mut BehaviorWorld) {
    let Overlay::TempCleanup(dialog) = &world.case().app.overlay else {
        panic!("temporary cleanup must remain open");
    };
    let TempCleanupState::Confirm { answer, .. } = &dialog.state else {
        panic!("deletion must require a separate confirmation");
    };
    assert_eq!(*answer, Confirm::No);
}

#[then("all temporary cleanup fixture files remain unchanged")]
fn all_kept(world: &mut BehaviorWorld) {
    let fixture = world.temp_cleanup.as_ref().unwrap();
    fixture.assert_old_content();
    fixture.assert_protected_content();
}

#[then("only the old temporary file has been removed")]
fn only_old_removed(world: &mut BehaviorWorld) {
    let fixture = world.temp_cleanup.as_ref().unwrap();
    assert!(!fixture.old.exists());
    fixture.assert_protected_content();
}

#[then("the changed temporary file is retained")]
fn changed_retained(world: &mut BehaviorWorld) {
    let fixture = world.temp_cleanup.as_ref().unwrap();
    fixture.assert_old_content();
    fixture.assert_protected_content();
}

#[then("the temporary cleanup dialog is closed")]
fn closed(world: &mut BehaviorWorld) {
    assert!(matches!(world.case().app.overlay, Overlay::None));
}
