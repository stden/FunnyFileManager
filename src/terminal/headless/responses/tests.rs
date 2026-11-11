//! Purpose: Terminal queries may be split across arbitrary output reads.

use super::*;

#[test]
fn cursor_queries_are_recognized_across_read_boundaries() {
    let mut queries = Queries::default();
    assert_eq!(queries.feed(b"text\x1b["), 0);
    assert_eq!(queries.feed(b"6"), 0);
    assert_eq!(queries.feed(b"n\x1b[6n"), 2);
    assert_eq!(queries.feed(b"\x1b[16n"), 0);
}
