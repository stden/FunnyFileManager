//! Purpose: Keeps focused navigation regression checks available during debugging.
//!
use funny_file_manager::models::Panel;
use std::{fs, io};
use tempfile::TempDir;

#[test]
fn test_navigation_step_by_step() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    // Create exactly 5 files
    for i in 1..=5 {
        fs::write(temp_dir.path().join(format!("file{i}.txt")), b"test")?;
    }

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Should have 6 entries (5 files + "..")
    assert_eq!(panel.entries.len(), 6);

    // Start at position 0 (should be "..")
    panel.select(Some(0));
    assert_eq!(panel.selected(), Some(0));
    assert_eq!(panel.entries[0].name, "..");

    // Move down once - should be at file1.txt
    panel.next();
    assert_eq!(
        panel.selected(),
        Some(1),
        "First next() should move to index 1"
    );
    assert_eq!(panel.entries[1].name, "file1.txt");

    // Move down again - should be at file2.txt
    panel.next();
    assert_eq!(
        panel.selected(),
        Some(2),
        "Second next() should move to index 2"
    );
    assert_eq!(panel.entries[2].name, "file2.txt");

    // Move up once - should be back at file1.txt
    panel.previous();
    assert_eq!(
        panel.selected(),
        Some(1),
        "previous() should move back to index 1"
    );
    assert_eq!(panel.entries[1].name, "file1.txt");

    Ok(())
}
