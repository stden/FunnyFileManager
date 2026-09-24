//! Purpose: Rendering the panels, overlays, command line and status line.

use super::*;

// ═══════════════════════════════════════════════════════════════════
// UI rendering — console mode and overlays
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_ui_renders_console_mode() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = setup_app(&temp)?;
    app.view = ViewMode::Console;
    let mut terminal = setup_terminal()?;
    terminal.draw(|f| ui::ui(f, &mut app))?;
    Ok(())
}

#[test]
fn test_ui_renders_help_overlay() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = setup_app(&temp)?;
    app.set_overlay(Overlay::Help);
    let mut terminal = setup_terminal()?;
    terminal.draw(|f| ui::ui(f, &mut app))?;
    Ok(())
}

#[test]
fn test_ui_renders_viewer_overlay() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = setup_app(&temp)?;
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "Test",
        vec!["line1".to_owned(), "line2".to_owned()],
    )));
    let mut terminal = setup_terminal()?;
    terminal.draw(|f| ui::ui(f, &mut app))?;
    Ok(())
}

#[test]
fn test_ui_renders_viewer_with_scroll() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = setup_app(&temp)?;
    let lines: Vec<String> = (0..50).map(|i| format!("line {i}")).collect();
    app.set_overlay(Overlay::Viewer(Viewer {
        scroll: ScrollTop::TOP.down(10, usize::MAX),
        ..Viewer::report("Scrolled", lines)
    }));
    let mut terminal = setup_terminal()?;
    terminal.draw(|f| ui::ui(f, &mut app))?;
    Ok(())
}

#[test]
fn test_ui_renders_delete_confirmation() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = setup_app(&temp)?;
    app.set_overlay(Overlay::DeleteConfirm(vec![temp.path().join("doomed.txt")]));
    let mut terminal = setup_terminal()?;
    terminal.draw(|f| ui::ui(f, &mut app))?;
    Ok(())
}

#[test]
fn test_ui_renders_new_file_input() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = setup_app(&temp)?;
    app.set_overlay(Overlay::NewFileInput("newfile.txt".to_owned()));
    let mut terminal = setup_terminal()?;
    terminal.draw(|f| ui::ui(f, &mut app))?;
    Ok(())
}

#[test]
fn test_ui_renders_drive_selection() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = setup_app(&temp)?;
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });
    let mut terminal = setup_terminal()?;
    terminal.draw(|f| ui::ui(f, &mut app))?;
    Ok(())
}

#[test]
fn test_ui_renders_command_line() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("cd /tmp".into()));
    let mut terminal = setup_terminal()?;
    terminal.draw(|f| ui::ui(f, &mut app))?;
    Ok(())
}

#[test]
fn test_ui_renders_quick_search_in_status() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = setup_app(&temp)?;
    app.quick_search = QuickSearch::Active("fi".to_owned());
    let mut terminal = setup_terminal()?;
    terminal.draw(|f| ui::ui(f, &mut app))?;
    Ok(())
}

#[test]
fn test_ui_renders_with_marked_files() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("marked.txt"), "m")?;
    let mut app = setup_app(&temp)?;
    app.left_panel.marked.insert(FileName::from("marked.txt"));
    let mut terminal = setup_terminal()?;
    terminal.draw(|f| ui::ui(f, &mut app))?;
    Ok(())
}

#[test]
fn test_ui_renders_right_panel_active() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = setup_app(&temp)?;
    app.active_panel = PanelSide::Right;
    let mut terminal = setup_terminal()?;
    terminal.draw(|f| ui::ui(f, &mut app))?;
    Ok(())
}

#[test]
fn test_ui_renders_with_no_selection() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = setup_app(&temp)?;
    app.left_panel.select(None);
    let mut terminal = setup_terminal()?;
    terminal.draw(|f| ui::ui(f, &mut app))?;
    Ok(())
}
