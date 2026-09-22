//! Purpose: Video mode (`Alt+F9`) — Far switches between 25 and 50 rows;
//! here the terminal is asked to resize, and the status bar reports what it
//! actually got.
//!
//! Takes the terminal because the resize is a terminal fact, not an `App`
//! one. The request arrives with the feature.

use std::io::Write;

use ratatui::{backend::Backend, Terminal};

use crate::models::App;

pub fn toggle<B: Backend + Write>(_terminal: &mut Terminal<B>, app: &mut App) {
    super::stub::report_coming(app, "Video mode");
}
