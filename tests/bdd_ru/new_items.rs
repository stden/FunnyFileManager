//! Purpose: Making folders with F7 and new files with Shift+F4, and the new-file dialog.

use super::*;

#[when("я нажимаю F7")]
fn when_f7(world: &mut FfmWorld) -> StepResult {
    handle_mkdir(world.app()?).context("F7")?;
    Ok(())
}

// Создание файлов
#[when("я открываю диалог нового файла")]
fn when_open_new_file(world: &mut FfmWorld) -> StepResult {
    world
        .app()?
        .set_overlay(Overlay::NewFileInput(String::new()));
    Ok(())
}

#[when(expr = "я ввожу имя файла {string}")]
fn when_type_filename(world: &mut FfmWorld, name: String) -> StepResult {
    world.app()?.set_overlay(Overlay::NewFileInput(name));
    Ok(())
}

#[when("я подтверждаю создание файла")]
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
        fs::File::create(&path).context("создать файл")?;
        app.set_status(Status::CommandOutput(format!("Created file: {input}")));
        funny_file_manager::utils::refresh_panels(app).context("обновить панели")?;
    }
    Ok(())
}

#[when(expr = "я ввожу символ {string} в диалог нового файла")]
fn when_new_file_char(world: &mut FfmWorld, ch: String) -> StepResult {
    let c = ch.chars().next().context("пустая строка вместо символа")?;
    let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty());
    process_popup_input(world.app()?, key);
    Ok(())
}

#[when("я отправляю клавишу Enter в диалог нового файла")]
fn when_new_file_enter(world: &mut FfmWorld) -> StepResult {
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    process_popup_input(world.app()?, key);
    Ok(())
}

#[when("я нажимаю Backspace в диалоге нового файла")]
fn when_new_file_backspace(world: &mut FfmWorld) -> StepResult {
    let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
    process_popup_input(world.app()?, key);
    Ok(())
}

#[when("я отправляю клавишу Esc в диалог нового файла")]
fn when_new_file_esc(world: &mut FfmWorld) -> StepResult {
    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    process_popup_input(world.app()?, key);
    Ok(())
}

#[then("диалог нового файла должен быть закрыт")]
fn then_new_file_closed(world: &mut FfmWorld) -> StepResult {
    assert!(world.app()?.get_new_file_input().is_none());
    Ok(())
}

#[then(expr = "буфер диалога нового файла должен содержать {string}")]
fn then_new_file_buf(world: &mut FfmWorld, t: String) -> StepResult {
    let buf = world.app()?.get_new_file_input().context("диалог")?;
    assert_eq!(buf, t.as_str());
    Ok(())
}
