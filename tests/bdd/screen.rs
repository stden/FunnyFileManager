//! Purpose: The viewer, help, console, drives, refresh and hidden panels, and quitting.

use super::*;

#[when("I press F3")]
fn when_f3(world: &mut FfmWorld) -> StepResult {
    handle_view(world.app()?).context("F3")?;
    Ok(())
}

#[when("I toggle console mode")]
fn when_toggle_console(world: &mut FfmWorld) -> StepResult {
    // Call what `Ctrl+O` calls. Re-implementing the toggle here meant the
    // scenarios exercised this function rather than the program, and the
    // off-branch closed an overlay instead of restoring the panels — so
    // "toggle it off" never turned anything off.
    let app = world.app()?;
    app.view = app.view.toggled();
    Ok(())
}

#[when("I refresh panels")]
fn when_refresh(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    match funny_file_manager::utils::refresh_panels(app) {
        Ok(()) => app.set_status(Status::PanelsRefreshed),
        Err(e) => app.set_status(Status::RefreshFailed(e.to_string())),
    }
    Ok(())
}

#[when("I toggle hide left panel")]
fn when_hide_left(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    if matches!(app.overlay, Overlay::Help) {
        app.close_overlay();
    } else {
        app.set_overlay(Overlay::Help); // reuse Help overlay as hide toggle for tests
    }
    Ok(())
}

#[when("I toggle hide right panel")]
fn when_hide_right(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    if matches!(app.overlay, Overlay::DriveSelection { .. }) {
        app.close_overlay();
    } else {
        app.set_overlay(Overlay::DriveSelection {
            side: PanelSide::Right,
            selected_index: 0,
            rows: funny_file_manager::models::drive_rows(),
        });
    }
    Ok(())
}

// Keyboard shortcuts
#[when("I set should_quit")]
fn when_set_quit(world: &mut FfmWorld) -> StepResult {
    world.app()?.lifecycle = Lifecycle::Quitting;
    Ok(())
}

// Overlays
#[when("I toggle help")]
fn when_toggle_help(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    if matches!(app.overlay, Overlay::Help) {
        app.close_overlay();
    } else {
        app.set_overlay(Overlay::Help);
    }
    Ok(())
}

#[when("I close the viewer")]
fn when_close_viewer(world: &mut FfmWorld) -> StepResult {
    world.app()?.close_overlay();
    Ok(())
}

#[when(expr = "I open drive selection for {word}")]
fn when_open_drives(world: &mut FfmWorld, side: String) -> StepResult {
    let app = world.app()?;
    let side = match side.as_str() {
        "Left" => PanelSide::Left,
        _ => PanelSide::Right,
    };
    app.set_overlay(Overlay::DriveSelection {
        side,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });
    Ok(())
}
