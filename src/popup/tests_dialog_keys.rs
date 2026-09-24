//! Unit tests: the dialog keys a user finds when they do not type straight
//! into the field.
//!
//! Every modal here answers Tab and the arrows as well as Enter, and those
//! arms are the ones nothing exercises: a test that types a name and presses
//! Enter never touches them, while a person who reaches for Tab meets them
//! immediately. An arm that does the wrong thing there is worse than a missing
//! one, because the dialog looks like it is working.

use std::fs;

use crossterm::event::KeyCode;
use tempfile::TempDir;

use super::process_popup_input;
use super::tests::key;
use crate::copy::{CopyField, CopyPrompt, ExistingFiles, Purpose};
use crate::menu::{Heading, Menu};
use crate::models::{
    App, Confirm, Lifecycle, MaskPrompt, MaskPurpose, NameField, NamePrompt, Overlay, Panel,
    PanelSide,
};

fn app_over(names: &[&str]) -> (App, TempDir) {
    let dir = TempDir::new().expect("test setup");
    for name in names {
        fs::write(dir.path().join(name), b"x").expect("test setup");
    }
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(dir.path().to_path_buf()).expect("test setup");
    (app, dir)
}

fn press(app: &mut App, code: KeyCode) {
    process_popup_input(app, key(code));
}

// ─── the information panel ──────────────────────────────────────────────────

/// Ctrl+L put it up, so Ctrl+L's own letter takes it down again — along with
/// the three keys that dismiss every other panel.
#[test]
fn the_information_panel_closes_on_any_of_its_four_keys() {
    for code in [
        KeyCode::Esc,
        KeyCode::Enter,
        KeyCode::F(10),
        KeyCode::Char('l'),
        KeyCode::Char('L'),
    ] {
        let (mut app, _dir) = app_over(&[]);
        app.set_overlay(Overlay::Information(Box::default()));
        press(&mut app, code);
        assert!(!app.overlay.is_active(), "{code:?} should close it");
    }
}

#[test]
fn the_information_panel_ignores_everything_else() {
    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::Information(Box::default()));
    press(&mut app, KeyCode::Char('q'));
    assert!(matches!(app.overlay, Overlay::Information(_)));
}

// ─── the quit dialog ────────────────────────────────────────────────────────

/// Two answers, so Left, Right and Tab all mean "the other one".
#[test]
fn every_movement_key_in_the_quit_dialog_means_the_other_button() {
    for code in [KeyCode::Left, KeyCode::Right, KeyCode::Tab] {
        let (mut app, _dir) = app_over(&[]);
        app.set_overlay(Overlay::QuitConfirm(Confirm::Yes));
        press(&mut app, code);
        let Overlay::QuitConfirm(answer) = app.overlay else {
            panic!("still open");
        };
        assert!(!answer.is_yes(), "{code:?} moved to No");
        assert!(app.lifecycle.is_running(), "moving is not answering");
    }
}

#[test]
fn enter_on_no_closes_the_quit_dialog_without_quitting() {
    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::QuitConfirm(Confirm::No));
    press(&mut app, KeyCode::Enter);
    assert!(!app.overlay.is_active());
    assert!(app.lifecycle.is_running(), "No means no");
}

#[test]
fn the_quit_dialog_ignores_a_key_that_is_not_an_answer() {
    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::QuitConfirm(Confirm::Yes));
    press(&mut app, KeyCode::Char('z'));
    assert!(matches!(app.overlay, Overlay::QuitConfirm(Confirm::Yes)));
}

/// `y`/`Y` and `F10` all answer the question directly, without needing the
/// cursor to be on Yes first — the same shortcut Far's own dialog offers.
#[test]
fn y_or_f10_quits_from_the_quit_dialog_regardless_of_focus() {
    for code in [KeyCode::Char('y'), KeyCode::Char('Y'), KeyCode::F(10)] {
        let (mut app, _dir) = app_over(&[]);
        app.set_overlay(Overlay::QuitConfirm(Confirm::No));
        press(&mut app, code);
        assert!(!app.lifecycle.is_running(), "{code:?} should quit");
    }
}

/// `n`/`N` and `Esc` all cancel directly, without needing the cursor to be on
/// No first.
#[test]
fn n_or_esc_cancels_the_quit_dialog_regardless_of_focus() {
    for code in [KeyCode::Char('n'), KeyCode::Char('N'), KeyCode::Esc] {
        let (mut app, _dir) = app_over(&[]);
        app.set_overlay(Overlay::QuitConfirm(Confirm::Yes));
        press(&mut app, code);
        assert!(!app.overlay.is_active(), "{code:?} should close it");
        assert!(app.lifecycle.is_running(), "{code:?} must not quit");
    }
}

// ─── the name dialog walks backwards too ────────────────────────────────────

#[test]
fn shift_tab_and_up_walk_the_make_folder_controls_backwards() {
    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));

    for code in [KeyCode::BackTab, KeyCode::Up] {
        let before = current_field(&app);
        press(&mut app, code);
        assert_ne!(current_field(&app), before, "{code:?} moved the cursor");
    }
    // Backwards from the name field is the last control, not the first.
    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    press(&mut app, KeyCode::BackTab);
    assert_eq!(current_field(&app), NameField::Cancel);
}

#[test]
fn left_steps_the_link_drop_down_backwards() {
    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    press(&mut app, KeyCode::Down);
    assert_eq!(current_field(&app), NameField::Link);

    press(&mut app, KeyCode::Left);
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert_eq!(prompt.link(), crate::models::LinkType::Symbolic);
}

fn current_field(app: &App) -> NameField {
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("the name dialog is open");
    };
    prompt.field()
}

// ─── the mask dialog ────────────────────────────────────────────────────────

fn open_mask(app: &mut App, offered: &str) {
    app.set_overlay(Overlay::MaskPrompt(MaskPrompt::new(
        MaskPurpose::Select,
        offered,
    )));
}

fn mask_prompt(app: &App) -> &MaskPrompt {
    let Overlay::MaskPrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    prompt
}

#[test]
fn tab_in_the_mask_dialog_means_the_other_button() {
    let (mut app, _dir) = app_over(&[]);
    open_mask(&mut app, "*");
    press(&mut app, KeyCode::Tab);
    assert!(mask_prompt(&app).on_cancel(), "Tab moved to Cancel");
}

/// The reported case: Right used to move the cursor to Cancel while the
/// typed `.img` still went into the field, and Enter then threw it away.
#[test]
fn left_and_right_in_the_mask_field_move_the_caret_not_the_focus() {
    let (mut app, _dir) = app_over(&["sparse.img"]);
    open_mask(&mut app, "*");
    press(&mut app, KeyCode::Right);
    for ch in ".img".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    assert!(!mask_prompt(&app).on_cancel(), "the field keeps the cursor");
    assert_eq!(mask_prompt(&app).mask(), "*.img");

    press(&mut app, KeyCode::Enter);
    assert!(app.left_panel.marked.contains("sparse.img"));
}

/// The caret moves inside the text, so a typo early in the mask is fixed in
/// place rather than by deleting back to it.
#[test]
fn the_caret_edits_the_mask_where_it_stands() {
    let (mut app, _dir) = app_over(&[]);
    open_mask(&mut app, "*.txt");
    for code in [KeyCode::Left, KeyCode::Left, KeyCode::Left] {
        press(&mut app, code);
    }
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(mask_prompt(&app).mask(), "*.xtxt");
    press(&mut app, KeyCode::Delete);
    press(&mut app, KeyCode::Home);
    press(&mut app, KeyCode::Delete);
    press(&mut app, KeyCode::End);
    press(&mut app, KeyCode::Backspace);
    assert_eq!(mask_prompt(&app).mask(), ".xx");
}

/// On a button, Left and Right walk the buttons, as in Far's dialogs.
#[test]
fn on_a_button_left_and_right_walk_the_buttons() {
    for code in [KeyCode::Left, KeyCode::Right] {
        let (mut app, _dir) = app_over(&[]);
        open_mask(&mut app, "*");
        press(&mut app, KeyCode::Tab);
        press(&mut app, code);
        assert!(!mask_prompt(&app).on_cancel(), "{code:?} went back to OK");
    }
}

#[test]
fn backspace_edits_the_mask() {
    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::MaskPrompt(MaskPrompt::new(
        MaskPurpose::Select,
        "*.rs",
    )));
    press(&mut app, KeyCode::Backspace);
    let Overlay::MaskPrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert_eq!(prompt.mask(), "*.r");
}

// ─── the copy dialog ────────────────────────────────────────────────────────

fn open_copy(app: &mut App, destination: &std::path::Path) {
    app.set_overlay(Overlay::CopyPrompt(Box::new(CopyPrompt::for_purpose(
        Purpose::Copy,
        "a.txt",
        destination,
    ))));
}

fn copy_field(app: &App) -> CopyField {
    let Overlay::CopyPrompt(prompt) = &app.overlay else {
        panic!("the copy dialog is open");
    };
    prompt.field
}

#[test]
fn shift_tab_and_up_walk_the_copy_controls_backwards() {
    let (mut app, dir) = app_over(&["a.txt"]);
    open_copy(&mut app, dir.path());
    press(&mut app, KeyCode::BackTab);
    assert_eq!(
        copy_field(&app),
        CopyField::Cancel,
        "backwards from the top"
    );
    press(&mut app, KeyCode::Up);
    assert_eq!(copy_field(&app), CopyField::Copy);
}

/// Space toggles the checkbox under the cursor, and does nothing on a control
/// that is not one.
#[test]
fn space_toggles_a_checkbox_field_in_the_copy_dialog() {
    let (mut app, dir) = app_over(&["a.txt"]);
    open_copy(&mut app, dir.path());
    if let Overlay::CopyPrompt(prompt) = &mut app.overlay {
        prompt.field = CopyField::Timestamps;
    }
    press(&mut app, KeyCode::Char(' '));
    let Overlay::CopyPrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert!(prompt.timestamps, "space must toggle the checkbox");
}

/// Backspace and plain typing edit the destination field, and only that one —
/// Far uses the arrows to move the caret there instead of changing the answer.
#[test]
fn backspace_and_typing_edit_the_copy_destination() {
    let (mut app, dir) = app_over(&["a.txt"]);
    open_copy(&mut app, dir.path());
    let original = {
        let Overlay::CopyPrompt(prompt) = &app.overlay else {
            panic!("still open");
        };
        prompt.destination.clone()
    };

    press(&mut app, KeyCode::Backspace);
    press(&mut app, KeyCode::Char('x'));

    let Overlay::CopyPrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    let mut expected = original;
    expected.pop();
    expected.push('x');
    assert_eq!(prompt.destination, expected);
}

#[test]
fn the_arrows_step_the_copy_dialogs_drop_down_both_ways() {
    let (mut app, dir) = app_over(&["a.txt"]);
    open_copy(&mut app, dir.path());
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    assert_eq!(copy_field(&app), CopyField::Existing);

    press(&mut app, KeyCode::Right);
    let forward = existing(&app);
    press(&mut app, KeyCode::Left);
    assert_eq!(existing(&app), ExistingFiles::Ask, "and back again");
    assert_ne!(forward, ExistingFiles::Ask);
}

fn existing(app: &App) -> ExistingFiles {
    let Overlay::CopyPrompt(prompt) = &app.overlay else {
        panic!("the copy dialog is open");
    };
    prompt.existing
}

/// Enter on Cancel means cancel wherever else it would mean go, and an empty
/// destination is not something to act on — it would resolve to the working
/// directory, which is not what an emptied field means.
#[test]
fn the_copy_dialog_closes_without_copying_on_cancel_or_an_empty_destination() {
    let (mut app, dir) = app_over(&["a.txt"]);
    open_copy(&mut app, dir.path());
    if let Overlay::CopyPrompt(prompt) = &mut app.overlay {
        prompt.field = CopyField::Cancel;
    }
    press(&mut app, KeyCode::Enter);
    assert!(!app.overlay.is_active());

    open_copy(&mut app, dir.path());
    if let Overlay::CopyPrompt(prompt) = &mut app.overlay {
        prompt.destination.clear();
    }
    press(&mut app, KeyCode::Enter);
    assert!(!app.overlay.is_active(), "nothing to copy to");
}

// ─── the menu bar ───────────────────────────────────────────────────────────

fn open_menu(app: &mut App, heading: Heading) {
    app.set_menu(Menu::open(heading));
}

fn menu_of(app: &App) -> Menu {
    let Overlay::Menu(menu) = &app.overlay else {
        panic!("the bar is open");
    };
    *menu
}

#[test]
fn left_and_right_walk_the_bar_in_both_directions() {
    let (mut app, _dir) = app_over(&[]);
    open_menu(&mut app, Heading::Files);

    press(&mut app, KeyCode::Left);
    assert_eq!(menu_of(&app).heading, Heading::Left);
    press(&mut app, KeyCode::Right);
    assert_eq!(menu_of(&app).heading, Heading::Files);
}

/// Walking the bar past a panel's heading only shows its menu: the active
/// panel stays put, so a Files or Commands row still acts on the panel the
/// user was in when F9 was pressed, and Esc leaves everything as it was.
#[test]
fn walking_across_a_panel_heading_keeps_the_active_panel() {
    let (mut app, _dir) = app_over(&[]);
    app.active_panel = PanelSide::Right;
    open_menu(&mut app, Heading::Right);

    press(&mut app, KeyCode::Right);
    assert_eq!(menu_of(&app).heading, Heading::Left);
    press(&mut app, KeyCode::Right);
    assert_eq!(menu_of(&app).heading, Heading::Files);
    assert_eq!(app.active_panel, PanelSide::Right, "walked past Left");

    press(&mut app, KeyCode::Left);
    press(&mut app, KeyCode::Esc);
    assert_eq!(
        app.active_panel,
        PanelSide::Right,
        "Esc on the Left heading"
    );
}

/// A row of the Left or Right menu acts on its own panel. The keys behind
/// those rows follow the active panel, so choosing one — and only choosing
/// one — makes that panel active.
#[test]
fn choosing_a_row_under_a_panel_heading_makes_that_panel_active() {
    let (mut app, _dir) = app_over(&[]);
    app.active_panel = PanelSide::Left;
    open_menu(&mut app, Heading::Right);
    assert_eq!(app.active_panel, PanelSide::Left, "only the menu is open");

    press(&mut app, KeyCode::Enter);

    assert!(!app.overlay.is_active());
    assert!(app.pending_key.is_some(), "the row's key is handed on");
    assert_eq!(app.active_panel, PanelSide::Right);
}

#[test]
fn up_and_down_walk_the_rows() {
    let (mut app, _dir) = app_over(&[]);
    open_menu(&mut app, Heading::Files);
    let first = menu_of(&app).row;

    press(&mut app, KeyCode::Down);
    let second = menu_of(&app).row;
    assert_ne!(second, first);
    press(&mut app, KeyCode::Up);
    assert_eq!(menu_of(&app).row, first);
}

/// Enter hands the chosen row's key on, and the bar leaves the screen first so
/// a command that opens a dialog is not fighting the drop-down for it.
#[test]
fn enter_closes_the_bar_and_hands_on_the_rows_key() {
    let (mut app, _dir) = app_over(&["a.txt"]);
    open_menu(&mut app, Heading::Files);

    press(&mut app, KeyCode::Enter);

    assert!(!app.overlay.is_active(), "the bar left the screen first");
    assert!(app.pending_key.is_some(), "and the command was handed on");
}

#[test]
fn a_key_the_bar_has_no_use_for_leaves_it_open() {
    let (mut app, _dir) = app_over(&[]);
    open_menu(&mut app, Heading::Files);
    press(&mut app, KeyCode::Insert);
    assert!(matches!(app.overlay, Overlay::Menu(_)), "still open");
}

/// A letter picks a row here; a letter this drop-down has no row for walks the
/// bar instead, which is how Far lets the bar be driven by its letters
/// whichever menu happens to be open.
#[test]
fn a_letter_picks_a_row_or_moves_to_the_heading_that_owns_it() {
    let (mut app, _dir) = app_over(&["a.txt"]);
    open_menu(&mut app, Heading::Files);
    press(&mut app, KeyCode::Char('d'));
    assert!(!app.overlay.is_active(), "Delete ran");

    // Files has no row on L, so it falls through to the Left heading.
    let (mut app, _dir) = app_over(&[]);
    open_menu(&mut app, Heading::Files);
    press(&mut app, KeyCode::Char('l'));
    assert_eq!(
        menu_of(&app).heading,
        Heading::Left,
        "moved to the Left menu"
    );

    let (mut app, _dir) = app_over(&[]);
    open_menu(&mut app, Heading::Files);
    press(&mut app, KeyCode::Char('9'));
    assert!(matches!(app.overlay, Overlay::Menu(_)), "no such letter");
}

#[test]
fn escape_closes_the_bar_and_runs_nothing() {
    let (mut app, _dir) = app_over(&[]);
    open_menu(&mut app, Heading::Files);
    press(&mut app, KeyCode::Esc);
    assert!(!app.overlay.is_active());
    assert!(app.pending_key.is_none());
    assert!(app.lifecycle.is_running(), "{:?}", Lifecycle::Running);
}

// ─── the guards, and the keys the dialogs have no use for ───────────────────

/// A key neither dialog claims must leave it exactly as it was, rather than
/// falling through to whatever the panels would have done with it.
#[test]
fn a_key_the_name_or_mask_dialog_has_no_use_for_changes_nothing() {
    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    press(&mut app, KeyCode::Insert);
    assert_eq!(current_field(&app), NameField::Name);
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert!(prompt.name().is_empty(), "and nothing was typed");

    app.set_overlay(Overlay::MaskPrompt(MaskPrompt::new(
        MaskPurpose::Select,
        "*",
    )));
    press(&mut app, KeyCode::Insert);
    let Overlay::MaskPrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert_eq!(prompt.mask(), "*");
}

/// Renaming onto a name that is taken refuses rather than overwriting: Far
/// asks, and this at least does not act.
#[test]
fn renaming_onto_a_name_that_is_taken_refuses() {
    use crate::models::FileName;

    let (mut app, dir) = app_over(&["a.txt", "b.txt"]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::rename(FileName::new(
        "a.txt",
    ))));
    for _ in 0..5 {
        press(&mut app, KeyCode::Backspace);
    }
    for ch in "b.txt".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    press(&mut app, KeyCode::Enter);

    assert!(
        dir.path().join("a.txt").exists(),
        "the original is still there"
    );
    assert_eq!(
        app.status,
        crate::status::Status::AlreadyExists("b.txt".into())
    );
}

/// F7 with several names makes each and counts them in the folder they went
/// into; the first name already taken stops the run and is the one named.
#[test]
fn make_folder_counts_what_it_made_and_names_what_was_taken() {
    use crate::status::Status;

    let (mut app, _dir) = app_over(&["taken"]);
    let directory = app.get_active_panel_ref().path.clone();
    let make = |app: &mut App, names: &str| {
        let mut prompt = NamePrompt::make_folder();
        prompt.focus(NameField::Multiple);
        prompt.toggle();
        prompt.focus(NameField::Name);
        names.chars().for_each(|ch| prompt.push(ch));
        app.set_overlay(Overlay::NamePrompt(prompt));
        press(app, KeyCode::Enter);
    };

    make(&mut app, "one;two");
    assert_eq!(
        app.status,
        Status::CreatedDirs {
            count: 2,
            directory
        }
    );
    make(&mut app, "three;taken");
    assert_eq!(app.status, Status::AlreadyExists("taken".into()));
}

/// Enter on a rename dialog whose name was never touched is a no-op: the
/// entry keeps its name, and nothing is even attempted.
#[test]
fn enter_on_an_unchanged_rename_does_nothing() {
    use crate::models::FileName;

    let (mut app, dir) = app_over(&["a.txt"]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::rename(FileName::new(
        "a.txt",
    ))));
    press(&mut app, KeyCode::Enter);

    assert!(!app.overlay.is_active(), "the dialog still closes");
    assert!(dir.path().join("a.txt").exists(), "nothing was renamed");
}

/// Enter on a "make folder" dialog whose name field was left blank does
/// nothing — a blank name is not a name to create.
#[test]
fn enter_on_a_blank_make_folder_name_does_nothing() {
    let (mut app, dir) = app_over(&[]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    press(&mut app, KeyCode::Enter);

    assert!(!app.overlay.is_active(), "the dialog still closes");
    assert_eq!(
        fs::read_dir(dir.path()).expect("read dir").count(),
        0,
        "no folder was created"
    );
}

/// A rename of something that is no longer on disk reports the failure rather
/// than claiming a rename that did not happen.
#[test]
fn renaming_something_that_is_gone_reports_it() {
    use crate::models::FileName;

    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::NamePrompt(NamePrompt::rename(FileName::new(
        "ghost.txt",
    ))));
    for _ in 0..9 {
        press(&mut app, KeyCode::Backspace);
    }
    for ch in "found.txt".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    press(&mut app, KeyCode::Enter);

    assert!(
        !app.status.to_string().starts_with("Renamed"),
        "{}",
        app.status
    );
}
