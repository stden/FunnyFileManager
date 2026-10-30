//! Purpose: Moving the cursor and the panels: arrows, paging, Enter, Tab, root, parent and search.

use super::*;

// ═══════════════════════════════════════════════════════════════════
// WHEN steps
// ═══════════════════════════════════════════════════════════════════

#[when("I create a panel at the temporary directory")]
fn when_create_panel(world: &mut FfmWorld) -> StepResult {
    world.panel = Some(Panel::new(world.temp_path()?).context("create panel")?);
    Ok(())
}

#[when("I press Down")]
fn when_down(world: &mut FfmWorld) -> StepResult {
    world.panel()?.next();
    Ok(())
}

#[when("I press Up")]
fn when_up(world: &mut FfmWorld) -> StepResult {
    world.panel()?.previous();
    Ok(())
}

#[when("I press Home")]
fn when_home(world: &mut FfmWorld) -> StepResult {
    world.panel()?.select(Some(0));
    Ok(())
}

#[when("I press End")]
fn when_end(world: &mut FfmWorld) -> StepResult {
    let last = world.panel()?.entries.len().saturating_sub(1);
    world.panel()?.select(Some(last));
    Ok(())
}

#[when("I press Enter")]
fn when_enter(world: &mut FfmWorld) -> StepResult {
    world.panel()?.enter().context("enter")?;
    Ok(())
}

#[when(expr = "I quick search for {string}")]
fn when_quick_search(world: &mut FfmWorld, q: String) -> StepResult {
    world.panel()?.find_first_matching(&q);
    Ok(())
}

#[when(expr = "I quick search for {string} in the app")]
fn when_quick_search_app(world: &mut FfmWorld, q: String) -> StepResult {
    let app = world.app()?;
    app.get_active_panel().find_first_matching(&q);
    Ok(())
}

// Panel operations. These call what the keys call, so the guard that keeps
// the cursor off a hidden panel (Ctrl+F1 / Ctrl+F2) is the one under test.
#[when("I press Tab")]
fn when_tab(world: &mut FfmWorld) -> StepResult {
    panels::switch_panel(world.app()?);
    Ok(())
}

#[when("I press Left")]
fn when_left(world: &mut FfmWorld) -> StepResult {
    panels::select(world.app()?, PanelSide::Left);
    Ok(())
}

#[when("I press Right")]
fn when_right(world: &mut FfmWorld) -> StepResult {
    panels::select(world.app()?, PanelSide::Right);
    Ok(())
}

#[when("I press Ctrl+F1")]
fn when_ctrl_f1(world: &mut FfmWorld) -> StepResult {
    panels::toggle(world.app()?, PanelSide::Left);
    Ok(())
}

#[when("I press Ctrl+F2")]
fn when_ctrl_f2(world: &mut FfmWorld) -> StepResult {
    panels::toggle(world.app()?, PanelSide::Right);
    Ok(())
}

#[when(expr = "I set the active panel to Right")]
fn when_set_right(world: &mut FfmWorld) -> StepResult {
    world.app()?.active_panel = PanelSide::Right;
    Ok(())
}

#[when("I swap panels")]
fn when_swap(world: &mut FfmWorld) -> StepResult {
    panels::swap_panels(world.app()?);
    Ok(())
}

#[when(expr = "I navigate to {string}")]
fn when_navigate_to(world: &mut FfmWorld, sub: String) -> StepResult {
    let app = world.app()?;
    let new_path = app.get_active_panel().path.join(&sub);
    let panel = app.get_active_panel();
    panel.path = new_path;
    panel.load_entries().context("reload the panel")?;
    panel.ensure_selection();
    Ok(())
}

#[when("I go to parent directory")]
fn when_go_parent(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    if let Some(parent) = app.get_active_panel().path.parent().map(Path::to_path_buf) {
        app.get_active_panel().path = parent;
        app.get_active_panel()
            .load_entries()
            .context("reload the panel")?;
    }
    Ok(())
}

#[when("I press PageUp")]
fn when_page_up(world: &mut FfmWorld) -> StepResult {
    let panel = world.panel()?;
    if !panel.entries.is_empty() {
        let cur = panel.selected().unwrap_or(0);
        panel.select(Some(cur.saturating_sub(10)));
    }
    Ok(())
}

#[when("I press PageDown")]
fn when_page_down(world: &mut FfmWorld) -> StepResult {
    let panel = world.panel()?;
    if !panel.entries.is_empty() {
        let cur = panel.selected().unwrap_or(0);
        let max = panel.entries.len() - 1;
        panel.select(Some((cur + 10).min(max)));
    }
    Ok(())
}

#[when("I switch to the right panel")]
fn when_switch_right(world: &mut FfmWorld) -> StepResult {
    world.app()?.active_panel = PanelSide::Right;
    Ok(())
}

#[when(expr = "I select {string} in the right panel")]
fn when_select_right(world: &mut FfmWorld, name: String) -> StepResult {
    let app = world.app()?;
    let idx = app
        .right_panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .ok_or_else(|| format!("entry '{name}' not found in right panel"))?;
    app.right_panel.select(Some(idx));
    Ok(())
}

#[when(expr = "I select {string} in the left panel")]
fn when_select_left(world: &mut FfmWorld, name: String) -> StepResult {
    let app = world.app()?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .ok_or_else(|| format!("entry '{name}' not found in left panel"))?;
    app.left_panel.select(Some(idx));
    Ok(())
}

#[when("the panel lists no entries")]
fn when_panel_lists_nothing(world: &mut FfmWorld) -> StepResult {
    world.panel()?.entries.clear();
    Ok(())
}

#[then("a missing entry is named on either side")]
fn then_missing_on_either_side(world: &mut FfmWorld) {
    let name = "no-such-entry".to_owned();
    bdd_runner::expect_error(
        when_select_right(world, name.clone()),
        "not found in right panel",
    );
    bdd_runner::expect_error(when_select_left(world, name), "not found in left panel");
}
