//! Image preview states are visible without making the renderer read a file.

use ratatui::Terminal;

use super::render;
use crate::models::{App, ImageState, PanelKind, Preview, QuickView};
use crate::test_support::{buffer_text, WritableTestBackend};

fn screen(state: ImageState) -> String {
    let mut view = QuickView::default();
    view.show(Some("picture.png".into()), Preview::Image(state));
    let mut app = App::new().expect("app");
    app.right_panel.kind = PanelKind::QuickView(Box::new(view));
    let mut terminal = Terminal::new(WritableTestBackend::new(60, 12)).expect("terminal");
    terminal
        .draw(|f| render(f, &app.right_panel, f.area(), false))
        .expect("draw");
    buffer_text(&terminal)
}

#[test]
fn waiting_for_a_worker_has_an_explanation() {
    assert!(screen(ImageState::Pending).contains("Loading image"));
    assert!(screen(ImageState::Loading(3)).contains("Loading image"));
}

#[test]
fn a_failed_image_shows_why_the_preview_is_unavailable() {
    assert!(screen(ImageState::Failed("bad image".into())).contains("bad image"));
}
