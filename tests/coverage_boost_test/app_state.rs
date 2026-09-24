//! Purpose: App state: panel switching and swapping, defaults and the status message.

use super::*;

// ═══════════════════════════════════════════════════════════════════
// App methods
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_app_switch_panel() -> io::Result<()> {
    let mut app = App::new()?;
    assert_eq!(app.active_panel, PanelSide::Left);
    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Right);
    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Left);
    Ok(())
}

#[test]
fn test_app_swap_panels() -> io::Result<()> {
    let temp1 = TempDir::new()?;
    let temp2 = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp1.path().to_path_buf())?;
    app.right_panel = Panel::new(temp2.path().to_path_buf())?;
    let left_path = app.left_panel.path.clone();
    let right_path = app.right_panel.path.clone();
    app.swap_panels();
    assert_eq!(app.left_panel.path, right_path);
    assert_eq!(app.right_panel.path, left_path);
    Ok(())
}

#[test]
fn test_app_get_inactive_panel() -> io::Result<()> {
    let mut app = App::new()?;
    app.active_panel = PanelSide::Left;
    let inactive_path = app.get_inactive_panel().path.clone();
    assert_eq!(inactive_path, app.right_panel.path);
    app.active_panel = PanelSide::Right;
    let inactive_path = app.get_inactive_panel().path.clone();
    assert_eq!(inactive_path, app.left_panel.path);
    Ok(())
}

#[test]
fn test_app_get_active_panel_ref() -> io::Result<()> {
    let app = App::new()?;
    let panel = app.get_active_panel_ref();
    assert_eq!(panel.path, app.left_panel.path);
    Ok(())
}

#[test]
fn test_app_get_inactive_panel_ref() -> io::Result<()> {
    let app = App::new()?;
    let panel = app.get_inactive_panel_ref();
    assert_eq!(panel.path, app.right_panel.path);
    Ok(())
}

// Viewer default
#[test]
fn test_viewer_default() {
    let v = Viewer::default();
    assert!(v.title().is_empty());
    assert!(v.lines.is_empty());
    assert_eq!(v.scroll, ScrollTop::TOP);
}

// App default status message
#[test]
fn test_app_default_status_message() -> io::Result<()> {
    let app = App::new()?;
    assert!(app.status_text().contains("F1 Help"));
    assert!(app.status_text().contains("F5 Copy"));
    Ok(())
}
