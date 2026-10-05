//! Unit tests: the rules of Far's Make folder dialog (F7) and its Rename box.
//!
//! The dialog's own logic — which control the cursor may rest on, which names
//! a field stands for, what the space bar means — is worth testing without a
//! terminal or a filesystem, neither of which a `#[test]` can arrange as
//! cheaply as calling a function.

use super::{LinkType, NameField, NamePrompt, NamePurpose};
use crate::models::FileName;

/// Every control the ring visits, from a fresh dialog, going forwards.
fn ring(prompt: &NamePrompt) -> Vec<NameField> {
    let mut prompt = prompt.clone();
    let mut seen = Vec::new();
    for _ in 0..NameField::ORDER.len() {
        prompt.advance(true);
        seen.push(prompt.field());
    }
    seen
}

// ─── Gray + / Gray −: the mask dialog ───────────────────────────────────────

/// Far offers the extension of the file under the cursor, because the
/// commonest use of the key is "all the other ones like this".
#[test]
fn the_mask_dialog_opens_on_the_extension_under_the_cursor() {
    use super::MaskPrompt;

    assert_eq!(MaskPrompt::suggest(Some("main.rs")), "*.rs");
    assert_eq!(MaskPrompt::suggest(Some("archive.tar.gz")), "*.gz");
    assert_eq!(MaskPrompt::suggest(Some("Makefile")), "*", "no extension");
    assert_eq!(
        MaskPrompt::suggest(Some(".gitignore")),
        "*",
        "a leading dot starts a name, not an extension"
    );
    assert_eq!(MaskPrompt::suggest(None), "*", "nothing under the cursor");
}

#[test]
fn the_mask_dialog_moves_between_its_two_buttons() {
    use super::{MaskPrompt, MaskPurpose};

    let mut prompt = MaskPrompt::new(MaskPurpose::Select, "*");
    assert!(!prompt.on_cancel(), "opens on OK");
    prompt.toggle_button();
    assert!(prompt.on_cancel());
    prompt.toggle_button();
    assert!(!prompt.on_cancel());
}

#[test]
fn the_two_group_keys_differ_only_in_which_way_they_move_the_selection() {
    use super::MaskPurpose;

    assert!(MaskPurpose::Select.selects());
    assert!(!MaskPurpose::Unselect.selects());
    assert_ne!(MaskPurpose::Select.title(), MaskPurpose::Unselect.title());
}

// ─── the small conversions the rest of the tree leans on ────────────────────

/// `FileName` is the newtype every mark and every rename goes through, and it
/// is built from both a borrowed and an owned name.
#[test]
fn a_file_name_can_be_built_from_either_kind_of_string() {
    let borrowed = FileName::new("notes.txt");
    let owned: FileName = String::from("notes.txt").into();
    assert_eq!(borrowed, owned);
    assert_eq!(owned.as_str(), "notes.txt");
}

mod tests_names;
mod tests_ring;
