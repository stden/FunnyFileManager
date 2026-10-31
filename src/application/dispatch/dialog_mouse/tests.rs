//! Unit tests for [`super`], moved out of the module so their
//! assertion-failure arms are not measured as unreached production lines.

use std::path::Path;

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use tempfile::TempDir;
use unicode_width::UnicodeWidthStr;

use super::*;
use crate::copy::{CopyField, CopyPrompt, Purpose};
use crate::menu::{Heading, Menu};
use crate::models::{
    Confirm, EditState, Editor, ExitChoice, NamePrompt, Overlay, Panel, PanelSide,
};
use crate::search::{SearchField, SearchPrompt};
use crate::WritableTestBackend;

mod apply;
mod buttons;
mod editor_body;
mod editor_dialogs;
mod lsp_editor;
mod menu_bar;
mod menu_dialog;
mod pointer;
mod pointer_clicks;
mod remote;
mod rows;
mod tasks;
mod tool_buttons;
mod wheel;
mod wsl;

fn app_at(dir: &TempDir) -> App {
    let mut app = App::new().expect("app");
    app.left_panel = Panel::new(dir.path().to_path_buf()).expect("left");
    app.right_panel = Panel::new(dir.path().to_path_buf()).expect("right");
    app
}

fn code(app: &mut App, index: usize) -> KeyCode {
    dialog_button_key(app, index)
        .expect("button maps to a key")
        .code
}

fn terminal() -> Terminal<WritableTestBackend> {
    Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal")
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    mouse_with(kind, column, row, KeyModifiers::empty())
}

/// A mouse event with the keys held down while it happens.
fn mouse_with(kind: MouseEventKind, column: u16, row: u16, modifiers: KeyModifiers) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers,
    }
}

/// Draw the app once, so the dialog records where its parts landed — the
/// tracker is what every click is answered from.
fn draw(app: &mut App, terminal: &mut Terminal<WritableTestBackend>) {
    terminal.draw(|f| crate::ui::ui(f, app)).expect("draw");
}

fn gesture(
    term: &mut Terminal<WritableTestBackend>,
    app: &mut App,
    kind: MouseEventKind,
    position: (u16, u16),
) {
    dispatch_dialog_mouse(term, app, mouse(kind, position.0, position.1));
}

/// Press the left button on the first place `label` is drawn.
fn click_text(term: &mut Terminal<WritableTestBackend>, app: &mut App, label: &str) {
    draw(app, term);
    let buffer = term.backend().buffer();
    let area = buffer.area;
    let position = (0..area.height)
        .find_map(|row| {
            let text: String = (0..area.width)
                .map(|column| buffer[(column, row)].symbol())
                .collect();
            text.split_once(label)
                .map(|(before, _)| (u16::try_from(before.width()).unwrap(), row))
        })
        .expect("rendered target");
    gesture(term, app, MouseEventKind::Down(MouseButton::Left), position);
}
