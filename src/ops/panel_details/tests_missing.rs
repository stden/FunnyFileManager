//! Purpose: Entries missing on disk are skipped when collecting Links or Owners metadata.

use super::*;
use tempfile::TempDir;

#[test]
fn missing_entries_are_skipped_by_links_and_owners_details() -> io::Result<()> {
    let dir = TempDir::new()?;
    let missing = [DirEntry {
        name: "missing.txt".to_owned(),
        ..DirEntry::default()
    }];

    let links = details_for(dir.path(), &missing, PanelView::Links, &[])?;
    assert!(links.links.is_empty());

    #[cfg(unix)]
    {
        let owners = details_for(dir.path(), &missing, PanelView::Owners, &[])?;
        assert!(owners.owners.is_empty());
    }
    Ok(())
}
