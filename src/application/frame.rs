//! Present each diff as a synchronized terminal update to avoid partial frames.

use std::io::{self, Write};

use crossterm::{
    execute, queue,
    terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate},
};
use ratatui::{backend::Backend, Terminal};

use crate::{
    models::{App, Viewport},
    ui::ui,
};

pub(super) fn draw_frame<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    queue!(terminal.backend_mut(), BeginSynchronizedUpdate)?;
    let drawn = terminal
        .draw(|f| {
            app.set_viewport(Viewport::new(f.area().height));
            ui(f, app);
        })
        .map(|_| ())
        .map_err(|error| io::Error::other(error.to_string()));
    // Always release presentation, including when the backend cannot draw.
    // Preserve that original error if ending the update also fails.
    let ended = execute!(terminal.backend_mut(), EndSynchronizedUpdate);
    drawn.and(ended)
}
