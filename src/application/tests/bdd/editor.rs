//! F4 editor: open, text input, unmodified-close and save-prompt, no LSP.

use super::BehaviorWorld;
use cucumber::{then, when};

#[then("the editor is open")]
fn editor_open(world: &mut BehaviorWorld) {
    assert!(
        world.case().app.get_editor().is_some(),
        "expected an Editor overlay to be active"
    );
}

/// Inserts `text` at the current editor cursor position via a paste event.
/// This exercises the same code path as a terminal paste (Ctrl+Shift+V /
/// bracketed-paste), which is the fastest way to type more than one character
/// without simulating individual key events.
#[when(expr = "I type the text {string}")]
fn type_text(world: &mut BehaviorWorld, text: String) {
    world.case().type_text(&text);
}
