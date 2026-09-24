//! Unit tests: F5/F6 must refuse a destination inside the source, must survive
//! a failing item in a marked batch, and F3 must not choke on binary files.

use crate::models::FileName;
use crate::ops::*;

use crate::models::{App, Overlay, Panel, PanelSide};

use std::fs;

use std::io;

use std::path::Path;

use tempfile::TempDir;

/// Panels: left on `left`, right on `right`, cursor on `name` in the left one.
fn app_with(left: &Path, right: &Path, name: &str) -> io::Result<App> {
    let mut app = App::new()?;
    app.left_panel = Panel::new(left.to_path_buf())?;
    app.right_panel = Panel::new(right.to_path_buf())?;
    app.active_panel = PanelSide::Left;
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .expect("test setup: entry must be listed");
    app.left_panel.select(Some(index));
    Ok(app)
}

mod transfers;
mod viewing;
