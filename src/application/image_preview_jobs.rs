//! Decode the most recently requested image away from the event loop.
//!
//! At most one decoder runs. Moving quickly through files replaces pending
//! work rather than queueing every intervening image; an obsolete result is
//! never published into a newly opened panel or viewer.

use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};

use super::Redraw;
use crate::models::{App, DecodedImage, ImageState};
#[cfg(test)]
use crate::models::{Overlay, PanelKind, Preview};

mod visibility;
use visibility::{has_ready_image, next_request, state_for, Target};

type Decoder = dyn Fn(&Path) -> Result<DecodedImage, String> + Send + Sync;

struct Running {
    id: u64,
    target: Target,
    path: PathBuf,
    receiver: mpsc::Receiver<Result<DecodedImage, String>>,
}

pub(super) struct ImagePreviewJobs {
    decoder: Arc<Decoder>,
    running: Option<Running>,
    next_id: u64,
    had_ready_image: bool,
}

impl Default for ImagePreviewJobs {
    fn default() -> Self {
        Self {
            decoder: Arc::new(crate::images::decode),
            running: None,
            next_id: 0,
            had_ready_image: false,
        }
    }
}

impl ImagePreviewJobs {
    #[cfg(test)]
    fn with_decoder(
        decoder: impl Fn(&Path) -> Result<DecodedImage, String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            decoder: Arc::new(decoder),
            ..Self::default()
        }
    }

    pub(super) fn refresh(&mut self, app: &mut App) -> Redraw {
        crate::ops::quickview::refresh(app);
        let redraw = self.pump(app);
        let has_image = has_ready_image(app);
        if self.had_ready_image && !has_image {
            crate::ui::image::clear();
        }
        self.had_ready_image = has_image;
        if crate::ui::image::pump() {
            Redraw::Needed
        } else {
            redraw
        }
    }

    pub(super) fn pump(&mut self, app: &mut App) -> Redraw {
        let redraw = self.collect(app);
        if self.running.is_some() {
            return redraw;
        }
        let Some((target, path, state)) = next_request(app) else {
            return redraw;
        };
        self.next_id = self.next_id.wrapping_add(1);
        let id = self.next_id;
        let (sender, receiver) = mpsc::channel();
        if spawn_decoder(path.clone(), Arc::clone(&self.decoder), sender).is_ok() {
            *state = ImageState::Loading(id);
            self.running = Some(Running {
                id,
                target,
                path,
                receiver,
            });
        } else {
            *state = ImageState::Failed("Cannot start image worker".to_owned());
        }
        Redraw::Needed
    }

    fn collect(&mut self, app: &mut App) -> Redraw {
        let Some(running) = self.running.take() else {
            return Redraw::Unchanged;
        };
        let result = match running.receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => {
                self.running = Some(running);
                return Redraw::Unchanged;
            }
            Err(mpsc::TryRecvError::Disconnected) => Err("Image worker stopped".to_owned()),
        };
        let Some(state) = state_for(app, running.target, &running.path) else {
            return Redraw::Unchanged;
        };
        if *state != ImageState::Loading(running.id) {
            return Redraw::Unchanged;
        }
        *state = result.map_or_else(ImageState::Failed, ImageState::Ready);
        Redraw::Needed
    }
}

#[cfg(test)]
thread_local! {
    pub(super) static FORCE_SPAWN_FAILURE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn spawn_decoder(
    path: PathBuf,
    decoder: Arc<Decoder>,
    sender: mpsc::Sender<Result<DecodedImage, String>>,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    #[cfg(test)]
    if FORCE_SPAWN_FAILURE.get() {
        return Err(std::io::Error::other("forced spawn failure"));
    }
    std::thread::Builder::new()
        .name("ffm-image-preview".to_owned())
        .spawn(move || drop(sender.send(decoder(&path))))
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_cleanup;
#[cfg(test)]
mod tests_failures;
#[cfg(test)]
mod tests_visibility;
