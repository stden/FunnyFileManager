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

#[test]
fn a_new_folder_dialog_opens_on_the_name_with_nothing_chosen() {
    let prompt = NamePrompt::make_folder();
    assert_eq!(prompt.field(), NameField::Name);
    assert_eq!(prompt.link(), LinkType::None);
    assert_eq!(prompt.link_target(), "");
    assert!(!prompt.multiple());
    assert_eq!(prompt.names(), Vec::<&str>::new(), "nothing typed yet");
}

/// The Target field is greyed until there is a link for it to describe, and
/// the cursor must not stop on a control that does nothing.
#[test]
fn the_target_field_joins_the_ring_only_once_a_link_is_chosen() {
    let mut prompt = NamePrompt::make_folder();
    assert!(!ring(&prompt).contains(&NameField::Target));

    prompt.focus(NameField::Link);
    prompt.cycle(true);
    assert_eq!(prompt.link(), LinkType::Symbolic);
    assert!(ring(&prompt).contains(&NameField::Target));
}

/// Shift+F6 is Far's plain Rename box: a name and two buttons, nothing else.
#[test]
fn the_rename_box_offers_only_the_name_and_the_buttons() {
    let prompt = NamePrompt::rename(FileName::new("notes.txt"));
    assert_eq!(prompt.purpose(), NamePurpose::Rename);
    let mut visited = ring(&prompt);
    visited.sort_by_key(|field| format!("{field:?}"));
    visited.dedup();
    assert_eq!(
        visited,
        vec![NameField::Cancel, NameField::Name, NameField::Ok],
        "the link controls belong to F7"
    );
}

/// A greyed control cannot be reached by clicking it either.
#[test]
fn focus_refuses_a_control_the_dialog_does_not_offer() {
    let mut prompt = NamePrompt::make_folder();
    prompt.focus(NameField::Target);
    assert_eq!(prompt.field(), NameField::Name, "Target is greyed");

    let mut prompt = NamePrompt::rename(FileName::new("notes.txt"));
    prompt.focus(NameField::Multiple);
    assert_eq!(prompt.field(), NameField::Name, "Rename has no checkbox");
}

#[test]
fn typing_lands_in_the_field_under_the_cursor() {
    let mut prompt = NamePrompt::make_folder();
    for ch in "docs".chars() {
        prompt.push(ch);
    }
    prompt.focus(NameField::Link);
    prompt.cycle(true);
    prompt.focus(NameField::Target);
    for ch in "/srv/docs".chars() {
        prompt.push(ch);
    }
    assert_eq!(prompt.name(), "docs");
    assert_eq!(prompt.link_target(), "/srv/docs");

    prompt.backspace();
    assert_eq!(prompt.link_target(), "/srv/doc");
    assert_eq!(prompt.name(), "docs", "the other field is untouched");
}

/// Typing has nowhere to go while the cursor is on a button or a checkbox, and
/// must not silently land in whichever field was edited last.
#[test]
fn typing_on_a_button_changes_nothing() {
    let mut prompt = NamePrompt::make_folder();
    prompt.push('a');
    prompt.focus(NameField::Ok);
    prompt.push('b');
    prompt.backspace();
    assert_eq!(prompt.name(), "a");
}

#[test]
fn the_checkbox_answers_the_space_bar_and_the_text_fields_do_not() {
    let mut prompt = NamePrompt::make_folder();
    assert!(prompt.field().is_text(), "a space is a space in the name");

    prompt.focus(NameField::Multiple);
    assert!(!prompt.field().is_text());
    prompt.toggle();
    assert!(prompt.multiple());
    prompt.toggle();
    assert!(!prompt.multiple());
}

/// The checkbox is what turns the name field into a list, so a `;` typed
/// without it names one folder with a semicolon in it.
#[test]
fn multiple_names_splits_the_field_and_the_checkbox_is_the_switch() {
    let mut prompt = NamePrompt::make_folder();
    for ch in "one;two;three".chars() {
        prompt.push(ch);
    }
    assert_eq!(prompt.names(), vec!["one;two;three"]);

    prompt.focus(NameField::Multiple);
    prompt.toggle();
    assert_eq!(prompt.names(), vec!["one", "two", "three"]);
}

/// A list is normally typed with a trailing separator, and the blank that
/// leaves is not an error.
#[test]
fn blank_parts_of_a_list_are_dropped_rather_than_failing_it() {
    let mut prompt = NamePrompt::make_folder();
    prompt.focus(NameField::Multiple);
    prompt.toggle();
    prompt.focus(NameField::Name);
    for ch in " one ; ; two; ".chars() {
        prompt.push(ch);
    }
    assert_eq!(prompt.names(), vec!["one", "two"]);
}

/// A separator would put the folder somewhere else, which is what F7 is not
/// for — and the rule has to survive the list, or one bad part smuggles it in.
#[test]
fn a_name_carrying_a_path_separator_is_refused_in_a_list_too() {
    let mut prompt = NamePrompt::make_folder();
    prompt.focus(NameField::Multiple);
    prompt.toggle();
    prompt.focus(NameField::Name);
    for ch in "good;../escape;also/bad".chars() {
        prompt.push(ch);
    }
    assert_eq!(prompt.names(), vec!["good"]);
}

#[test]
fn a_rename_to_the_same_name_is_nothing_to_do() {
    let mut prompt = NamePrompt::rename(FileName::new("notes.txt"));
    assert_eq!(prompt.target(), None, "unchanged");
    prompt.push('!');
    assert_eq!(prompt.target(), Some("notes.txt!"));
}

#[test]
fn the_link_drop_down_is_a_ring_in_both_directions() {
    let mut prompt = NamePrompt::make_folder();
    prompt.focus(NameField::Link);
    for _ in 0..LinkType::FOLDER_CHOICES.len() {
        prompt.cycle(true);
    }
    assert_eq!(prompt.link(), LinkType::None, "a full turn comes back");
    prompt.cycle(false);
    assert_eq!(prompt.link(), LinkType::Symbolic);
}

/// The arrows belong to the control under the cursor: anywhere but the
/// drop-down they must leave the answer alone.
#[test]
fn the_arrows_do_nothing_away_from_the_drop_down() {
    let mut prompt = NamePrompt::make_folder();
    prompt.cycle(true);
    assert_eq!(prompt.link(), LinkType::None);
}

/// Every control the dialog draws has a row, and no row names a control that
/// is not there — the mouse and the renderer have to agree, or a click lands
/// on the wrong thing.
#[test]
fn every_row_the_mouse_can_hit_names_a_control_the_dialog_offers() {
    let mut prompt = NamePrompt::make_folder();
    prompt.focus(NameField::Link);
    prompt.cycle(true);

    let hit: Vec<NameField> = (0..8).filter_map(|row| prompt.field_at_row(row)).collect();
    assert_eq!(
        hit,
        vec![
            NameField::Name,
            NameField::Link,
            NameField::Target,
            NameField::Multiple
        ],
        "the row order the renderer draws"
    );

    let rename = NamePrompt::rename(FileName::new("notes.txt"));
    let hit: Vec<NameField> = (0..8).filter_map(|row| rename.field_at_row(row)).collect();
    assert_eq!(hit, vec![NameField::Name], "Rename draws nothing else");
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
