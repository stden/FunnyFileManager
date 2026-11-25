//! Purpose: Sort mode cycling and panel ordering.

use super::*;

#[test]
fn test_sort_mode_mutations() {
    // Test next() cycling
    let mode = SortMode::Name;
    assert!(matches!(mode.next(), SortMode::Extension));
    assert!(matches!(mode.next().next(), SortMode::Size));
    assert!(matches!(mode.next().next().next(), SortMode::Name));

    // Test label() returning correct strings
    assert_eq!(SortMode::Name.label(), "Name");
    assert_eq!(SortMode::Extension.label(), "Ext");
    assert_eq!(SortMode::Size.label(), "Size");
}

#[test]
fn test_panel_sort_mutations() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    // Create files with different extensions and sizes
    File::create(temp_dir.path().join("b.txt"))?.write_all(b"small")?;
    File::create(temp_dir.path().join("a.rs"))?.write_all(b"medium file")?;
    File::create(temp_dir.path().join("c.md"))?.write_all(b"very large file content")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Test sorting by name
    panel.sort_mode = SortMode::Name;
    panel.load_entries()?;
    assert_eq!(panel.entries[0].name, "..");
    assert_eq!(panel.entries[1].name, "a.rs");
    assert_eq!(panel.entries[2].name, "b.txt");
    assert_eq!(panel.entries[3].name, "c.md");

    // Test sorting by extension
    panel.sort_mode = SortMode::Extension;
    panel.load_entries()?;
    assert_eq!(panel.entries[0].name, "..");
    for (entry, extension) in panel.entries[1..].iter().zip(["md", "rs", "txt"]) {
        assert!(entry
            .name
            .rsplit_once('.')
            .is_some_and(|(_, suffix)| suffix.eq_ignore_ascii_case(extension)));
    }

    // Test sorting by size
    panel.sort_mode = SortMode::Size;
    panel.load_entries()?;
    assert_eq!(panel.entries[0].name, "..");
    // Files should be sorted by size
    let sizes: Vec<u64> = panel.entries[1..].iter().map(|e| e.size.bytes()).collect();
    for i in 1..sizes.len() {
        assert!(sizes[i] <= sizes[i - 1], "Should be sorted by size");
    }

    Ok(())
}
