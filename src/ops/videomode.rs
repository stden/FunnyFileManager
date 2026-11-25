//! Purpose: Video mode (`Alt+F9`) — ask the terminal for 25 or 50 rows and
//! report the height it actually got.
//!
//! Many terminals, tmux among them, ignore the request, and a key that does
//! something invisible cannot be told from one that was ignored.

use std::io;

use crate::{models::App, status::Status, terminal::VideoMode};

/// Alt+F9 and the Commands menu row: ask for the other height and say what
/// the terminal did with the request.
pub fn toggle(app: &mut App) -> io::Result<()> {
    let outcome = if app.live.terminal_mode == crate::models::TerminalMode::Headless {
        headless_mode()?
    } else {
        crate::terminal::toggle_video_mode()?
    };
    app.set_status(Status::CommandOutput(describe(outcome)));
    Ok(())
}

fn headless_mode() -> io::Result<VideoMode> {
    let size = crate::terminal::headless::viewport().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotConnected,
            "No headless terminal is installed",
        )
    })?;
    let rows = if size.height > 30 { 25 } else { 50 };
    crate::terminal::headless::resize(size.width, rows)?;
    Ok(VideoMode {
        requested_rows: rows,
        rows,
    })
}

/// The status line: the new height, or the height the terminal has instead.
/// Short enough for one panel's info row.
fn describe(outcome: VideoMode) -> String {
    let VideoMode {
        requested_rows,
        rows,
    } = outcome;
    if rows == requested_rows {
        format!("Video mode: {rows} rows")
    } else {
        format!("Video mode: asked for {requested_rows} rows, the terminal has {rows}")
    }
}

#[cfg(test)]
mod tests;
