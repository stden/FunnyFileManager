//! Unit tests: `Ctrl+Q` turns the passive panel into a preview of the active
//! cursor, and the preview follows that cursor.

use std::fs;

use tempfile::TempDir;

use super::{refresh, toggle};
use crate::models::{App, Panel, PanelKind, PanelSide, Preview, Refusal};

mod descriptions;

/// `alpha.txt`, `beta.txt`, a `nested` folder and a binary, in one directory.
fn fixture() -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("alpha.txt"), "alpha one\nalpha two\n").expect("write");
    fs::write(dir.path().join("beta.rs"), "fn beta() {}\n").expect("write");
    fs::write(dir.path().join("blob.bin"), b"\x00\x01\x02").expect("write");
    let nested = dir.path().join("nested");
    fs::create_dir(&nested).expect("mkdir");
    fs::write(nested.join("inner.txt"), "12345").expect("write");
    fs::write(nested.join("other.txt"), "1234567").expect("write");
    dir
}

fn app_in(dir: &TempDir) -> App {
    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    App::with_panels(panel.clone(), panel)
}

fn select(app: &mut App, name: &str) {
    let panel = app.get_active_panel();
    let index = panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("no entry {name}"));
    panel.select(Some(index));
}

fn preview(app: &App, side: PanelSide) -> &Preview {
    match &app.panel(side).kind {
        PanelKind::QuickView(view) => &view.preview,
        other => panic!("not a quick view: {other:?}"),
    }
}

mod content;
mod toggling;
