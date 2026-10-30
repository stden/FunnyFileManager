//! Share frames across queued panel movements, with bounded work per frame.

use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

use crossterm::event::{Event, KeyEventKind, MouseEventKind};
use ratatui::{backend::Backend, Terminal};

use super::input::{BufferedInput, EventSource};
use crate::{
    action::{self, Action, Navigation},
    models::{App, Overlay, PanelKind, ViewMode},
};

const MAX_EVENTS: usize = 64;
const FRAME_BUDGET: Duration = Duration::from_millis(8);

pub(super) fn accepts(app: &App, event: &Event) -> bool {
    if !matches!(app.overlay, Overlay::None)
        || app.view != ViewMode::Panels
        || !matches!(app.get_active_panel_ref().kind, PanelKind::Files)
    {
        return false;
    }
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => matches!(
            action::resolve(&app.keymap.panels, *key),
            Action::Navigate(
                Navigation::NavUp
                    | Navigation::NavDown
                    | Navigation::PageUp
                    | Navigation::PageDown
                    | Navigation::First
                    | Navigation::Last
            )
        ),
        Event::Mouse(mouse) => matches!(
            mouse.kind,
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
        ),
        _ => false,
    }
}

pub(super) fn drain<B: Backend + Write, S: EventSource>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    input: &mut BufferedInput<S>,
    clicks: &mut crate::mouse::PanelClicks,
    started: Instant,
) -> io::Result<()> {
    for _ in 1..MAX_EVENTS {
        if started.elapsed() >= FRAME_BUDGET {
            break;
        }
        let Some(event) = input.next_navigation(app) else {
            break;
        };
        super::event::handle_event(terminal, app, event, clicks)?;
    }
    Ok(())
}
