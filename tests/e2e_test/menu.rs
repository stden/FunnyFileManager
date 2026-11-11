//! Purpose: The menu bar: clock, F9, arrows, letters, rows and mouse.

use super::*;

// ─── F9, the menu bar ───────────────────────────────────────────────────────

/// The bar is always on screen, as Far's is, and the clock sits at its end.
#[test]
fn e2e_the_menu_bar_is_always_visible_with_the_clock() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let screen = run_until(left.path(), right.path(), vec![key(KeyCode::Down)]);
    let first = screen.lines().next().unwrap_or_default();

    for heading in ["Left", "Files", "Commands", "Options", "Right"] {
        assert!(first.contains(heading), "{heading} on the bar: {first:?}");
    }
    assert!(
        first.contains("AM") || first.contains("PM"),
        "the clock is on the bar: {first:?}"
    );
}

/// F9 opens the menu, and the drop-down shows what Far's shows.
#[test]
fn e2e_f9_opens_the_menu() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let screen = run_until(left.path(), right.path(), vec![key(KeyCode::F(9))]);

    assert!(screen.contains("Info panel"), "{screen}");
    assert!(screen.contains("Ctrl+L"), "the shortcut is printed too");
    assert!(screen.contains("Re-read"));
}

/// Right walks to the next heading, which shows a different list.
#[test]
fn e2e_the_arrows_walk_the_menu_bar() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::F(9)), key(KeyCode::Right)],
    );
    assert!(
        screen.contains("Rename or move"),
        "the Files menu: {screen}"
    );
    assert!(screen.contains("Make folder"));
}

/// Escape closes it and leaves the panels alone.
#[test]
fn e2e_escape_closes_the_menu() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::F(9)), key(KeyCode::Esc)],
    );
    assert!(!screen.contains("Info panel"), "{screen}");
}

/// A menu row runs the command by pressing its own key. Choosing "Make
/// folder" must put up exactly the dialog F7 does.
#[test]
fn e2e_choosing_a_menu_row_runs_its_command() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let screen = run_until(
        left.path(),
        right.path(),
        vec![
            key(KeyCode::F(9)),
            key(KeyCode::Right),     // Files
            key(KeyCode::Char('m')), // Make folder — Far's own letter
        ],
    );
    assert!(
        screen.contains("Create the folder"),
        "the F7 dialog, reached from the menu: {screen}"
    );
}

/// A letter belongs to the open drop-down first, and to the bar only if no
/// row claims it. Far works this way, and it has to: the drop-down is what the
/// user is looking at, so a letter that jumped out of it would be a surprise.
/// With Far's full menu every drop-down claims all five heading letters, so
/// the row always wins.
#[test]
fn e2e_a_letter_belongs_to_the_open_menu_before_the_bar() {
    let (left, right) = fixture(&[("a.txt", "a")]);

    // No row in the Files menu answers to "l", so it reaches the bar.
    let jumped = run_until(
        left.path(),
        right.path(),
        vec![
            key(KeyCode::F(9)),
            key(KeyCode::Right),
            key(KeyCode::Char('l')),
        ],
    );
    assert!(jumped.contains("Brief"), "the Left menu: {jumped}");

    // "c" is Change drive, which the Left menu does have — so it runs that
    // rather than jumping to Commands.
    let chose = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::F(9)), key(KeyCode::Char('c'))],
    );
    assert!(
        !chose.contains("Swap panels"),
        "the row wins over the heading: {chose}"
    );
}

/// Panel view modes remain visible in the main menu.
#[test]
fn e2e_the_menu_shows_panel_view_modes() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let screen = run_until(left.path(), right.path(), vec![key(KeyCode::F(9))]);
    assert!(screen.contains("Brief"), "panel modes are drawn");
}

/// Clicking a heading opens its menu. The bar is the one part of Far's chrome
/// a new user reaches for with the pointer rather than the keyboard.
#[test]
fn e2e_clicking_a_heading_opens_its_menu() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let bar = run_until(left.path(), right.path(), vec![key(KeyCode::Down)]);
    let (column, row) = find(&bar, "Commands");

    let screen = run_until(left.path(), right.path(), vec![click(column, row)]);
    assert!(screen.contains("Swap panels"), "{screen}");
}

/// …and clicking a row runs it, the same as choosing it with Enter.
#[test]
fn e2e_clicking_a_menu_row_runs_its_command() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let bar = run_until(left.path(), right.path(), vec![key(KeyCode::Down)]);
    let (heading_x, heading_y) = find(&bar, "Files");

    let open = run_until(left.path(), right.path(), vec![click(heading_x, heading_y)]);
    let (row_x, row_y) = find(&open, "Make folder");

    let screen = run_until(
        left.path(),
        right.path(),
        vec![click(heading_x, heading_y), click(row_x, row_y)],
    );
    assert!(screen.contains("Create the folder"), "{screen}");
}

/// Commands → Find file must open the search dialog, never F7's Make folder.
#[test]
fn e2e_clicking_find_file_opens_find_file_not_make_folder() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let bar = run_until(left.path(), right.path(), vec![key(KeyCode::Down)]);
    let (heading_x, heading_y) = find(&bar, "Commands");
    let open = run_until(left.path(), right.path(), vec![click(heading_x, heading_y)]);
    let (row_x, row_y) = find(&open, "Find file");
    let screen = run_until(
        left.path(),
        right.path(),
        vec![click(heading_x, heading_y), click(row_x, row_y)],
    );
    assert!(screen.contains("Find file"), "{screen}");
    assert!(screen.contains("File masks"), "{screen}");
    assert!(
        !screen.contains("Create the folder"),
        "must not open mkdir:\n{screen}"
    );
}

/// Clicking away closes the menu, which is what clicking away from an open
/// menu means everywhere else.
#[test]
fn e2e_clicking_away_closes_the_menu() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::F(9)), click(SCREEN_W - 2, SCREEN_H - 4)],
    );
    assert!(!screen.contains("Info panel"), "{screen}");
}
