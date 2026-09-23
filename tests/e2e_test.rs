//! End-to-end tests: keystrokes in, filesystem and screen out.
//!
//! Every test here drives the real event loop through `run_app_with_event_reader`
//! — the same function `main` calls — and then asserts on what a user could
//! actually observe: the files on disk, and the cells on the screen.
//!
//! That is the difference from the rest of the suite. A test that calls
//! `handle_copy` and checks a struct field proves the handler works; it cannot
//! prove that F5 reaches it, that the key is not swallowed by an overlay, that
//! the dispatcher routes it, or that the result is drawn. Those are exactly the
//! joints that broke while this program was being made to look like Far, and
//! they are only visible from the outside.
//!
//! The event loop consumes the `App`, which is a feature here: after the loop
//! there is no state left to peek at, so an assertion has to be about something
//! real.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use funny_file_manager::models::{App, Panel};
use funny_file_manager::{run_app_with_event_reader, WritableTestBackend};
use ratatui::Terminal;
use std::fs;
use std::io;
use std::path::Path;
use tempfile::TempDir;

const SCREEN_W: u16 = 100;
const SCREEN_H: u16 = 24;

// ─── harness ────────────────────────────────────────────────────────────────

const fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    })
}

const fn key_with(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    })
}

/// The keys that end a session: F10 asks, Enter answers Yes.
fn quit() -> Vec<Event> {
    vec![key(KeyCode::F(10)), key(KeyCode::Enter)]
}

fn typed(text: &str) -> Vec<Event> {
    text.chars().map(|ch| key(KeyCode::Char(ch))).collect()
}

/// Whether the key sequence is expected to end the session by itself.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ends {
    /// The sequence quits; the loop must return of its own accord.
    Yes,
    /// The sequence stops with something still on screen, so it can be
    /// asserted against. The loop then runs out of events, which is how the
    /// test says "stop here" and not a failure.
    No,
}

/// Run the real event loop over `events` and return the screen it left behind.
fn drive(left: &Path, right: &Path, events: Vec<Event>, ends: Ends) -> String {
    let mut app = App::new().expect("app");
    app.left_panel = Panel::new(left.to_path_buf()).expect("left panel");
    app.right_panel = Panel::new(right.to_path_buf()).expect("right panel");

    let mut terminal =
        Terminal::new(WritableTestBackend::new(SCREEN_W, SCREEN_H)).expect("terminal");
    let mut stream = events.into_iter();
    let outcome = run_app_with_event_reader(&mut terminal, app, || {
        stream
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "out of events"))
    });
    if ends == Ends::Yes {
        // An error here means the session did not end when it was told to,
        // which is a bug in the program and not in the test.
        outcome.expect("the session must end on F10 + Enter");
    }

    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The common case: a sequence that quits at the end.
fn run(left: &Path, right: &Path, events: Vec<Event>) -> String {
    drive(left, right, events, Ends::Yes)
}

/// A sequence that stops with a dialog or overlay still up.
fn run_until(left: &Path, right: &Path, events: Vec<Event>) -> String {
    drive(left, right, events, Ends::No)
}

/// A pair of directories, the left one seeded with `files`.
fn fixture(files: &[(&str, &str)]) -> (TempDir, TempDir) {
    let left = TempDir::new().expect("left dir");
    let right = TempDir::new().expect("right dir");
    for (name, body) in files {
        fs::write(left.path().join(name), body).expect("seed file");
    }
    (left, right)
}

// ─── the panels ─────────────────────────────────────────────────────────────

/// The starting screen: a listing with Far's four columns and the key bar.
#[test]
fn e2e_a_session_opens_on_two_panels_and_a_key_bar() {
    let (left, right) = fixture(&[("alpha.txt", "a")]);
    let screen = run(left.path(), right.path(), quit());

    assert!(screen.contains("alpha"), "the file is listed:\n{screen}");
    assert!(screen.contains("Name"), "column headers:\n{screen}");
    assert!(screen.contains("Size"), "column headers:\n{screen}");
    assert!(screen.contains("Date"), "column headers:\n{screen}");
    assert!(screen.contains("Time"), "column headers:\n{screen}");
    assert!(screen.contains("1Help"), "the key bar:\n{screen}");
    assert!(screen.contains("12Screen"), "all twelve keys:\n{screen}");
}

/// Far names a directory rather than measuring it, and prints a file's byte
/// count outright.
#[test]
fn e2e_the_size_column_names_folders_and_counts_files() {
    let left = TempDir::new().expect("dir");
    let right = TempDir::new().expect("dir");
    fs::create_dir(left.path().join("docs")).expect("dir");
    fs::write(left.path().join("data.bin"), vec![0u8; 4096]).expect("file");

    let screen = run(left.path(), right.path(), quit());

    assert!(screen.contains("Folder"), "directories:\n{screen}");
    assert!(screen.contains("Up"), "the parent row:\n{screen}");
    assert!(screen.contains("4096"), "the exact byte count:\n{screen}");
}

/// Far splits a file row at its final dot and right-aligns the extension.
#[test]
fn e2e_a_file_name_is_split_from_its_extension() {
    let (left, right) = fixture(&[("report.md", "x")]);
    let screen = run(left.path(), right.path(), quit());

    let row = screen
        .lines()
        .find(|line| line.contains("report"))
        .expect("the file is listed");
    assert!(row.contains("report"), "{row}");
    assert!(row.contains("md"), "the extension is shown: {row}");
    assert!(
        !row.contains("report.md"),
        "name and extension are separate columns: {row}"
    );
}

/// Tab moves the cursor bar to the other panel, which is what makes every
/// two-panel operation aim somewhere.
#[test]
fn e2e_tab_switches_the_active_panel() {
    let (left, right) = fixture(&[("only-left.txt", "x")]);
    fs::write(right.path().join("only-right.txt"), "y").expect("seed");

    let mut events = vec![key(KeyCode::Tab)];
    events.extend(quit());
    let screen = run(left.path(), right.path(), events);

    assert!(screen.contains("only-right"), "both panels draw:\n{screen}");
}

// ─── file operations ────────────────────────────────────────────────────────

/// F5 copies the entry under the cursor to the other panel. The assertion is
/// the file on disk, which is the only thing that matters afterwards.
#[test]
fn e2e_f5_copies_the_selected_file_to_the_other_panel() {
    let (left, right) = fixture(&[("payload.txt", "contents")]);

    // F5 opens the Copy dialog; Enter takes the offered destination, which is
    // the other panel.
    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(5)), key(KeyCode::Enter)];
    events.extend(quit());
    run(left.path(), right.path(), events);

    let copied = right.path().join("payload.txt");
    assert!(copied.exists(), "F5 must copy to the other panel");
    assert_eq!(fs::read_to_string(copied).unwrap(), "contents");
    assert!(
        left.path().join("payload.txt").exists(),
        "a copy leaves the original"
    );
}

/// F6 moves rather than copies: the original must be gone.
#[test]
fn e2e_f6_moves_the_selected_file_to_the_other_panel() {
    let (left, right) = fixture(&[("moving.txt", "contents")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(6)), key(KeyCode::Enter)];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(right.path().join("moving.txt").exists(), "moved in");
    assert!(
        !left.path().join("moving.txt").exists(),
        "a move leaves nothing behind"
    );
}

/// F8 must NOT delete on its own — the confirmation is the whole point.
#[test]
fn e2e_f8_asks_before_deleting_and_cancelling_keeps_the_file() {
    let (left, right) = fixture(&[("precious.txt", "keep me")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(8)), key(KeyCode::Esc)];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        left.path().join("precious.txt").exists(),
        "cancelling the dialog must keep the file"
    );
}

/// …and confirming it does delete.
#[test]
fn e2e_f8_deletes_once_confirmed() {
    let (left, right) = fixture(&[("doomed.txt", "bye")]);

    let mut events = vec![
        key(KeyCode::Down),
        key(KeyCode::F(8)),
        key(KeyCode::Char('y')),
    ];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        !left.path().join("doomed.txt").exists(),
        "confirming must delete the file"
    );
}

/// The delete dialog names the file it is about, on Far's red ground.
#[test]
fn e2e_the_delete_dialog_names_the_file() {
    let (left, right) = fixture(&[("named.txt", "x")]);

    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::Down), key(KeyCode::F(8))],
    );

    assert!(screen.contains("Delete"), "the dialog is titled:\n{screen}");
    assert!(
        screen.contains("Do you wish to delete the file?"),
        "and asks:\n{screen}"
    );
    assert!(screen.contains("named.txt"), "naming the file:\n{screen}");
    assert!(screen.contains("{ Delete }"), "default button:\n{screen}");
    assert!(screen.contains("[ Cancel ]"), "cancel button:\n{screen}");
}

/// Shift+F4 creates an empty file with the name it is given.
#[test]
fn e2e_shift_f4_creates_a_new_file() {
    let (left, right) = fixture(&[]);

    let mut events = vec![key_with(KeyCode::F(4), KeyModifiers::SHIFT)];
    events.extend(typed("fresh.txt"));
    events.push(key(KeyCode::Enter));
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        left.path().join("fresh.txt").is_file(),
        "Shift+F4 creates it"
    );
}

// ─── the editor ─────────────────────────────────────────────────────────────

/// F4 opens the editor, typing changes the buffer, F2 writes it back.
#[test]
fn e2e_f4_edits_a_file_and_f2_saves_it() {
    let (left, right) = fixture(&[("edit-me.txt", "old")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(4))];
    events.extend(typed("NEW "));
    events.push(key(KeyCode::F(2)));
    events.push(key(KeyCode::Esc));
    events.extend(quit());
    run(left.path(), right.path(), events);

    let saved = fs::read_to_string(left.path().join("edit-me.txt")).expect("file still there");
    assert!(
        saved.starts_with("NEW "),
        "the typed text must reach the file, got {saved:?}"
    );
}

/// Leaving a modified buffer must ask rather than discard silently.
#[test]
fn e2e_leaving_a_modified_buffer_asks_and_no_discards_the_edit() {
    let (left, right) = fixture(&[("keep.txt", "original")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(4))];
    events.extend(typed("X"));
    events.push(key(KeyCode::Esc)); // opens the unsaved-changes dialog
    events.push(key(KeyCode::Char('n'))); // discard
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert_eq!(
        fs::read_to_string(left.path().join("keep.txt")).unwrap(),
        "original",
        "answering No must leave the file alone"
    );
}

/// F7 inside the editor searches, and the caret lands on the match — which is
/// visible because the editor bar reports the line it is on.
#[test]
fn e2e_f7_in_the_editor_finds_a_line_and_moves_the_caret() {
    let (left, right) = fixture(&[("haystack.txt", "one\ntwo\nthree\nneedle\nfive")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(4)), key(KeyCode::F(7))];
    events.extend(typed("needle"));
    events.push(key(KeyCode::Enter));
    let screen = run_until(left.path(), right.path(), events);

    let bar = screen.lines().next().expect("the editor bar");
    assert!(
        bar.contains("Ln 4/5"),
        "the caret must be on the matching line, bar was: {bar}"
    );
}

/// The F7 dialog is on screen with Far's controls.
#[test]
fn e2e_the_editor_search_dialog_offers_fars_controls() {
    let (left, right) = fixture(&[("file.txt", "content")]);

    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::Down), key(KeyCode::F(4)), key(KeyCode::F(7))],
    );

    assert!(screen.contains("Search for"), "{screen}");
    assert!(screen.contains("Case sensitive"), "{screen}");
    assert!(screen.contains("Whole words"), "{screen}");
    assert!(screen.contains("Find previous"), "{screen}");
    assert!(screen.contains("Find next"), "{screen}");
}

// ─── overlays ───────────────────────────────────────────────────────────────

/// F1 opens help and F10 closes it — F10 leaves whatever is in front of you.
#[test]
fn e2e_f1_opens_help_and_f10_closes_it() {
    let (left, right) = fixture(&[("a.txt", "x")]);

    let mut events = vec![key(KeyCode::F(1)), key(KeyCode::F(10))];
    events.extend(quit());
    let screen = run(left.path(), right.path(), events);

    assert!(
        !screen.contains("Far Manager style navigation"),
        "help must be closed again:\n{screen}"
    );
    assert!(
        screen.contains("1Help"),
        "and the panels are back:\n{screen}"
    );
}

/// F10 asks before quitting, so a stray press cannot end the session.
#[test]
fn e2e_f10_asks_before_quitting_and_no_returns_to_the_panels() {
    let (left, right) = fixture(&[("still-here.txt", "x")]);

    // Stop while the answer is still on screen: a sequence that went on to
    // quit would leave the final frame showing the dialog it had just
    // confirmed, and prove nothing about dismissing it.
    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::F(10)), key(KeyCode::Char('n'))],
    );

    assert!(
        screen.contains("still-here"),
        "answering No must return to the panels:\n{screen}"
    );
    assert!(
        !screen.contains("Do you want to quit"),
        "and the dialog is gone:\n{screen}"
    );
}

/// F10 followed by Yes really does end the session — which every other test
/// here depends on, so it is worth asserting once directly.
#[test]
fn e2e_f10_then_yes_ends_the_session() {
    let (left, right) = fixture(&[]);
    let screen = run_until(left.path(), right.path(), vec![key(KeyCode::F(10))]);
    assert!(
        screen.contains("Do you want to quit"),
        "F10 asks first:\n{screen}"
    );

    // `run` would panic if the loop did not return on its own.
    run(left.path(), right.path(), quit());
}

/// F3 views a file without being able to change it.
#[test]
fn e2e_f3_views_a_files_contents() {
    let (left, right) = fixture(&[("readme.txt", "VIEWABLE CONTENT")]);

    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::Down), key(KeyCode::F(3))],
    );

    assert!(screen.contains("VIEWABLE CONTENT"), "{screen}");
    assert_eq!(
        fs::read_to_string(left.path().join("readme.txt")).unwrap(),
        "VIEWABLE CONTENT",
        "viewing must not change the file"
    );
}

// ─── the command line ───────────────────────────────────────────────────────

/// A printable key opens the command line holding it, the way a shell does.
#[test]
fn e2e_typing_opens_the_command_line_and_a_command_runs() {
    let (left, right) = fixture(&[]);

    let mut events = typed("mkdir made-by-command");
    events.push(key(KeyCode::Enter));
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        left.path().join("made-by-command").is_dir(),
        "the typed command must run"
    );
}

/// Esc abandons a typed command rather than running it.
#[test]
fn e2e_escape_abandons_a_typed_command() {
    let (left, right) = fixture(&[]);

    let mut events = typed("mkdir never-made");
    events.push(key(KeyCode::Esc));
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        !left.path().join("never-made").exists(),
        "an abandoned command must not run"
    );
}

// ─── marking ────────────────────────────────────────────────────────────────

/// Insert marks the entry under the cursor and moves on, so a run of files can
/// be marked by holding one key — and F5 then copies all of them.
#[test]
fn e2e_marked_files_are_copied_together() {
    let (left, right) = fixture(&[("one.txt", "1"), ("two.txt", "2"), ("three.txt", "3")]);

    let mut events = vec![
        key(KeyCode::Down),
        key(KeyCode::Insert),
        key(KeyCode::Insert),
        key(KeyCode::F(5)),
        key(KeyCode::Enter),
    ];
    events.extend(quit());
    run(left.path(), right.path(), events);

    let copied = fs::read_dir(right.path())
        .expect("read right")
        .filter_map(Result::ok)
        .count();
    assert_eq!(copied, 2, "both marked files must be copied");
}

// ─── the mouse ──────────────────────────────────────────────────────────────

/// A click on the key bar, at the column the label was drawn in.
fn click(column: u16, row: u16) -> Event {
    click_with(column, row, KeyModifiers::empty())
}

fn click_with(column: u16, row: u16, modifiers: KeyModifiers) -> Event {
    Event::Mouse(crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column,
        row,
        modifiers,
    })
}

fn scroll_down(column: u16, row: u16) -> Event {
    Event::Mouse(crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::ScrollDown,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    })
}

/// Clicking `5Copy` must do exactly what F5 does — the click is translated into
/// the key, not into a second implementation of copying.
#[test]
fn e2e_clicking_the_key_bar_runs_the_function_key() {
    let (left, right) = fixture(&[("clicked.txt", "by mouse")]);

    // The bar is the last row; at 100 columns each of the twelve cells is 8
    // wide with the first four one wider, so F5 covers columns 36..44.
    // Clicking 5Copy opens the Copy dialog, exactly as pressing F5 does;
    // Enter then accepts the offered destination.
    let mut events = vec![
        key(KeyCode::Down),
        click(38, SCREEN_H - 1),
        key(KeyCode::Enter),
    ];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        right.path().join("clicked.txt").exists(),
        "clicking 5Copy must copy, as F5 does"
    );
}

/// Mouse modifiers are part of the gesture. Shift-clicking `4Edit` therefore
/// means Shift+F4 (new file), not bare F4 (edit the selected entry).
#[test]
fn e2e_shift_clicking_the_key_bar_preserves_modifiers() {
    let (left, right) = fixture(&[("selected.txt", "leave me alone")]);

    // F4 covers columns 27..36 at this width.
    let screen = run_until(
        left.path(),
        right.path(),
        vec![click_with(30, SCREEN_H - 1, KeyModifiers::SHIFT)],
    );

    assert!(
        screen.contains("Create New File (Shift+F4)"),
        "Shift-click must take the Shift+F4 path:\n{screen}"
    );
}

/// Every painted key cell is a real control, not decoration. F2 opens the
/// user menu, F11 opens the plugin surface and F12 lists the screens.
#[test]
fn e2e_clicking_the_previously_dead_key_cells_runs_their_painted_actions() {
    let (left, right) = fixture(&[("a.txt", "a")]);

    // F2 covers columns 9..18 at this width.
    let user_menu = run_until(left.path(), right.path(), vec![click(12, SCREEN_H - 1)]);
    assert!(
        user_menu.contains("User menu") && user_menu.contains("Edit user menu"),
        "2UserMn must open the available command menu:\n{user_menu}"
    );

    // F11 covers 84..92 and F12 covers 92..100 at this width.
    let plugin = run_until(left.path(), right.path(), vec![click(86, SCREEN_H - 1)]);
    assert!(
        plugin.contains("Plugin commands") && plugin.contains("MCP status"),
        "11Plugin must open Far's plugin commands list:\n{plugin}"
    );

    let screens = run_until(left.path(), right.path(), vec![click(96, SCREEN_H - 1)]);
    assert!(
        screens.contains("Screens") && screens.contains("Console transcript"),
        "12Screen must offer the available screens:\n{screens}"
    );
}

/// Scrolling over either panel makes that panel active and moves its cursor,
/// so the next operation acts on exactly what the pointer was over.
#[test]
fn e2e_wheel_moves_the_cursor_in_the_panel_under_the_pointer() {
    let (left, right) = fixture(&[("from-left.txt", "left")]);
    fs::write(right.path().join("from-right.txt"), "right").expect("seed right file");

    let mut events = vec![
        scroll_down(SCREEN_W - 10, 4),
        key(KeyCode::F(5)),
        key(KeyCode::Enter),
    ];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        left.path().join("from-right.txt").exists(),
        "the wheel must activate the right panel and select its file"
    );
}

/// Overlays use the same wheel gesture: in the viewer it is the Down key, so
/// the first line leaves the viewport and the next one appears at the bottom.
#[test]
fn e2e_wheel_scrolls_the_open_viewer() {
    let body = (0..30)
        .map(|line| format!("viewer-line-{line:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (left, right) = fixture(&[("long.txt", &body)]);

    let screen = run_until(
        left.path(),
        right.path(),
        vec![
            key(KeyCode::Down),
            key(KeyCode::F(3)),
            scroll_down(SCREEN_W / 2, SCREEN_H / 2),
        ],
    );

    assert!(
        !screen.contains("viewer-line-00") && screen.contains("viewer-line-20"),
        "the wheel must scroll the viewer by one line:\n{screen}"
    );
}

/// …and the same for a destructive one, which must still show its dialog: the
/// click goes through the key, so it goes through the guard too.
#[test]
fn e2e_clicking_delete_still_asks_first() {
    let (left, right) = fixture(&[("guarded.txt", "x")]);

    // F8 is the eighth cell: columns 60..68 at 100 wide.
    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::Down), click(62, SCREEN_H - 1)],
    );

    assert!(
        screen.contains("Do you wish to delete the file?"),
        "the mouse must not bypass the confirmation:\n{screen}"
    );
    assert!(
        left.path().join("guarded.txt").exists(),
        "and nothing is deleted yet"
    );
}

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

/// A modal owns the screen: the panels are still drawn behind it, but they are
/// not what the pointer is over.
#[test]
fn e2e_a_click_does_not_reach_the_panels_behind_a_dialog() {
    let (left, right) = fixture(&[("safe.txt", "x")]);

    let mut events = vec![
        key(KeyCode::F(10)),     // the quit dialog is up
        click(38, SCREEN_H - 1), // a click on "5Copy" underneath it
        key(KeyCode::Char('n')), // dismiss
    ];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        !right.path().join("safe.txt").exists(),
        "a click behind a dialog must not run the key bar"
    );
}

// ─── naming things ──────────────────────────────────────────────────────────

/// F7 asks for a name, as Far does, instead of inventing `new_folder`.
#[test]
fn e2e_f7_asks_for_the_folder_name_and_uses_it() {
    let (left, right) = fixture(&[]);

    let mut events = vec![key(KeyCode::F(7))];
    events.extend(typed("chosen-name"));
    events.push(key(KeyCode::Enter));
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        left.path().join("chosen-name").is_dir(),
        "the typed name must be the folder's name"
    );
    assert!(
        !left.path().join("new_folder").exists(),
        "and nothing is invented alongside it"
    );
}

#[test]
fn e2e_escaping_the_folder_dialog_creates_nothing() {
    let (left, right) = fixture(&[]);

    let mut events = vec![key(KeyCode::F(7))];
    events.extend(typed("abandoned"));
    events.push(key(KeyCode::Esc));
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert_eq!(
        fs::read_dir(left.path()).unwrap().count(),
        0,
        "cancelling must leave the directory empty"
    );
}

/// Shift+F6 renames where the file is, rather than moving it anywhere.
#[test]
fn e2e_shift_f6_renames_the_selected_file_in_place() {
    let (left, right) = fixture(&[("before.txt", "same bytes")]);

    let mut events = vec![
        key(KeyCode::Down),
        key_with(KeyCode::F(6), KeyModifiers::SHIFT),
    ];
    // The dialog opens on the current name, so clear it first.
    events.extend(std::iter::repeat_n(
        key(KeyCode::Backspace),
        "before.txt".len(),
    ));
    events.extend(typed("after.txt"));
    events.push(key(KeyCode::Enter));
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(left.path().join("after.txt").exists(), "renamed");
    assert!(!left.path().join("before.txt").exists(), "old name is gone");
    assert_eq!(
        fs::read_to_string(left.path().join("after.txt")).unwrap(),
        "same bytes",
        "a rename must not touch the contents"
    );
    assert!(
        !right.path().join("after.txt").exists(),
        "and must not move it to the other panel — that is F6"
    );
}

/// The dialog opens on the current name, so a small edit is a small edit.
#[test]
fn e2e_the_rename_dialog_opens_on_the_current_name() {
    let (left, right) = fixture(&[("report.txt", "x")]);

    let screen = run_until(
        left.path(),
        right.path(),
        vec![
            key(KeyCode::Down),
            key_with(KeyCode::F(6), KeyModifiers::SHIFT),
        ],
    );

    assert!(screen.contains("Rename"), "the dialog is titled:\n{screen}");
    assert!(
        screen.contains("Rename report.txt to:"),
        "and says what it is renaming:\n{screen}"
    );
}

/// Renaming onto an existing name must refuse rather than overwrite.
#[test]
fn e2e_a_rename_onto_an_existing_name_is_refused() {
    let (left, right) = fixture(&[("keep.txt", "original"), ("other.txt", "other")]);

    let mut events = vec![
        key(KeyCode::Down),
        key(KeyCode::Down), // land on other.txt (after "..", keep.txt)
        key_with(KeyCode::F(6), KeyModifiers::SHIFT),
    ];
    events.extend(std::iter::repeat_n(
        key(KeyCode::Backspace),
        "other.txt".len(),
    ));
    events.extend(typed("keep.txt"));
    events.push(key(KeyCode::Enter));
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert_eq!(
        fs::read_to_string(left.path().join("keep.txt")).unwrap(),
        "original",
        "the existing file must survive"
    );
}

// ─── directory statistics ───────────────────────────────────────────────────

/// Ctrl+L measures the active directory and says where the space is.
#[test]
fn e2e_ctrl_l_reports_what_the_directory_contains() {
    let left = TempDir::new().expect("dir");
    let right = TempDir::new().expect("dir");
    fs::create_dir_all(left.path().join("heavy")).expect("dir");
    fs::write(left.path().join("heavy/blob.bin"), vec![0u8; 50_000]).expect("file");
    fs::write(left.path().join("main.rs"), "fn main() {}").expect("file");
    fs::write(left.path().join("notes.md"), "hello").expect("file");

    let screen = run_until(
        left.path(),
        right.path(),
        vec![key_with(KeyCode::Char('l'), KeyModifiers::CONTROL)],
    );

    assert!(screen.contains("Information"), "the panel opens:\n{screen}");
    assert!(screen.contains("Largest folders"), "{screen}");
    assert!(
        screen.contains("heavy"),
        "the big folder is named:\n{screen}"
    );
    assert!(screen.contains("By content"), "{screen}");
    assert!(
        screen.contains("Source"),
        "the .rs file is categorised:\n{screen}"
    );
    assert!(screen.contains("Documents"), "and the .md file:\n{screen}");
    assert!(
        screen.contains("3 files"),
        "the totals are there:\n{screen}"
    );
}

/// …and Esc puts it away again.
#[test]
fn e2e_the_information_panel_closes() {
    let (left, right) = fixture(&[("a.txt", "x")]);

    let mut events = vec![
        key_with(KeyCode::Char('l'), KeyModifiers::CONTROL),
        key(KeyCode::Esc),
    ];
    events.extend(quit());
    let screen = run(left.path(), right.path(), events);

    assert!(
        !screen.contains("Largest folders"),
        "the panel must close:\n{screen}"
    );
}

/// Deleting a file leaves the cursor where it was, so the next file is under
/// it. Far works this way, and it is what makes deleting several files in a
/// row possible: jumping back to the top after each one means hunting for the
/// place again every time.
///
/// Asserted by deleting twice without moving. If the cursor stayed put, the
/// second delete takes the neighbour; if it sprang back to the top it lands on
/// "..", which deletes nothing at all.
#[test]
fn e2e_the_cursor_stays_put_after_a_delete() {
    let (left, right) = fixture(&[
        ("a.txt", "a"),
        ("b.txt", "b"),
        ("c.txt", "c"),
        ("d.txt", "d"),
    ]);

    // Two Downs: past "..", past "a.txt", onto "b.txt".
    let mut events = vec![key(KeyCode::Down), key(KeyCode::Down)];
    events.push(key(KeyCode::F(8)));
    events.push(key(KeyCode::Char('y')));
    events.push(key(KeyCode::F(8)));
    events.push(key(KeyCode::Char('y')));
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(!left.path().join("b.txt").exists(), "the first delete");
    assert!(
        !left.path().join("c.txt").exists(),
        "the second delete must take c.txt, which moved into b.txt's row — a \
         cursor thrown back to the top would sit on \"..\" and delete nothing"
    );
    assert!(
        left.path().join("a.txt").exists() && left.path().join("d.txt").exists(),
        "and nothing else is touched"
    );
}

// ─── F5, the Copy dialog ────────────────────────────────────────────────────

/// F5 asks before copying, and offers the other panel as the default answer.
#[test]
fn e2e_f5_opens_the_copy_dialog_showing_far_s_controls() {
    let (left, right) = fixture(&[("report.txt", "data")]);

    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::Down), key(KeyCode::F(5))],
    );

    assert!(screen.contains("Copy"), "the dialog is titled Copy");
    assert!(
        screen.contains("\"report.txt\""),
        "and names what it is about"
    );
    for control in [
        "Access rights:",
        "Default",
        "Inherit",
        "Already existing files:",
        "Ask",
        "Preserve all timestamps",
        "contents of symbolic links",
        "Process multiple destinations",
        "Use filter",
        "Cancel",
    ] {
        assert!(screen.contains(control), "the dialog must show {control:?}");
    }
}

/// Enter accepts the offered destination, which is the other panel — the same
/// thing F5 used to do outright, now as the dialog's default answer.
#[test]
fn e2e_enter_in_the_copy_dialog_copies_to_the_other_panel() {
    let (left, right) = fixture(&[("report.txt", "data")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(5)), key(KeyCode::Enter)];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(right.path().join("report.txt").exists());
}

/// Escape leaves everything alone — the dialog is a question, not a delay.
#[test]
fn e2e_escaping_the_copy_dialog_copies_nothing() {
    let (left, right) = fixture(&[("report.txt", "data")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(5)), key(KeyCode::Esc)];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(!right.path().join("report.txt").exists());
    assert!(left.path().join("report.txt").exists());
}

/// Press `f_key` on the first entry, clear the offered destination and type
/// `destination` instead, then accept the dialog and quit.
fn transfer_to_typed(left: &Path, right: &Path, f_key: u8, destination: &str) {
    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(f_key))];
    for _ in 0..right.to_string_lossy().len() + 8 {
        events.push(key(KeyCode::Backspace));
    }
    events.extend(typed(destination));
    events.push(key(KeyCode::Enter));
    events.extend(quit());
    run(left, right, events);
}

/// The point of the dialog: a destination that is neither panel. The
/// trailing separator asks for a folder, created on the way (Far: "append
/// backslash to its name").
#[test]
fn e2e_the_copy_dialog_uses_the_typed_destination() {
    let (left, right) = fixture(&[("report.txt", "data")]);
    let elsewhere = left.path().join("elsewhere");

    let folder = format!("{}{}", elsewhere.display(), std::path::MAIN_SEPARATOR);
    transfer_to_typed(left.path(), right.path(), 5, &folder);

    assert!(
        elsewhere.join("report.txt").exists(),
        "the copy lands where it was told, creating the directory"
    );
    assert!(
        !right.path().join("report.txt").exists(),
        "and not in the other panel"
    );
}

/// F6 is "Rename or move": a new name for the one file renames it, rather
/// than making a folder of that name and moving the file inside.
#[test]
fn e2e_f6_to_a_new_name_renames_the_file() {
    let (left, right) = fixture(&[("report.txt", "data")]);
    let renamed = left.path().join("final.txt");

    transfer_to_typed(left.path(), right.path(), 6, &renamed.to_string_lossy());

    assert!(renamed.is_file(), "a file named final.txt, not a folder");
    assert_eq!(fs::read_to_string(&renamed).expect("renamed"), "data");
    assert!(
        !left.path().join("report.txt").exists(),
        "the old name is gone"
    );
}

/// Tab reaches the checkboxes and space ticks them.
#[test]
fn e2e_tab_and_space_work_the_copy_dialog_s_checkboxes() {
    let (left, right) = fixture(&[("report.txt", "data")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(5))];
    // Destination → rights → existing → "Preserve all timestamps".
    events.extend([key(KeyCode::Tab), key(KeyCode::Tab), key(KeyCode::Tab)]);
    events.push(key(KeyCode::Char(' ')));
    let screen = run_until(left.path(), right.path(), events);

    assert!(
        screen.contains("[x] Preserve all timestamps"),
        "space must tick the box the cursor is on"
    );
}

/// F6 is the same dialog under a different name: Far calls moving within a
/// directory a rename, and offers one question for both.
#[test]
fn e2e_f6_opens_the_rename_move_dialog() {
    let (left, right) = fixture(&[("report.txt", "data")]);

    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::Down), key(KeyCode::F(6))],
    );

    assert!(screen.contains("Rename/Move"), "{screen}");
    assert!(screen.contains("Rename or move"));
    assert!(
        screen.contains("Rename }") || screen.contains("{ Rename"),
        "the button says what the key does"
    );
}

/// Enter moves rather than copies: the original must be gone.
#[test]
fn e2e_the_rename_move_dialog_moves() {
    let (left, right) = fixture(&[("report.txt", "data")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(6)), key(KeyCode::Enter)];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(right.path().join("report.txt").exists(), "it arrives");
    assert!(!left.path().join("report.txt").exists(), "and it leaves");
}

/// A move has one destination and no filter, so Far greys both. The cursor
/// must skip them: stopping on a control that does nothing is worse than not
/// drawing it.
#[test]
fn e2e_the_rename_move_dialog_skips_the_controls_it_greys() {
    let (left, right) = fixture(&[("report.txt", "data")]);

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(6))];
    // Destination → rights → existing → timestamps → symlinks → then the
    // greyed pair is skipped and the next Tab lands on the Rename button.
    events.extend(std::iter::repeat_n(key(KeyCode::Tab), 5));
    events.push(key(KeyCode::Char(' ')));
    let screen = run_until(left.path(), right.path(), events);

    assert!(
        !screen.contains("[x] Process multiple destinations"),
        "space must not tick a greyed box: {screen}"
    );
}

/// F10 twice must quit. It asked the question, so it answers it: closing the
/// dialog instead made the key its own undo, and a reader feeding F10 forever
/// — which is what a stuck key is — never reached the end of the session.
#[test]
fn e2e_pressing_f10_twice_quits() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    run(
        left.path(),
        right.path(),
        vec![key(KeyCode::F(10)), key(KeyCode::F(10))],
    );
}

/// …and Escape still cancels, so the dialog is a real question.
#[test]
fn e2e_escape_cancels_the_quit_dialog() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::F(10)), key(KeyCode::Esc)],
    );
    // The panel splits name and extension into columns, so the file is not
    // one string on screen — the dialog being gone is the thing to assert.
    assert!(
        !screen.contains("Do you want to quit"),
        "Escape must dismiss the dialog: {screen}"
    );
}

// ─── the mouse inside dialogs ───────────────────────────────────────────────

/// Where `needle` sits on the rendered screen, as (column, row).
///
/// Dialogs are centred and sized to their content, so their buttons have no
/// fixed address. Finding the text is how a test clicks what a user would.
fn find(screen: &str, needle: &str) -> (u16, u16) {
    // Columns, not byte offsets. The frames are box-drawing characters at
    // three bytes each, so `str::find` answers a different question than the
    // one a mouse event asks.
    let wanted: Vec<char> = needle.chars().collect();
    for (row, line) in screen.lines().enumerate() {
        let columns: Vec<char> = line.chars().collect();
        if let Some(column) = columns
            .windows(wanted.len())
            .position(|window| window == wanted.as_slice())
        {
            return (
                u16::try_from(column).expect("column fits"),
                u16::try_from(row).expect("row fits"),
            );
        }
    }
    panic!("{needle:?} is not on screen:\n{screen}");
}

fn drag(column: u16, row: u16) -> Event {
    Event::Mouse(crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Drag(crossterm::event::MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::empty(),
    })
}

/// Clicking Yes must quit, exactly as pressing Y does. The mouse was ignored
/// entirely while a dialog was up, so every button in the program was
/// keyboard-only.
#[test]
fn e2e_clicking_yes_in_the_quit_dialog_quits() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let screen = run_until(left.path(), right.path(), vec![key(KeyCode::F(10))]);
    let (column, row) = find(&screen, "Yes");

    // No trailing quit(): if the click does not quit, the loop runs out of
    // events and the test fails on that.
    run(
        left.path(),
        right.path(),
        vec![key(KeyCode::F(10)), click(column, row)],
    );
}

/// …and clicking No must not.
#[test]
fn e2e_clicking_no_in_the_quit_dialog_returns_to_the_panels() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let screen = run_until(left.path(), right.path(), vec![key(KeyCode::F(10))]);
    let (column, row) = find(&screen, "No");

    let after = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::F(10)), click(column, row)],
    );
    assert!(
        !after.contains("Do you want to quit"),
        "No dismisses the dialog: {after}"
    );
}

/// The delete dialog is the one where a misclick costs the most, so its
/// buttons are worth their own test.
#[test]
fn e2e_clicking_cancel_in_the_delete_dialog_keeps_the_file() {
    let (left, right) = fixture(&[("precious.txt", "keep")]);
    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::Down), key(KeyCode::F(8))],
    );
    let (column, row) = find(&screen, "Cancel");

    let mut events = vec![key(KeyCode::Down), key(KeyCode::F(8)), click(column, row)];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(left.path().join("precious.txt").exists());
}

/// A checkbox answers to the click that reaches it.
#[test]
fn e2e_clicking_a_checkbox_in_the_copy_dialog_ticks_it() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::Down), key(KeyCode::F(5))],
    );
    let (column, row) = find(&screen, "[ ] Preserve all timestamps");

    let after = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::Down), key(KeyCode::F(5)), click(column, row)],
    );
    assert!(
        after.contains("[x] Preserve all timestamps"),
        "the click must tick the box: {after}"
    );
}

/// A button click names the button to run; it must not accidentally execute
/// whichever control happened to have keyboard focus before the click.
#[test]
fn e2e_clicking_copy_runs_copy_even_when_cancel_had_focus() {
    let (left, right) = fixture(&[("mouse-copy.txt", "copy me")]);
    let mut open = vec![key(KeyCode::Down), key(KeyCode::F(5))];
    // Destination -> ... -> Copy -> Cancel.
    open.extend(std::iter::repeat_n(key(KeyCode::Tab), 8));

    let screen = run_until(left.path(), right.path(), open.clone());
    let (column, row) = find(&screen, "{ Copy }");

    open.push(click(column, row));
    open.extend(quit());
    run(left.path(), right.path(), open);

    assert!(
        right.path().join("mouse-copy.txt").exists(),
        "clicking Copy must copy even after Cancel held keyboard focus"
    );
}

/// Search direction buttons also name an action independent of the old focus.
/// Clicking Find previous from the query field must run backwards immediately.
#[test]
fn e2e_clicking_find_previous_runs_the_named_search() {
    let body = "first needle\nmiddle\nlast needle";
    let (left, right) = fixture(&[("search.txt", body)]);
    let mut open = vec![key(KeyCode::Down), key(KeyCode::F(4)), key(KeyCode::F(7))];
    open.extend(typed("needle"));

    let screen = run_until(left.path(), right.path(), open.clone());
    let (column, row) = find(&screen, "Find previous");
    open.push(click(column, row));

    let after = run_until(left.path(), right.path(), open);
    assert!(
        !after.contains("Search for") && after.contains("Ln 3/3"),
        "Find previous must close the dialog and select the last match:\n{after}"
    );
}

/// A press on a blank part of the dialog takes hold of it, and a drag carries
/// it — which is how you look at what a dialog is covering.
#[test]
fn e2e_a_dialog_can_be_dragged_by_the_mouse() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let before = run_until(left.path(), right.path(), vec![key(KeyCode::F(10))]);
    let (_, title_row) = find(&before, "Quit");

    // Grab the title bar and pull it up and left.
    let (grab_x, grab_y) = (SCREEN_W / 2, title_row);
    let after = run_until(
        left.path(),
        right.path(),
        vec![
            key(KeyCode::F(10)),
            click(grab_x, grab_y),
            drag(grab_x - 6, grab_y - 3),
        ],
    );

    let (_, moved_row) = find(&after, "Quit");
    assert!(
        moved_row < title_row,
        "the dialog must follow the pointer: was row {title_row}, now {moved_row}\n{after}"
    );
}

/// Clicking the panels behind a dialog does nothing. That is what modal means,
/// and it is why the click has to be recognised as being outside rather than
/// simply falling through to the panel hit-test.
#[test]
fn e2e_clicking_outside_a_dialog_does_nothing() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let after = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::F(10)), click(1, 1)],
    );
    assert!(
        after.contains("Do you want to quit"),
        "the dialog stays up: {after}"
    );
}

/// A dialog dragged aside does not leave the next one displaced: each opens
/// centred, where the user expects to find it.
#[test]
fn e2e_a_new_dialog_opens_centred_after_an_earlier_one_was_dragged() {
    let (left, right) = fixture(&[("a.txt", "a")]);
    let centred = run_until(left.path(), right.path(), vec![key(KeyCode::F(10))]);
    let (_, centred_row) = find(&centred, "Quit");

    let after = run_until(
        left.path(),
        right.path(),
        vec![
            // Drag the quit dialog away, dismiss it, then open it again.
            key(KeyCode::F(10)),
            click(SCREEN_W / 2, centred_row),
            drag(SCREEN_W / 2, centred_row - 4),
            key(KeyCode::Esc),
            key(KeyCode::F(10)),
        ],
    );

    let (_, reopened_row) = find(&after, "Quit");
    assert_eq!(reopened_row, centred_row, "reopened centred:\n{after}");
}

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

// ─── scrolling a long listing ───────────────────────────────────────────────

/// Only the visible rows are built now, so scrolling is worth driving through
/// the real event loop rather than trusting the arithmetic alone.
#[test]
fn e2e_scrolling_a_long_listing_follows_the_cursor() {
    let left = TempDir::new().expect("left dir");
    let right = TempDir::new().expect("right dir");
    for index in 0..200 {
        fs::write(left.path().join(format!("file-{index:04}.txt")), b"x").expect("seed");
    }

    // At the top: the first files are on screen and the last are not.
    let top = run_until(left.path(), right.path(), vec![key(KeyCode::Down)]);
    assert!(top.contains("file-0000"), "{top}");
    assert!(!top.contains("file-0199"), "the end is not on screen yet");

    // End jumps to the last entry, which must now be visible.
    let bottom = run_until(left.path(), right.path(), vec![key(KeyCode::End)]);
    assert!(
        bottom.contains("file-0199"),
        "End must scroll the last entry into view:\n{bottom}"
    );
    assert!(!bottom.contains("file-0000"), "and the top is gone");

    // Home brings it back.
    let again = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::End), key(KeyCode::Home)],
    );
    assert!(again.contains("file-0000"), "{again}");
}

/// Paging down and back up puts the cursor where it started.
///
/// The cursor, not the view: the offset does not come back to zero — after
/// paging up onto the second entry the listing is drawn from there, with ".."
/// scrolled off above it. That is how it behaved before the window was made
/// lazy too, so it is asserted as it is rather than quietly changed here.
#[test]
fn e2e_paging_down_and_back_returns_the_cursor() {
    let left = TempDir::new().expect("left dir");
    let right = TempDir::new().expect("right dir");
    for index in 0..200 {
        fs::write(left.path().join(format!("file-{index:04}.txt")), b"x").expect("seed");
    }

    let there_and_back = run_until(
        left.path(),
        right.path(),
        vec![
            key(KeyCode::Down),
            key(KeyCode::PageDown),
            key(KeyCode::PageDown),
            key(KeyCode::PageUp),
            key(KeyCode::PageUp),
        ],
    );

    // The panel's info row names the entry under the cursor.
    assert!(
        there_and_back.contains("file-0000.txt"),
        "two pages down and two back is the entry it began on:\n{there_and_back}"
    );
}

/// Every row a listing has must be reachable one keypress at a time, with the
/// cursor always on screen — the property the window exists to guarantee.
#[test]
fn e2e_holding_down_walks_the_whole_listing() {
    let left = TempDir::new().expect("left dir");
    let right = TempDir::new().expect("right dir");
    for index in 0..60 {
        fs::write(left.path().join(format!("file-{index:04}.txt")), b"x").expect("seed");
    }

    // Sixty Downs from "..": past the end, so it settles on the last entry.
    let events: Vec<Event> = std::iter::repeat_n(key(KeyCode::Down), 60).collect();
    let screen = run_until(left.path(), right.path(), events);
    assert!(
        screen.contains("file-0059"),
        "the last entry must be on screen:\n{screen}"
    );
}
