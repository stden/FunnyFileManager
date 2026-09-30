//! Purpose: Pins what each key means to the shared mask dialog, apart from any panel.

use crossterm::event::{KeyCode, KeyModifiers};

use super::{mask_key, MaskKey};
use crate::mask::Mask;
use crate::models::{ClipboardSlot, MaskPrompt, MaskPurpose};

fn offering(purpose: MaskPurpose, offered: &str) -> MaskPrompt {
    MaskPrompt::new(purpose, offered)
}

fn key(prompt: &mut MaskPrompt, code: KeyCode) -> MaskKey {
    mask_key(
        prompt,
        code,
        KeyModifiers::NONE,
        &mut ClipboardSlot::default(),
    )
}

#[test]
fn enter_on_ok_accepts_the_purpose_and_the_mask_as_typed() {
    let mut prompt = offering(MaskPurpose::Select, "*.txt");
    assert_eq!(
        key(&mut prompt, KeyCode::Enter),
        MaskKey::Accepted(MaskPurpose::Select, Mask::parse("*.txt"))
    );

    let mut prompt = offering(MaskPurpose::Unselect, "*");
    for ch in "fire*".chars() {
        assert_eq!(key(&mut prompt, KeyCode::Char(ch)), MaskKey::Editing);
    }
    assert_eq!(
        key(&mut prompt, KeyCode::Enter),
        MaskKey::Accepted(MaskPurpose::Unselect, Mask::parse("fire*"))
    );
}

#[test]
fn esc_f10_and_enter_on_cancel_cancel() {
    for code in [KeyCode::Esc, KeyCode::F(10)] {
        let mut prompt = offering(MaskPurpose::Select, "*.txt");
        assert_eq!(key(&mut prompt, code), MaskKey::Cancelled, "{code:?}");
    }
    let mut prompt = offering(MaskPurpose::Select, "*.txt");
    assert_eq!(key(&mut prompt, KeyCode::Tab), MaskKey::Editing);
    assert!(prompt.on_cancel());
    assert_eq!(key(&mut prompt, KeyCode::Enter), MaskKey::Cancelled);
}

/// On a button Left and Right walk the buttons; in the field they move the
/// caret and leave the focus alone.
#[test]
fn left_and_right_walk_the_buttons_only_from_a_button() {
    let mut prompt = offering(MaskPurpose::Select, "*.txt");
    assert_eq!(key(&mut prompt, KeyCode::Left), MaskKey::Editing);
    assert!(!prompt.on_cancel(), "in the field Left moves the caret");
    assert_eq!(prompt.after_caret(), "t");

    assert_eq!(key(&mut prompt, KeyCode::Tab), MaskKey::Editing);
    assert_eq!(key(&mut prompt, KeyCode::Right), MaskKey::Editing);
    assert!(!prompt.on_cancel(), "on Cancel Right goes back to OK");
}

/// A Control or Alt chord is not text: the field keeps its offer. (Ctrl+X
/// would cut it: that one is the clipboard's.)
#[test]
fn control_and_alt_characters_do_not_type() {
    for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT] {
        let mut prompt = offering(MaskPurpose::Select, "*.txt");
        assert_eq!(
            mask_key(
                &mut prompt,
                KeyCode::Char('q'),
                modifiers,
                &mut ClipboardSlot::default()
            ),
            MaskKey::Editing
        );
        assert_eq!(prompt.mask(), "*.txt");
        assert!(prompt.is_offered());
    }
}

#[test]
fn editing_keys_change_the_field_and_keep_the_dialog_open() {
    let mut prompt = offering(MaskPurpose::Select, "*.txt");
    for code in [
        KeyCode::Home,
        KeyCode::Delete,
        KeyCode::End,
        KeyCode::Backspace,
    ] {
        assert_eq!(key(&mut prompt, code), MaskKey::Editing, "{code:?}");
    }
    assert_eq!(prompt.mask(), ".tx");
    assert_eq!(key(&mut prompt, KeyCode::F(3)), MaskKey::Editing);
    assert_eq!(prompt.mask(), ".tx", "an unbound key changes nothing");
}
