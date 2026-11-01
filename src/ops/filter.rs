//! Purpose: Open and apply the file panel filter (`Ctrl+I`).

use crate::lineedit::LineEditor;
use crate::mask::Mask;
use crate::models::{App, FilterInput, FilterPrompt, Overlay};
use crate::status::Status;

/// Ctrl+I: open the File panel filter dialog.
pub fn open(app: &mut App) {
    let panel = app.get_active_panel_ref();
    let filter = &panel.filter;
    let edit_panel_include = panel.options.filter.is_empty() && filter.include.is_some();
    let dialog = FilterPrompt {
        include: if edit_panel_include {
            filter.include.as_ref().map_or("", Mask::source).to_owned()
        } else {
            panel.options.filter.clone()
        },
        exclude: filter.exclude.as_ref().map_or("", Mask::source).to_owned(),
        hide_dotfiles: filter.hide_dotfiles,
    };
    app.filter_input = FilterInput {
        include: LineEditor::new(&dialog.include),
        additional: (!panel.options.filter.is_empty())
            .then(|| {
                filter
                    .include
                    .as_ref()
                    .map(|mask| LineEditor::new(mask.source()))
            })
            .flatten(),
        exclude: LineEditor::new(&dialog.exclude),
        edit_panel_include,
        ..FilterInput::default()
    };
    app.set_overlay(Overlay::Filter(dialog));
}

/// Apply the filter only when a complete replacement listing was read.
pub(crate) fn apply(app: &mut App) {
    let Overlay::Filter(dialog) = &app.overlay else {
        return;
    };
    let parse = |line: &str| (!line.trim().is_empty()).then(|| Mask::parse(line));
    let include = app.filter_input.include.text();
    let exclude = app.filter_input.exclude.text();
    let mut updated = app.get_active_panel_ref().clone();
    if app.filter_input.edit_panel_include {
        updated.filter.include = parse(include);
    } else {
        include.clone_into(&mut updated.options.filter);
        if let Some(additional) = &app.filter_input.additional {
            updated.filter.include = parse(additional.text());
        }
    }
    updated.filter.exclude = parse(exclude);
    updated.filter.hide_dotfiles = dialog.hide_dotfiles;
    if let Err(error) = updated.load_entries() {
        let message = format!("Cannot refresh folder: {error}");
        app.filter_input.error = Some(message.clone());
        app.set_status(Status::Error(message));
        return;
    }
    *app.get_active_panel() = updated;
    app.close_overlay();
    app.set_status(Status::CommandOutput("Filter applied".to_owned()));
}
