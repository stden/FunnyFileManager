//! Purpose: Tests for the selection: moves and the scroll offset, a cursor
//! that follows its process or keeps its row, marks by key, family, mask and
//! inversion, the wrapping search and the PID jump.

use super::tests_support::{child_of, key, listing, pid, process, resample, shown};
use super::{CommandView, Cursor, Found, ListMotion, Notice, ProcessSample, SearchFrom, TaskList};
use crate::mask::Mask;
use crate::models::MaskPurpose;
use crate::scroll::visible_window;

const PAGE: usize = 5;

/// Twenty processes, PIDs 1 to 20, named `p1` to `p20`.
fn twenty() -> TaskList {
    listing(
        (1..=20)
            .map(|raw| process(raw, &format!("p{raw}")))
            .collect(),
    )
}

fn cursor_pid(list: &TaskList) -> Option<u32> {
    list.cursor().map(|cursor| cursor.key().pid().get())
}

fn marked(list: &TaskList) -> Vec<u32> {
    list.view()
        .iter()
        .filter(|shown| list.is_marked(shown.key()))
        .map(|shown| shown.key().pid().get())
        .collect()
}

#[test]
fn the_first_snapshot_puts_the_cursor_on_the_first_row() {
    let list = twenty();
    assert_eq!(cursor_pid(&list), Some(1));
    assert_eq!(list.offset(), 0);
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

#[test]
fn space_marks_and_moves_down_and_marks_follow_their_process() {
    let mut list = twenty();
    list.toggle_mark(PAGE);
    list.toggle_mark(PAGE);
    assert_eq!(marked(&list), [1, 2]);
    assert_eq!(cursor_pid(&list), Some(3));
    list.move_cursor(ListMotion::Up, PAGE);
    list.toggle_mark(PAGE);
    assert_eq!(marked(&list), [1]);
    assert_eq!(list.marked_count(), 1);

    resample(&mut list, vec![process(2, "p2"), process(3, "p3")]);
    assert_eq!(
        marked(&list),
        Vec::<u32>::new(),
        "PID 1's mark left with it"
    );
    resample(&mut list, (1..=3).map(|raw| process(raw, "p")).collect());
    assert!(
        !list.is_marked(key(1, 1)),
        "a pruned mark does not come back"
    );
}

#[test]
fn c_marks_the_process_and_its_descendants() {
    let mut list = listing(vec![
        process(1, "init"),
        child_of(1, process(2, "sshd")),
        child_of(2, process(3, "bash")),
        child_of(3, process(4, "vim")),
        process(5, "cron"),
    ]);
    list.select_index(1, PAGE);
    list.mark_family();
    assert_eq!(marked(&list), [2, 3, 4]);
}

#[test]
fn u_unmarks_all_and_star_inverts_the_shown_rows() {
    let mut list = twenty();
    list.toggle_mark(PAGE);
    list.set_filter("p1");
    assert_eq!(shown(&list).len(), 11, "p1 and p10 to p19");
    list.invert_marks();
    assert_eq!(list.marked_count(), 10);
    assert!(!list.is_marked(key(1, 1)));
    list.set_filter("");
    list.unmark_all();
    assert_eq!(list.marked_count(), 0);
}

#[test]
fn a_mask_marks_by_name_and_says_how_many_changed() {
    let mut list = twenty();
    let ones = Mask::parse("p1*");
    let selected = list.mark_by_mask(MaskPurpose::Select, &ones);
    assert_eq!(
        selected,
        Notice::Marked {
            changed: 11,
            total: 11
        }
    );
    let again = list.mark_by_mask(MaskPurpose::Select, &Mask::parse("p1,p2"));
    assert_eq!(
        again,
        Notice::Marked {
            changed: 1,
            total: 12
        }
    );
    let dropped = list.mark_by_mask(MaskPurpose::Unselect, &Mask::parse("p1?"));
    assert_eq!(
        dropped,
        Notice::Marked {
            changed: 10,
            total: 2
        }
    );
    assert_eq!(marked(&list), [1, 2]);
}

fn commands() -> TaskList {
    let command = |raw: u32, name: &str, line: &str| ProcessSample {
        command: line.to_owned(),
        ..process(raw, name)
    };
    listing(vec![
        command(1, "python3", "python3 /srv/x/train.py"),
        command(2, "bash", "-bash"),
        command(3, "python3", "python3 /srv/x/serve.py"),
    ])
}

#[test]
fn search_starts_at_the_cursor_and_wraps() {
    let mut list = commands();
    assert_eq!(list.find("PYTH", SearchFrom::Cursor, PAGE), Found::Match);
    assert_eq!(cursor_pid(&list), Some(1), "the cursor's own row matches");
    assert_eq!(
        list.find("pyth", SearchFrom::AfterCursor, PAGE),
        Found::Match
    );
    assert_eq!(cursor_pid(&list), Some(3));
    assert_eq!(
        list.find("pyth", SearchFrom::AfterCursor, PAGE),
        Found::Match
    );
    assert_eq!(cursor_pid(&list), Some(1), "wrapped past the end");
    assert_eq!(list.find("perl", SearchFrom::Cursor, PAGE), Found::NoMatch);
    assert_eq!(cursor_pid(&list), Some(1), "a miss leaves the cursor");
}

#[test]
fn search_reads_the_last_column_as_shown() {
    let mut list = commands();
    assert_eq!(list.find("serve", SearchFrom::Cursor, PAGE), Found::Match);
    list.toggle_command_view();
    assert_eq!(list.command_view(), CommandView::Name);
    assert_eq!(list.find("serve", SearchFrom::Cursor, PAGE), Found::NoMatch);
}

#[test]
fn a_pid_jump_lands_only_on_a_shown_pid() {
    let mut list = twenty();
    assert_eq!(list.jump_to_pid(pid(17), PAGE), Found::Match);
    assert_eq!(cursor_pid(&list), Some(17));
    assert_eq!(list.offset(), 12);
    assert_eq!(list.jump_to_pid(pid(21), PAGE), Found::NoMatch);
    assert_eq!(cursor_pid(&list), Some(17));
}
