use std::path::Path;

use super::{AccessRights, CopyField, CopyPrompt, ExistingFiles};

#[cfg(test)]
mod tests_purpose;

fn prompt() -> CopyPrompt {
    CopyPrompt::new("\"notes.txt\"", Path::new("/tmp/target"))
}

#[test]
fn the_dialog_opens_on_the_destination_offering_the_other_panel() {
    let prompt = prompt();
    assert_eq!(prompt.field, CopyField::Destination);
    assert_eq!(prompt.destination, "/tmp/target");
    assert_eq!(prompt.caption(), "Copy \"notes.txt\" t&o:");
}

#[test]
fn tab_walks_every_control_and_comes_back() {
    let mut field = CopyField::Destination;
    let mut seen = vec![field];
    for _ in 0..8 {
        field = field.next();
        seen.push(field);
    }
    assert_eq!(field.next(), CopyField::Destination, "the order is a ring");
    seen.sort_by_key(|field| format!("{field:?}"));
    seen.dedup();
    assert_eq!(seen.len(), 9, "every control is reachable exactly once");
}

#[test]
fn shift_tab_is_the_exact_reverse_of_tab() {
    let mut field = CopyField::Destination;
    for _ in 0..9 {
        assert_eq!(field.next().previous(), field);
        field = field.next();
    }
}

/// The space bar belongs to the checkboxes. On the destination field it must
/// stay a space, or a path with a space in it cannot be typed.
#[test]
fn space_toggles_only_checkboxes() {
    let mut prompt = prompt();
    prompt.toggle();
    assert_eq!(prompt, prompt.clone(), "no field changed on Destination");
    assert!(!prompt.timestamps);

    prompt.field = CopyField::Timestamps;
    prompt.toggle();
    assert!(prompt.timestamps);
    prompt.toggle();
    assert!(!prompt.timestamps, "and toggles back");
}

/// Every checkbox the transfer can carry out answers to the space bar. The
/// two nothing implements are checkboxes too, and are covered by
/// [`controls_nothing_implements_cannot_be_ticked`].
#[test]
fn every_wired_checkbox_answers_to_the_space_bar() {
    for field in [
        CopyField::Timestamps,
        CopyField::SymlinkContents,
        CopyField::MultipleDestinations,
        CopyField::Filter,
    ] {
        assert!(field.is_checkbox(), "{field:?}");
        let mut prompt = prompt();
        prompt.field = field;
        let before = prompt.clone();
        prompt.toggle();
        if field.is_wired() {
            assert_ne!(prompt, before, "{field:?} must change");
        } else {
            assert_eq!(prompt, before, "{field:?} must not change");
        }
    }
}

#[test]
fn the_text_field_and_the_buttons_are_not_checkboxes() {
    for field in [
        CopyField::Destination,
        CopyField::Rights,
        CopyField::Existing,
        CopyField::Copy,
        CopyField::Cancel,
    ] {
        assert!(!field.is_checkbox(), "{field:?}");
    }
}

#[test]
fn the_arrows_cycle_the_radio_group_both_ways() {
    let mut prompt = prompt();
    prompt.field = CopyField::Rights;
    assert_eq!(prompt.rights, AccessRights::Default);
    prompt.cycle(true);
    assert_eq!(prompt.rights, AccessRights::Copy);
    prompt.cycle(true);
    assert_eq!(prompt.rights, AccessRights::Inherit);
    prompt.cycle(true);
    assert_eq!(prompt.rights, AccessRights::Default, "and wraps");
    prompt.cycle(false);
    assert_eq!(prompt.rights, AccessRights::Inherit, "backwards too");
}

#[test]
fn the_arrows_cycle_the_existing_files_choice_both_ways() {
    let mut prompt = prompt();
    prompt.field = CopyField::Existing;
    let forward: Vec<_> = (0..4)
        .map(|_| {
            prompt.cycle(true);
            prompt.existing
        })
        .collect();
    assert_eq!(
        forward,
        vec![
            ExistingFiles::Overwrite,
            ExistingFiles::Skip,
            ExistingFiles::OnlyNewer,
            ExistingFiles::Ask,
        ]
    );
    prompt.cycle(false);
    assert_eq!(prompt.existing, ExistingFiles::OnlyNewer);
}

/// `next` and `previous` must be inverses, or the arrow keys disagree about
/// where they are in the list.
#[test]
fn the_choices_are_reversible() {
    for start in [
        ExistingFiles::Ask,
        ExistingFiles::Overwrite,
        ExistingFiles::Skip,
        ExistingFiles::OnlyNewer,
    ] {
        assert_eq!(start.next().previous(), start, "{start:?}");
    }
    for start in AccessRights::ALL {
        assert_eq!(start.next().previous(), start, "{start:?}");
    }
}

#[test]
fn the_arrows_leave_the_text_field_alone() {
    let mut prompt = prompt();
    let before = prompt.clone();
    prompt.cycle(true);
    prompt.cycle(false);
    assert_eq!(prompt, before);
}

#[test]
fn a_blank_destination_is_not_ready() {
    let mut prompt = prompt();
    assert!(prompt.is_ready());
    prompt.destination = "   ".to_owned();
    assert!(
        !prompt.is_ready(),
        "blank must not fall back to the working directory"
    );
    prompt.destination = String::new();
    assert!(!prompt.is_ready());
}

#[test]
fn every_access_right_has_a_label_and_an_index() {
    for (index, right) in AccessRights::ALL.into_iter().enumerate() {
        assert_eq!(right.index(), index);
        assert!(!AccessRights::LABELS[index].is_empty());
    }
}

#[test]
fn every_existing_files_choice_has_a_label() {
    let labels: Vec<_> = [
        ExistingFiles::Ask,
        ExistingFiles::Overwrite,
        ExistingFiles::Skip,
        ExistingFiles::OnlyNewer,
    ]
    .into_iter()
    .map(ExistingFiles::label)
    .collect();
    assert_eq!(labels, vec!["Ask", "Overwrite", "Skip", "Only newer files"]);
}
