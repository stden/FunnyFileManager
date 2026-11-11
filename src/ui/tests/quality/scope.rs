//! Check active modal text on its real colours, excluding inactive shadowed underlay.

use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, Terminal};

use crate::models::{App, Overlay};

pub(super) fn active_text(buffer: &Buffer, app: &mut App) -> Buffer {
    if matches!(app.overlay, Overlay::None | Overlay::CommandLine(_)) {
        return buffer.clone();
    }
    let mut terminal =
        Terminal::new(TestBackend::new(buffer.area.width, buffer.area.height)).unwrap();
    terminal
        .draw(|frame| {
            if matches!(app.overlay, Overlay::Menu(_)) {
                let bar = Rect::new(0, 0, frame.area().width, 1);
                crate::ui::menu::render_bar(frame, app, bar);
                crate::ui::menu::render_drop_down(frame, app, bar);
            } else {
                crate::ui::render_overlays(frame, app);
            }
        })
        .unwrap();
    let mut active = buffer.clone();
    for (cell, mask) in active
        .content
        .iter_mut()
        .zip(&terminal.backend().buffer().content)
    {
        if mask.symbol().trim().is_empty() {
            cell.set_symbol(" ");
        }
    }
    active
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::super::{contrast, fixtures, render};
    use super::*;

    fn help() -> (fixtures::Case, Buffer) {
        let mut case = fixtures::cases()
            .into_iter()
            .find(|case| case.name == "help")
            .unwrap();
        let buffer = render(&mut case, "blue", "on", 80, 24);
        (case, buffer)
    }

    #[test]
    fn an_unreadable_active_label_is_never_exempted() {
        let (mut case, mut buffer) = help();
        let active = active_text(&buffer, &mut case.app);
        let index = active
            .content
            .iter()
            .position(|cell| cell.symbol() == "H")
            .unwrap();
        let position = buffer.pos_of(index);
        buffer[position].set_fg(Color::Black).set_bg(Color::Black);
        assert!(
            !contrast::check(&active_text(&buffer, &mut case.app)).is_empty(),
            "unreadable active text must fail"
        );
    }

    #[test]
    fn an_active_caption_cannot_escape_by_using_disabled_colours() {
        let (mut case, mut buffer) = help();
        let active = active_text(&buffer, &mut case.app);
        let index = active
            .content
            .iter()
            .position(|cell| cell.symbol() == "H")
            .unwrap();
        let position = buffer.pos_of(index);
        buffer[position]
            .set_fg(crate::ui::palette::DISABLED_FG)
            .set_bg(crate::ui::palette::FIELD_BG);
        assert!(
            !contrast::check(&active_text(&buffer, &mut case.app)).is_empty(),
            "disabled colours cannot exempt active text"
        );
    }

    #[test]
    fn inactive_underlay_is_exempt_but_the_same_panel_text_is_checked() {
        let (mut case, mut buffer) = help();
        buffer[(0, 0)]
            .set_symbol("x")
            .set_fg(Color::Black)
            .set_bg(Color::Black);
        assert!(
            !contrast::check(&buffer).is_empty(),
            "unreadable underlay is visible before modal scoping"
        );
        let failures = contrast::check(&active_text(&buffer, &mut case.app));
        assert!(failures.is_empty(), "{}", failures.join("\n"));
        case.app.set_overlay(Overlay::None);
        assert!(
            !contrast::check(&active_text(&buffer, &mut case.app)).is_empty(),
            "the same text must fail when the panel becomes active"
        );
    }
}
