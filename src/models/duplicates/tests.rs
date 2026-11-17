use super::{Duplicates, DuplicatesField, DEFAULT_GROUPS, DEFAULT_MIN_BLOCK};

#[test]
fn a_new_dialog_starts_on_the_folder_with_the_defaults() {
    let dialog = Duplicates::new("/work/project");
    assert_eq!(dialog.folder.text(), "/work/project");
    assert_eq!(dialog.mask.text(), "*");
    assert_eq!(dialog.min_block.text(), "6");
    assert_eq!(dialog.groups.text(), "50");
    assert_eq!(DEFAULT_MIN_BLOCK, 6);
    assert_eq!(DEFAULT_GROUPS, 50);
    assert_eq!(dialog.field(), DuplicatesField::Folder);
}

#[test]
fn tab_walks_the_four_fields_then_start_and_cancel_and_wraps() {
    let mut dialog = Duplicates::new("/");
    let mut seen = Vec::new();
    for _ in 0..6 {
        dialog.tab(true);
        seen.push(dialog.field());
    }
    assert_eq!(
        seen,
        [
            DuplicatesField::Mask,
            DuplicatesField::MinBlock,
            DuplicatesField::Groups,
            DuplicatesField::Start,
            DuplicatesField::Cancel,
            DuplicatesField::Folder,
        ]
    );
    dialog.tab(false);
    assert_eq!(dialog.field(), DuplicatesField::Cancel, "backward wraps");
}

#[test]
fn typing_goes_only_to_a_text_field() {
    let mut dialog = Duplicates::new("/");
    dialog.tab(true);
    dialog
        .active_line_mut()
        .expect("mask is a text field")
        .insert_char('r');
    assert_eq!(dialog.mask.text(), "*r");
    dialog.focus(DuplicatesField::Start);
    assert!(dialog.active_line_mut().is_none());
    assert!(dialog.on_button());
}

#[test]
fn start_and_cancel_toggle_with_the_arrows() {
    let mut dialog = Duplicates::new("/");
    dialog.focus(DuplicatesField::Start);
    dialog.toggle_button();
    assert!(dialog.on_cancel());
    dialog.toggle_button();
    assert_eq!(dialog.field(), DuplicatesField::Start);
}

#[test]
fn the_arrows_leave_a_text_field_where_it_is() {
    let mut dialog = Duplicates::new("/");
    for field in [
        DuplicatesField::Folder,
        DuplicatesField::Mask,
        DuplicatesField::MinBlock,
        DuplicatesField::Groups,
    ] {
        dialog.focus(field);
        dialog.toggle_button();
        assert_eq!(dialog.field(), field);
        assert!(!dialog.on_button());
        assert!(!dialog.on_cancel());
    }
}

#[test]
fn each_text_field_edits_its_own_answer() {
    let mut dialog = Duplicates::new("/");
    for (field, mark) in [
        (DuplicatesField::Folder, 'f'),
        (DuplicatesField::Mask, 'm'),
        (DuplicatesField::MinBlock, 'b'),
        (DuplicatesField::Groups, 'g'),
    ] {
        dialog.focus(field);
        dialog
            .active_line_mut()
            .expect("a text field")
            .insert_char(mark);
    }
    assert_eq!(dialog.folder.text(), "/f");
    assert_eq!(dialog.mask.text(), "*m");
    assert_eq!(dialog.min_block.text(), "6b");
    assert_eq!(dialog.groups.text(), "50g");
}
