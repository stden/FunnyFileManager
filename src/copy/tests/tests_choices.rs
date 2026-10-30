//! Purpose: The copy dialog's choices: the access-rights and existing-files
//! radio groups, their labels, and the rules they hand to the copy.

use super::*;

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
fn every_access_right_has_a_label_and_an_index() {
    for (index, right) in AccessRights::ALL.into_iter().enumerate() {
        assert_eq!(right.index(), index);
        assert_ne!(AccessRights::LABELS[index].len(), 0);
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

/// "Access rights" decide each copied file's mode as it is written: only
/// Inherit leaves the file as the kernel created it, umask and all.
#[test]
fn only_inherit_leaves_a_copied_file_as_it_was_created() {
    use crate::utils::FileMode;

    assert_eq!(AccessRights::Default.file_mode(), FileMode::Source);
    assert_eq!(AccessRights::Copy.file_mode(), FileMode::Source);
    assert_eq!(AccessRights::Inherit.file_mode(), FileMode::Fresh);
}

/// The rules handed to the copy carry every dialog choice that reaches the
/// entries: the link box, the access rights for files and for folders (only
/// Copy restates a folder's mode), and the timestamps box.
#[test]
fn the_copy_rules_carry_the_link_choice_the_rights_and_the_times() {
    use crate::utils::{FileMode, LinkPolicy, Times};

    let mut dialog = prompt();
    dialog.rights = AccessRights::Inherit;
    let rules = dialog.rules();
    assert_eq!(rules.links, LinkPolicy::Recreate);
    assert_eq!(rules.modes, FileMode::Fresh);
    assert_eq!(rules.folder_modes, FileMode::Fresh);
    assert_eq!(rules.times, Times::Fresh);
    dialog.rights = AccessRights::Copy;
    dialog.timestamps = true;
    let rules = dialog.rules();
    assert_eq!(rules.folder_modes, FileMode::Source);
    assert_eq!(rules.times, Times::Source);
    assert_eq!(AccessRights::Default.folder_mode(), FileMode::Fresh);
}

/// A move copies only across filesystems, standing in for a rename that
/// keeps everything: it keeps every time and folder mode unless Inherit
/// asks for the destination's rights.
#[test]
fn a_move_keeps_times_and_folder_modes_as_mv_does() {
    use crate::copy::Purpose;
    use crate::utils::{FileMode, Times};

    let mut dialog = prompt();
    dialog.purpose = Purpose::Move;
    let rules = dialog.rules();
    assert_eq!(rules.times, Times::Source);
    assert_eq!(rules.folder_modes, FileMode::Source);
    assert_eq!(rules, crate::utils::CopyRules::MOVE);
    dialog.rights = AccessRights::Inherit;
    assert_eq!(dialog.rules().folder_modes, FileMode::Fresh);
}
