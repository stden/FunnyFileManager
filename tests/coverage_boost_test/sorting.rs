//! Purpose: Sort modes: cycling, labels and ordering by size or extension.

use super::*;

// ═══════════════════════════════════════════════════════════════════
// SortMode methods
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_sort_mode_next() {
    assert_eq!(SortMode::Name.next(), SortMode::Extension);
    assert_eq!(SortMode::Extension.next(), SortMode::Size);
    assert_eq!(SortMode::Size.next(), SortMode::Name);
}

#[test]
fn test_sort_mode_label() {
    assert_eq!(SortMode::Name.label(), "Name");
    assert_eq!(SortMode::Extension.label(), "Ext");
    assert_eq!(SortMode::Size.label(), "Size");
}

// Panel sort by Size ordering
#[test]
fn test_panel_sort_by_size() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("small.txt"), "x")?;
    fs::write(temp.path().join("large.txt"), "xxxxxxxxxxxxxxxxxxxx")?;
    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.sort_mode = SortMode::Size;
    panel.load_entries()?;
    // Directories first, then files by size descending
    let files: Vec<_> = panel.entries.iter().filter(|e| !e.kind.is_dir()).collect();
    if files.len() >= 2 {
        assert!(files[0].size >= files[1].size);
    }
    Ok(())
}

// Panel sort by Extension ordering
#[test]
fn test_panel_sort_by_extension() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("data.csv"), "1")?;
    fs::write(temp.path().join("notes.txt"), "2")?;
    fs::write(temp.path().join("image.png"), "3")?;
    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.sort_mode = SortMode::Extension;
    panel.load_entries()?;
    let files: Vec<_> = panel
        .entries
        .iter()
        .filter(|e| !e.kind.is_dir() && e.name != "..")
        .collect();
    if files.len() >= 2 {
        let ext_a = std::path::Path::new(&files[0].name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let ext_b = std::path::Path::new(&files[files.len() - 1].name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        assert!(ext_a.to_lowercase() <= ext_b.to_lowercase());
    }
    Ok(())
}
