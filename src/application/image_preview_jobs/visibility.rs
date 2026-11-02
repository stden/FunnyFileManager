//! Match image work to the panels and overlays the UI actually draws.

use crate::models::{App, ImageState, Overlay, PanelKind, PanelSide, Preview, ViewMode};

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
