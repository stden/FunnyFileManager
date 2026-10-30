//! Purpose: the default keys — each written as its own text reads, none
//! given to two actions of a context, and all of them, in the table's
//! order, what `Keymap::default()` holds.

use super::*;

#[test]
fn every_default_is_written_as_its_text_reads() {
    for screen in screens() {
        for row in &screen.rows {
            for chord in row.defaults {
                let read = chord.to_string().parse::<KeyChord>();
                assert_eq!(read, Ok(*chord), "{}.{}: {chord}", screen.context, row.name);
            }
        }
    }
}

#[test]
fn no_default_is_given_to_two_actions_of_a_context() {
    for screen in screens() {
        let chords: Vec<KeyChord> = screen
            .rows
            .iter()
            .flat_map(|row| row.defaults.iter().copied())
            .collect();
        assert_eq!(screen.bound, chords.len(), "{}", screen.context);
        assert_eq!(repeated(texts(&chords)), [""; 0], "{}", screen.context);
    }
}

#[test]
fn the_default_keymap_gives_every_action_its_defaults_in_their_order() {
    for screen in screens() {
        for row in &screen.rows {
            assert_eq!(
                row.in_force, row.defaults,
                "{}.{}",
                screen.context, row.name
            );
        }
    }
}

#[test]
fn the_default_tables_decide_nothing() {
    assert_eq!(tables::PANELS.layered(&Explicit::new()).conflicts, []);
    assert_eq!(tables::EDITOR.layered(&Explicit::new()).conflicts, []);
    assert_eq!(tables::TASK_LIST.layered(&Explicit::new()).conflicts, []);
    assert_eq!(tables::COMMAND_LINE.layered(&Explicit::new()).conflicts, []);
    assert_eq!(tables::VIEWER.defaults(), Keymap::default().viewer);
}

/// A sample of the design's Appendix A from every context; the rendered
/// default file pins the rest, name by name.
const SAMPLE: [(Context, &str, &[&str]); 22] = [
    (Context::Panels, "toggle_mark", &["Ins", "Space"]),
    (Context::Panels, "go_root", &["Ctrl+\\"]),
    (Context::Panels, "rename", &["Shift+F6"]),
    (Context::Panels, "wipe", &["Alt+Del"]),
    (Context::Panels, "view_alternative_full", &["Ctrl+0"]),
    (Context::Panels, "wsl_manager", &["Ctrl+Alt+W"]),
    (Context::Panels, "command_line", &[":"]),
    (Context::Panels, "compare_folders", &[]),
    (Context::TreePanel, "find_next", &["Ctrl+Enter"]),
    (Context::NetboxPanel, "new_session", &["Shift+F4"]),
    (
        Context::CommandLine,
        "word_left",
        &["Ctrl+Left", "Alt+Left", "Alt+B"],
    ),
    (
        Context::CommandLineSearch,
        "cancel",
        &["Esc", "Ctrl+G", "Ctrl+C"],
    ),
    (Context::CommandLineCompletion, "next", &["Down", "Tab"]),
    (Context::Viewer, "close", &["Esc", "Enter", "F3", "F10"]),
    (Context::Editor, "go_to_definition", &["Ctrl+]", "Ctrl+5"]),
    (Context::Editor, "select_page_down", &["Shift+PgDn"]),
    (
        Context::TaskList,
        "close",
        &["Esc", "F10", "q", "Ctrl+C", "Ctrl+W"],
    ),
    (Context::TaskListSearch, "find_next", &["F3"]),
    (Context::WslManager, "update_packages", &["F6"]),
    (Context::LanguageServers, "add", &["Ins", "F7"]),
    (Context::PathEditor, "move_entry_up", &["Ctrl+Up"]),
    (Context::Vault, "lock_and_quit", &["Ctrl+C"]),
];

#[test]
fn the_defaults_are_the_designs() {
    for (context, action, keys) in SAMPLE {
        let screen = screen_of(context);
        assert_eq!(
            texts(&screen.row(action).in_force),
            keys,
            "{context}.{action}"
        );
    }
}
