//! Purpose: Given steps that open the app or a panel with a selection, marks, dialogs or a sort.

use super::*;

#[given("панель во временном каталоге")]
fn given_panel_at_temp(world: &mut FfmWorld) -> StepResult {
    world.panel = Some(Panel::new(world.temp_path()?).context("создать панель")?);
    Ok(())
}

#[given(expr = "выделение на позиции {int}")]
fn given_selection_at(world: &mut FfmWorld, index: usize) -> StepResult {
    world.panel()?.select(Some(index));
    Ok(())
}

#[given(expr = "я выбираю запись {string}")]
fn given_select_entry(world: &mut FfmWorld, name: String) -> StepResult {
    let panel = world.panel()?;
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .ok_or_else(|| format!("запись '{name}' не найдена"))?;
    panel.select(Some(idx));
    Ok(())
}

#[given("приложение настроено с исходным каталогом и каталогом назначения")]
fn given_app_src_dst(world: &mut FfmWorld) -> StepResult {
    let mut app = App::new().context("создать приложение")?;
    app.left_panel = Panel::new(world.src_path()?).context("левая панель")?;
    app.right_panel = Panel::new(world.dst_path()?).context("правая панель")?;
    app.active_panel = PanelSide::Left;
    world.original_left_path = Some(app.left_panel.path.clone());
    world.app = Some(app);
    Ok(())
}

#[given(expr = "приложение в подкаталоге {string} во временном каталоге")]
fn given_app_at_sub(world: &mut FfmWorld, sub: String) -> StepResult {
    let path = world.temp_path()?.join(&sub);
    let mut app = App::new().context("создать приложение")?;
    app.left_panel = Panel::new(path).context("панель")?;
    app.right_panel = Panel::new(world.temp_path()?).context("правая панель")?;
    world.app = Some(app);
    Ok(())
}

#[given(expr = "я выбираю {string} в левой панели")]
fn given_select_left(world: &mut FfmWorld, name: String) -> StepResult {
    let app = world.app()?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .ok_or_else(|| format!("запись '{name}' не найдена в левой панели"))?;
    app.left_panel.select(Some(idx));
    Ok(())
}

#[given(expr = "я выбираю {string} в правой панели")]
fn given_select_right(world: &mut FfmWorld, name: String) -> StepResult {
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

#[given(expr = "{string} помечен")]
fn given_marked(world: &mut FfmWorld, name: String) -> StepResult {
    world.panel()?.marked.insert(FileName::new(name));
    Ok(())
}

#[given(expr = "закладка {int} пуста")]
fn given_bookmark_empty(world: &mut FfmWorld, slot: usize) -> StepResult {
    world
        .app()?
        .bookmarks
        .clear(BookmarkSlot::from_usize(slot).context("slot")?);
    Ok(())
}

#[given("я переключаюсь на правую панель")]
fn given_switch_right(world: &mut FfmWorld) -> StepResult {
    world.app()?.active_panel = PanelSide::Right;
    Ok(())
}

#[given("левая панель без выделения")]
fn given_no_selection(world: &mut FfmWorld) -> StepResult {
    world.app()?.left_panel.select(None);
    Ok(())
}

#[given(expr = "командная строка активна с текстом {string}")]
fn given_cmdline_active(world: &mut FfmWorld, text: String) -> StepResult {
    world.app()?.set_overlay(Overlay::CommandLine(text.into()));
    Ok(())
}

#[given("диалог нового файла открыт")]
fn given_new_file_dialog(world: &mut FfmWorld) -> StepResult {
    world
        .app()?
        .set_overlay(Overlay::NewFileInput(String::new().into()));
    Ok(())
}

#[given(expr = "диалог нового файла открыт с текстом {string}")]
fn given_new_file_dialog_with_text(world: &mut FfmWorld, text: String) -> StepResult {
    world.app()?.set_overlay(Overlay::NewFileInput(text.into()));
    Ok(())
}

#[given(expr = "я помечаю {string} в левой панели")]
fn given_mark_left(world: &mut FfmWorld, name: String) -> StepResult {
    world.app()?.left_panel.marked.insert(FileName::new(name));
    Ok(())
}

#[given(expr = "панель во временном каталоге с сортировкой по размеру")]
fn given_panel_sorted_by_size(world: &mut FfmWorld) -> StepResult {
    let mut p = Panel::new(world.temp_path()?).context("панель")?;
    p.sort_mode = SortMode::Size;
    p.load_entries().context("загрузить")?;
    world.panel = Some(p);
    Ok(())
}
