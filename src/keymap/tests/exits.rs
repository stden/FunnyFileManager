//! Purpose: the actions without which a screen cannot be left are the
//! design's (§6.4), and each has a default every terminal delivers.

use super::*;

/// `tree_panel`, `netbox_panel` and `task_list_search` fall through to a
/// parent and need none.
const WAYS_OUT: [(Context, &[&str]); 14] = [
    (Context::Panels, &["confirm_quit", "toggle_console"]),
    (Context::TreePanel, &[]),
    (Context::NetboxPanel, &[]),
    (Context::CommandLine, &["submit", "cancel"]),
    (Context::CommandLineSearch, &["cancel"]),
    (Context::CommandLineCompletion, &["close"]),
    (Context::Viewer, &["close"]),
    (Context::Editor, &["close"]),
    (Context::TaskList, &["close"]),
    (Context::TaskListSearch, &[]),
    (Context::WslManager, &["close"]),
    (Context::LanguageServers, &["close"]),
    (Context::PathEditor, &["close"]),
    (Context::Vault, &["close"]),
];

#[test]
fn the_ways_out_of_every_screen_are_the_designs() {
    for (context, expected) in WAYS_OUT {
        let screen = screen_of(context);
        let ways_out: Vec<&str> = screen
            .rows
            .iter()
            .filter(|row| row.role == Role::Exit)
            .map(|row| row.name.as_str())
            .collect();
        assert_eq!(ways_out, expected, "{context}");
    }
}

#[test]
fn every_way_out_has_a_default_every_terminal_delivers() {
    for screen in screens() {
        for row in screen.rows.iter().filter(|row| row.role == Role::Exit) {
            let delivered = row
                .defaults
                .iter()
                .any(|chord| chord.delivery().on_legacy());
            assert!(delivered, "{}.{}", screen.context, row.name);
        }
    }
}

#[test]
fn a_table_names_its_ways_out() {
    let names: Vec<&str> = tables::PANELS
        .exits()
        .map(|row| row.name.as_str())
        .collect();
    assert_eq!(names, ["confirm_quit", "toggle_console"]);
    assert_eq!(tables::TREE_PANEL.exits().count(), 0);
    assert_eq!(tables::TREE_PANEL.rows().count(), 15);
}

/// The panels with both ways out unbound in the file: both come back, and
/// nothing else moves.
#[test]
fn the_panels_cannot_lose_the_quit_question_or_the_console() {
    let mut file = Explicit::new();
    file.insert(ActionName::new("confirm_quit"), Vec::new());
    file.insert(ActionName::new("toggle_console"), Vec::new());
    let layered = tables::PANELS.layered(&file);
    let defaults = Keymap::default().panels;
    assert_eq!(layered.bindings.chords().count(), defaults.chords().count());
    for (chord, action) in defaults.chords() {
        assert_eq!(layered.bindings.command(chord), Some(action), "{chord}");
    }
    assert_eq!(
        layered.conflicts,
        [
            Conflict::ExitRestored {
                action: ActionName::new("confirm_quit")
            },
            Conflict::ExitRestored {
                action: ActionName::new("toggle_console")
            },
        ]
    );
}
