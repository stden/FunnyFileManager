//! Unit tests: a panel's columns and fields — columns shed from the right on
//! a narrow panel, Far's Size words and footer sizes, the extension split
//! and right-aligned in the name field.

use super::*;
use crate::models::DirEntry;
use crate::models::EntryKind;
use crate::models::FileSize;
use crate::models::Modified;
use crate::models::PanelView;

/// Far shows Name | Size | Date | Time, and drops the trailing columns — Time
/// first — rather than let the name fall below [`MIN_NAME_WIDTH`].
///
/// Dropping from the left instead would leave a Time column beside rows with
/// no names in them, which is the failure this ordering exists to prevent.
#[test]
fn test_panel_layout_sheds_columns_from_the_right_on_a_narrow_panel() {
    // 78 interior cells: everything fits, name takes the rest.
    let wide = panel_layout(PanelView::Full, Cells::new(78));
    assert_eq!(wide.columns.len(), 3);
    assert_eq!(wide.name, Cells::new(78 - (6 + 1) - (8 + 1) - (5 + 1)));

    // 38 interior cells (an 80-column screen split in two, less borders) still
    // affords all four columns — this is the everyday case.
    assert_eq!(
        panel_layout(PanelView::Full, Cells::new(38)).columns.len(),
        3
    );

    // 28 cells cannot: Time goes, and the name clears its floor again.
    let narrow = panel_layout(PanelView::Full, Cells::new(28));
    assert_eq!(narrow.columns.len(), 2);
    assert!(narrow.name >= Cells::new(8));

    // 18 cells keep only Size, and 8 cells keep nothing but the name.
    assert_eq!(
        panel_layout(PanelView::Full, Cells::new(18)).columns.len(),
        1
    );
    assert_eq!(
        panel_layout(PanelView::Full, Cells::new(8)).columns.len(),
        0
    );
}

/// The Size column names what a row *is* before it measures it.
#[test]
fn test_size_field_speaks_fars_vocabulary() {
    let dir = DirEntry {
        name: "docs".to_owned(),
        kind: EntryKind::Directory,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };
    assert_eq!(size_field(&dir), "Folder");

    let parent = DirEntry {
        name: "..".to_owned(),
        ..dir
    };
    assert_eq!(size_field(&parent), "Up");

    // A byte count is printed outright while it fits the seven-cell column.
    let file = DirEntry {
        name: "a.txt".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(78_743),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };
    assert_eq!(size_field(&file), "78743");

    // Past that it is abbreviated rather than clipped to a wrong number.
    let big = DirEntry {
        size: FileSize::from_bytes(12_345_678),
        ..file
    };
    assert_eq!(size_field(&big), "11.8 M");
}

/// Far's compact byte count keeps a decimal only while there is room for one.
#[test]
fn test_abbreviate_bytes_matches_fars_footer_format() {
    assert_eq!(abbreviate_bytes(0), "0");
    assert_eq!(abbreviate_bytes(999), "999");
    assert_eq!(abbreviate_bytes(390_144), "381 K");
    assert_eq!(abbreviate_bytes(53_050_064_896), "49.4 G");
}

/// A file's name splits at its FINAL dot; a directory's never splits, and
/// neither does a leading dot — `.env` is a hidden file, not an empty name
/// with an "env" extension.
#[test]
fn test_split_extension_follows_fars_rules() {
    let file = |name: &str| DirEntry {
        name: name.to_owned(),
        kind: EntryKind::File,
        ..DirEntry::default()
    };

    assert_eq!(
        split_extension(&file("Receipt-2893.pdf")),
        ("Receipt-2893", "pdf")
    );
    assert_eq!(
        split_extension(&file("docs.code-workspace")),
        ("docs", "code-workspace")
    );
    assert_eq!(
        split_extension(&file("archive.tar.gz")),
        ("archive.tar", "gz")
    );
    assert_eq!(split_extension(&file(".env")), (".env", ""));
    assert_eq!(split_extension(&file("README")), ("README", ""));

    let dir = DirEntry {
        name: ".cargo".to_owned(),
        kind: EntryKind::Directory,
        ..DirEntry::default()
    };
    assert_eq!(split_extension(&dir), (".cargo", ""));
}

/// The extension is right-aligned against the Size column, so a listing lines
/// up by type without being sorted by it.
#[test]
fn test_name_field_right_aligns_the_extension() {
    let entry = DirEntry {
        name: "a.txt".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };
    assert_eq!(name_field(&entry, Cells::new(10)), "a      txt");

    // Down to one cell for the base the split still pays for itself: the dot
    // becomes the gap, which is exactly what the extension column is.
    assert_eq!(name_field(&entry, Cells::new(5)), "a txt");

    // Below that it would leave the base no room at all, so the undivided name
    // keeps it — a lone right-aligned "txt" identifies the file less than a
    // cut "a.t}" does.
    assert_eq!(name_field(&entry, Cells::new(4)), "a.t}");

    // A name longer than its column is cut with Far's marker, and the
    // extension still lands against the right edge.
    let long = DirEntry {
        name: "очень-длинное-имя.txt".to_owned(),
        ..entry
    };
    let field = name_field(&long, Cells::new(12));
    assert_eq!(Cells::of(&field), Cells::new(12));
    assert!(field.ends_with("txt"), "got: '{field}'");
    assert!(field.contains('}'), "got: '{field}'");
}
