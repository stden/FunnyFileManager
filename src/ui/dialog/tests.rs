//! Purpose: Pin the shadow's colour scaling at the ends of the `u8` range.

use super::row::Row;
use super::shadow::shadowed_component;

#[test]
fn a_shadow_keeps_three_eighths_of_each_colour_component() {
    assert_eq!(shadowed_component(0), 0);
    assert_eq!(shadowed_component(8), 3);
    assert_eq!(shadowed_component(9), 3, "integer maths rounds down");
    assert_eq!(shadowed_component(u8::MAX), 95);
}

#[test]
fn text_field_follows_its_focus_and_enabled_flags() {
    let row = Row::text_field("Name", "notes", true, true);
    assert!(row.is_focused());
    assert!(row.is_enabled());
    let idle = Row::text_field("Name", "notes", false, false);
    assert!(!idle.is_focused());
    assert!(!idle.is_enabled());
}
