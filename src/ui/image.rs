//! Render prepared terminal image protocols without blocking the UI thread.

use std::cell::RefCell;

use ratatui::{
    layout::{Rect, Size},
    widgets::Paragraph,
    Frame,
};
use ratatui_image::Image;

use crate::{
    image_render::{Key, Mode, Worker},
    models::DecodedImage,
};

struct Renderer {
    mode: Mode,
    worker: Option<Worker>,
    failed: bool,
}

thread_local! {
    static RENDERER: RefCell<Renderer> = const { RefCell::new(Renderer {
        mode: Mode::Halfblocks, worker: None, failed: false,
    }) };
}

/// Select a protocol explicitly; do not probe or consume terminal input.
pub fn set_mode(mode: Mode) {
    RENDERER.with_borrow_mut(|renderer| renderer.mode = mode);
}

/// Poll completed work without blocking. A completed protocol needs a redraw.
pub fn pump() -> bool {
    RENDERER.with_borrow_mut(|renderer| renderer.worker.as_mut().is_some_and(Worker::pump))
}

/// Release cached frames when no image remains visible in either panel/viewer.
pub fn clear() {
    RENDERER.with_borrow_mut(|renderer| {
        if let Some(worker) = &mut renderer.worker {
            worker.clear();
        }
    });
}

/// Render an image, or a short preparation message until its worker finishes.
pub fn render(frame: &mut Frame<'_>, area: Rect, image: &DecodedImage, zoom: u8, pan: (u32, u32)) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    RENDERER.with_borrow_mut(|renderer| {
        renderer.start_worker();
        let Some(worker) = renderer.worker.as_mut() else {
            unavailable(frame, area);
            return;
        };
        let key = Key {
            image: image.clone(),
            size: Size::new(area.width, area.height),
            zoom: zoom.max(1),
            pan,
            mode: renderer.mode,
        };
        draw_cached(frame, area, worker, key);
    });
}

fn draw_cached(frame: &mut Frame<'_>, area: Rect, worker: &mut Worker, key: Key) {
    if let Some((_, protocol)) = worker.cache.iter().find(|(cached, _)| *cached == key) {
        if let Some(protocol) = protocol {
            frame.render_widget(Image::new(protocol), area);
        } else {
            unavailable(frame, area);
        }
    } else {
        worker.request(key);
        frame.render_widget(Paragraph::new("Preparing image…"), area);
    }
}

fn unavailable(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(
        Paragraph::new("Cannot show image · Esc, then Enter to open externally"),
        area,
    );
}

impl Renderer {
    fn start_worker(&mut self) {
        if self.worker.is_none() && !self.failed {
            match Worker::start() {
                Ok(worker) => self.worker = Some(worker),
                Err(_) => self.failed = true,
            }
        }
    }
}

#[cfg(test)]
mod tests;
