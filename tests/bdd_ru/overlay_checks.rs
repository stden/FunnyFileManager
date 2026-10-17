//! Purpose: Then steps about the status line, the viewer, help, console, drives and quitting.

use super::*;

#[then(expr = "сообщение статуса должно содержать {string}")]
fn then_status(world: &mut FfmWorld, t: String) -> StepResult {
    let m = &world.app()?.status_text();
    assert!(m.contains(&t), "'{m}' должно содержать '{t}'");
    Ok(())
}

#[then("сообщение статуса не должно быть пустым")]
fn then_status_not_empty(world: &mut FfmWorld) -> StepResult {
    assert!(!world.app()?.status_text().is_empty());
    Ok(())
}

#[then("просмотрщик должен быть открыт")]
fn then_viewer_open(world: &mut FfmWorld) -> StepResult {
    assert!(world.app()?.get_viewer().is_some());
    Ok(())
}

#[then("информационная панель должна быть открыта")]
fn then_information_open(world: &mut FfmWorld) -> StepResult {
    assert!(
        world
            .app()?
            .get_active_panel_ref()
            .get_selected_entry()
            .is_some_and(|entry| entry.directory_size.is_measured()),
        "F3 на каталоге измеряет его размер в панели, как в Far"
    );
    Ok(())
}

#[then("просмотрщик не должен быть открыт")]
fn then_viewer_closed(world: &mut FfmWorld) -> StepResult {
    assert!(world.app()?.get_viewer().is_none());
    Ok(())
}

#[then(expr = "заголовок просмотрщика должен содержать {string}")]
fn then_viewer_title(world: &mut FfmWorld, t: String) -> StepResult {
    let title = world.app()?.get_viewer().context("просмотрщик")?.title();
    assert!(title.contains(&t), "'{title}' должен содержать '{t}'");
    Ok(())
}

#[then(expr = "просмотрщик должен показывать {string}")]
fn then_viewer_shows(world: &mut FfmWorld, t: String) -> StepResult {
    let lines = &world.app()?.get_viewer().context("просмотрщик")?.lines;
    assert!(lines.join("\n").contains(&t));
    Ok(())
}

#[then("режим консоли должен быть включён")]
fn then_console_on(world: &mut FfmWorld) -> StepResult {
    assert_eq!(world.app()?.view, ViewMode::Console);
    Ok(())
}

#[then("режим консоли должен быть выключен")]
fn then_console_off(world: &mut FfmWorld) -> StepResult {
    assert_ne!(world.app()?.view, ViewMode::Console);
    Ok(())
}

// Справка и выход
#[then("справка должна быть видна")]
fn then_help_on(world: &mut FfmWorld) -> StepResult {
    assert!(matches!(world.app()?.overlay, Overlay::Help));
    Ok(())
}

#[then("приложение должно завершаться")]
fn then_quitting(world: &mut FfmWorld) -> StepResult {
    assert!(!world.app()?.lifecycle.is_running());
    Ok(())
}

#[then("диалог выбора дисков должен быть видимым")]
fn then_drives_visible(world: &mut FfmWorld) -> StepResult {
    assert!(matches!(
        world.app()?.overlay,
        Overlay::DriveSelection { .. }
    ));
    Ok(())
}

// --- THEN: новые шаги проверки ---

#[then(expr = "позиция прокрутки просмотрщика должна быть больше {int}")]
fn then_scroll_gt(world: &mut FfmWorld, n: usize) -> StepResult {
    let scroll = world
        .app()?
        .get_viewer()
        .context("просмотрщик")?
        .scroll
        .lines();
    assert!(scroll > n, "прокрутка {scroll} должна быть > {n}");
    Ok(())
}

#[then(expr = "позиция прокрутки просмотрщика должна быть {int}")]
fn then_scroll_eq(world: &mut FfmWorld, n: usize) -> StepResult {
    let scroll = world
        .app()?
        .get_viewer()
        .context("просмотрщик")?
        .scroll
        .lines();
    assert_eq!(scroll, n);
    Ok(())
}

#[then("справка не должна быть видна")]
fn then_help_off(world: &mut FfmWorld) -> StepResult {
    assert!(!matches!(world.app()?.overlay, Overlay::Help));
    Ok(())
}

#[then("диалог выбора дисков не должен быть видимым")]
fn then_drives_hidden(world: &mut FfmWorld) -> StepResult {
    assert!(!matches!(
        world.app()?.overlay,
        Overlay::DriveSelection { .. }
    ));
    Ok(())
}

#[then(expr = "индекс выбора дисков должен быть {int}")]
fn then_drive_index(world: &mut FfmWorld, idx: usize) -> StepResult {
    assert!(matches!(
        world.app()?.overlay,
        Overlay::DriveSelection {
            selected_index,
            ..
        } if selected_index == idx
    ));
    Ok(())
}
