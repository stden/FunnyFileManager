//! Purpose: an Options page's row — no key, Far's command, and the title
//! its dialog shows as the summary.

use super::*;
use crate::keymap::ActionName;

#[test]
fn a_page_is_a_keyless_far_command_under_its_dialogs_title() {
    let colors = page("settings_colors", Page::Colors);
    assert_eq!(colors.name, ActionName::new("settings_colors"));
    assert_eq!(colors.command, Menu(Settings(Page::Colors)));
    assert_eq!(colors.defaults, []);
    assert_eq!(colors.origin, FarNoKey);
    assert_eq!(colors.summary, "Colors");
}

#[test]
fn every_page_of_the_options_menu_has_its_row() {
    let pages: Vec<&str> = OPTIONS.rows.iter().map(|row| row.summary).collect();
    assert_eq!(pages.len(), 22);
    assert_eq!(pages.first(), Some(&Page::System.title()));
    assert_eq!(pages.last(), Some(&Page::Highlighting.title()));
    for row in OPTIONS.rows {
        assert!(row.name.as_str().starts_with("settings_"), "{}", row.name);
        assert!(matches!(row.command, Menu(Settings(_))), "{}", row.name);
    }
}
