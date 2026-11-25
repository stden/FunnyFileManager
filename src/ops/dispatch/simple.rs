//! Purpose: Simple unit-variant colon commands handled outside the main dispatcher match.

use crate::command::Command;
use crate::models::{App, Lifecycle, Overlay};
use crate::status::Status;

type SimpleHandler = fn(&mut App);

pub(super) fn run(app: &mut App, command: &Command) {
    let handlers: [(Command, SimpleHandler); 8] = [
        (Command::Pwd, cmd_pwd),
        (Command::MarkAll, cmd_mark_all),
        (Command::UnmarkAll, cmd_unmark_all),
        (Command::Help, cmd_help),
        (Command::Stats, super::super::performance::open),
        (Command::Lsp, super::super::lsp_editor::open),
        (Command::Wsl, super::super::wsl::open_manager),
        (Command::Quit, cmd_quit),
    ];
    for (candidate, handler) in handlers {
        if candidate == *command {
            handler(app);
        }
    }
}

fn cmd_pwd(app: &mut App) {
    let path = app.get_active_panel().path.clone();
    app.set_status(Status::Path(path));
}

fn cmd_mark_all(app: &mut App) {
    let panel = app.get_active_panel();
    panel.mark_all();
    let count = panel.marked.len();
    app.set_status(Status::Marked { count });
}

fn cmd_unmark_all(app: &mut App) {
    app.get_active_panel().clear_marks();
    app.set_status(Status::UnmarkedAll);
}

fn cmd_help(app: &mut App) {
    app.set_overlay(Overlay::Help);
}

fn cmd_quit(app: &mut App) {
    app.lifecycle = Lifecycle::Quitting;
}
