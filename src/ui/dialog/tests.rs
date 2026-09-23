//! Purpose: Pin the shadow's colour scaling at the ends of the `u8` range.

use super::shadowed_component;

#[test]
fn a_shadow_keeps_three_eighths_of_each_colour_component() {
    assert_eq!(shadowed_component(0), 0);
    assert_eq!(shadowed_component(8), 3);
    assert_eq!(shadowed_component(9), 3, "integer maths rounds down");
    assert_eq!(shadowed_component(u8::MAX), 95);
}
