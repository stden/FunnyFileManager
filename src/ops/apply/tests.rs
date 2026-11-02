//! Unit tests: the batch's summary line, and the two ways a run can fail
//! before the shell even gets the command — a `!@` list that cannot be
//! written, and a shell that will not start.

use std::fs;

use tempfile::TempDir;

use super::{context, run, run_with_list_dir, summary};
use crate::console::ConsoleLine;
use crate::models::{App, Panel};
use crate::status::Status;

fn app_at(dir: &TempDir) -> App {
    fs::write(dir.path().join("a.txt"), "alpha\n").expect("a.txt");
    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(dir.path().to_path_buf()).expect("left");
    assert!(app.get_active_panel().select_named("a.txt"));
    app
}

fn status_text(app: &App) -> String {
    match &app.status {
        Status::CommandOutput(text) => text.clone(),
        other => panic!("a command summary, got {other:?}"),
    }
}

mod list_files;
mod running;
mod short_names;
