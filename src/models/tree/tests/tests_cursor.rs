//! Purpose: unit tests for moving the tree's cursor: arrows, pages, structure
//! moves, and quick search.

use super::*;

#[test]
fn arrows_stop_at_both_ends_and_pages_move_by_the_given_rows() {
    let mut tree = root();
    tree.reveal(Path::new("/home/bob"), &mut lister);
    tree.cursor = 0;
    tree.up();
    assert_eq!(tree.cursor, 0);
    tree.down();
    tree.down();
    assert_eq!(tree.cursor, 2);
    tree.page_down(2);
    assert_eq!(tree.cursor, 4);
    tree.page_down(100);
    assert_eq!(tree.cursor, 5, "stops on the last row");
    tree.page_up(3);
    assert_eq!(tree.cursor, 2);
    tree.last();
    assert_eq!(tree.cursor, 5);
    tree.first();
    assert_eq!(tree.cursor, 0);
}

#[test]
fn parent_child_and_sibling_moves_follow_the_structure_not_the_rows() {
    let mut tree = root();
    tree.reveal(Path::new("/home/bob"), &mut lister);
    // rows: / etc home ann bob var
    tree.cursor = 4; // bob
    assert!(tree.move_to_parent());
    assert_eq!(tree.selected(), Some(Path::new("/home")));
    assert!(tree.move_to_first_child());
    assert_eq!(tree.selected(), Some(Path::new("/home/ann")));
    assert!(tree.next_sibling());
    assert_eq!(tree.selected(), Some(Path::new("/home/bob")));
    assert!(!tree.next_sibling(), "bob is the last of its level");
    assert!(tree.previous_sibling());
    assert_eq!(tree.selected(), Some(Path::new("/home/ann")));
    tree.cursor = 1; // etc, collapsed
    assert!(
        !tree.move_to_first_child(),
        "nothing shown under a collapsed row"
    );
    assert!(
        tree.next_sibling(),
        "the sibling after etc is home, past none"
    );
    assert_eq!(tree.selected(), Some(Path::new("/home")));
    assert!(tree.next_sibling());
    assert_eq!(
        tree.selected(),
        Some(Path::new("/var")),
        "skips home's kids"
    );
    tree.cursor = 0;
    assert!(!tree.move_to_parent(), "the root has no parent on screen");
}

#[test]
fn quick_search_matches_folder_names_from_the_top_then_the_next_one() {
    let mut tree = root();
    tree.reveal(Path::new("/home/bob"), &mut lister);
    tree.cursor = 5;
    assert!(tree.find_first("H"));
    assert_eq!(tree.selected(), Some(Path::new("/home")));
    assert!(!tree.find_first("zzz"));
    assert_eq!(
        tree.selected(),
        Some(Path::new("/home")),
        "no match, no move"
    );
    tree.cursor = 0;
    assert!(tree.find_next("a"));
    assert_eq!(tree.selected(), Some(Path::new("/home/ann")));
    assert!(tree.find_next("a"), "wraps around to the only other match");
    assert_eq!(tree.selected(), Some(Path::new("/home/ann")));
    assert!(!tree.find_first(""), "an empty query matches nothing");
}

#[test]
fn reveal_and_search_refuse_paths_and_prefixes_that_cannot_match() {
    let mut tree = root();
    assert!(tree.expand(&mut lister));
    assert_eq!(tree.rows()[1].depth(), 1);
    assert!(!tree.reveal(Path::new("/tmp"), &mut lister));
    assert!(!tree.find_next(""));
    assert!(!tree.find_next("no-such-folder"));
    let mut empty = FolderTree::default();
    assert!(!empty.find_next("a"));
    assert!(!empty.next_sibling());
    assert!(!empty.previous_sibling());
    assert!(!empty.move_to_first_child());
}
