//! Purpose: Dialog input: walking the fields, ticking rows and moving focus.

use super::*;

#[expect(
    clippy::expect_used,
    reason = "test fixture: a failed setup fails the calling test; clippy.toml allows that in \
              #[test] fns and #[cfg(test)] items, not in integration-test helpers"
)]
fn app_with(temp: &TempDir, dialog: DiskCleanupDialog) -> App {
    let panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_overlay(Overlay::DiskCleanup(Box::new(dialog)));
    app
}

#[expect(
    clippy::panic,
    reason = "test assertion: a closed cleanup dialog fails the calling test; clippy.toml allows \
              that in #[test] fns and #[cfg(test)] items, not in integration-test helpers"
)]
fn dialog_of(app: &App) -> &DiskCleanupDialog {
    let Overlay::DiskCleanup(ref dialog) = app.overlay else {
        panic!("the cleanup dialog must still be open");
    };
    dialog
}

#[test]
fn advance_field_walks_the_rows_then_the_buttons_and_wraps() {
    let mut dialog = dialog_with(vec![candidate("first", "/a"), candidate("second", "/b")]);

    // Forward: every row in turn, then Clean up, then Cancel, then back round.
    let forward = [
        CleanupField::Item(1),
        CleanupField::Run,
        CleanupField::Cancel,
        CleanupField::Item(0),
    ];
    for expected in forward {
        disk_cleanup::advance_field(&mut dialog, true);
        assert_eq!(dialog.field, expected);
    }

    // Backward retraces the same ring in reverse.
    let backward = [
        CleanupField::Cancel,
        CleanupField::Run,
        CleanupField::Item(1),
        CleanupField::Item(0),
    ];
    for expected in backward {
        disk_cleanup::advance_field(&mut dialog, false);
        assert_eq!(dialog.field, expected);
    }
}

#[test]
fn toggle_checked_refuses_a_row_that_is_not_on_this_machine() {
    let present = CleanupCandidate {
        state: CandidateState::Present {
            size: None,
            checked: false,
        },
        ..candidate("present", "/a")
    };
    let mut dialog = dialog_with(vec![present, candidate("absent", "/b")]);

    disk_cleanup::toggle_checked(&mut dialog);
    assert!(dialog.candidates[0].is_checked(), "a present row ticks");
    disk_cleanup::toggle_checked(&mut dialog);
    assert!(!dialog.candidates[0].is_checked(), "and unticks again");

    dialog.field = CleanupField::Item(1);
    disk_cleanup::toggle_checked(&mut dialog);
    assert!(
        !dialog.candidates[1].is_checked(),
        "a dimmed row cannot be selected for deletion"
    );

    // On a button there is no checkbox to flip, and nothing may change.
    dialog.field = CleanupField::Run;
    disk_cleanup::toggle_checked(&mut dialog);
    assert!(dialog.candidates.iter().all(|c| !c.is_checked()));
}

#[test]
fn toggle_and_advance_ticks_the_row_under_the_cursor_then_moves_on() {
    let temp = TempDir::new().expect("test setup");
    let rows = vec![
        CleanupCandidate {
            state: CandidateState::Present {
                size: None,
                checked: false,
            },
            ..candidate("first", "/a")
        },
        CleanupCandidate {
            state: CandidateState::Present {
                size: None,
                checked: false,
            },
            ..candidate("second", "/b")
        },
    ];
    let mut app = app_with(&temp, dialog_with(rows));

    disk_cleanup::toggle_and_advance(&mut app);

    let dialog = dialog_of(&app);
    assert!(dialog.candidates[0].is_checked());
    assert_eq!(dialog.field, CleanupField::Item(1));
}

#[test]
fn space_checked_ticks_the_row_without_moving_the_cursor() {
    let temp = TempDir::new().expect("test setup");
    let rows = vec![CleanupCandidate {
        state: CandidateState::Present {
            size: None,
            checked: false,
        },
        ..candidate("only", "/a")
    }];
    let mut app = app_with(&temp, dialog_with(rows));

    disk_cleanup::space_checked(&mut app);

    let dialog = dialog_of(&app);
    assert!(dialog.candidates[0].is_checked());
    assert_eq!(
        dialog.field,
        CleanupField::Item(0),
        "Space ticks in place; only Enter and the arrows move"
    );
}

#[test]
fn move_focus_walks_the_dialog_in_both_directions() {
    let temp = TempDir::new().expect("test setup");
    let rows = vec![candidate("first", "/a"), candidate("second", "/b")];
    let mut app = app_with(&temp, dialog_with(rows));

    disk_cleanup::move_focus(&mut app, true);
    assert_eq!(dialog_of(&app).field, CleanupField::Item(1));

    disk_cleanup::move_focus(&mut app, false);
    assert_eq!(dialog_of(&app).field, CleanupField::Item(0));

    assert!(
        dialog_of(&app).candidates.iter().all(|c| !c.is_checked()),
        "moving the cursor never ticks anything"
    );
}

#[test]
fn the_dialog_wrappers_ignore_every_other_overlay() {
    let temp = TempDir::new().expect("test setup");
    let panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_overlay(Overlay::Help);

    // Each wrapper reads the overlay itself, so a stale key for a dialog that
    // has already closed must fall through rather than panic.
    disk_cleanup::toggle_and_advance(&mut app);
    disk_cleanup::space_checked(&mut app);
    disk_cleanup::move_focus(&mut app, true);

    assert!(matches!(app.overlay, Overlay::Help));
}
