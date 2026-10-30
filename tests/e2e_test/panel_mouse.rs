//! Purpose: Clicking and double-clicking panel rows, and focus between panels and command line.

use super::*;

/// Clicking a panel row selects that entry and makes its side active.
#[test]
fn e2e_clicking_a_panel_row_selects_that_entry() {
    let (left, right) = fixture(&[("aaa.txt", "1"), ("bbb.txt", "2"), ("ccc.txt", "3")]);

    // The menu bar is row 0; row 3 is the first entry ("..") and row 4 the
    // second — `aaa.txt`.
    // Clicking it and pressing F5 proves the selection followed the pointer.
    let mut events = vec![click(5, 4), key(KeyCode::F(5)), key(KeyCode::Enter)];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        right.path().join("aaa.txt").exists(),
        "F5 must act on the row that was clicked"
    );
}

/// A panel double-click is the mouse spelling of Enter, so it must navigate
/// through a directory rather than merely selecting it twice.
#[test]
fn e2e_double_clicking_a_directory_enters_it() {
    let (left, right) = fixture(&[]);
    let child = left.path().join("child");
    fs::create_dir(&child).expect("seed child directory");
    fs::write(child.join("inside-child.txt"), "visible after entering").expect("seed child file");

    // Row 3 is `..`; directories sort first, so row 4 is `child`.
    let screen = run_until(left.path(), right.path(), vec![click(5, 4), click(5, 4)]);

    assert!(
        screen.contains("inside-child"),
        "the second click must perform Enter on the selected directory:\n{screen}"
    );
}

/// The synthetic parent row is activated through the exact same Enter path.
#[test]
fn e2e_double_clicking_parent_leaves_the_directory() {
    let (left, right) = fixture(&[]);
    let child = left.path().join("child");
    fs::create_dir(&child).expect("seed child directory");
    fs::write(left.path().join("only-in-parent.txt"), "parent marker").expect("seed parent file");

    // `..` is always the first visible entry, at screen row 3.
    let screen = run_until(&child, right.path(), vec![click(5, 3), click(5, 3)]);

    assert!(
        screen.contains("only-in-parent"),
        "double-clicking `..` must leave the current directory:\n{screen}"
    );
}

/// Clicking the right panel makes it active, so the next operation aims the
/// other way.
#[test]
fn e2e_clicking_the_other_panel_makes_it_active() {
    let (left, right) = fixture(&[("from-left.txt", "x")]);
    fs::write(right.path().join("from-right.txt"), "y").expect("seed");

    // Click a right-panel entry row, then copy: it must land in the LEFT panel.
    let mut events = vec![
        click(SCREEN_W - 10, 4),
        key(KeyCode::F(5)),
        key(KeyCode::Enter),
    ];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        left.path().join("from-right.txt").exists(),
        "the active panel must have followed the click"
    );
}

/// The command prompt gives way when a visible panel is clicked, and the same
/// click still selects the row under the pointer rather than being discarded.
#[test]
fn e2e_clicking_a_panel_from_the_command_line_focuses_that_panel() {
    let (left, right) = fixture(&[]);
    fs::write(right.path().join("from-right.txt"), "y").expect("seed");

    // Open an empty command line, click the right-panel file, then copy. If the
    // prompt kept focus, F5 would be swallowed and nothing would reach left.
    let mut events = vec![
        key(KeyCode::Char(':')),
        click(SCREEN_W - 10, 4),
        key(KeyCode::F(5)),
        key(KeyCode::Enter),
    ];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        left.path().join("from-right.txt").exists(),
        "the panel click must close the prompt and focus the clicked side"
    );
}

/// Panel chrome has no row to select, but it still belongs to its panel. A
/// click there closes the command line and preserves that panel's selection.
#[test]
fn e2e_clicking_panel_chrome_from_the_command_line_focuses_that_panel() {
    let (left, right) = fixture(&[]);
    fs::write(right.path().join("selected-on-right.txt"), "y").expect("seed");

    let mut events = vec![
        key(KeyCode::Tab),  // focus the right panel
        key(KeyCode::Down), // select its file
        key(KeyCode::Tab),  // put focus back on the left
        key(KeyCode::Char(':')),
        click(SCREEN_W - 10, 2), // right panel's column-header row
        key(KeyCode::F(5)),
        key(KeyCode::Enter),
    ];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        left.path().join("selected-on-right.txt").exists(),
        "panel chrome must close the prompt, focus its side, and keep its selection"
    );
}

/// Ctrl+O removes the panels from the screen. Coordinates that would normally
/// hit a panel must therefore leave the command line alone in console mode.
#[test]
fn e2e_clicking_the_console_keeps_the_command_line_active() {
    let (left, right) = fixture(&[]);

    let mut events = vec![key_with(KeyCode::Char('o'), KeyModifiers::CONTROL)];
    events.extend(typed("mkdir kept-after-console-click"));
    events.extend([
        click(SCREEN_W - 10, 4),
        key(KeyCode::Enter),
        key_with(KeyCode::Char('o'), KeyModifiers::CONTROL),
    ]);
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        left.path().join("kept-after-console-click").is_dir(),
        "a click on the console must not dismiss the command being edited"
    );
}

/// Only a visible panel takes focus from the command line. In particular, a
/// click on the function-key bar must not let a destructive panel action run.
#[test]
fn e2e_the_command_line_owns_clicks_outside_the_panels() {
    let (left, right) = fixture(&[("must-survive.txt", "x")]);

    let mut events = vec![key(KeyCode::Down)];
    events.extend(typed("unfinished command"));
    events.push(click(62, SCREEN_H - 1)); // 8Delete
    let screen = run_until(left.path(), right.path(), events);

    assert!(
        screen.contains("unfinished command"),
        "the prompt stays open"
    );
    assert!(
        left.path().join("must-survive.txt").exists(),
        "the key-bar click must not reach the selected file"
    );
}
