//! Measure input-to-frame latency; never include the blocking input reader.

use crate::{
    models::{App, Overlay},
    performance::Performance,
};
use crossterm::event::{Event, KeyCode, KeyEvent};
use std::time::Instant;

pub(super) struct Pending {
    action: String,
    started: Instant,
}

impl Pending {
    pub(super) fn start(app: &App, event: &Event, started: Instant) -> Self {
        Self {
            action: label(app, event),
            started,
        }
    }

    pub(super) fn finish(self, stats: &mut Performance, rendering: Instant, finished: Instant) {
        stats.record(
            &self.action,
            rendering.saturating_duration_since(self.started),
            finished.saturating_duration_since(rendering),
        );
    }
}

fn label(app: &App, event: &Event) -> String {
    match event {
        Event::Key(key) => key_label(app, *key),
        Event::Mouse(mouse) => format!("{}/Mouse({:?})", context(&app.overlay), mouse.kind),
        Event::Paste(_) => format!("{}/Paste", context(&app.overlay)),
        Event::Resize(_, _) => "Resize".to_owned(),
        Event::FocusGained => "FocusGained".to_owned(),
        Event::FocusLost => "FocusLost".to_owned(),
    }
}

fn key_label(app: &App, key: KeyEvent) -> String {
    if key.code == KeyCode::Enter {
        if let Some(label) = submitted_label(&app.overlay) {
            return label;
        }
    }
    if matches!(app.overlay, Overlay::None) {
        let action = crate::action::resolve(&app.keymap.panels, key);
        return crate::diagnostics::action_label(action);
    }
    if matches!(app.overlay, Overlay::Editor(_)) {
        let command = crate::action::editor::resolve(&app.keymap.editor, key);
        return match command {
            crate::action::editor::EditorCommand::Insert(_) => "Editor/Insert".to_owned(),
            other => format!("Editor/{other:?}"),
        };
    }
    format!(
        "{}/{:?}/{}",
        context(&app.overlay),
        key.modifiers,
        crate::diagnostics::key_label(key.code)
    )
}

fn submitted_label(overlay: &Overlay) -> Option<String> {
    use crate::models::{ChoiceTarget, MenuDialog};
    match overlay {
        Overlay::CommandLine(line) => Some(command_label(line.line.text())),
        Overlay::Menu(menu) => menu
            .entry()
            .and_then(crate::menu::Entry::action)
            .and_then(menu_row_label),
        Overlay::MenuDialog(dialog) => {
            let MenuDialog::Choices { rows, selected, .. } = dialog.as_ref() else {
                return None;
            };
            match &rows.get(*selected)?.target {
                ChoiceTarget::UpdatePackages(plan) => {
                    Some(format!("PackageUpdate/{}", plan.manager.label()))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// The name of a menu row's operation: the command it holds, whichever
/// family names it, so the stats screen keeps one line per menu function.
/// A row that holds a bare panel action is labelled as its key would be.
fn menu_row_label(action: crate::action::Action) -> Option<String> {
    use crate::action::Action;
    let command = match action {
        Action::Menu(command) => format!("{command:?}"),
        Action::Panel(command) => format!("{command:?}"),
        Action::File(command) => format!("{command:?}"),
        Action::Tool(command) => format!("{command:?}"),
        _ => return None,
    };
    Some(format!("Menu/{command}"))
}

fn command_label(text: &str) -> String {
    use crate::command::Command;
    let kind = match Command::parse(text) {
        Command::Shell(_) => "Shell",
        Command::NewConsole(_) => "NewConsole",
        Command::Cd(_) => "Cd",
        Command::Sort(_) => "Sort",
        Command::BookmarkSet(_) => "BookmarkSet",
        Command::BookmarkGo(_) => "BookmarkGo",
        Command::Invalid { .. } => "Invalid",
        Command::Empty => "Empty",
        other => return format!("Command/{other:?}"),
    };
    format!("Command/{kind}")
}

fn context(overlay: &Overlay) -> &'static str {
    match overlay {
        Overlay::None => "Panels",
        Overlay::Editor(_) => "Editor",
        Overlay::Viewer(_) => "Viewer",
        Overlay::CommandLine(_) => "CommandLine",
        Overlay::Menu(_) => "Menu",
        Overlay::Settings(_) => "Settings",
        Overlay::Wsl(_) => "WSL",
        Overlay::PathEditor(_) => "PathEditor",
        Overlay::LspEditor(_) => "LspEditor",
        #[cfg(feature = "keepass")]
        Overlay::Kdbx(_) => "KeePass",
        Overlay::DriveSelection { .. } => "DriveSelection",
        Overlay::QuitConfirm(_) => "QuitConfirm",
        Overlay::Help => "Help",
        Overlay::Tasks(_) => "Tasks",
        other => file_dialog_context(other),
    }
}

/// The dialogs that act on the panels' files, each by its own name; any other
/// overlay is a "Dialog".
fn file_dialog_context(overlay: &Overlay) -> &'static str {
    match overlay {
        Overlay::CopyPrompt(_) => "Transfer",
        Overlay::NamePrompt(_) => "NamePrompt",
        Overlay::DeleteConfirm(_) => "Delete",
        Overlay::WipeConfirm(_) => "Wipe",
        Overlay::ArchivePrompt(_) => "Archive",
        Overlay::ExtractPrompt(_) => "Extract",
        Overlay::ArchiveCommands(_) => "ArchiveCommands",
        Overlay::Attributes(_) => "Attributes",
        Overlay::ApplyCommand(_) => "ApplyCommand",
        Overlay::Describe(_) => "Describe",
        Overlay::FindFile(_) => "FindFile",
        Overlay::FindFolder(_) => "FindFolder",
        Overlay::NewFileInput(_) => "NewFile",
        _ => "Dialog",
    }
}

#[cfg(test)]
mod tests;
