//! Purpose: Pin the saturating cell-to-screen-coordinate conversion at its boundaries.

use super::screen_u16;

#[test]
fn screen_u16_keeps_every_value_a_terminal_can_address() {
    assert_eq!(screen_u16(0), 0);
    assert_eq!(screen_u16(80), 80);
    assert_eq!(screen_u16(usize::from(u16::MAX)), u16::MAX);
}

/// `usize` has no negative values; overflow is the only way out of range.
#[test]
fn screen_u16_saturates_past_the_last_coordinate_instead_of_wrapping() {
    assert_eq!(screen_u16(usize::from(u16::MAX) + 1), u16::MAX);
    assert_eq!(screen_u16(usize::MAX), u16::MAX);
}
