//! Purpose: Each bracket names the partner it pairs with, in both directions.

use super::super::mate;

#[test]
fn a_square_bracket_pairs_with_its_partner_both_ways() {
    assert_eq!(mate('['), Some((b'[', b']')));
    assert_eq!(mate(']'), Some((b']', b'[')));
}
