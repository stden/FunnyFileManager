//! Shared fixture, key and rendered-screen steps.

use super::{BehaviorWorld, Case};
use crate::models::Overlay;
use cucumber::{given, then, when};

#[given("an isolated acceptance workspace")]
fn workspace(world: &mut BehaviorWorld) {
    world.case = Some(Case::new());
    world.case().paint();
}

#[when(expr = "I dispatch the keys {string}")]
fn dispatch_keys(world: &mut BehaviorWorld, keys: String) {
    world.case().press(&keys);
}

#[then(expr = "the rendered screen contains {string}")]
fn contains(world: &mut BehaviorWorld, expected: String) {
    let screen = world.case().screen.text();
    assert!(screen.contains(&expected), "missing {expected:?}\n{screen}");
}

#[then(expr = "the rendered screen excludes {string}")]
fn excludes(world: &mut BehaviorWorld, text: String) {
    let screen = world.case().screen.text();
    assert!(!screen.contains(&text), "unexpected {text:?}\n{screen}");
}

#[then(expr = "the {word} panel cursor selects {string}")]
fn selected(world: &mut BehaviorWorld, side: String, name: String) {
    let panel = world.case().app.panel(Case::side(&side));
    assert_eq!(panel.get_selected_entry().unwrap().name, name);
}

#[then("the terminal presents complete synchronized frames")]
fn synchronized(world: &mut BehaviorWorld) {
    let writes = world.case().screen.writes();
    let (frames, remainder) = writes.as_chunks::<16>();
    assert_ne!(frames.len(), 0);
    assert_eq!(remainder, b"");
    assert!(frames
        .iter()
        .all(|frame| frame == b"\x1b[?2026h\x1b[?2026l"));
}

#[then("the overlay is closed")]
fn overlay_closed(world: &mut BehaviorWorld) {
    let overlay = &world.case().app.overlay;
    assert!(
        matches!(overlay, Overlay::None),
        "expected no overlay, but one is open"
    );
}
