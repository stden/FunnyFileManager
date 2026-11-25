//! Purpose: The viewer, console, drives and dialogs: opening, scrolling, answering and closing.

use super::*;

#[when("я нажимаю F3")]
fn when_f3(world: &mut FfmWorld) -> StepResult {
    handle_view(world.app()?).context("F3")?;
    Ok(())
}

#[when("я переключаю режим консоли")]
fn when_toggle_console(world: &mut FfmWorld) -> StepResult {
    // То же, что делает Ctrl+O. Своя реализация переключателя означала, что
    // сценарии проверяли эту функцию, а не программу: ветка «выключить»
    // закрывала оверлей вместо возврата к панелям, и потому не выключала.
    let app = world.app()?;
    app.view = app.view.toggled();
    Ok(())
}

// Просмотрщик
#[when("я закрываю просмотрщик")]
fn when_close_viewer(world: &mut FfmWorld) -> StepResult {
    world.app()?.close_overlay();
    Ok(())
}

// Выход и диски
#[when("я устанавливаю флаг выхода")]
fn when_set_quit(world: &mut FfmWorld) -> StepResult {
    world.app()?.lifecycle = Lifecycle::Quitting;
    Ok(())
}

#[when(expr = "я открываю выбор дисков для {word}")]
fn when_open_drives(world: &mut FfmWorld, side: String) -> StepResult {
    let app = world.app()?;
    let panel_side = match side.as_str() {
        "Left" => PanelSide::Left,
        _ => PanelSide::Right,
    };
    app.set_overlay(Overlay::DriveSelection {
        side: panel_side,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });
    Ok(())
}

// --- WHEN: новые шаги действий ---

#[when("я прокручиваю просмотрщик вниз")]
fn when_scroll_viewer_down(world: &mut FfmWorld) -> StepResult {
    let key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
    process_popup_input(world.app()?, key);
    Ok(())
}

#[when("я прокручиваю просмотрщик вверх")]
fn when_scroll_viewer_up(world: &mut FfmWorld) -> StepResult {
    let key = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
    process_popup_input(world.app()?, key);
    Ok(())
}

#[when("я отправляю клавишу Esc в оверлей")]
fn when_overlay_esc(world: &mut FfmWorld) -> StepResult {
    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    process_popup_input(world.app()?, key);
    Ok(())
}

#[when("я отправляю клавишу Enter в оверлей")]
fn when_overlay_enter(world: &mut FfmWorld) -> StepResult {
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    process_popup_input(world.app()?, key);
    Ok(())
}

#[when("я отправляю клавишу Y в оверлей")]
fn when_overlay_y(world: &mut FfmWorld) -> StepResult {
    let key = KeyEvent::new(KeyCode::Char('Y'), KeyModifiers::empty());
    process_popup_input(world.app()?, key);
    Ok(())
}

#[when("я отправляю клавишу N в оверлей")]
fn when_overlay_n(world: &mut FfmWorld) -> StepResult {
    let key = KeyEvent::new(KeyCode::Char('N'), KeyModifiers::empty());
    process_popup_input(world.app()?, key);
    Ok(())
}

#[when("я нажимаю Down в выборе дисков")]
fn when_drives_down(world: &mut FfmWorld) -> StepResult {
    let key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
    process_popup_input(world.app()?, key);
    Ok(())
}

#[when("я нажимаю Up в выборе дисков")]
fn when_drives_up(world: &mut FfmWorld) -> StepResult {
    let key = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
    process_popup_input(world.app()?, key);
    Ok(())
}

#[when("я нажимаю End в выборе дисков")]
fn when_drives_end(world: &mut FfmWorld) -> StepResult {
    let key = KeyEvent::new(KeyCode::End, KeyModifiers::empty());
    process_popup_input(world.app()?, key);
    Ok(())
}

#[when("я нажимаю Home в выборе дисков")]
fn when_drives_home(world: &mut FfmWorld) -> StepResult {
    let key = KeyEvent::new(KeyCode::Home, KeyModifiers::empty());
    process_popup_input(world.app()?, key);
    Ok(())
}

#[when("я отправляю клавишу Enter в выбор дисков")]
fn when_drives_enter(world: &mut FfmWorld) -> StepResult {
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    process_popup_input(world.app()?, key);
    Ok(())
}
