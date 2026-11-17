//! Purpose: Tests for the per-process `/proc` parsers besides `stat`, whose
//! tests are in `tests_parse_stat.rs`: clock ticks as exact durations,
//! `statm` pages in bytes, the effective uid in `status`, and `cmdline` as
//! one line.

use std::time::Duration;

use super::parse::{command_line, effective_uid, statm, ClockTicks, PageSize};
use crate::models::{MemoryBytes, MemoryUsage};

pub(super) fn hertz() -> ClockTicks {
    ClockTicks::new(100).expect("USER_HZ is not zero")
}

#[test]
fn ticks_become_exact_durations() {
    assert_eq!(ClockTicks::new(0), None);
    assert_eq!(hertz().duration(12_345), Duration::from_millis(123_450));
    let thirds = ClockTicks::new(3).expect("not zero");
    assert_eq!(thirds.duration(4), Duration::new(1, 333_333_333));
}

#[test]
fn statm_multiplies_pages_and_refuses_to_overflow() {
    let page = PageSize::new(4096).expect("not zero");
    let expected = MemoryUsage {
        virtual_size: MemoryBytes::from_bytes(4_096_000),
        resident: MemoryBytes::from_bytes(819_200),
        shared: Some(MemoryBytes::from_bytes(204_800)),
    };
    assert_eq!(statm("1000 200 50 10 0 300 0\n", page), Some(expected));
    assert_eq!(statm("18446744073709551615 1 1 0 0 0 0\n", page), None);
    assert_eq!(statm("1000 200\n", page), None);
    assert_eq!(statm("1000 x 50\n", page), None);
    assert_eq!(PageSize::new(0), None);
}

#[test]
fn status_gives_the_effective_uid() {
    let status =
        "Name:\tbash\nUmask:\t0022\nUid:\t1000\t1001\t1002\t1003\nGid:\t100\t100\t100\t100\n";
    assert_eq!(effective_uid(status), Some(1001));
    assert_eq!(effective_uid("Uid:\t1000\n"), None);
    assert_eq!(effective_uid("Name:\tbash\n"), None);
    assert_eq!(effective_uid("Uid:\t1000\tmany\n"), None);
    assert_eq!(
        effective_uid("Uid:\t0\t4000000000\t0\t0\n"),
        Some(4_000_000_000)
    );
}

#[test]
fn a_command_line_joins_its_arguments() {
    let train = "python3\0/srv/x/train.py\0--epochs\0";
    assert_eq!(
        command_line(train, "python3"),
        "python3 /srv/x/train.py --epochs"
    );
    assert_eq!(
        command_line("a\0\0b\0", "x"),
        "a  b",
        "an empty argument stays"
    );
    let rewritten = "nginx: worker process";
    assert_eq!(command_line(rewritten, "nginx"), rewritten);
}

#[test]
fn an_empty_command_line_shows_the_name_in_brackets() {
    for empty in ["", "\0", "\0\0", " "] {
        assert_eq!(command_line(empty, "kthreadd"), "[kthreadd]", "{empty:?}");
    }
}

#[test]
fn a_command_line_cut_mid_character_still_decodes() {
    // The reader stops at a byte count, which can fall inside a character.
    let cut = 4096;
    let long = format!("{}é tail", "x".repeat(cut - 1));
    let read = &long.as_bytes()[..cut];
    let line = command_line(&String::from_utf8_lossy(read), "x");
    assert!(line.ends_with('\u{fffd}'), "the cut character is replaced");
    assert_eq!(line.chars().count(), cut);
}
