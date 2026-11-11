//! Match image work to the panels and overlays the UI actually draws.

use std::path::{Path, PathBuf};

use crate::models::{App, ImageState, Overlay, PanelKind, PanelSide, Preview, ViewMode};

#[derive(Clone, Copy)]
pub(super) enum Target {
    Viewer,
    Panel(PanelSide),
}

pub(super) fn panel_visible(app: &App, side: PanelSide) -> bool {
    app.view == ViewMode::Panels
        && !matches!(&app.overlay, Overlay::Viewer(_) | Overlay::Editor(_))
        && app.panels_on_screen().shows(side)
}

pub(super) fn has_ready_image(app: &App) -> bool {
    matches!(&app.overlay, Overlay::Viewer(viewer) if matches!(viewer.image, Some(ImageState::Ready(_))))
        || [PanelSide::Left, PanelSide::Right].into_iter().any(|side| {
            panel_visible(app, side)
                && matches!(&app.panel(side).kind, PanelKind::QuickView(view) if matches!(view.preview, Preview::Image(ImageState::Ready(_))))
        })
}

/// Full-screen viewer has priority, then the visible quick-view panels.
pub(super) fn next_request(app: &mut App) -> Option<(Target, PathBuf, &mut ImageState)> {
    let left_visible = panel_visible(app, PanelSide::Left);
    let right_visible = panel_visible(app, PanelSide::Right);
    if let Some(request) = viewer_request(&mut app.overlay) {
        return Some(request);
    }
    if left_visible {
        if let Some(request) = panel_request(&mut app.left_panel, PanelSide::Left) {
            return Some(request);
        }
    }
    if right_visible {
        return panel_request(&mut app.right_panel, PanelSide::Right);
    }
    None
}

fn viewer_request(overlay: &mut Overlay) -> Option<(Target, PathBuf, &mut ImageState)> {
    let Overlay::Viewer(viewer) = overlay else {
        return None;
    };
    let path = viewer.viewed_file()?.path.clone();
    let state = viewer
        .image
        .as_mut()
        .filter(|state| **state == ImageState::Pending)?;
    Some((Target::Viewer, path, state))
}

fn panel_request(
    panel: &mut crate::models::Panel,
    side: PanelSide,
) -> Option<(Target, PathBuf, &mut ImageState)> {
    let PanelKind::QuickView(view) = &mut panel.kind else {
        return None;
    };
    let path = view.shown.clone()?;
    let Preview::Image(state) = &mut view.preview else {
        return None;
    };
    (*state == ImageState::Pending).then_some((Target::Panel(side), path, state))
}

pub(super) fn state_for<'a>(
    app: &'a mut App,
    target: Target,
    path: &Path,
) -> Option<&'a mut ImageState> {
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
