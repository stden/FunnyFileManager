//! Purpose: Updating the platform and the selected packages, then opening a normal panel.

use super::*;

const MENU_SELECTED: Color = Color::Rgb(0, 0, 0);

#[test]
fn text_ui_updates_platform_and_selected_packages_then_opens_a_normal_panel() -> io::Result<()> {
    let root = TempDir::new()?;
    fs::write(root.path().join("root-visible.txt"), "distribution file")?;
    let backend = Arc::new(FakeWsl::new(root.path().to_path_buf()));
    run(Arc::<FakeWsl>::clone(&backend), update_and_open_steps())?;
    assert_eq!(
        backend.operations(),
        [
            WslOperation::Refresh,
            WslOperation::Update,
            WslOperation::UpdatePackages(distribution("Debian")),
            WslOperation::Open(distribution("Debian")),
        ]
    );
    assert_eq!(
        fs::read_to_string(root.path().join("root-visible.txt"))?,
        "distribution file"
    );
    Ok(())
}

fn assert_manager_controls(buffer: &Buffer) {
    let frame = frame_text(buffer);
    for label in [
        "Enter Open root",
        "F2 Refresh list",
        "F5 Update WSL",
        "F6 Update packages",
    ] {
        assert!(frame.contains(label), "missing {label}:\n{frame}");
    }
    assert_selected(buffer, "Ubuntu", MENU_SELECTED);
}

fn assert_compact_manager(buffer: &Buffer) {
    assert_selected(buffer, "Debian", MENU_SELECTED);
    let frame = frame_text(buffer);
    for label in ["Open", "F5", "F6", "Esc"] {
        assert!(
            frame.contains(label),
            "missing compact control {label}:\n{frame}"
        );
    }
}

fn update_and_open_steps() -> Vec<Step> {
    vec![
        open_manager(),
        Step::WaitFor("Debian"),
        Step::Inspect(assert_manager_controls),
        key(KeyCode::Down),
        Step::Resize(Size::new(18, 6)),
        Step::Inspect(assert_compact_manager),
        Step::Resize(Size::new(90, 20)),
        key(KeyCode::F(5)),
        Step::WaitFor(PLATFORM_DONE),
        key(KeyCode::F(6)),
        Step::WaitFor(PACKAGES_DONE),
        key(KeyCode::Enter),
        // Panel names and extensions occupy separate columns.
        Step::WaitFor("root-visible"),
        Step::Inspect(assert_panels),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ]
}
