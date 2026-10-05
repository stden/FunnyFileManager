//! Purpose: The terminal's height, as every consumer that pages by the screen
//! reads it.
//!
//! The panels' PgUp/PgDn, the console and the file viewer all move by "one
//! screen", and none of their key handlers has a terminal to ask. Each used to
//! find out its own way — the panels and the console asked `terminal.size()`
//! at dispatch time, the viewer had the renderer write the body height back
//! into the model. One answer now: `lib::dispatch_key` asks the terminal once
//! before the key and stores the result on `App`; a resize event refreshes it
//! before the redraw.
//!
//! A child of `models` rather than a corner of `ui`, so that the model can
//! read it without depending on the presentation layer.

/// Screen height assumed when the backend cannot report its size (a detached or
/// non-tty terminal). The classic 24-line screen keeps paging usable instead of
/// degrading to a one-row step.
pub const FALLBACK_TERMINAL_HEIGHT: u16 = 24;

/// Rows the terminal had when the current key was dispatched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport(u16);

impl Viewport {
    /// A terminal `rows` high.
    #[must_use]
    pub const fn new(rows: u16) -> Self {
        Self(rows)
    }

    /// The terminal's height, in rows.
    #[must_use]
    pub const fn rows(self) -> u16 {
        self.0
    }
}

impl Default for Viewport {
    /// The height a backend that cannot say gets — see
    /// [`FALLBACK_TERMINAL_HEIGHT`].
    fn default() -> Self {
        Self(FALLBACK_TERMINAL_HEIGHT)
    }
}

#[cfg(test)]
mod tests;
