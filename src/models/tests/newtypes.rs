//! Purpose: The small value types — `FileName`, `BookmarkSlot` and `Bookmarks`, `EntryKind` and `FileSize`.

use super::*;

// ── FileName ──────────────────────────────────────────────────────────────

#[test]
fn a_file_name_looks_up_by_str_without_allocating() {
    // `Borrow<str>` is what keeps mark lookups free: the panel holds owned
    // names but every query site has a `&str` from the directory listing.
    let mut marks: std::collections::BTreeSet<FileName> = std::collections::BTreeSet::new();
    marks.insert(FileName::from("readme.md"));
    assert!(marks.contains("readme.md"));
    assert!(!marks.contains("other.md"));
    assert!(marks.remove("readme.md"));
    assert_eq!(marks.len(), 0);
}

#[test]
fn a_file_name_orders_like_the_string_it_wraps() {
    // The derived Ord must agree with `str`'s, or `Borrow<str>` lookups would
    // search a differently-sorted tree and miss.
    let mut names = [
        FileName::from("beta"),
        FileName::from("Alpha"),
        FileName::from("alpha"),
    ];
    names.sort();
    let sorted: Vec<&str> = names.iter().map(FileName::as_str).collect();
    let mut expected = vec!["beta", "Alpha", "alpha"];
    expected.sort_unstable();
    assert_eq!(sorted, expected);
}

#[test]
fn a_file_name_joins_onto_a_directory_as_one_component() {
    // `AsRef<Path>` is the one correct way to turn a mark into a path, and a
    // name is always exactly one component.
    let joined = Path::new("/srv/work").join(FileName::from("notes.txt"));
    assert_eq!(joined, Path::new("/srv/work/notes.txt"));
    assert_eq!(FileName::from("notes.txt").to_string(), "notes.txt");
}

// ── BookmarkSlot / Bookmarks ──────────────────────────────────────────────

#[test]
fn a_bookmark_slot_covers_exactly_zero_through_nine() {
    assert!(BookmarkSlot::new(0).is_ok());
    assert!(BookmarkSlot::new(BookmarkSlot::MAX).is_ok());
    assert!(BookmarkSlot::new(BookmarkSlot::MAX + 1).is_err());
    assert!(BookmarkSlot::from_usize(BookmarkSlot::COUNT).is_err());
    assert_eq!(BookmarkSlot::COUNT, usize::from(BookmarkSlot::MAX) + 1);
}

/// `from_usize` narrows through `u8`: 256 must not wrap to slot 0.
#[test]
fn a_bookmark_slot_from_usize_rejects_values_that_would_wrap_a_u8() {
    assert_eq!(BookmarkSlot::from_usize(0).map(BookmarkSlot::as_u8), Ok(0));
    assert_eq!(BookmarkSlot::from_usize(9).map(BookmarkSlot::as_u8), Ok(9));
    for out_of_range in [
        10,
        usize::from(u8::MAX),
        usize::from(u8::MAX) + 1,
        265,
        usize::MAX,
    ] {
        assert_eq!(
            BookmarkSlot::from_usize(out_of_range),
            Err("bookmark slot must be 0-9"),
            "{out_of_range}"
        );
    }
}

#[test]
fn every_slot_has_somewhere_to_go() {
    // The store has one entry per slot, which is what makes the lookup total
    // and let the two runtime bounds checks be deleted.
    let mut bookmarks = Bookmarks::default();
    assert_eq!(bookmarks.slots().len(), BookmarkSlot::COUNT);

    for n in 0..=BookmarkSlot::MAX {
        let slot = BookmarkSlot::new(n).expect("legal slot");
        assert_eq!(bookmarks.get(slot), None);
        bookmarks.set(slot, PathBuf::from(format!("/slot{n}")));
        assert_eq!(bookmarks.get(slot), Some(Path::new(&format!("/slot{n}"))));
        bookmarks.clear(slot);
        assert_eq!(bookmarks.get(slot), None);
    }
}

// ── EntryKind / FileSize ──────────────────────────────────────────────────

#[test]
fn a_symlink_to_a_directory_behaves_as_a_directory_and_stays_a_link() {
    // The bug the two bools caused: a symlinked directory rendered as a tiny
    // file that could not be entered, but whose whole target tree got copied.
    let kind = EntryKind::SymlinkToDirectory;
    assert!(kind.is_dir(), "Enter must descend into it");
    assert!(kind.is_symlink(), "and it is still a link");
}

#[test]
fn a_broken_link_is_neither_a_directory_nor_a_plain_file() {
    let kind = EntryKind::BrokenSymlink;
    assert!(!kind.is_dir(), "there is nothing to enter");
    assert!(kind.is_symlink());
    // The old encoding could not tell this from a symlink to a file: both were
    // (is_dir = false, is_symlink = true).
    assert_ne!(kind, EntryKind::SymlinkToFile);
}

#[test]
fn plain_entries_carry_no_link() {
    assert!(EntryKind::Directory.is_dir());
    assert!(!EntryKind::Directory.is_symlink());
    assert!(!EntryKind::File.is_dir());
    assert!(!EntryKind::File.is_symlink());
    assert!(!EntryKind::SymlinkToFile.is_dir());
    assert_eq!(EntryKind::default(), EntryKind::File);
}

#[test]
fn a_file_size_renders_at_every_unit_boundary() {
    // The rendering moved onto the type, so it cannot be applied to a line
    // count or a byte offset by accident.
    assert_eq!(FileSize::from_bytes(0).to_string(), "0 B");
    assert_eq!(FileSize::from_bytes(1023).to_string(), "1023 B");
    assert_eq!(FileSize::from_bytes(1024).to_string(), "1.00 KB");
    assert_eq!(FileSize::from_bytes(1024 * 1024).to_string(), "1.00 MB");
    assert_eq!(
        FileSize::from_bytes(1024 * 1024 * 1024).to_string(),
        "1.00 GB"
    );
}

#[test]
fn a_file_size_converts_to_megabytes_and_back_to_bytes() {
    let size = FileSize::from_bytes(3 * 1024 * 1024);
    assert!((size.megabytes() - 3.0).abs() < f64::EPSILON);
    assert_eq!(size.bytes(), 3 * 1024 * 1024);
    assert_eq!(FileSize::default(), FileSize::from_bytes(0));
    // Ordering is the byte order, so sorting a panel by size still works.
    assert!(FileSize::from_bytes(1) < FileSize::from_bytes(2));
}
