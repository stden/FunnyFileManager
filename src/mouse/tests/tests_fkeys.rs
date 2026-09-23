//! Purpose: The F-key bar — how its cells tile the row and which key a column names.

use super::*;

/// The twelve cells must cover the row exactly — no gap a click can fall
/// through, and no overlap where two keys claim the same column.
#[test]
fn the_key_cells_tile_the_row_with_no_gaps_or_overlaps() {
    for width in [12, 13, 80, 85, 100, 200] {
        let widths = fkey_widths(width, KEYS);
        assert_eq!(widths.len(), KEYS);
        assert_eq!(
            widths.iter().copied().map(u32::from).sum::<u32>(),
            u32::from(width),
            "the cells must spend the whole row at width {width}"
        );

        // Every column answers with exactly one key, in order.
        let mut seen = Vec::new();
        for column in 0..width {
            seen.push(fkey_at(width, KEYS, column).expect("every column belongs to a key"));
        }
        assert!(
            seen.windows(2).all(|pair| pair[1] >= pair[0]),
            "keys must run left to right at width {width}"
        );
        assert_eq!(*seen.first().unwrap(), 1, "the row starts on F1");
        assert_eq!(*seen.last().unwrap(), 12, "and ends on F12");
    }
}

/// The remainder goes to the leftmost cells, which is what stops the last one
/// being five columns wider than its neighbours.
#[test]
fn a_remainder_widens_the_left_cells_not_the_last_one() {
    // 85 columns over 12 keys: 7 each, with 1 left over for the first.
    let widths = fkey_widths(85, KEYS);
    assert_eq!(widths[0], 8);
    assert_eq!(widths[1], 7);
    assert_eq!(*widths.last().unwrap(), 7);
}

#[test]
fn a_column_past_the_end_of_the_bar_hits_nothing() {
    assert_eq!(fkey_at(80, KEYS, 80), None);
    assert_eq!(fkey_at(80, KEYS, 999), None);
}

/// A bar narrower than the number of keys gives some cells no columns at all.
/// They must be unclickable rather than swallowing their neighbours' columns.
#[test]
fn a_bar_too_narrow_for_twelve_keys_still_answers_consistently() {
    let widths = fkey_widths(5, KEYS);
    assert_eq!(widths.iter().copied().map(u32::from).sum::<u32>(), 5);
    for column in 0..5 {
        assert!(fkey_at(5, KEYS, column).is_some(), "column {column}");
    }
}
