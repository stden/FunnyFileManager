//! Purpose: English Cucumber steps for Far-style directory reading progress and Esc interruption.

use std::{cell::RefCell, rc::Rc};

use funny_file_manager::{
    progress::{self, DirectoryRead, Painter, Snapshot},
    ui::reading_banner,
};
use ratatui::layout::Rect;

use super::*;

#[derive(Clone, Default)]
struct EnglishReadCapture(Rc<RefCell<Vec<DirectoryRead>>>);

impl Painter for EnglishReadCapture {
    fn paint(&mut self, _snapshot: &Snapshot) {}

    fn paint_directory_read(&mut self, read: &DirectoryRead) {
        self.0.borrow_mut().push(*read);
    }
}

#[given(expr = "a folder {string} in the source with {int} numbered files")]
fn given_source_folder_with_files(
    world: &mut FfmWorld,
    folder: String,
    total: usize,
) -> StepResult {
    let dir = world.src_path()?.join(folder);
    fs::create_dir_all(&dir).context("create source subfolder")?;
    for index in 0..total {
        fs::write(dir.join(format!("item_{index:03}.txt")), "x").context("write source item")?;
    }
    Ok(())
}

#[when(expr = "I render the app and enter {string} with immediate reading progress")]
fn when_enter_with_reading_progress(world: &mut FfmWorld, folder: String) -> StepResult {
    drop(world.screen_text()?);
    let capture = EnglishReadCapture::default();
    let recorded = Rc::clone(&capture.0);
    {
        let _guard = progress::install_with(Box::new(capture), Duration::ZERO);
        let app = world.app()?;
        let target = app.left_panel.path.join(folder);
        app.left_panel
            .navigate_to(target)
            .context("navigate with progress")?;
    }
    world.last_reading = recorded.borrow().last().copied();
    Ok(())
}

#[then(expr = "the reading banner shows {string} on the left panel border")]
fn then_reading_banner_shows(world: &mut FfmWorld, expected: String) -> StepResult {
    let sample = world.last_reading.context("no reading progress captured")?;
    assert_eq!(sample.header.map(|header| header.x), Some(0));
    let buffer = reading_banner(Rect::new(0, 0, 80, 24), &sample);
    let rendered: String = (buffer.area.left()..buffer.area.right())
        .filter_map(|col| {
            buffer
                .cell((col, buffer.area.y))
                .map(ratatui::buffer::Cell::symbol)
        })
        .collect();
    assert!(
        rendered.contains(&expected),
        "missing {expected:?} in {rendered:?}"
    );
    Ok(())
}

#[when(expr = "I reload the panel and interrupt reading after {int} files")]
fn when_reload_interrupted(world: &mut FfmWorld, limit: usize) -> StepResult {
    let mut visited = 0usize;
    world
        .panel()?
        .load_entries_with_cancel(|_| {
            visited += 1;
            visited >= limit
        })
        .context("interruptible reload")
}

#[then(expr = "the panel lists {string} and {int} files")]
fn then_panel_lists_partial(world: &mut FfmWorld, first: String, count: usize) -> StepResult {
    let panel = world.panel()?;
    assert_eq!(
        panel.entries.first().map(|entry| entry.name.as_str()),
        Some(first.as_str())
    );
    assert_eq!(panel.entries.len(), count + 1);
    Ok(())
}
