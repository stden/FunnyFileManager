//! Purpose: Russian Cucumber steps for parallel streaming delete progress and Esc cancellation.

use std::{cell::RefCell, rc::Rc, time::Duration};

use funny_file_manager::{
    ops::handle_delete_with_cancel,
    progress::{self, Operation, Painter, Phase, Snapshot, Tally},
};

use super::*;

thread_local! {
    static RU_DELETE_FRAMES: RefCell<Vec<Snapshot>> = const { RefCell::new(Vec::new()) };
}

#[derive(Clone, Default)]
struct RuDeleteLog(Rc<RefCell<Vec<Snapshot>>>);

impl Painter for RuDeleteLog {
    fn paint(&mut self, snapshot: &Snapshot) {
        self.0.borrow_mut().push(snapshot.clone());
    }
}

#[when(expr = "я переношу {string} в исходный каталог и удаляю его с мгновенной индикацией")]
fn when_ru_move_and_delete_with_progress(world: &mut FfmWorld, folder: String) -> StepResult {
    let from = world.temp_path()?.join(&folder);
    let dest = world.src_path()?.join(&folder);
    fs::rename(&from, &dest).context("переместить тестовый каталог")?;
    let logger = RuDeleteLog::default();
    let frames = Rc::clone(&logger.0);
    {
        let _active = progress::install_with(Box::new(logger), Duration::ZERO);
        let app = world.app()?;
        app.left_panel
            .load_entries()
            .context("обновить левую панель")?;
        let row = app
            .left_panel
            .entries
            .iter()
            .position(|entry| entry.name == folder)
            .ok_or_else(|| format!("не найден {folder}"))?;
        app.left_panel.select(Some(row));
        handle_delete_confirm(app).context("подтверждение F8")?;
        handle_delete(app).context("удаление F8")?;
    }
    assert!(!dest.exists());
    RU_DELETE_FRAMES.with(|slot| *slot.borrow_mut() = std::mem::take(&mut *frames.borrow_mut()));
    Ok(())
}

#[then(
    expr = "индикация удаления насчитывает {int} файлов в рабочей фазе без предварительного сканирования"
)]
fn then_ru_delete_streamed(_world: &mut FfmWorld, files: u64) -> StepResult {
    let frames = RU_DELETE_FRAMES.with(|slot| slot.borrow().clone());
    let last = frames.last().context("нет кадров индикации удаления")?;
    assert!(frames.iter().all(|frame| frame.phase == Phase::Working));
    assert_eq!(last.operation, Operation::Delete);
    assert_eq!(last.total, Tally { files, bytes: 0 });
    assert_eq!(last.done, last.total);
    Ok(())
}

#[when("я запускаю удаление по F8 и прерываю его по Esc после первого кадра прогресса")]
fn when_ru_interrupt_delete(world: &mut FfmWorld) -> StepResult {
    let logger = RuDeleteLog::default();
    let _active = progress::install_with(Box::new(logger), Duration::ZERO);
    let app = world.app()?;
    handle_delete_confirm(app).context("подтверждение F8")?;
    handle_delete_with_cancel(app, |painted| painted).context("прерывание удаления")
}

#[then(expr = "удаление останавливается со статусом {string} и сохраняет {string}")]
fn then_ru_delete_cancelled(
    world: &mut FfmWorld,
    expected_status: String,
    kept: String,
) -> StepResult {
    let kept_path = world.src_path()?.join(kept);
    let app = world.app()?;
    assert_eq!(app.status.to_string(), expected_status);
    assert!(matches!(app.overlay, Overlay::None));
    assert!(kept_path.exists());
    Ok(())
}
