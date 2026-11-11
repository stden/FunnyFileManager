//! Unit tests for [`super`], moved out of the module so their
//! assertion-failure arms are not measured as unreached production lines.

use super::*;
use crate::models::Panel;
use std::fs::{self, File};
use tempfile::TempDir;

pub(super) fn make_app(temp: &TempDir) -> App {
    let mut app = App::new().expect("app");
    app.left_panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    app.right_panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    app
}

mod cd_quoting;
mod console_transcript;
mod panel_commands;
mod session_commands;
