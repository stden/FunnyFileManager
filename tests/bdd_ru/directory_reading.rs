//! Purpose: Russian Cucumber steps for long directory reading progress and Esc interruption.

use std::{cell::RefCell, rc::Rc, time::Duration};

use funny_file_manager::{
    progress::{self, DirectoryRead, Painter, Snapshot},
    ui::reading_banner,
};
use ratatui::layout::Rect;

use super::*;

#[derive(Clone, Default)]
struct RuBannerLog(Rc<RefCell<Vec<DirectoryRead>>>);

impl Painter for RuBannerLog {
    fn paint(&mut self, _snapshot: &Snapshot) {}

    fn paint_directory_read(&mut self, read: &DirectoryRead) {
        self.0.borrow_mut().push(*read);
    }
}

#[given(expr = "подкаталог {string} с {int} файлами во временном каталоге")]
fn given_ru_populated_subdir(world: &mut FfmWorld, sub: String, amount: usize) -> StepResult {
    let target = world.temp_path()?.join(sub);
    fs::create_dir_all(&target).context("создать подкаталог")?;
    for idx in 0..amount {
        fs::write(target.join(format!("ru_{idx:02}.dat")), b"1").context("записать файл")?;
    }
    Ok(())
}

#[when(expr = "я вхожу в подкаталог {string} с мгновенной индикацией чтения")]
fn when_ru_enter_with_banner(world: &mut FfmWorld, sub: String) -> StepResult {
    let logger = RuBannerLog::default();
    let events = Rc::clone(&logger.0);
    {
        let _active = progress::install_with(Box::new(logger), Duration::ZERO);
        let destination = world.temp_path()?.join(sub);
        world
            .panel()?
            .navigate_to(destination)
            .context("переход с индикацией")?;
    }
    world.reading_sample = events.borrow().last().copied();
    Ok(())
}

#[then(expr = "баннер чтения содержит {string}")]
fn then_ru_banner_contains(world: &mut FfmWorld, needle: String) -> StepResult {
    let state = world.reading_sample.context("нет данных баннера")?;
    let strip = reading_banner(Rect::new(0, 0, 100, 25), &state);
    let line: String = (strip.area.left()..strip.area.right())
        .filter_map(|x| {
            strip
                .cell((x, strip.area.y))
                .map(ratatui::buffer::Cell::symbol)
        })
        .collect();
    assert!(line.contains(&needle), "ожидалось {needle:?} в {line:?}");
    Ok(())
}

#[when(expr = "я вхожу в подкаталог {string} и прерываю чтение после {int} файлов")]
fn when_ru_enter_interrupted(world: &mut FfmWorld, sub: String, cap: usize) -> StepResult {
    let destination = world.temp_path()?.join(sub);
    let mut counter = 0usize;
    world
        .panel()?
        .navigate_to_with_cancel(destination, |_| {
            counter += 1;
            counter >= cap
        })
        .context("прерванный переход")
}

#[then(expr = "в панели отображается {string} и {int} прочитанных файла")]
fn then_ru_partial_entries(world: &mut FfmWorld, dotdot: String, kept: usize) -> StepResult {
    let panel = world.panel()?;
    assert_eq!(
        panel.entries.first().map(|row| row.name.as_str()),
        Some(dotdot.as_str())
    );
    assert_eq!(panel.entries.len(), kept + 1);
    Ok(())
}
