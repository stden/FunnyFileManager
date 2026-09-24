//! Purpose: The command line: running commands, inserting names and paths, and what it holds.

use super::*;

// Command line — use process_popup_input to execute commands
#[when(expr = "I execute command {string}")]
fn when_exec_cmd(world: &mut FfmWorld, cmd: String) -> StepResult {
    let app = world.app()?;
    app.set_overlay(Overlay::CommandLine(cmd.into()));
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    process_popup_input(app, key);
    Ok(())
}

#[when("I go to root")]
fn when_go_root(world: &mut FfmWorld) -> StepResult {
    when_exec_cmd(world, "cd /".to_owned())
}

#[when("I insert filename into command line")]
fn when_insert_filename(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    if let Some(name) = app
        .get_active_panel()
        .get_selected_entry()
        .map(|e| e.name.clone())
    {
        if app.get_command_line().is_none() {
            app.set_overlay(Overlay::CommandLine(String::new().into()));
        }
        let cmd = app.get_command_line_mut().context("command line")?;
        if !cmd.is_empty() {
            cmd.insert_char(' ');
        }
        cmd.insert_str(&name);
    }
    Ok(())
}

#[when("I insert full path into command line")]
fn when_insert_path(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    if let Some(path) = app.get_active_panel().selected_path() {
        let s = path.display().to_string();
        if app.get_command_line().is_none() {
            app.set_overlay(Overlay::CommandLine(String::new().into()));
        }
        let cmd = app.get_command_line_mut().context("command line")?;
        if !cmd.is_empty() {
            cmd.insert_char(' ');
        }
        cmd.insert_str(&s);
    }
    Ok(())
}

// Keyboard shortcuts
#[then(expr = "the command line should contain {string}")]
fn then_cmd_has(world: &mut FfmWorld, t: String) -> StepResult {
    let cmd = world.app()?.get_command_line().unwrap_or("");
    assert!(cmd.contains(&t), "cmd '{cmd}' should contain '{t}'");
    Ok(())
}
