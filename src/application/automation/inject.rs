//! Purpose: Dispatch a temporary synthetic chord without changing the user's keymap.

use std::io::Write;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::Backend, Terminal};

use crate::{
    action::{Action, Bindings, KeyChord},
    automation::catalog::Invocation,
    models::App,
};

pub(super) fn dispatch<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    invocation: Invocation,
) {
    let original = app.keymap.clone();
    let event = KeyEvent::new(KeyCode::F(24), KeyModifiers::NONE);
    if let Some(chord) = KeyChord::from_event(event) {
        app.keymap.panels.bind(Action::QuickSearchClear, chord);
        // An open completion list otherwise gets first refusal on the chord.
        if matches!(invocation, Invocation::CommandLine(_)) {
            app.keymap.command_line_completion = Bindings::default();
        }
        bind(app, invocation, chord);
        super::super::dispatch_key(terminal, app, event);
    }
    app.keymap = original;
}

fn bind(app: &mut App, invocation: Invocation, chord: KeyChord) {
    macro_rules! bind_contexts {
        ($($variant:ident => $field:ident),+ $(,)?) => {
            match invocation {
                $(Invocation::$variant(command) => { app.keymap.$field.bind(command, chord); })+
            }
        };
    }
    bind_contexts! {
        Panels => panels, TreePanel => tree_panel, NetboxPanel => netbox_panel,
        CommandLine => command_line, CommandLineSearch => command_line_search,
        CommandLineCompletion => command_line_completion, Viewer => viewer,
        Editor => editor, TaskList => task_list, TaskListSearch => task_list_search,
        WslManager => wsl_manager, LanguageServers => language_servers,
        PathEditor => path_editor, Vault => vault,
    }
}

pub(super) const fn find_next() -> KeyEvent {
    // The existing task-search interpreter owns F3 directly, not a binding.
    KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE)
}
