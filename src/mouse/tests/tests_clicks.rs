//! Purpose: Click sequences — when a second press on the same entry activates it.

use super::*;

#[test]
fn a_second_press_on_the_same_entry_activates_it() {
    let now = Instant::now();
    let mut clicks = PanelClicks::default();

    assert_eq!(clicks.press(PanelSide::Left, 4, now), PanelRowClick::Select);
    assert_eq!(
        clicks.press(PanelSide::Left, 4, now + Duration::from_millis(250)),
        PanelRowClick::Activate
    );
    assert_eq!(
        clicks.press(PanelSide::Left, 4, now + Duration::from_millis(300)),
        PanelRowClick::Select,
        "the third press starts a new sequence"
    );
}

#[test]
fn another_entry_or_a_late_press_starts_a_new_sequence() {
    let now = Instant::now();
    let at = |millis| now + Duration::from_millis(millis);
    let mut clicks = PanelClicks::default();

    assert_eq!(clicks.press(PanelSide::Left, 1, now), PanelRowClick::Select);
    assert_eq!(
        clicks.press(PanelSide::Right, 1, at(100)),
        PanelRowClick::Select,
        "the same row on the other panel is another entry"
    );
    assert_eq!(
        clicks.press(PanelSide::Right, 1, at(700)),
        PanelRowClick::Select,
        "a slow second press is not a double-click"
    );
    clicks.reset();
    assert_eq!(
        clicks.press(PanelSide::Right, 1, at(750)),
        PanelRowClick::Select,
        "an unrelated action breaks the sequence"
    );
}
