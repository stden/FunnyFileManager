//! Purpose: Then steps about panel paths, the active panel, entries and the selection.

use super::*;

// ═══════════════════════════════════════════════════════════════════
// ТОГДА (Then) — шаги проверки
// ═══════════════════════════════════════════════════════════════════

#[then(expr = "выделение должно быть на позиции {int}")]
fn then_sel(world: &mut FfmWorld, i: usize) -> StepResult {
    assert_eq!(world.panel()?.selected(), Some(i));
    Ok(())
}

#[then(expr = "путь панели должен заканчиваться на {string}")]
fn then_path_ends(world: &mut FfmWorld, s: String) -> StepResult {
    let p = if let Some(ref panel) = world.panel {
        panel.path.display().to_string()
    } else {
        world.app()?.get_active_panel().path.display().to_string()
    };
    assert!(p.ends_with(&s), "'{p}' должен заканчиваться на '{s}'");
    Ok(())
}

#[then(expr = "путь активной панели должен заканчиваться на {string}")]
fn then_active_ends(world: &mut FfmWorld, s: String) -> StepResult {
    let p = world.app()?.get_active_panel().path.display().to_string();
    assert!(p.ends_with(&s), "'{p}' должен заканчиваться на '{s}'");
    Ok(())
}

#[then("активная панель должна быть в корне файловой системы источника")]
fn then_active_at_root(world: &mut FfmWorld) -> StepResult {
    let source = world
        .src_path()?
        .canonicalize()
        .context("исходный каталог")?;
    let root = source
        .ancestors()
        .last()
        .context("корень файловой системы")?;
    let actual = world
        .app()?
        .get_active_panel()
        .path
        .canonicalize()
        .context("активный каталог")?;
    assert_eq!(actual, root);
    Ok(())
}

#[then(expr = "выбранная запись должна быть {string}")]
fn then_selected(world: &mut FfmWorld, n: String) -> StepResult {
    assert_eq!(
        world
            .panel()?
            .get_selected_entry()
            .context("выделение")?
            .name,
        n
    );
    Ok(())
}

#[then(expr = "активная панель должна быть {word}")]
fn then_active_panel(world: &mut FfmWorld, side: String) -> StepResult {
    let expected = match side.as_str() {
        "Left" => PanelSide::Left,
        _ => PanelSide::Right,
    };
    assert_eq!(world.app()?.active_panel, expected);
    Ok(())
}

#[then("путь левой панели должен отличаться от исходного")]
fn then_left_differs(world: &mut FfmWorld) -> StepResult {
    let orig = world.original_left_path.clone();
    if let Some(orig) = orig {
        assert_ne!(&world.app()?.left_panel.path, &orig);
    }
    Ok(())
}

#[then(expr = "панель должна содержать запись {string}")]
fn then_panel_has_entry(world: &mut FfmWorld, name: String) -> StepResult {
    let panel = world.panel()?;
    assert!(
        panel.entries.iter().any(|e| e.name == name),
        "панель не содержит запись '{name}'"
    );
    Ok(())
}

#[then(expr = "первая файловая запись должна быть {string}")]
fn then_first_file_entry(world: &mut FfmWorld, name: String) -> StepResult {
    let panel = world.panel()?;
    let first_file = panel
        .entries
        .iter()
        .find(|e| !e.kind.is_dir() && e.name != "..")
        .context("нет файлов")?;
    assert_eq!(first_file.name, name);
    Ok(())
}

#[then(expr = "action_entry_names должен содержать {int} элемента")]
fn then_action_names_count_a(world: &mut FfmWorld, n: usize) -> StepResult {
    assert_eq!(world.panel()?.action_entry_names().len(), n);
    Ok(())
}

#[then(expr = "action_entry_names должен содержать {int} элемент")]
fn then_action_names_count_b(world: &mut FfmWorld, n: usize) -> StepResult {
    assert_eq!(world.panel()?.action_entry_names().len(), n);
    Ok(())
}

#[then("панель должна иметь допустимое выделение")]
#[expect(
    clippy::unwrap_used,
    reason = "the assert on the previous line proves sel is Some"
)]
fn then_valid_selection(world: &mut FfmWorld) -> StepResult {
    let panel = world.panel()?;
    if !panel.entries.is_empty() {
        let sel = panel.selected();
        assert!(sel.is_some());
        assert!(sel.unwrap() < panel.entries.len());
    }
    Ok(())
}

#[then("неактивная панель не должна быть пустой")]
fn then_inactive_not_empty(world: &mut FfmWorld) -> StepResult {
    assert_ne!(world.app()?.get_inactive_panel().entries.len(), 0);
    Ok(())
}
