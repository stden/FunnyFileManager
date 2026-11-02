//! Purpose: Pins the name list a red question shows: five names, then a count.

use super::preview_lines;

fn preview(count: usize) -> Vec<String> {
    let files: Vec<usize> = (1..=count).collect();
    let mut lines = vec!["Question?".to_owned()];
    preview_lines(&files, |n| format!("file{n}"), &mut lines);
    lines
}

#[test]
fn no_names_add_nothing_after_the_question() {
    assert_eq!(preview(0), ["Question?"]);
}

/// Five fit whole: the boundary lists every name and no count.
#[test]
fn five_names_are_all_listed_without_a_count() {
    assert_eq!(
        preview(5),
        ["Question?", "file1", "file2", "file3", "file4", "file5"]
    );
}

/// The sixth is the first one counted instead of named.
#[test]
fn a_sixth_name_becomes_a_count() {
    assert_eq!(
        preview(6),
        [
            "Question?",
            "file1",
            "file2",
            "file3",
            "file4",
            "file5",
            "… and 1 more"
        ]
    );
}

/// A long list names five items and counts the rest: only the named items
/// are formatted, and the count is every item that was not named.
#[test]
fn a_long_list_formats_five_names_and_counts_the_rest() {
    let files: Vec<usize> = (1..=100_000).collect();
    let mut formatted = 0;
    let mut lines = Vec::new();
    preview_lines(
        &files,
        |n| {
            formatted += 1;
            format!("file{n}")
        },
        &mut lines,
    );
    assert_eq!(lines.len(), 6);
    assert_eq!(lines[5], "… and 99995 more");
    assert_eq!(formatted, 5);
}
