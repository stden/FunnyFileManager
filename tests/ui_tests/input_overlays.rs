//! Purpose: Delete confirmation, new-file input and drive selection overlays.

use super::*;

// ---------------------------------------------------------------------------
// Delete confirmation overlay tests
// ---------------------------------------------------------------------------

#[test]
fn test_delete_confirm_shows_file_count() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.set_overlay(Overlay::DeleteConfirm(vec![
        PathBuf::from("/tmp/a.txt"),
        PathBuf::from("/tmp/b.txt"),
        PathBuf::from("/tmp/c.txt"),
    ]));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    assert!(
        all_text.contains("Do you wish to delete 3 items?"),
        "Delete confirm should count the items, got:\n{all_text}"
    );
    // Braces mark the button Enter starts on; brackets mark the rest.
    assert!(
        all_text.contains("{ Delete }") && all_text.contains("[ Cancel ]"),
        "Delete confirm should offer both buttons, got:\n{all_text}"
    );
}

#[test]
fn test_delete_confirm_truncates_long_list() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    let paths: Vec<PathBuf> = (0..10)
        .map(|i| PathBuf::from(format!("file{i}.txt")))
        .collect();
    app.set_overlay(Overlay::DeleteConfirm(paths));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    assert!(
        all_text.contains("… and 5 more"),
        "Should summarise the files past the first five, got:\n{all_text}"
    );
}

// ---------------------------------------------------------------------------
// New file input overlay tests
// ---------------------------------------------------------------------------

#[test]
fn test_new_file_input_overlay() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.set_overlay(Overlay::NewFileInput("newfile.txt".to_owned().into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    assert!(
        all_text.contains("Create New File"),
        "New file overlay should show title"
    );
    assert!(
        all_text.contains("newfile.txt"),
        "New file overlay should show typed filename"
    );
}

#[test]
fn test_new_file_input_empty() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.set_overlay(Overlay::NewFileInput(String::new().into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    assert!(
        all_text.contains("Enter filename"),
        "Empty new file input should show prompt"
    );
}

// ---------------------------------------------------------------------------
// Drive selection overlay tests
// ---------------------------------------------------------------------------

#[test]
fn test_drive_selection_overlay_left() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    // Far titles it "Change drive" for either panel: which one it targets is
    // already answered by which key opened it, so the dialog does not repeat it.
    assert!(
        all_text.contains("Change drive"),
        "the drive dialog should be drawn for the left panel"
    );
}

#[test]
fn test_drive_selection_overlay_right() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Right,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let mut all_text = String::new();
    for y in 0..24 {
        all_text.push_str(&row_text(&terminal, y));
        all_text.push('\n');
    }

    assert!(
        all_text.contains("Change drive"),
        "the drive dialog should be drawn for the right panel"
    );
}
