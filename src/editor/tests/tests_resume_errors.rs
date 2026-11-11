//! Purpose: Tests of `handle_edit_with` when resuming the terminal fails after the editor returns.
use super::*;

struct FailingResumeOps;

impl TerminalOps<WritableTestBackend> for FailingResumeOps {
    fn enable_raw(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn disable_raw(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn enter_alternate(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Err(io::Error::other("enter_alternate"))
    }

    fn leave_alternate(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }

    fn enable_mouse(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }

    fn disable_mouse(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }

    fn clear(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }

    fn show_cursor(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }

    fn wait_for_key(
        &mut self,
        _terminal: &mut Terminal<WritableTestBackend>,
        _prompt: &str,
    ) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_handle_edit_with_surfaces_resume_error() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    assert!(select_entry(&mut app.left_panel, "file.txt"));

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut terminal_ops = FailingResumeOps;

    let error = handle_edit_with(&mut terminal, &mut app, &mut terminal_ops, |_, _, _| {
        Ok(std::process::ExitStatus::default())
    })
    .expect_err("resume failure should be returned");

    assert_eq!(error.to_string(), "enter_alternate");
    assert!(app.status_text().contains("Edited"));
    Ok(())
}

#[test]
fn test_handle_edit_with_refresh_error_wins_over_resume_error() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"content").expect("write file");

    let mut app = build_app(&temp_dir);
    app.active_panel = PanelSide::Right;
    assert!(select_entry(&mut app.right_panel, "file.txt"));
    app.left_panel.path = unreachable_folder();

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut terminal_ops = FailingResumeOps;

    let error = handle_edit_with(&mut terminal, &mut app, &mut terminal_ops, |_, _, _| {
        Ok(std::process::ExitStatus::default())
    })
    .expect_err("refresh failure should be returned");

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(app.status_text().contains("Edited"));
    Ok(())
}
