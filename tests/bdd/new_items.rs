//! Purpose: Making folders with F7 and new files with Shift+F4.

use super::*;

fn push_make_folder(events: &mut Vec<Event>, name: &str, multiple_names: bool) {
    events.push(key_event(KeyCode::F(7)));
    push_text(events, name);
    if multiple_names {
        events.push(key_event(KeyCode::Down));
        events.push(key_event(KeyCode::Down));
        events.push(key_event(KeyCode::Char(' ')));
    }
    events.push(key_event(KeyCode::Enter));
}

#[when("I press F7")]
fn when_f7(world: &mut FfmWorld) -> StepResult {
    handle_mkdir(world.app()?).context("F7")?;
    Ok(())
}

#[when("I open new file dialog")]
fn when_open_new_file(world: &mut FfmWorld) -> StepResult {
    world
        .app()?
        .set_overlay(Overlay::NewFileInput(String::new().into()));
    Ok(())
}

#[when("I open the make folder dialog")]
fn when_open_make_folder(world: &mut FfmWorld) -> StepResult {
    world
        .app()?
        .set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    Ok(())
}

#[when(expr = "I type folder name {string}")]
fn when_type_folder_name(world: &mut FfmWorld, name: String) -> StepResult {
    world.type_in_popup(&name)
}

#[when("I enable processing multiple folder names")]
fn when_enable_multiple_folder_names(world: &mut FfmWorld) -> StepResult {
    // Name -> link type -> multiple names. The disabled target field is
    // intentionally skipped, matching keyboard navigation in the dialog.
    world.send_popup_key(KeyCode::Down)?;
    world.send_popup_key(KeyCode::Down)?;
    world.send_popup_key(KeyCode::Char(' '))
}

#[when("I confirm the make folder dialog")]
fn when_confirm_make_folder(world: &mut FfmWorld) -> StepResult {
    world.send_popup_key(KeyCode::Enter)
}

#[when("I cancel the make folder dialog")]
fn when_cancel_make_folder(world: &mut FfmWorld) -> StepResult {
    world.send_popup_key(KeyCode::Esc)
}

#[when(expr = "I create folder {string} through F7")]
fn when_create_folder_through_f7(world: &mut FfmWorld, name: String) -> StepResult {
    let mut events = Vec::with_capacity(name.chars().count() + 3);
    push_make_folder(&mut events, &name, false);
    push_quit(&mut events);
    world.run_key_workflow(events, "F7 folder workflow")
}

#[when(expr = "I scaffold project {string} with folders {string} through F7")]
fn when_scaffold_project_through_f7(
    world: &mut FfmWorld,
    project: String,
    folders: String,
) -> StepResult {
    let mut events = Vec::new();
    push_make_folder(&mut events, &project, false);
    events.push(key_event(KeyCode::Enter));
    push_make_folder(&mut events, &folders, true);
    push_quit(&mut events);
    world.run_key_workflow(events, "F7 project scaffolding workflow")
}

#[when(expr = "I create nested folder path {string} through F7")]
fn when_create_nested_folder_path_through_f7(world: &mut FfmWorld, path: String) -> StepResult {
    let components = path
        .split('/')
        .filter(|component| !component.is_empty())
        .collect::<Vec<_>>();
    assert_ne!(components.len(), 0, "nested folder path cannot be empty");

    let mut events = Vec::new();
    for (index, component) in components.iter().enumerate() {
        push_make_folder(&mut events, component, false);
        if index + 1 < components.len() {
            events.push(key_event(KeyCode::Enter));
        }
    }
    push_quit(&mut events);
    world.run_key_workflow(events, "F7 nested-folder workflow")
}

#[when(expr = "I create folder {string} on the right panel through F7")]
fn when_create_folder_on_right_panel_through_f7(world: &mut FfmWorld, name: String) -> StepResult {
    let mut events = vec![key_event(KeyCode::Tab)];
    push_make_folder(&mut events, &name, false);
    push_quit(&mut events);
    world.run_key_workflow(events, "right-panel F7 folder workflow")
}

#[when(expr = "I type filename {string}")]
fn when_type_filename(world: &mut FfmWorld, name: String) -> StepResult {
    world.app()?.set_overlay(Overlay::NewFileInput(name.into()));
    Ok(())
}

#[when("I confirm new file")]
fn when_confirm_new_file(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    let input = app.get_new_file_input().unwrap_or_default().to_owned();
    app.close_overlay();
    let input = input.trim().to_owned();
    if input.is_empty() {
        app.set_status(Status::CommandOutput("Filename cannot be empty".to_owned()));
        return Ok(());
    }
    let path = app.get_active_panel().path.join(&input);
    if path.exists() {
        app.set_status(Status::CommandOutput(format!(
            "File already exists: {input}"
        )));
    } else {
        fs::File::create(&path).context("create file")?;
        app.set_status(Status::CommandOutput(format!("Created file: {input}")));
        funny_file_manager::utils::refresh_panels(app).context("refresh panels")?;
    }
    Ok(())
}

#[when(expr = "I create new file {string} in the active panel")]
fn when_create_new_file(world: &mut FfmWorld, name: String) -> StepResult {
    let app = world.app()?;
    let path = app.get_active_panel().path.join(&name);
    fs::File::create(&path).context("create file")?;
    funny_file_manager::utils::refresh_panels(app).context("refresh panels")?;
    Ok(())
}

#[then("the make folder dialog should be closed")]
fn then_make_folder_dialog_closed(world: &mut FfmWorld) -> StepResult {
    assert!(!world.app()?.is_overlay_active());
    Ok(())
}
