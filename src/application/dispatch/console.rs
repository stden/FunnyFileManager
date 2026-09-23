//! Purpose: apply actions while `Ctrl+O` holds the panels off screen.

use std::io::Write;

use ratatui::{backend::Backend, Terminal};

use crate::action::{Action, ToolCommand};
use crate::models::{App, CommandLine, Lifecycle, Overlay, ViewMode};
use crate::ops;

use super::prompts::{open_command_line_with, open_menu};

/// Apply an action while `Ctrl+O` holds the panels off screen.
///
/// Far Manager's console mode is "show me the output *and let me keep
/// working*", so the command line still opens and still runs commands — their
/// output lands in the transcript right behind the prompt. The navigation keys
/// scroll that transcript instead of a panel nobody can see, and every action
/// that would touch the panels is swallowed: they are not on screen to act on,
/// and F8 silently deleting a file behind a full-screen console is exactly the
/// kind of thing the old modal `Overlay::Console` was protecting against.
pub(super) fn console_action<B>(terminal: &Terminal<B>, app: &mut App, action: Action)
where
    B: Backend + Write,
{
    // The console is drawn full-screen, so one page is the terminal itself.
    let page = terminal
        .size()
        .map_or(crate::ui::FALLBACK_TERMINAL_HEIGHT, |size| size.height);
    let page = usize::from(page).max(1);

    match action {
        Action::Quit | Action::ConfirmQuit => app.lifecycle = Lifecycle::Quitting,
        Action::MenuCommand(command) => ops::menu::execute(app, command),
        Action::Dialog(crate::action::DialogCommand::Menu) => {
            app.view = ViewMode::Panels;
            open_menu(app);
        }
        Action::ToggleConsole => app.view = app.view.toggled(),
        // F12 lists the screens from either of them.
        Action::Tool(ToolCommand::Screens) => ops::screens::open(app),
        Action::NavUp => app.console.scroll_up(1),
        Action::NavDown => app.console.scroll_down(1),
        Action::PageUp => app.console.scroll_up(page),
        Action::PageDown => app.console.scroll_down(page),
        Action::First => app.console.scroll_to_top(),
        Action::Last => app.console.scroll_to_bottom(),
        Action::CommandLine | Action::CommandLineChar(_) => {
            // Typing in a terminal you have scrolled back through snaps it to
            // the bottom: the prompt is down there, and typing at a prompt you
            // cannot see is typing blind.
            app.console.scroll_to_bottom();
            match action {
                Action::CommandLineChar(ch) => open_command_line_with(app, ch),
                _ => app.set_overlay(Overlay::CommandLine(CommandLine::default())),
            }
        }
        _ => {}
    }
}
