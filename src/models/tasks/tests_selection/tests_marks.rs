//! Purpose: unit tests for marks by key, family, mask and inversion.

use super::*;

fn marked(list: &TaskList) -> Vec<u32> {
    list.view()
        .iter()
        .filter(|shown| list.is_marked(shown.key()))
        .map(|shown| shown.key().pid().get())
        .collect()
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
