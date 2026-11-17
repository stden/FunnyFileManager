//! Image content and its adaptive control guide in the F3 viewer.

use ratatui::{layout::Rect, style::Style, widgets::Paragraph, Frame};

use crate::models::{ImageState, Viewer};
use crate::ui::palette;

pub(super) fn render(frame: &mut Frame<'_>, viewer: &Viewer, state: &ImageState, body: Rect) {
    if body.height < 2 {
        return render_state(frame, viewer, state, body);
    }
    let content = Rect {
        height: body.height - 1,
        ..body
    };
    render_state(frame, viewer, state, content);
    let footer = Rect {
        y: body.y + body.height - 1,
        height: 1,
        ..body
    };
    let style = Style::default()
        .bg(palette::STATUS_BAR_BG)
        .fg(palette::STATUS_BAR_FG);
    frame.render_widget(Paragraph::new(guide(body.width)).style(style), footer);
}

fn guide(width: u16) -> &'static str {
    if width >= 45 {
        "+/- Zoom · 0 Fit · Arrows Move · Esc Close"
    } else if width >= 30 {
        "+/- Zoom · 0 Fit · Arrows · Esc"
    } else if width >= 19 {
        "+/- Zoom 0 Fit Esc"
    } else {
        "+/- 0 Esc"
    }
}

fn render_state(frame: &mut Frame<'_>, viewer: &Viewer, state: &ImageState, body: Rect) {
    match state {
        ImageState::Ready(image) => {
            crate::ui::image::render(frame, body, image, viewer.image_zoom, viewer.image_pan);
        }
        ImageState::Pending | ImageState::Loading(_) => {
            frame.render_widget(Paragraph::new("Loading image…"), body);
        }
        ImageState::Failed(reason) => {
            let safe = crate::ui::printable_name(reason);
            frame.render_widget(
                Paragraph::new(format!(
                    "Cannot show image: {safe}\nEsc, then Enter to open externally"
                )),
                body,
            );
        }
    }
}
