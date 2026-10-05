//! Purpose: F8 with nothing to delete and F7 with no free folder name.

use super::*;

// -----------------------------------------------------------------------
// handle_delete_confirm: no selection returns early
// -----------------------------------------------------------------------

#[test]
fn test_handle_delete_confirm_no_selection_returns_early() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.left_panel.select(None);

    handle_delete_confirm(&mut app)?;

    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));

    Ok(())
}

// -----------------------------------------------------------------------
// handle_mkdir exhausted counter
// -----------------------------------------------------------------------

#[test]
fn test_handle_mkdir_exhausted() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let base = temp_dir.path();

    // Create new_folder and new_folder_1..9999
    fs::create_dir(base.join("new_folder"))?;
    for i in 1..=9999u32 {
        fs::create_dir(base.join(format!("new_folder_{i}")))?;
    }

    let mut app = App::new()?;
    app.left_panel = Panel::new(base.to_path_buf())?;
    app.right_panel = Panel::new(base.to_path_buf())?;

    let err = handle_mkdir(&mut app).expect_err("should fail after 9999 attempts");
    assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);

    Ok(())
}

#[test]
fn test_delete_confirm_dotdot_noop() -> io::Result<()> {
    let temp = TempDir::new()?;
    let subdir = temp.path().join("child");
    fs::create_dir(&subdir)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(subdir)?;
    app.left_panel.load_entries()?;

    // Select ".."
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == ".." {
            app.left_panel.select(Some(i));
            break;
        }
    }

    handle_delete_confirm(&mut app)?;
    assert!(
        !matches!(app.overlay, Overlay::DeleteConfirm(_)),
        "should not confirm delete on .."
    );
    Ok(())
}

#[test]
fn test_delete_confirm_no_selection_returns_ok() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    handle_delete_confirm(&mut app)?;
    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));
    Ok(())
}
