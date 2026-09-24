//! Purpose: Then steps about the status line, overlays, panels on screen and quitting.

use super::*;

#[then(expr = "the status message should contain {string}")]
fn then_status(world: &mut FfmWorld, t: String) -> StepResult {
    let m = &world.app()?.status_text();
    assert!(m.contains(&t), "'{m}' should contain '{t}'");
    Ok(())
}

#[then(expr = "the {string} diagnostic should match the current platform")]
fn then_windows_diagnostic(world: &mut FfmWorld, title: String) -> StepResult {
    let app = world.app()?;
    #[cfg(windows)]
    {
        assert_eq!(app.status, Status::OpenedView(title.clone()));
        let viewer = app.get_viewer().context("Windows diagnostic report")?;
        assert_eq!(viewer.title(), title);
        assert!(!viewer.lines.is_empty(), "report must include its result");
    }
    #[cfg(not(windows))]
    {
        let tool = match title.as_str() {
            "Windows System Messages" => "system",
            "Windows Performance" => "performance",
            other => return Err(format!("unknown Windows diagnostic: {other}")),
        };
        assert_eq!(app.status, Status::WindowsOnly { tool });
        assert!(app.get_viewer().is_none());
    }
    Ok(())
}

#[then("the status message should not be empty")]
fn then_status_not_empty(world: &mut FfmWorld) -> StepResult {
    assert!(!world.app()?.status_text().is_empty());
    Ok(())
}

#[then(expr = "the status message should not contain {string}")]
fn then_status_not_contain(world: &mut FfmWorld, t: String) -> StepResult {
    let m = &world.app()?.status_text();
    assert!(!m.contains(&t), "'{m}' should NOT contain '{t}'");
    Ok(())
}

#[then("the viewer should be open")]
fn then_viewer_open(world: &mut FfmWorld) -> StepResult {
    assert!(world.app()?.get_viewer().is_some());
    Ok(())
}

#[then("the viewer should not be open")]
fn then_viewer_closed(world: &mut FfmWorld) -> StepResult {
    assert!(world.app()?.get_viewer().is_none());
    Ok(())
}

#[then(expr = "the viewer title should contain {string}")]
fn then_viewer_title(world: &mut FfmWorld, t: String) -> StepResult {
    let title = world.app()?.get_viewer().context("viewer")?.title();
    assert!(title.contains(&t), "'{title}' should contain '{t}'");
    Ok(())
}

#[then(expr = "the viewer should show {string}")]
fn then_viewer_shows(world: &mut FfmWorld, t: String) -> StepResult {
    let lines = &world.app()?.get_viewer().context("viewer")?.lines;
    assert!(lines.join("\n").contains(&t));
    Ok(())
}

#[then(expr = "only the {word} panel should be on screen")]
fn then_only_panel_on_screen(world: &mut FfmWorld, side: String) -> StepResult {
    let expected = match side.as_str() {
        "Left" => PanelsShown::LeftOnly,
        _ => PanelsShown::RightOnly,
    };
    assert_eq!(world.app()?.panels_shown, expected);
    Ok(())
}

#[then("both panels should be on screen")]
fn then_both_panels_on_screen(world: &mut FfmWorld) -> StepResult {
    assert_eq!(world.app()?.panels_shown, PanelsShown::Both);
    Ok(())
}

#[then(expr = "the {word} directory should be drawn")]
fn then_directory_drawn(world: &mut FfmWorld, which: String) -> StepResult {
    let name = world.directory_name(&which)?;
    let screen = world.screen_text()?;
    assert!(screen.contains(&name), "{name} is on screen:\n{screen}");
    Ok(())
}

#[then(expr = "the {word} directory should not be drawn")]
fn then_directory_not_drawn(world: &mut FfmWorld, which: String) -> StepResult {
    let name = world.directory_name(&which)?;
    let screen = world.screen_text()?;
    assert!(!screen.contains(&name), "{name} is off screen:\n{screen}");
    Ok(())
}

#[then("console mode should be off")]
fn then_console_off(world: &mut FfmWorld) -> StepResult {
    assert_ne!(world.app()?.view, ViewMode::Console);
    Ok(())
}

#[then("console mode should be on")]
fn then_console_on(world: &mut FfmWorld) -> StepResult {
    assert_eq!(world.app()?.view, ViewMode::Console);
    Ok(())
}

#[then("help should be visible")]
fn then_help_on(world: &mut FfmWorld) -> StepResult {
    assert!(matches!(world.app()?.overlay, Overlay::Help));
    Ok(())
}

#[then("help should not be visible")]
fn then_help_off(world: &mut FfmWorld) -> StepResult {
    assert!(!matches!(world.app()?.overlay, Overlay::Help));
    Ok(())
}

#[then("the app should be quitting")]
fn then_quitting(world: &mut FfmWorld) -> StepResult {
    assert!(!world.app()?.lifecycle.is_running());
    Ok(())
}

#[then("drive selection should be visible")]
fn then_drives_visible(world: &mut FfmWorld) -> StepResult {
    assert!(matches!(
        world.app()?.overlay,
        Overlay::DriveSelection { .. }
    ));
    Ok(())
}
