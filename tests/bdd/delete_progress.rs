//! Purpose: English Cucumber steps for parallel streaming delete progress and Esc cancellation.

use std::{cell::RefCell, rc::Rc};

use funny_file_manager::{
    ops::handle_delete_with_cancel,
    progress::{self, Operation, Painter, Phase, Snapshot, Tally},
};

use super::*;

thread_local! {
    static CAPTURED_DELETE: RefCell<Vec<Snapshot>> = const { RefCell::new(Vec::new()) };
}

#[derive(Clone, Default)]
struct DeleteCapture(Rc<RefCell<Vec<Snapshot>>>);

impl Painter for DeleteCapture {
    fn paint(&mut self, snapshot: &Snapshot) {
        self.0.borrow_mut().push(snapshot.clone());
    }
}

#[when("I delete the selection with immediate progress capture")]
fn when_delete_with_progress_capture(world: &mut FfmWorld) -> StepResult {
    let capture = DeleteCapture::default();
    let recorded = Rc::clone(&capture.0);
    {
        let _guard = progress::install_with(Box::new(capture), Duration::ZERO);
        let app = world.app()?;
        handle_delete_confirm(app).context("F8 confirm")?;
        handle_delete(app).context("F8 delete")?;
    }
    CAPTURED_DELETE.with(|slot| *slot.borrow_mut() = std::mem::take(&mut *recorded.borrow_mut()));
    Ok(())
}

#[then(expr = "the delete progress streamed {int} files in working phase without scanning")]
fn then_delete_streamed_without_scanning(_world: &mut FfmWorld, files: u64) -> StepResult {
    let frames = CAPTURED_DELETE.with(|slot| slot.borrow().clone());
    let last = frames.last().context("no delete progress frames painted")?;
    assert!(
        frames.iter().all(|frame| frame.phase == Phase::Working),
        "expected no Scanning phase in {frames:?}"
    );
    assert_eq!(last.operation, Operation::Delete);
    assert_eq!(last.total, Tally { files, bytes: 0 });
    assert_eq!(last.done, last.total);
    Ok(())
}

#[then(expr = "{string} does not exist in the source")]
fn then_missing_in_source(world: &mut FfmWorld, name: String) -> StepResult {
    assert!(!world.src_path()?.join(name).exists());
    Ok(())
}

#[when("I press F8 and interrupt deletion with Esc after the first progress paint")]
fn when_f8_and_interrupt_delete(world: &mut FfmWorld) -> StepResult {
    let capture = DeleteCapture::default();
    let _guard = progress::install_with(Box::new(capture), Duration::ZERO);
    let app = world.app()?;
    handle_delete_confirm(app).context("F8 confirm")?;
    handle_delete_with_cancel(app, |painted| painted).context("interruptible delete")
}

#[then(expr = "deletion stops cleanly with {string} and keeps {string} in the source")]
fn then_delete_cancelled_keeps_file(
    world: &mut FfmWorld,
    expected_status: String,
    kept: String,
) -> StepResult {
    let src_kept = world.src_path()?.join(kept);
    let app = world.app()?;
    assert_eq!(app.status.to_string(), expected_status);
    assert!(matches!(app.overlay, Overlay::None));
    assert!(src_kept.exists());
    Ok(())
}
