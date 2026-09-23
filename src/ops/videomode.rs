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
    let outcome = crate::terminal::toggle_video_mode()?;
    app.set_status(Status::CommandOutput(describe(outcome)));
    Ok(())
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
