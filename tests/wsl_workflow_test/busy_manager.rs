//! Purpose: Closing a busy manager keeps the keyboard working and still reports completion.

use super::*;

const PANEL_SELECTED: Color = Color::Rgb(85, 150, 216);

#[test]
fn text_ui_closes_a_busy_manager_keeps_keyboard_working_and_reports_completion() -> io::Result<()> {
    let root = TempDir::new()?;
    let (release, gate) = mpsc::channel();
    let mut backend = FakeWsl::new(root.path().to_path_buf());
    backend.release_update = Some(Mutex::new(gate));
    let backend = Arc::new(backend);
    run(
        Arc::<FakeWsl>::clone(&backend),
        vec![
            open_manager(),
            Step::WaitFor("Debian"),
            key(KeyCode::F(5)),
            Step::WaitFor("Updating WSL..."),
            key(KeyCode::F(6)), // Busy state must suppress a second operation.
            key(KeyCode::Esc),
            Step::Inspect(assert_panels),
            key(KeyCode::Tab),
            key(KeyCode::Down),
            Step::Inspect(|buffer| assert_selected(buffer, "right-only", PANEL_SELECTED)),
            Step::ReleaseUpdate(release),
            Step::WaitFor(PLATFORM_DONE),
            Step::Inspect(assert_background_panels),
            key(KeyCode::F(10)),
            key(KeyCode::Enter),
        ],
    )?;
    assert_eq!(
        backend.operations(),
        [WslOperation::Refresh, WslOperation::Update]
    );
    Ok(())
}

fn assert_background_panels(buffer: &Buffer) {
    assert_panels(buffer);
    assert_selected(buffer, "right-only", PANEL_SELECTED);
    assert!(frame_text(buffer).contains("left-only"));
}
