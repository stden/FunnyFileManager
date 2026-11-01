//! Hotkey styling distinguishes enabled controls on light and dark grounds.

use super::*;

#[test]
fn hotkeys_follow_their_ground() {
    for background in [DIALOG_BG, FIELD_BG, HIGHLIGHT_BG] {
        let style = hotkey_style(Style::default().bg(background));
        assert_eq!(style.fg, Some(DIALOG_FG));
        assert!(style.add_modifier.contains(Modifier::UNDERLINED));
    }
    for base in [Style::default(), Style::default().bg(BACKGROUND)] {
        assert_eq!(hotkey_style(base).fg, Some(HOTKEY));
    }
}
