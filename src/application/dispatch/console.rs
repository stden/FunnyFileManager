//! Purpose: apply actions while `Ctrl+O` holds the panels off screen.

use std::io::Write;

use ratatui::{backend::Backend, Terminal};

use crate::action::{Action, PanelCommand, ToolCommand};
use crate::models::{App, CommandLine, Lifecycle, Overlay, ViewMode};
use crate::ops;

use super::prompts::{open_command_line_with, open_menu};
use super::report;

/// Apply an action while `Ctrl+O` holds the panels off screen.
///
/// Far Manager's console mode is "show me the output *and let me keep
/// working*", so the command line still opens and still runs commands — their
/// output lands in the transcript right behind the prompt. The navigation keys
/// scroll that transcript instead of a panel nobody can see, and every action
/// that would touch the panels is swallowed: they are not on screen to act on,
/// and F8 silently deleting a file behind a full-screen console is exactly the
/// kind of thing the old modal `Overlay::Console` was protecting against. The
/// F9 menu's commands still run, as they always have: each opens its own
/// dialog or window over the transcript. A sort mode or a view mode is not a
/// menu row but a change to how the hidden panel looks, and is swallowed.
///
/// `menu_key` says the F9 menu's own keys took the key that produced `action`
/// (or it was a menu row or a gesture, which are menu commands already). Only
/// then do the menu's commands run; the same commands reached through another
/// chord are swallowed, as they were before the keymap.
pub(super) fn console_action<B>(
    terminal: &Terminal<B>,
    app: &mut App,
    action: Action,
    menu_key: bool,
) where
    B: Backend + Write,
{
    let page = transcript_page(terminal);
    match action {
        Action::Quit | Action::ConfirmQuit => app.lifecycle = Lifecycle::Quitting,
        Action::Menu(_) | Action::Panel(_) | Action::File(_) | Action::Tool(_) => {
            console_menu_action(app, action, menu_key);
        }
        Action::Dialog(crate::action::DialogCommand::Menu) => {
            app.view = ViewMode::Panels;
            open_menu(app);
        }
        Action::ToggleConsole => app.view = app.view.toggled(),
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

/// The menu's commands in console mode: the F9 menu's commands run when the
/// menu's own key took them, and F12 always lists the screens.
fn console_menu_action(app: &mut App, action: Action, menu_key: bool) {
    match action {
        // F12 lists the screens from either of them, the same list F9 shows.
        Action::Tool(ToolCommand::Screens) => {
            report(app, |app| {
                ops::menu::execute_tool(app, ToolCommand::Screens)
            });
        }
        Action::Menu(command) if menu_key => ops::menu::execute(app, command),
        // Ctrl+F3 … F11 change a panel nobody can see. The view modes are
        // `Action::PanelView`, swallowed by the arm at the end of `console_action`.
        Action::Panel(PanelCommand::Sort(_)) => {}
        Action::Panel(command) if menu_key => ops::menu::execute_panel(app, command),
        Action::File(command) if menu_key => ops::menu::execute_file(app, command),
        Action::Tool(command) if menu_key => {
            report(app, |app| ops::menu::execute_tool(app, command));
        }
        _ => {}
    }
}

/// One page of the transcript: the console is drawn full-screen, so it is
/// the terminal itself.
fn transcript_page<B: Backend>(terminal: &Terminal<B>) -> usize {
    let rows = terminal
        .size()
        .map_or(crate::ui::FALLBACK_TERMINAL_HEIGHT, |size| size.height);
    usize::from(rows).max(1)
}
