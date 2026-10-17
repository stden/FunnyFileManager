//! Unit tests: the nesting scan counts every level the parser would recurse
//! into, and nothing a comment, CDATA or an attribute value holds.

use super::nested_deeper_than;

#[test]
fn the_limit_is_accepted_and_one_level_more_is_not() {
    assert!(!nested_deeper_than("<a><b><c/></b></a>", 3));
    assert!(nested_deeper_than("<a><b><c/></b></a>", 2));
    assert!(!nested_deeper_than("<a><b>text</b><b>x</b></a>", 2));
    assert!(!nested_deeper_than("", 0));
    assert!(nested_deeper_than("<a>", 0));
}

#[test]
fn siblings_and_closed_elements_do_not_add_up() {
    let siblings = "<a/>".repeat(1000);
    assert!(!nested_deeper_than(&format!("<r>{siblings}</r>"), 2));
    let closed = "<a></a>".repeat(1000);
    assert!(!nested_deeper_than(&format!("<r>{closed}</r>"), 2));
}

/// Nothing inside a comment, CDATA, a processing instruction or an attribute
/// value closes an element, so no text can hide real nesting from the scan.
#[test]
fn markup_inside_comments_cdata_and_values_hides_no_level() {
    for filler in [
        "<!-- </a></a></a> -->",
        "<![CDATA[</a></a></a>]]>",
        "<?pi </a></a></a>?>",
        "<x v=\"/>\"/>",
    ] {
        let deep = format!("<a>{filler}<a><a><a></a></a></a></a>");
        assert!(nested_deeper_than(&deep, 3), "{filler}");
        assert!(!nested_deeper_than(&deep, 4), "{filler}");
    }
    let opening_values = "<a v='>' w=\"/>\">".repeat(5);
    assert!(nested_deeper_than(&opening_values, 4));
}

/// Malformed markup is counted as deep as it could go: the parser stops at
/// it, so an overcount refuses only a file that would not parse.
#[test]
fn a_cut_short_tag_counts_as_a_level() {
    assert!(nested_deeper_than("<a v=\"<b><c>", 2));
    assert!(nested_deeper_than("<a<b<c", 2));
    assert!(nested_deeper_than("<a><b><c v='never closed", 2));
    assert!(nested_deeper_than("<!DOCTYPE x><a><b><c>", 2));
    assert!(!nested_deeper_than("<!-- never closed <a><b><c>", 0));
}
