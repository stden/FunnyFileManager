//! Unit tests: the folder tree expands one level at a time through an injected
//! lister, keeps its rows depth-first with Far's guide lines, and moves the
//! cursor the way the tree panel and the Find folder dialog both need.

use std::path::{Path, PathBuf};

use super::{Branch, Expansion, FolderTree, Guide, TreeRow};

/// A fixed tree, so no test reads the disk:
///
/// ```text
/// /
/// ├─etc
/// │ └─ssh
/// ├─home
/// │ ├─ann
/// │ └─bob
/// └─var
/// ```
fn lister(dir: &Path) -> Vec<PathBuf> {
    let children: &[&str] = match dir.to_str().unwrap_or_default() {
        "/" => &["/etc", "/home", "/var"],
        "/etc" => &["/etc/ssh"],
        "/home" => &["/home/ann", "/home/bob"],
        _ => &[],
    };
    children.iter().map(PathBuf::from).collect()
}

fn root() -> FolderTree {
    FolderTree::rooted_at(PathBuf::from("/"))
}

fn names(tree: &FolderTree) -> Vec<String> {
    tree.rows().iter().map(TreeRow::name).collect()
}

#[test]
fn a_fresh_tree_shows_only_its_root() {
    let tree = root();
    assert_eq!(names(&tree), vec!["/"]);
    assert_eq!(tree.selected(), Some(Path::new("/")));
    assert_eq!(tree.rows()[0].branch, Branch::Root);
    assert_eq!(tree.rows()[0].expansion, Expansion::Collapsed);
}

#[test]
fn expanding_the_root_lists_one_level_and_marks_the_last_child() {
    let mut tree = root();
    assert!(tree.expand(&mut lister));
    assert_eq!(names(&tree), vec!["/", "etc", "home", "var"]);
    assert_eq!(tree.rows()[0].expansion, Expansion::Expanded);
    assert_eq!(tree.rows()[1].branch, Branch::Middle);
    assert_eq!(tree.rows()[3].branch, Branch::Last);
    assert!(
        !tree.expand(&mut lister),
        "an expanded row has nothing more to expand"
    );
}

#[test]
fn guides_carry_the_line_past_a_sibling_that_continues() {
    let mut tree = root();
    tree.expand(&mut lister);
    tree.cursor = 1; // etc
    tree.expand(&mut lister);
    // etc has a later sibling, so its child sits under a continuing line.
    assert_eq!(names(&tree), vec!["/", "etc", "ssh", "home", "var"]);
    assert_eq!(tree.rows()[2].guides, vec![Guide::Line]);
    tree.cursor = 4; // var, the last child of the root
    assert!(
        !tree.expand(&mut lister),
        "a folder with no subfolders expands to nothing"
    );
    assert_eq!(tree.rows()[4].expansion, Expansion::Leaf);
    assert!(
        !tree.collapse(),
        "a leaf was never open, so Left climbs instead"
    );
}

#[test]
fn a_child_of_the_last_sibling_sits_under_a_gap() {
    let mut tree = root();
    tree.reveal(Path::new("/home/bob"), &mut lister);
    // home is not the last child (var is), so its children get a line; but
    // a deeper tree under the last child gets a gap. Build one to check.
    let mut deep = FolderTree::rooted_at(PathBuf::from("/"));
    let mut deep_lister = |dir: &Path| -> Vec<PathBuf> {
        match dir.to_str().unwrap_or_default() {
            "/" => vec![PathBuf::from("/a"), PathBuf::from("/z")],
            "/z" => vec![PathBuf::from("/z/inner")],
            _ => Vec::new(),
        }
    };
    deep.reveal(Path::new("/z/inner"), &mut deep_lister);
    assert_eq!(names(&deep), vec!["/", "a", "z", "inner"]);
    assert_eq!(deep.rows()[3].guides, vec![Guide::Gap]);
    assert_eq!(names(&tree), vec!["/", "etc", "home", "ann", "bob", "var"]);
}

#[test]
fn reveal_expands_every_ancestor_and_lands_the_cursor_on_the_path() {
    let mut tree = root();
    assert!(tree.reveal(Path::new("/home/bob"), &mut lister));
    assert_eq!(names(&tree), vec!["/", "etc", "home", "ann", "bob", "var"]);
    assert_eq!(tree.selected(), Some(Path::new("/home/bob")));
    assert!(
        !tree.reveal(Path::new("/nowhere/at/all"), &mut lister),
        "a path the listing does not know is reported, not invented"
    );
    assert_eq!(
        tree.selected(),
        Some(Path::new("/home/bob")),
        "cursor stays"
    );
}

#[test]
fn collapse_hides_the_children_and_keeps_the_cursor_on_the_folder() {
    let mut tree = root();
    tree.reveal(Path::new("/home/bob"), &mut lister);
    tree.cursor = 2; // home
    assert!(tree.collapse());
    assert_eq!(names(&tree), vec!["/", "etc", "home", "var"]);
    assert_eq!(tree.selected(), Some(Path::new("/home")));
    assert!(!tree.collapse(), "already collapsed");
    // Expanding again does not list again: the children are remembered.
    let mut calls = 0;
    let mut counting = |dir: &Path| {
        calls += 1;
        lister(dir)
    };
    tree.expand(&mut counting);
    assert_eq!(calls, 0);
    assert_eq!(names(&tree), vec!["/", "etc", "home", "ann", "bob", "var"]);
}

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
fn refresh_relists_the_expanded_folders_and_drops_what_disappeared() {
    let mut tree = root();
    tree.reveal(Path::new("/home/bob"), &mut lister);
    let mut changed = |dir: &Path| -> Vec<PathBuf> {
        match dir.to_str().unwrap_or_default() {
            "/" => vec![PathBuf::from("/home"), PathBuf::from("/opt")],
            "/home" => vec![PathBuf::from("/home/ann")],
            _ => Vec::new(),
        }
    };
    tree.cursor = 4; // bob, about to vanish
    tree.refresh(&mut changed);
    assert_eq!(names(&tree), vec!["/", "home", "ann", "opt"]);
    assert!(tree.cursor < tree.rows().len(), "the cursor stays on a row");
    tree.cursor = 1;
    assert!(tree.collapse(), "home is still expanded after the refresh");
}

#[test]
fn an_empty_default_tree_has_no_rows_and_no_selection() {
    let mut tree = FolderTree::default();
    assert!(tree.rows().is_empty());
    assert_eq!(tree.selected(), None);
    tree.down();
    tree.last();
    assert!(!tree.expand(&mut lister));
    assert!(!tree.collapse());
    assert!(!tree.find_first("a"));
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

#[test]
fn the_last_child_is_a_last_branch_and_has_no_previous_sibling() {
    let mut tree = root();
    assert!(tree.expand(&mut lister));
    assert_eq!(tree.rows().last().expect("child").depth(), 1);
    tree.down();
    assert!(!tree.previous_sibling());
}
