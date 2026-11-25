//! Purpose: unit tests for the controls each name dialog offers, and moving and
//! clicking between them.

use super::*;

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
