//! Purpose: unit tests for cursor moves, the scroll offset, clicks, and a cursor
//! that keeps its row when its process vanishes.

use super::*;

#[test]
fn the_first_snapshot_puts_the_cursor_on_the_first_row() {
    let list = twenty();
    assert_eq!(cursor_pid(&list), Some(1));
    assert_eq!(list.offset(), 0);
}

/// Samples 1..=5 a second apart, with `busy` using the whole second.
fn cpu_sorted(list: &mut TaskList, busy: u32) {
    use super::super::tests_support::{counters, sample, system};
    let processes = (1..=5)
        .map(|raw| ProcessSample {
            cpu_time: std::time::Duration::from_secs(u64::from(raw == busy)),
            ..process(raw, "p")
        })
        .collect();
    let now = std::time::Instant::now();
    list.take_request(now);
    let total = counters(2, 1, 1);
    list.apply_sample(Ok(sample(system(total, &[total]), processes)), now);
}

/// The first snapshot is ordered by PID; the next one by CPU%. The cursor
/// nobody had moved followed PID 1 down the list: it keeps the first row.
#[test]
fn an_unmoved_cursor_keeps_the_first_row_when_cpu_sorts_the_list() {
    let mut list = listing((1..=5).map(|raw| process(raw, "p")).collect());
    assert_eq!(cursor_pid(&list), Some(1));
    cpu_sorted(&mut list, 4);
    assert_eq!(shown(&list).first(), Some(&4), "sorted by CPU%");
    assert_eq!(list.cursor().map(Cursor::index), Some(0));
    assert_eq!(cursor_pid(&list), Some(4));
}

/// Once the user puts the cursor on a process, it follows that process.
#[test]
fn a_moved_cursor_follows_its_process_when_the_list_re_sorts() {
    let mut list = listing((1..=5).map(|raw| process(raw, "p")).collect());
    list.move_cursor(ListMotion::Down, PAGE);
    cpu_sorted(&mut list, 4);
    assert_eq!(cursor_pid(&list), Some(2));
    assert_ne!(
        list.cursor().map(Cursor::index),
        Some(1),
        "it moved with PID 2"
    );
}

#[test]
fn an_empty_list_has_no_cursor_and_ignores_moves() {
    let mut list = listing(Vec::new());
    list.move_cursor(ListMotion::End, PAGE);
    list.toggle_mark(PAGE);
    list.mark_family();
    assert_eq!(list.cursor(), None);
    assert_eq!(list.marked_count(), 0);
    assert_eq!(list.find("x", SearchFrom::Cursor, PAGE), Found::NoMatch);
}

#[test]
fn every_motion_moves_and_stops_at_the_ends() {
    let mut list = twenty();
    let steps = [
        (ListMotion::Up, 1),
        (ListMotion::Down, 2),
        (ListMotion::PageDown, 7),
        (ListMotion::End, 20),
        (ListMotion::Down, 20),
        (ListMotion::PageUp, 15),
        (ListMotion::Home, 1),
        (ListMotion::PageUp, 1),
    ];
    for (motion, pid) in steps {
        list.move_cursor(motion, PAGE);
        assert_eq!(cursor_pid(&list), Some(pid), "{motion:?}");
    }
}

#[test]
fn the_offset_follows_the_visible_window() {
    let mut list = twenty();
    let mut offset = 0;
    for motion in [ListMotion::PageDown, ListMotion::PageDown, ListMotion::Up] {
        list.move_cursor(motion, PAGE);
        let index = list.cursor().map(Cursor::index);
        offset = visible_window(20, PAGE, offset, index).start;
        assert_eq!(list.offset(), offset, "{motion:?}");
    }
    assert_eq!(list.offset(), 6);
}

#[test]
fn a_click_selects_the_row_clamped_to_the_list() {
    let mut list = twenty();
    list.select_index(7, PAGE);
    assert_eq!(cursor_pid(&list), Some(8));
    list.select_index(99, PAGE);
    assert_eq!(cursor_pid(&list), Some(20));
}

#[test]
fn a_vanished_process_leaves_the_cursor_on_its_row() {
    let mut list = twenty();
    list.select_index(19, PAGE);
    resample(&mut list, (1..=10).map(|raw| process(raw, "p")).collect());
    assert_eq!(list.cursor().map(Cursor::index), Some(9));
    assert_eq!(cursor_pid(&list), Some(10));

    list.select_index(4, PAGE);
    resample(
        &mut list,
        (1..=10)
            .filter(|raw| *raw != 5)
            .map(|raw| process(raw, "p"))
            .collect(),
    );
    assert_eq!(cursor_pid(&list), Some(6), "the row's new process");
}
