//! Decode the most recently requested image away from the event loop.
//!
//! At most one decoder runs. Moving quickly through files replaces pending
//! work rather than queueing every intervening image; an obsolete result is
//! never published into a newly opened panel or viewer.

use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};

use super::Redraw;
use crate::models::{App, DecodedImage, ImageState, Overlay, PanelKind, PanelSide, Preview};

mod visibility;
use visibility::has_ready_image;

type Decoder = dyn Fn(&Path) -> Result<DecodedImage, String> + Send + Sync;

#[derive(Clone, Copy)]
enum Target {
    Viewer,
    Panel(PanelSide),
}

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
        let Some((target, path)) = next_request(app) else {
            return redraw;
        };
        self.next_id = self.next_id.wrapping_add(1);
        let id = self.next_id;
        let (sender, receiver) = mpsc::channel();
        let worker_path = path.clone();
        let decoder = Arc::clone(&self.decoder);
        let spawned = std::thread::Builder::new()
            .name("ffm-image-preview".to_owned())
            .spawn(move || drop(sender.send(decoder(&worker_path))));
        if let Some(state) = state_for(app, target, &path) {
            if spawned.is_ok() {
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
        }
        Redraw::Needed
    }

    fn collect(&mut self, app: &mut App) -> Redraw {
        let Some(running) = &self.running else {
            return Redraw::Unchanged;
        };
        let result = match running.receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return Redraw::Unchanged,
            Err(mpsc::TryRecvError::Disconnected) => Err("Image worker stopped".to_owned()),
        };
        let Some(running) = self.running.take() else {
            return Redraw::Unchanged;
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

/// Full-screen viewer has priority, then the visible quick-view panels.
fn next_request(app: &App) -> Option<(Target, PathBuf)> {
    if let Overlay::Viewer(viewer) = &app.overlay {
        if viewer.image == Some(ImageState::Pending) {
            if let Some(file) = viewer.viewed_file() {
                return Some((Target::Viewer, file.path.clone()));
            }
        }
    }
    [PanelSide::Left, PanelSide::Right]
        .into_iter()
        .find_map(|side| {
            if !visibility::panel_visible(app, side) {
                return None;
            }
            let PanelKind::QuickView(view) = &app.panel(side).kind else {
                return None;
            };
            if view.preview != Preview::Image(ImageState::Pending) {
                return None;
            }
            view.shown.clone().map(|path| (Target::Panel(side), path))
        })
}

fn state_for<'a>(app: &'a mut App, target: Target, path: &Path) -> Option<&'a mut ImageState> {
    match target {
        Target::Viewer => {
            let Overlay::Viewer(viewer) = &mut app.overlay else {
                return None;
            };
            if viewer.viewed_file().is_none_or(|file| file.path != path) {
                return None;
            }
            viewer.image.as_mut()
        }
        Target::Panel(side) => {
            let PanelKind::QuickView(view) = &mut app.panel_mut(side).kind else {
                return None;
            };
            if view.shown.as_deref() != Some(path) {
                return None;
            }
            let Preview::Image(state) = &mut view.preview else {
                return None;
            };
            Some(state)
        }
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_cleanup;
#[cfg(test)]
mod tests_failures;
#[cfg(test)]
mod tests_visibility;
