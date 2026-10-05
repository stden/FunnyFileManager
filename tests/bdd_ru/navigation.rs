//! Purpose: Moving the cursor and the panels: arrows, paging, Enter, Tab, parent and search.

use super::*;

// ═══════════════════════════════════════════════════════════════════
// КОГДА (When) — шаги действий
// ═══════════════════════════════════════════════════════════════════

#[when("я нажимаю Down")]
fn when_down(world: &mut FfmWorld) -> StepResult {
    world.panel()?.next();
    Ok(())
}

#[when("я нажимаю Up")]
fn when_up(world: &mut FfmWorld) -> StepResult {
    world.panel()?.previous();
    Ok(())
}

#[when("я нажимаю Home")]
fn when_home(world: &mut FfmWorld) -> StepResult {
    world.panel()?.select(Some(0));
    Ok(())
}

#[when("я нажимаю End")]
fn when_end(world: &mut FfmWorld) -> StepResult {
    let last = world.panel()?.entries.len().saturating_sub(1);
    world.panel()?.select(Some(last));
    Ok(())
}

#[when("я нажимаю Enter")]
fn when_enter(world: &mut FfmWorld) -> StepResult {
    world.panel()?.enter().context("enter")?;
    Ok(())
}

#[when("я нажимаю PageUp")]
fn when_page_up(world: &mut FfmWorld) -> StepResult {
    let panel = world.panel()?;
    if !panel.entries.is_empty() {
        let cur = panel.selected().unwrap_or(0);
        panel.select(Some(cur.saturating_sub(10)));
    }
    Ok(())
}

#[when("я нажимаю PageDown")]
fn when_page_down(world: &mut FfmWorld) -> StepResult {
    let panel = world.panel()?;
    if !panel.entries.is_empty() {
        let cur = panel.selected().unwrap_or(0);
        let max = panel.entries.len() - 1;
        panel.select(Some((cur + 10).min(max)));
    }
    Ok(())
}

#[when("я нажимаю Tab")]
fn when_tab(world: &mut FfmWorld) -> StepResult {
    world.app()?.switch_panel();
    Ok(())
}

#[when("я нажимаю Left")]
fn when_left(world: &mut FfmWorld) -> StepResult {
    world.app()?.active_panel = PanelSide::Left;
    Ok(())
}

#[when("я нажимаю Right")]
fn when_right(world: &mut FfmWorld) -> StepResult {
    world.app()?.active_panel = PanelSide::Right;
    Ok(())
}

#[when("я меняю панели местами")]
fn when_swap(world: &mut FfmWorld) -> StepResult {
    world.app()?.swap_panels();
    Ok(())
}

#[when("я перехожу в родительский каталог")]
fn when_go_parent(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    if let Some(parent) = app.get_active_panel().path.parent().map(Path::to_path_buf) {
        app.get_active_panel().path = parent;
        app.get_active_panel()
            .load_entries()
            .context("перезагрузить панель")?;
    }
    Ok(())
}

#[when("я переключаюсь на правую панель")]
fn when_switch_right(world: &mut FfmWorld) -> StepResult {
    world.app()?.active_panel = PanelSide::Right;
    Ok(())
}

#[when(expr = "я выбираю {string} в правой панели")]
fn when_select_right(world: &mut FfmWorld, name: String) -> StepResult {
    let app = world.app()?;
    let idx = app
        .right_panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .ok_or_else(|| format!("запись '{name}' не найдена в правой панели"))?;
    app.right_panel.select(Some(idx));
    Ok(())
}

// Быстрый поиск
#[when(expr = "я ищу {string}")]
fn when_quick_search(world: &mut FfmWorld, q: String) -> StepResult {
    world.panel()?.find_first_matching(&q);
    Ok(())
}

#[when(expr = "я выбираю запись {string}")]
fn when_select_entry(world: &mut FfmWorld, name: String) -> StepResult {
    let panel = world.panel()?;
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .ok_or_else(|| format!("запись '{name}' не найдена"))?;
    panel.select(Some(idx));
    Ok(())
}

#[when(expr = "я ищу следующее совпадение {string}")]
fn when_find_next(world: &mut FfmWorld, q: String) -> StepResult {
    world.panel()?.find_next_matching(&q);
    Ok(())
}

#[when("я перезагружаю панель")]
fn when_reload_panel(world: &mut FfmWorld) -> StepResult {
    world.panel()?.load_entries().context("загрузить")?;
    Ok(())
}

#[when("я вызываю ensure_selection")]
fn when_ensure_selection(world: &mut FfmWorld) -> StepResult {
    world.panel()?.ensure_selection();
    Ok(())
}
