//! Purpose: Panel contents: column headers, entries, directory markers, sizes and marks.

use super::*;

// ---------------------------------------------------------------------------
// Top-row tests
// ---------------------------------------------------------------------------

#[test]
fn test_column_headers_replace_the_old_banner() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // The banner is gone — it only repeated the F-key bar.
    let screen = terminal.backend().to_string();
    assert!(!screen.contains("Far Manager style"), "{screen}");

    // Row 2 is Far's column header: menu bar, top border, then the header.
    let header = row_text(&terminal, 2);
    assert!(header.contains("Name"), "got: '{header}'");
    assert!(header.contains("Size"), "got: '{header}'");
    assert!(header.contains("Date"), "got: '{header}'");
    assert!(header.contains("Time"), "got: '{header}'");
}

// ---------------------------------------------------------------------------
// Panel rendering tests
// ---------------------------------------------------------------------------

#[test]
fn test_panels_show_directory_entries() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Collect all rendered text to check panel entries appear somewhere
    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    assert!(all_text.contains("docs"), "Panel should show 'docs' entry");
    // Far splits a file row at its final dot and right-aligns the extension
    // against the Size column, so the two halves are no longer adjacent.
    assert!(
        all_text.contains("readme") && all_text.contains("md"),
        "Panel should show 'readme.md' split into name and extension"
    );
    assert!(
        all_text.contains("main") && all_text.contains("rs"),
        "Panel should show 'main.rs' split into name and extension"
    );
    assert!(all_text.contains("src"), "Right panel should show 'src'");
}

#[test]
fn test_panels_show_dir_marker() {
    // Default 80-column terminal: the size column must fit there, not only on a
    // wide one — a fixed 30-cell name field used to leave 3 columns for it.
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    // Far names a directory instead of measuring it.
    assert!(
        all_text.contains("Folder"),
        "Directories should be labelled 'Folder'"
    );
    assert!(all_text.contains("Up"), "'..' should be labelled 'Up'");
}

#[test]
fn test_panel_shows_file_sizes() {
    // Default 80-column terminal — see test_panels_show_dir_marker.
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    // Far prints the byte count itself while it fits the seven-cell column.
    assert!(
        all_text.contains("4096"),
        "Should show the exact byte count '4096'"
    );
}

#[test]
fn test_marked_files_appear() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.left_panel.marked.insert(FileName::from("readme.md"));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // The marked file should still be visible
    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }
    assert!(all_text.contains("readme"));
}
