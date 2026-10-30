//! Purpose: unit tests for the list's view: the cursor across a sample and a resort,
//! the sort key, the filter, scope and kernel-thread switches, the owners, the
//! last column, and the viewport clamp `App` applies on resize.

use super::*;

fn owned_by(owner: &str, raw: u32, name: &str) -> ProcessSample {
    ProcessSample {
        owner: Owner::Named(owner.to_owned()),
        ..process(raw, name)
    }
}

#[test]
fn the_cursor_stays_on_its_process_across_a_sample_and_a_resort() {
    let mut list = listing(vec![
        process(1, "init"),
        process(2, "sshd"),
        process(3, "vim"),
    ]);
    list.select_index(2, 10);
    assert_eq!(list.cursor().map(Cursor::key), Some(key(3, 3)));

    resample(
        &mut list,
        vec![process(16, "new"), process(3, "vim"), process(1, "init")],
    );
    assert_eq!(shown(&list), [1, 3, 16]);
    assert_eq!(
        list.cursor().map(|at| (at.key(), at.index())),
        Some((key(3, 3), 1))
    );

    list.choose_sort(SortKey::Pid);
    list.choose_sort(SortKey::Pid);
    assert_eq!(shown(&list), [16, 3, 1]);
    assert_eq!(list.cursor().map(Cursor::index), Some(1));
}

#[test]
fn choosing_a_sort_key_leaves_the_tree_and_the_same_key_reverses() {
    let mut list = listing(vec![process(1, "init"), child_of(1, process(2, "sshd"))]);
    list.toggle_tree();
    assert_eq!(list.options().arrangement, Arrangement::Tree);
    list.choose_sort(SortKey::Name);
    assert_eq!(list.options().arrangement, Arrangement::Flat);
    assert_eq!(shown(&list), [1, 2]);
    list.choose_sort(SortKey::Name);
    assert_eq!(list.options().sort.order(), SortOrder::Reversed);
    assert_eq!(shown(&list), [2, 1]);
    list.invert_order();
    assert_eq!(shown(&list), [1, 2]);
}

#[test]
fn the_filter_the_scope_and_kernel_threads_rebuild_the_view() {
    let kthreadd = ProcessSample {
        kind: ProcessKind::KernelThread,
        ..process(2, "kthreadd")
    };
    let mut list = listing(vec![
        owned_by("root", 1, "init"),
        kthreadd,
        process(3, "firefox"),
        owned_by("alice", 4, "chromium"),
    ]);
    assert_eq!(shown(&list), [1, 3, 4]);
    list.toggle_kernel_threads();
    assert_eq!(list.options().kernel_threads, KernelThreads::Shown);
    assert_eq!(shown(&list), [1, 2, 3, 4]);
    list.set_filter("FIRE|chrom");
    assert_eq!(list.options().filter.line(), "FIRE|chrom");
    assert_eq!(shown(&list), [3, 4]);
    list.set_scope(UserScope::Only(Owner::Named("bob".to_owned())));
    assert_eq!(shown(&list), [3]);
    list.set_filter("");
    list.set_scope(UserScope::All);
    assert_eq!(shown(&list), [1, 2, 3, 4]);
}

#[test]
fn the_owners_are_listed_once_each_in_order() {
    let list = listing(vec![
        owned_by("root", 1, "init"),
        process(2, "shell"),
        owned_by("alice", 3, "editor"),
        process(4, "vim"),
    ]);
    let names = ["alice", "bob", "root"].map(|name| Owner::Named(name.to_owned()));
    assert_eq!(list.owners(), names);
}

#[test]
fn the_last_column_switches_and_scrolls_back_to_its_start() {
    let mut list = listing(vec![process(1, "init")]);
    list.pan_right(20);
    list.pan_right(1);
    list.pan_left(5);
    assert_eq!(list.command_scroll().cells(), 16);
    list.toggle_command_view();
    assert_eq!(list.command_view(), CommandView::Name);
    assert_eq!(list.command_scroll().cells(), 0);
}

#[test]
fn a_shorter_terminal_keeps_the_cursor_on_screen() {
    let processes = (1..=40).map(|raw| process(raw, "worker")).collect();
    let mut list = listing(processes);
    let tall = Viewport::new(30);
    assert_eq!(
        list.page_for(tall),
        21,
        "one core: no bars, nine rows of chrome"
    );
    list.move_cursor(ListMotion::End, list.page_for(tall));
    assert_eq!(list.offset(), 40 - 21);

    let short = Viewport::new(24);
    let mut direct = list.clone();
    direct.clamp_to(short);
    assert_eq!(direct.offset(), 40 - 15);

    let mut app = App::with_panels(
        Panel::from_entries(PathBuf::from("/srv/x"), Vec::new()),
        Panel::from_entries(PathBuf::from("/srv/x"), Vec::new()),
    );
    app.set_overlay(Overlay::Tasks(Box::new(list)));
    app.set_viewport(short);
    let Overlay::Tasks(list) = &app.overlay else {
        panic!("the task list stays open");
    };
    assert_eq!(list.page_for(short), 15);
    assert_eq!(list.offset(), 40 - 15, "the last row is on the bottom row");
    assert_eq!(list.header_shape(short).page(), 15);
}
