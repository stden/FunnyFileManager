//! Purpose: Given steps that open the app or a panel with a selection, marks or bookmarks.

use super::*;

#[given("a panel at the temporary directory")]
fn given_panel_at_temp(world: &mut FfmWorld) -> StepResult {
    world.panel = Some(Panel::new(world.temp_path()?).context("create panel")?);
    Ok(())
}

#[given(expr = "a panel at {string} in the temporary directory")]
fn given_panel_at_sub(world: &mut FfmWorld, sub: String) -> StepResult {
    let path = world.temp_path()?.join(&sub);
    world.panel = Some(Panel::new(path).context("create panel")?);
    Ok(())
}

#[given(expr = "the selection is at index {int}")]
fn given_selection_at(world: &mut FfmWorld, index: usize) -> StepResult {
    world.panel()?.select(Some(index));
    Ok(())
}

#[given(expr = "I select the entry {string}")]
fn given_select_entry(world: &mut FfmWorld, name: String) -> StepResult {
    let panel = world.panel()?;
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .ok_or_else(|| format!("entry '{name}' not found"))?;
    panel.select(Some(idx));
    Ok(())
}

#[given("the app is set up with source and destination")]
fn given_app_src_dst(world: &mut FfmWorld) -> StepResult {
    let mut app = App::new().context("create app")?;
    app.left_panel = Panel::new(world.src_path()?).context("left panel")?;
    app.right_panel = Panel::new(world.dst_path()?).context("right panel")?;
    app.active_panel = PanelSide::Left;
    world.original_left_path = Some(app.left_panel.path.clone());
    world.app = Some(app);
    Ok(())
}

#[given(expr = "the app is set up at {string} in the source")]
fn given_app_at_src_sub(world: &mut FfmWorld, sub: String) -> StepResult {
    let src_sub = world.src_path()?.join(&sub);
    let dst = world.dst_path()?;
    let mut app = App::new().context("create app")?;
    app.left_panel = Panel::new(src_sub).context("left panel")?;
    app.right_panel = Panel::new(dst).context("right panel")?;
    app.active_panel = PanelSide::Left;
    world.original_left_path = Some(app.left_panel.path.clone());
    world.app = Some(app);
    Ok(())
}

#[given(expr = "an app at {string} in the temporary directory")]
fn given_app_at_sub(world: &mut FfmWorld, sub: String) -> StepResult {
    let path = world.temp_path()?.join(&sub);
    let mut app = App::new().context("create app")?;
    app.left_panel = Panel::new(path).context("panel")?;
    app.right_panel = Panel::new(world.temp_path()?).context("right panel")?;
    world.app = Some(app);
    Ok(())
}

#[given("an app at the temporary directory")]
fn given_app_at_temp(world: &mut FfmWorld) -> StepResult {
    let path = world.temp_path()?;
    let mut app = App::new().context("create app")?;
    app.left_panel = Panel::new(path.clone()).context("left panel")?;
    app.right_panel = Panel::new(path).context("right panel")?;
    world.app = Some(app);
    Ok(())
}

#[given(expr = "I select {string} in the left panel")]
fn given_select_left(world: &mut FfmWorld, name: String) -> StepResult {
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

#[given(expr = "I select {string} in the active panel")]
fn given_select_active(world: &mut FfmWorld, name: String) -> StepResult {
    let app = world.app()?;
    let idx = app
        .get_active_panel()
        .entries
        .iter()
        .position(|e| e.name == name)
        .ok_or_else(|| format!("entry '{name}' not found"))?;
    app.get_active_panel().select(Some(idx));
    Ok(())
}

#[given(expr = "I select {string} in the right panel")]
fn given_select_right(world: &mut FfmWorld, name: String) -> StepResult {
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

#[given(expr = "I mark {string} in the left panel")]
#[when(expr = "I mark {string} in the left panel")]
fn given_mark_left(world: &mut FfmWorld, name: String) -> StepResult {
    world.app()?.left_panel.marked.insert(FileName::new(name));
    Ok(())
}

#[given(expr = "the file {string} is marked")]
fn given_file_marked(world: &mut FfmWorld, name: String) -> StepResult {
    world
        .app()?
        .get_active_panel()
        .marked
        .insert(FileName::new(name));
    Ok(())
}

#[given(expr = "{string} is marked")]
fn given_marked(world: &mut FfmWorld, name: String) -> StepResult {
    world.panel()?.marked.insert(FileName::new(name));
    Ok(())
}

#[given(expr = "bookmark {int} is empty")]
fn given_bookmark_empty(world: &mut FfmWorld, slot: usize) -> StepResult {
    world
        .app()?
        .bookmarks
        .clear(BookmarkSlot::from_usize(slot).context("slot")?);
    Ok(())
}

#[given("the left panel has no selection")]
fn given_no_selection(world: &mut FfmWorld) -> StepResult {
    world.app()?.left_panel.select(None);
    Ok(())
}

#[then("a missing entry is named in each panel")]
fn then_missing_entries(world: &mut FfmWorld) {
    let name = "no-such-entry".to_owned();
    bdd_runner::expect_error(given_select_entry(world, name.clone()), "not found");
    bdd_runner::expect_error(
        given_select_left(world, name.clone()),
        "not found in left panel",
    );
    bdd_runner::expect_error(given_select_active(world, name.clone()), "not found");
    bdd_runner::expect_error(given_select_right(world, name), "not found in right panel");
}
