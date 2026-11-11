//! Purpose: A description file name is one plain name: a blank one, or one with
//! a path separator in it, is refused.

use super::*;

#[test]
fn a_description_name_must_be_one_plain_file_name() {
    assert_eq!(validate(SettingsPage::Descriptions, "Descript.ion"), Ok(()));
    assert_eq!(
        validate(SettingsPage::Descriptions, ""),
        Err("Enter one filename".to_owned())
    );
    assert!(validate(SettingsPage::Descriptions, "notes/x.txt").is_err());
}
