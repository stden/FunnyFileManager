//! Purpose: the fifteen tables — the fourteen contexts of the design spec
//! (§4.1) and the quick view, a state of `viewer` — and the panel family
//! among them, each as its resolver and the popup over it read a key today.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::action::tree::{self, TreeCommand};
use funny_file_manager::action::viewer::{self, ViewerCommand};
use funny_file_manager::action::{self, netbox, Action, ActionScope, Bindings};
use funny_file_manager::keymap::Keymap;

use super::outcome::{consumed, debug, Context, Outcome};
use super::overlays::{
    command_line, editor_family, editor_overlay, language_servers, line_completion, line_family,
    line_search, line_search_family, path_editor, task_list, task_list_family, task_list_search,
    vault_browser, vault_family, viewer_overlay, wsl_family, wsl_manager,
};

/// Every table, in the order of the design spec's §4.1, the quick view
/// after the viewer it is a state of.
pub const CONTEXTS: [Context; 15] = [
    context("panels", panels, panels_family),
    context("tree_panel", tree_panel, tree_family),
    context("netbox_panel", netbox_panel, no_family),
    context("quick_view", quick_view, no_family),
    context("viewer", viewer_overlay, no_family),
    context("editor", editor_overlay, editor_family),
    context("command_line", command_line, line_family),
    context("command_line_search", line_search, line_search_family),
    context("command_line_completion", line_completion, no_family),
    context("task_list", task_list, task_list_family),
    context("task_list_search", task_list_search, no_family),
    context("wsl_manager", wsl_manager, wsl_family),
    context("language_servers", language_servers, no_family),
    context("path_editor", path_editor, no_family),
    context("vault", vault_browser, vault_family),
];

const fn context(
    name: &'static str,
    run: fn(KeyEvent) -> Outcome,
    family: fn(KeyEvent) -> Outcome,
) -> Context {
    Context { name, run, family }
}

/// A context whose fallback consumes no command of its own: it swallows the
/// key, hands it on or refuses it.
fn no_family(_: KeyEvent) -> Outcome {
    Outcome::Nothing
}

/// The panels' keys under the defaults: what every panel resolver reads.
fn panels_keys() -> Bindings<Action> {
    Keymap::default().panels
}

fn panels(event: KeyEvent) -> Outcome {
    consumed(&action::resolve(&panels_keys(), event))
}

/// Ctrl with a character clears the quick search, Alt with one searches,
/// any other character opens the command line holding it, and every other
/// key clears the quick search.
fn panels_family(event: KeyEvent) -> Outcome {
    let KeyCode::Char(ch) = event.code else {
        return consumed(&Action::QuickSearchClear);
    };
    if event.modifiers.contains(KeyModifiers::CONTROL) {
        consumed(&Action::QuickSearchClear)
    } else if event.modifiers.contains(KeyModifiers::ALT) {
        consumed(&Action::QuickSearch(ch))
    } else {
        consumed(&Action::CommandLineChar(ch))
    }
}

/// The tree's own command, or the panels action the key falls through to.
fn tree_panel(event: KeyEvent) -> Outcome {
    tree::resolve(event).map_or_else(|| falls_through(event), |command| consumed(&command))
}

/// A character without Ctrl searches the tree; the tree hands on the rest.
fn tree_family(event: KeyEvent) -> Outcome {
    match event.code {
        KeyCode::Char(ch) if !event.modifiers.contains(KeyModifiers::CONTROL) => {
            consumed(&TreeCommand::Search(ch))
        }
        _ => Outcome::Nothing,
    }
}

/// The panel's own command by the keymap's default keys, matched exactly,
/// or the panels action the key would run: handed on when it is the
/// program's, refused when it would act on the entries hidden beneath the
/// panel (`popup::netbox::refuse_entry_key`).
fn netbox_panel(event: KeyEvent) -> Outcome {
    if let Some(command) = netbox::resolve(&Keymap::default().netbox_panel, event) {
        return consumed(&command);
    }
    let action = action::resolve(&panels_keys(), event);
    match action.scope() {
        ActionScope::Program => Outcome::FallsThrough(debug(&action)),
        ActionScope::Entries => Outcome::Refuses(debug(&action)),
    }
}

/// The viewer's scroll commands over the panels: the rest, the viewer's
/// closing keys included, is the panels' (`popup::quickview::process`).
fn quick_view(event: KeyEvent) -> Outcome {
    match viewer::resolve(&Keymap::default().viewer, event) {
        Some(command @ ViewerCommand::Scroll(_)) => consumed(&command),
        Some(ViewerCommand::Close) | None => falls_through(event),
    }
}

fn falls_through(event: KeyEvent) -> Outcome {
    Outcome::FallsThrough(debug(&action::resolve(&panels_keys(), event)))
}
