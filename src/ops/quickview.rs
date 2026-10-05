//! Purpose: The Quick view panel (`Ctrl+Q`) — the passive panel shows the file
//! under the active cursor, and follows it.
//!
//! Far's `Ctrl+Q` acts on the panel *without* the cursor: the reader keeps
//! browsing where they were and the other half of the screen answers "what is
//! this?" for each row they land on. Pressed from inside the preview, it
//! turns that panel back into a file list. The event loop calls [`refresh`]
//! once per event, before drawing, so a cursor moved by any route — a key, a
//! click, a quick search — is previewed before the screen shows it.

use std::path::{Path, PathBuf};

use crate::fileload::{read_for_text_ui, Loaded};
use crate::menu::SettingsPage;
use crate::models::settings::Settings;
use crate::models::{App, FileSize, Panel, PanelKind, PanelSide, Preview, Refusal, TextPreview};
use crate::status::Status;

/// `Ctrl+Q`: the preview on the passive panel, or the file list back.
pub fn toggle(app: &mut App) {
    let active = app.active_panel;
    let side = if app.get_active_panel_ref().kind.is_quick_view() {
        active
    } else {
        opposite(active)
    };
    let panel = app.panel_mut(side);
    let (kind, report) = if panel.kind.is_quick_view() {
        (PanelKind::Files, "Quick view off")
    } else {
        (PanelKind::QuickView(Box::default()), "Quick view on")
    };
    panel.kind = kind;
    app.set_status(Status::CommandOutput(report.to_owned()));
    refresh(app);
}

/// Bring every quick-view panel up to date with the cursor opposite it.
///
/// Cheap when nothing moved: a path comparison per panel, no disk. The file
/// is re-read only when the cursor is on a different row than last time, so a
/// file edited in place keeps its old preview until the cursor leaves and
/// returns — `Ctrl+Q` twice does the same.
pub fn refresh(app: &mut App) {
    for side in [PanelSide::Left, PanelSide::Right] {
        follow(app, side);
    }
}

/// Apply changed description settings to previews already on screen.
pub(crate) fn reload(app: &mut App) {
    for side in [PanelSide::Left, PanelSide::Right] {
        if let PanelKind::QuickView(view) = &mut app.panel_mut(side).kind {
            view.show(None, Preview::Nothing);
        }
    }
    refresh(app);
}

fn follow(app: &mut App, side: PanelSide) {
    let target = target_of(app.panel(opposite(side)));
    let PanelKind::QuickView(view) = &app.panel(side).kind else {
        return;
    };
    if view.shown == target {
        return;
    }
    let preview = target
        .as_deref()
        .map_or(Preview::Nothing, |path| load(path, &app.settings));
    if let PanelKind::QuickView(view) = &mut app.panel_mut(side).kind {
        view.show(target, preview);
    }
}

/// What the panel opposite a preview is pointing at: the selected entry of a
/// file list (`..` is nothing), the folder under a tree's cursor, and nothing
/// when that panel is itself a preview.
fn target_of(panel: &Panel) -> Option<PathBuf> {
    match &panel.kind {
        PanelKind::Files => panel.selected_path(),
        PanelKind::Tree(tree) => tree.selected().map(Path::to_path_buf),
        PanelKind::QuickView(_) | PanelKind::Sessions(_) | PanelKind::Remote(_) => None,
    }
}

/// Prefer a folder's configured description, falling back to its totals.
fn load(path: &Path, settings: &Settings) -> Preview {
    if path.is_dir() {
        return folder_description(path, settings).map_or_else(
            || Preview::Folder(Box::new(crate::stats::analyse(path))),
            Preview::Text,
        );
    }
    load_file(path)
}

fn folder_description(path: &Path, settings: &Settings) -> Option<TextPreview> {
    if !settings.enabled(SettingsPage::Information) {
        return None;
    }
    settings
        .get(SettingsPage::FolderDescriptions)
        .split(';')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .find_map(|name| match load_file(&path.join(name)) {
            Preview::Text(text) => Some(text),
            _ => None,
        })
}

/// Read a regular text file for the preview, declining what F3 declines.
fn load_file(path: &Path) -> Preview {
    match read_for_text_ui(path) {
        Err(error) => Preview::Refused(Refusal::Unreadable(error.to_string())),
        Ok(Loaded::NotRegular) => Preview::Refused(Refusal::NotRegular),
        Ok(Loaded::TooLarge(size)) => {
            Preview::Refused(Refusal::TooLarge(FileSize::from_bytes(size)))
        }
        // The test F3 uses: the preview can only render garbage for a binary.
        Ok(Loaded::Bytes(bytes)) if crate::encoding::is_binary(&bytes) => {
            Preview::Refused(Refusal::Binary)
        }
        Ok(Loaded::Bytes(bytes)) => Preview::Text(TextPreview::decoded(path, &bytes)),
    }
}

const fn opposite(side: PanelSide) -> PanelSide {
    match side {
        PanelSide::Left => PanelSide::Right,
        PanelSide::Right => PanelSide::Left,
    }
}

#[cfg(test)]
mod tests;
