//! Unit tests: attribute rendering on both platforms, timestamp fallbacks.

use super::*;

#[test]
fn mode_bits_render_like_ls() {
    assert_eq!(Attributes::from_bits(0o100_644).mode_string(), "-rw-r--r--");
    assert_eq!(Attributes::from_bits(0o040_755).mode_string(), "drwxr-xr-x");
    assert_eq!(Attributes::from_bits(0o120_777).mode_string(), "lrwxrwxrwx");
    assert_eq!(Attributes::from_bits(0o104_755).mode_string(), "-rwsr-xr-x");
    assert_eq!(Attributes::from_bits(0o102_644).mode_string(), "-rw-r-Sr--");
    assert_eq!(Attributes::from_bits(0o041_777).mode_string(), "drwxrwxrwt");
    assert_eq!(Attributes::from_bits(0o000_000).mode_string(), "?---------");
}

#[test]
fn windows_flags_render_as_far_letters_in_order() {
    assert_eq!(Attributes::from_bits(0x27).flag_letters(), "RHSA");
    assert_eq!(Attributes::from_bits(0x21).flag_letters(), "RA");
    assert_eq!(Attributes::from_bits(0x10).flag_letters(), "");
}

#[test]
fn display_uses_the_platform_renderer() {
    let attributes = Attributes::from_bits(if cfg!(windows) { 0x21 } else { 0o100_644 });
    let expected = if cfg!(windows) { "RA" } else { "-rw-r--r--" };
    assert_eq!(attributes.to_string(), expected);
    assert_eq!(attributes.bits(), attributes.bits());
}

#[test]
fn a_refused_timestamp_is_unknown_and_sorts_first() {
    let refused = Modified::from_result(Err(std::io::Error::other("no ctime")));
    assert_eq!(refused, Modified::UNKNOWN);
    assert!(refused.date().is_empty());
    let known = Modified::new(std::time::UNIX_EPOCH);
    assert!(refused < known);
}

#[test]
fn a_default_entry_carries_the_neutral_facts() {
    let entry = DirEntry::default();
    assert_eq!(entry.links, 0);
    assert!(entry.owner.is_none());
    assert!(entry.description.is_none());
    assert_eq!(entry.attributes, Attributes::default());
}
