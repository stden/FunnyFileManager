use super::{FindFile, FindFileField};

#[test]
fn default_mask_is_star_and_cursor_is_on_it() {
    let dialog = FindFile::default();
    assert_eq!(dialog.mask.text(), "*");
    assert_eq!(dialog.field(), FindFileField::Mask);
    assert!(dialog.containing.is_empty());
}

#[test]
fn tab_walks_mask_containing_find_cancel() {
    let mut dialog = FindFile::default();
    dialog.tab(true);
    assert_eq!(dialog.field(), FindFileField::Containing);
    dialog.tab(true);
    assert_eq!(dialog.field(), FindFileField::Find);
    dialog.tab(true);
    assert!(dialog.on_cancel());
    dialog.tab(true);
    assert_eq!(dialog.field(), FindFileField::Mask);
}

#[test]
fn typing_goes_to_the_focused_field() {
    let mut dialog = FindFile::default();
    dialog.push('r');
    dialog.push('s');
    assert_eq!(dialog.mask.text(), "*rs");
    dialog.tab(true);
    dialog.push('a');
    assert_eq!(dialog.containing.text(), "a");
    dialog.backspace();
    assert!(dialog.containing.is_empty());
}

#[test]
fn tab_backwards_and_buttons_ignore_typing() {
    let mut dialog = FindFile::default();
    dialog.tab(false);
    assert!(dialog.on_cancel());
    dialog.toggle_button();
    assert_eq!(dialog.field(), FindFileField::Find);
    assert!(dialog.on_button());
    dialog.push('z');
    dialog.backspace();
    assert_eq!(dialog.mask.text(), "*");
    dialog.toggle_button();
    assert!(dialog.on_cancel());
    dialog.tab(false);
    assert_eq!(dialog.field(), FindFileField::Find);
    dialog.toggle_button();
    dialog.toggle_button();
    dialog.tab(false);
    assert_eq!(dialog.field(), FindFileField::Containing);
    let mut idle = FindFile::default();
    idle.toggle_button();
    assert_eq!(idle.field(), FindFileField::Mask);
}

#[test]
fn active_line_mut_is_the_mask_until_the_cursor_leaves_the_fields() {
    let mut dialog = FindFile::default();
    dialog.active_line_mut().expect("mask").insert_str("x");
    assert_eq!(dialog.mask.text(), "*x");
    dialog.tab(true);
    dialog.tab(true);
    assert!(dialog.active_line_mut().is_none());
}
