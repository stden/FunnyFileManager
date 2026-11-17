//! Purpose: Sorting by name, extension and size, and the order that results.

use super::*;

#[when(expr = "I create a panel with sort mode Name")]
fn when_sort_name(world: &mut FfmWorld) -> StepResult {
    let mut p = Panel::new(world.temp_path()?).context("panel")?;
    p.sort_mode = SortMode::Name;
    p.load_entries().context("load")?;
    world.panel = Some(p);
    Ok(())
}

#[when(expr = "I create a panel with sort mode Extension")]
fn when_sort_ext(world: &mut FfmWorld) -> StepResult {
    let mut p = Panel::new(world.temp_path()?).context("panel")?;
    p.sort_mode = SortMode::Extension;
    p.load_entries().context("load")?;
    world.panel = Some(p);
    Ok(())
}

#[when(expr = "I cycle sort mode {int} times")]
fn when_cycle_sort(world: &mut FfmWorld, n: usize) -> StepResult {
    for _ in 0..n {
        world.panel()?.cycle_sort_mode();
    }
    Ok(())
}

#[when("I sort by name")]
fn when_sort_by_name(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    let panel = app.get_active_panel();
    panel.sort_mode = SortMode::Name;
    panel.load_entries().context("reload the panel")?;
    app.set_status(Status::Sort("Name".to_owned()));
    Ok(())
}

#[when("I sort by extension")]
fn when_sort_by_ext(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    let panel = app.get_active_panel();
    panel.sort_mode = SortMode::Extension;
    panel.load_entries().context("reload the panel")?;
    app.set_status(Status::Sort("Extension".to_owned()));
    Ok(())
}

#[when("I sort by size")]
fn when_sort_by_size(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    let panel = app.get_active_panel();
    panel.sort_mode = SortMode::Size;
    panel.load_entries().context("reload the panel")?;
    app.set_status(Status::Sort("Size".to_owned()));
    Ok(())
}

#[then("the sort mode should be back to Name")]
fn then_sort_name(world: &mut FfmWorld) -> StepResult {
    assert_eq!(world.panel()?.sort_mode, SortMode::Name);
    Ok(())
}

#[then("the first extension should come before the last alphabetically")]
fn then_ext_sorted(world: &mut FfmWorld) -> StepResult {
    let files: Vec<_> = world
        .panel()?
        .entries
        .iter()
        .filter(|e| !e.kind.is_dir() && e.name != "..")
        .collect();
    if let [first, .., last] = files.as_slice() {
        let a = first.name.rsplit('.').next().unwrap_or("");
        let b = last.name.rsplit('.').next().unwrap_or("");
        assert!(a <= b, "'{a}' should <= '{b}'");
    }
    Ok(())
}

// Command line
#[then(expr = "the sort mode should be {word}")]
fn then_sort_mode(world: &mut FfmWorld, mode: String) -> StepResult {
    let expected = match mode.as_str() {
        "Name" => SortMode::Name,
        "Extension" => SortMode::Extension,
        "Size" => SortMode::Size,
        _ => return Err(format!("unknown sort mode: {mode}")),
    };
    assert_eq!(world.app()?.get_active_panel().sort_mode, expected);
    Ok(())
}

#[then(expr = "the sort mode {word} is rejected")]
fn then_sort_mode_rejected(world: &mut FfmWorld, mode: String) {
    bdd_runner::expect_error(then_sort_mode(world, mode), "unknown sort mode");
}
