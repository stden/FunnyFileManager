//! Unit tests: `Ctrl+F1` / `Ctrl+F2` take a panel off the screen and give
//! its half to the other, the cursor never lands on a hidden panel, and the
//! last panel standing cannot be hidden.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::Terminal;

use super::{select, swap_panels, switch_panel, toggle};
use crate::application::dispatch_key;
use crate::menu::{Entry, Heading, Menu};
use crate::models::{App, Overlay, PanelSide, PanelsShown};
use crate::mouse::{hit, Target};
use crate::test_support::{buffer_text, key_event, key_event_with_mods, WritableTestBackend};
use crate::ui::{checked_entry, ui, FKEY_COUNT};

fn app() -> App {
    App::new().expect("test app")
}

fn panel_on_ticked(app: &App, heading: Heading) -> bool {
    let entry = Menu::open(heading)
        .entries()
        .iter()
        .find(|entry| matches!(entry, Entry::Item { label, .. } if *label == "Panel &On/Off"))
        .expect("the Panel On/Off row");
    matches!(
        checked_entry(app, *entry),
        Entry::Item { checked: true, .. }
    )
}

/// Two distinctly named directories, one per panel, so a title on screen
/// says which panel is drawn.
fn two_directories(root: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let left_dir = root.join("left_side_dir");
    let right_dir = root.join("right_side_dir");
    std::fs::create_dir_all(&left_dir).expect("left dir");
    std::fs::create_dir_all(&right_dir).expect("right dir");
    (left_dir, right_dir)
}

fn draw(app: &mut App) -> String {
    let mut terminal = terminal();
    terminal.draw(|f| ui(f, app)).expect("draw");
    buffer_text(&terminal)
}

fn terminal() -> Terminal<WritableTestBackend> {
    Terminal::new(WritableTestBackend::new(80, 24)).expect("test terminal")
}

mod drawing;
mod hidden_panel_keys;
mod toggling;
