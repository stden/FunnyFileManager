//! Purpose: Interruptible directory reads stop cleanly with partial entries and `..`.

use tempfile::TempDir;

use super::*;

#[test]
fn interrupted_directory_read_returns_partial_entries_with_parent_row() -> io::Result<()> {
    let temp = TempDir::new()?;
    for index in 0..5 {
        fs::write(temp.path().join(format!("item_{index}.txt")), b"x")?;
    }
    fs::write(
        temp.path().join("descript.ion"),
        "item_0.txt Should be skipped on interrupt\n",
    )?;

    let mut seen = 0usize;
    let outcome = read_for_panel(
        temp.path(),
        &crate::description::default_names(),
        None,
        |_shown| {
            seen += 1;
            seen >= 2
        },
    )?;

    assert!(outcome.interrupted);
    assert_eq!(outcome.listing.entries.len(), 2);
    assert_eq!(
        outcome.listing.parent.as_ref().map(|row| row.name.as_str()),
        Some("..")
    );
    assert!(outcome
        .listing
        .entries
        .iter()
        .all(|entry| entry.description.is_none()));
    Ok(())
}

#[test]
fn read_for_panel_skips_entries_whose_metadata_fails() -> io::Result<()> {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            FORCE_ENTRY_METADATA_ERROR.set(false);
        }
    }

    let temp = TempDir::new()?;
    fs::write(temp.path().join("broken.txt"), b"x")?;
    FORCE_ENTRY_METADATA_ERROR.set(true);
    let _reset = Reset;

    let outcome = read_for_panel(temp.path(), &[], None, |_| false)?;
    assert!(!outcome.interrupted);
    assert_eq!(outcome.listing.entries.len(), 0);
    Ok(())
}
