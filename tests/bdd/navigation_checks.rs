//! Purpose: Then steps about panel paths, the active panel and the selection.

use super::*;

// ═══════════════════════════════════════════════════════════════════
// THEN steps
// ═══════════════════════════════════════════════════════════════════

#[then(expr = "the panel should have {int} entries")]
fn then_panel_count(world: &mut FfmWorld, n: usize) -> StepResult {
    assert_eq!(world.panel()?.entries.len(), n);
    Ok(())
}

#[then(expr = "entry {int} should be {string}")]
fn then_entry(world: &mut FfmWorld, i: usize, name: String) -> StepResult {
    let entries = &world.panel()?.entries;
    let entry = entries.get(i).ok_or_else(|| {
        format!(
            "index out of bounds: the len is {} but the index is {i}",
            entries.len()
        )
    })?;
    assert_eq!(entry.name, name);
    Ok(())
}

#[then(expr = "the selection should be at index {int}")]
fn then_sel(world: &mut FfmWorld, i: usize) -> StepResult {
    assert_eq!(world.panel()?.selected(), Some(i));
    Ok(())
}

#[then(expr = "the panel path should end with {string}")]
fn then_path_ends(world: &mut FfmWorld, s: String) -> StepResult {
    let p = if let Some(ref panel) = world.panel {
        panel.path.display().to_string()
    } else {
        world.app()?.get_active_panel().path.display().to_string()
    };
    assert!(p.ends_with(&s), "'{p}' should end with '{s}'");
    Ok(())
}

#[then(expr = "the active panel path should end with {string}")]
fn then_active_ends(world: &mut FfmWorld, s: String) -> StepResult {
    let p = world.app()?.get_active_panel().path.display().to_string();
    assert!(p.ends_with(&s), "'{p}' should end with '{s}'");
    Ok(())
}

#[then(expr = "the active panel path should not end with {string}")]
fn then_active_not_ends(world: &mut FfmWorld, s: String) -> StepResult {
    let p = world.app()?.get_active_panel().path.display().to_string();
    assert!(!p.ends_with(&s), "'{p}' should not end with '{s}'");
    Ok(())
}

#[then("the active panel should be at the source filesystem root")]
fn then_active_at_root(world: &mut FfmWorld) -> StepResult {
    let source = world
        .src_path()?
        .canonicalize()
        .context("source directory")?;
    let root = source.ancestors().last().context("filesystem root")?;
    let actual = world
        .app()?
        .get_active_panel()
        .path
        .canonicalize()
        .context("active directory")?;
    assert_eq!(actual, root);
    Ok(())
}

#[then("the active panel should be at the source directory")]
fn then_active_at_source(world: &mut FfmWorld) -> StepResult {
    let source = world
        .src_path()?
        .canonicalize()
        .context("source directory")?;
    let actual = world
        .app()?
        .get_active_panel()
        .path
        .canonicalize()
        .context("active directory")?;
    assert_eq!(actual, source);
    Ok(())
}

#[then(expr = "the active panel selection should be {string}")]
fn then_active_selection(world: &mut FfmWorld, name: String) -> StepResult {
    let selected = world
        .app()?
        .get_active_panel_ref()
        .get_selected_entry()
        .map(|entry| entry.name.as_str());
    assert_eq!(selected, Some(name.as_str()));
    Ok(())
}

#[then(expr = "the selected entry should be {string}")]
fn then_selected(world: &mut FfmWorld, n: String) -> StepResult {
    assert_eq!(world.panel()?.get_selected_entry().context("sel")?.name, n);
    Ok(())
}

// Panel operations
#[then(expr = "the active panel should be {word}")]
fn then_active_panel(world: &mut FfmWorld, side: String) -> StepResult {
    let expected = match side.as_str() {
        "Left" => PanelSide::Left,
        _ => PanelSide::Right,
    };
    assert_eq!(world.app()?.active_panel, expected);
    Ok(())
}

#[then("the left panel path should differ from original")]
fn then_left_differs(world: &mut FfmWorld) -> StepResult {
    let orig = world.original_left_path.clone();
    if let Some(orig) = orig {
        assert_ne!(&world.app()?.left_panel.path, &orig);
    }
    Ok(())
}

// Editor
#[then("the left panel should have entries")]
fn then_left_has_entries(world: &mut FfmWorld) -> StepResult {
    assert!(!world.app()?.left_panel.entries.is_empty());
    Ok(())
}

#[then("the right panel should have entries")]
fn then_right_has_entries(world: &mut FfmWorld) -> StepResult {
    assert!(!world.app()?.right_panel.entries.is_empty());
    Ok(())
}

#[then("the left panel should have a valid selection")]
#[expect(
    clippy::unwrap_used,
    reason = "the assert on the previous line proves sel is Some"
)]
fn then_left_valid_sel(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    let sel = app.left_panel.selected();
    assert!(sel.is_some(), "left panel should have a selection");
    assert!(
        sel.unwrap() < app.left_panel.entries.len(),
        "selection should be within bounds"
    );
    Ok(())
}
