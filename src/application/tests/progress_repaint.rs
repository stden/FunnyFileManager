//! Unit tests: a progress dialog painted over the last frame is wiped by a
//! full repaint once the operation is over, and nothing else is.

use std::time::Duration;

use super::*;
use crate::application::loop_state::{draw_frame, repaint_after_progress};
use crate::models::Panel;
use crate::progress::{Painter, Snapshot};

struct Nowhere;

impl Painter for Nowhere {
    fn paint(&mut self, _snapshot: &Snapshot) {}
}

fn drawn() -> (tempfile::TempDir, App, Terminal<WritableTestBackend>) {
    let directory = tempfile::TempDir::new().unwrap();
    let mut app = App::with_panels(
        Panel::new(directory.path().to_path_buf()).unwrap(),
        Panel::new(directory.path().to_path_buf()).unwrap(),
    );
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).unwrap();
    draw_frame(&mut terminal, &mut app).unwrap();
    (directory, app, terminal)
}

#[test]
fn a_painted_dialog_clears_the_screen_for_a_full_frame() {
    let (_directory, _app, mut terminal) = drawn();
    let _installed = crate::progress::install_with(Box::new(Nowhere), Duration::ZERO);
    let run = crate::progress::begin(crate::progress::Operation::Delete, &[]);
    crate::progress::entering(std::path::Path::new("x"));
    drop(run);

    repaint_after_progress(&mut terminal).unwrap();

    assert!(
        buffer_text(&terminal).trim().is_empty(),
        "the screen was cleared"
    );
}

#[test]
fn without_a_dialog_the_frame_stays() {
    let (_directory, _app, mut terminal) = drawn();
    repaint_after_progress(&mut terminal).unwrap();
    assert!(!buffer_text(&terminal).trim().is_empty());
}
