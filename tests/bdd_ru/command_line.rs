//! Purpose: The command line: running commands, typing and what it holds.

use super::*;

#[when("я перехожу в корневой каталог")]
fn when_go_root(world: &mut FfmWorld) -> StepResult {
    when_exec_cmd(world, "cd /".to_owned())
}

// Командная строка
#[when(expr = "я выполняю команду {string}")]
fn when_exec_cmd(world: &mut FfmWorld, cmd: String) -> StepResult {
    let app = world.app()?;
    app.set_overlay(Overlay::CommandLine(cmd.into()));
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    process_popup_input(app, key);
    Ok(())
}

#[when("я нажимаю Backspace в командной строке")]
fn when_cmdline_backspace(world: &mut FfmWorld) -> StepResult {
    let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
    process_popup_input(world.app()?, key);
    Ok(())
}

#[when(expr = "я ввожу символ {string} в командную строку")]
fn when_cmdline_char(world: &mut FfmWorld, ch: String) -> StepResult {
    let c = ch.chars().next().context("пустая строка вместо символа")?;
    let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty());
    process_popup_input(world.app()?, key);
    Ok(())
}

#[then("командная строка должна быть закрыта")]
fn then_cmdline_closed(world: &mut FfmWorld) -> StepResult {
    assert!(world.app()?.get_command_line().is_none());
    Ok(())
}

#[then(expr = "командная строка должна содержать {string}")]
fn then_cmdline_has(world: &mut FfmWorld, t: String) -> StepResult {
    let cmd = world
        .app()?
        .get_command_line()
        .context("командная строка")?;
    assert_eq!(cmd, t.as_str());
    Ok(())
}
