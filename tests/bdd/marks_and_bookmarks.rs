//! Purpose: Marking files and setting and using bookmarks, and the checks on them.

use super::*;

#[when(expr = "I set bookmark {int}")]
fn when_set_bookmark(world: &mut FfmWorld, slot: usize) -> StepResult {
    let path = world.app()?.get_active_panel().path.clone();
    world
        .app()?
        .bookmarks
        .set(BookmarkSlot::from_usize(slot).context("slot")?, path);
    Ok(())
}

#[when(expr = "I go to bookmark {int}")]
fn when_go_bookmark(world: &mut FfmWorld, slot: usize) -> StepResult {
    let app = world.app()?;
    if let Some(path) = app
        .bookmarks
        .get(BookmarkSlot::from_usize(slot).context("slot")?)
        .map(Path::to_path_buf)
    {
        let panel = app.get_active_panel();
        panel.path = path;
        panel.load_entries().context("reload the panel")?;
        panel.ensure_selection();
    } else {
        app.set_status(Status::CommandOutput(format!("Bookmark {slot} is empty")));
    }
    Ok(())
}

// Marking
#[when("I toggle mark")]
fn when_toggle_mark(world: &mut FfmWorld) -> StepResult {
    world.panel()?.toggle_mark_selected();
    Ok(())
}

#[when("I clear all marks")]
fn when_clear_marks(world: &mut FfmWorld) -> StepResult {
    world.panel()?.clear_marks();
    Ok(())
}

#[when("I select all files")]
fn when_select_all(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    let panel = app.get_active_panel();
    let names: Vec<String> = panel
        .entries
        .iter()
        .filter(|e| !e.kind.is_dir() && e.name != "..")
        .map(|e| e.name.clone())
        .collect();
    let all_marked = !names.is_empty() && names.iter().all(|n| panel.marked.contains(n.as_str()));
    if all_marked {
        panel.clear_marks();
    } else {
        for n in &names {
            panel.marked.insert(FileName::new(n.clone()));
        }
    }
    Ok(())
}

#[then(expr = "bookmark {int} should be set to {string}")]
fn then_bookmark(world: &mut FfmWorld, slot: usize, sub: String) -> StepResult {
    let bm = world
        .app()?
        .bookmarks
        .get(BookmarkSlot::from_usize(slot).context("slot")?)
        .context("bookmark")?;
    assert!(bm.display().to_string().contains(&sub));
    Ok(())
}

#[then("the active panel should have marks")]
fn then_has_marks(world: &mut FfmWorld) -> StepResult {
    assert!(!world.app()?.get_active_panel().marked.is_empty());
    Ok(())
}

#[then("the active panel should have no marks")]
fn then_no_marks(world: &mut FfmWorld) -> StepResult {
    assert!(world.app()?.get_active_panel().marked.is_empty());
    Ok(())
}

// Marking
#[then(expr = "{string} should be marked")]
fn then_marked(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(world.panel()?.marked.contains(n.as_str()));
    Ok(())
}

#[then(expr = "{string} should not be marked")]
fn then_not_marked(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(!world.panel()?.marked.contains(n.as_str()));
    Ok(())
}

#[then(expr = "{string} should be marked in the app")]
fn then_marked_app(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(world.app()?.get_active_panel().marked.contains(n.as_str()));
    Ok(())
}

#[then(expr = "{string} should not be marked in the app")]
fn then_not_marked_app(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(!world.app()?.get_active_panel().marked.contains(n.as_str()));
    Ok(())
}

#[then("the panel should have no marks")]
fn then_panel_no_marks(world: &mut FfmWorld) -> StepResult {
    assert!(world.panel()?.marked.is_empty());
    Ok(())
}
