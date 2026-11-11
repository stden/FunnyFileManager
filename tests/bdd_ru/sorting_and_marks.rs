//! Purpose: Sorting, marking and bookmarks, and the checks on them.

use super::*;

// Закладки
#[when(expr = "я устанавливаю закладку {int}")]
fn when_set_bookmark(world: &mut FfmWorld, slot: usize) -> StepResult {
    let path = world.app()?.get_active_panel().path.clone();
    world
        .app()?
        .bookmarks
        .set(BookmarkSlot::from_usize(slot).context("slot")?, path);
    Ok(())
}

#[when(expr = "я перехожу к закладке {int}")]
fn when_go_bookmark(world: &mut FfmWorld, slot: usize) -> StepResult {
    let app = world.app()?;
    if let Some(path) = app
        .bookmarks
        .get(BookmarkSlot::from_usize(slot).context("slot")?)
        .map(Path::to_path_buf)
    {
        let panel = app.get_active_panel();
        panel.path = path;
        panel.load_entries().context("перезагрузить панель")?;
        panel.ensure_selection();
    } else {
        app.set_status(Status::CommandOutput(format!("Bookmark {slot} is empty")));
    }
    Ok(())
}

// Сортировка
#[when("я создаю панель с сортировкой по расширению")]
fn when_sort_ext(world: &mut FfmWorld) -> StepResult {
    let mut p = Panel::new(world.temp_path()?).context("панель")?;
    p.sort_mode = SortMode::Extension;
    p.load_entries().context("загрузить")?;
    world.panel = Some(p);
    Ok(())
}

#[when(expr = "я переключаю режим сортировки {int} раза")]
fn when_cycle_sort_3(world: &mut FfmWorld, n: usize) -> StepResult {
    for _ in 0..n {
        world.panel()?.cycle_sort_mode();
    }
    Ok(())
}

#[when(expr = "я переключаю режим сортировки {int} раз")]
fn when_cycle_sort_n(world: &mut FfmWorld, n: usize) -> StepResult {
    for _ in 0..n {
        world.panel()?.cycle_sort_mode();
    }
    Ok(())
}

// Пометка
#[when("я переключаю пометку")]
fn when_toggle_mark(world: &mut FfmWorld) -> StepResult {
    world.panel()?.toggle_mark_selected();
    Ok(())
}

#[when("я снимаю все пометки")]
fn when_clear_marks(world: &mut FfmWorld) -> StepResult {
    world.panel()?.clear_marks();
    Ok(())
}

#[when("я выделяю все файлы")]
fn when_select_all(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    let panel = app.get_active_panel();
    let names: Vec<String> = panel
        .entries
        .iter()
        .filter(|e| !e.kind.is_dir() && e.name != "..")
        .map(|e| e.name.clone())
        .collect();
    for n in &names {
        panel.marked.insert(FileName::new(n.clone()));
    }
    Ok(())
}

#[then(expr = "режим сортировки должен быть {word}")]
fn then_sort_mode(world: &mut FfmWorld, mode: String) -> StepResult {
    let expected = match mode.as_str() {
        "Name" => SortMode::Name,
        "Extension" => SortMode::Extension,
        "Size" => SortMode::Size,
        _ => return Err(format!("неизвестный режим сортировки: {mode}")),
    };
    assert_eq!(world.app()?.get_active_panel().sort_mode, expected);
    Ok(())
}

#[then(expr = "режим сортировки {word} отвергается")]
fn then_sort_mode_rejected(world: &mut FfmWorld, mode: String) {
    bdd_runner::expect_error(then_sort_mode(world, mode), "неизвестный режим сортировки");
}

#[then("режим сортировки должен вернуться к Name")]
fn then_sort_name(world: &mut FfmWorld) -> StepResult {
    assert_eq!(world.panel()?.sort_mode, SortMode::Name);
    Ok(())
}

#[then("первое расширение должно быть раньше последнего по алфавиту")]
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
        assert!(a <= b, "'{a}' должно быть <= '{b}'");
    }
    Ok(())
}

// Пометка
#[then(expr = "{string} должен быть помечен")]
fn then_marked(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(world.panel()?.marked.contains(n.as_str()));
    Ok(())
}

#[then(expr = "{string} не должен быть помечен")]
fn then_not_marked(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(!world.panel()?.marked.contains(n.as_str()));
    Ok(())
}

#[then("панель не должна иметь пометок")]
fn then_panel_no_marks(world: &mut FfmWorld) -> StepResult {
    assert_eq!(world.panel()?.marked.len(), 0);
    Ok(())
}

#[then("активная панель должна иметь пометки")]
fn then_has_marks(world: &mut FfmWorld) -> StepResult {
    assert_ne!(world.app()?.get_active_panel().marked.len(), 0);
    Ok(())
}

#[then("активная панель не должна иметь пометок")]
fn then_no_marks(world: &mut FfmWorld) -> StepResult {
    assert_eq!(world.app()?.get_active_panel().marked.len(), 0);
    Ok(())
}

// Закладки
#[then(expr = "закладка {int} должна указывать на {string}")]
fn then_bookmark(world: &mut FfmWorld, slot: usize, sub: String) -> StepResult {
    let bm = world
        .app()?
        .bookmarks
        .get(BookmarkSlot::from_usize(slot).context("slot")?)
        .context("закладка")?;
    assert!(bm.display().to_string().contains(&sub));
    Ok(())
}
