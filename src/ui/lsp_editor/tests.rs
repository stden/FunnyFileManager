//! Purpose: Pin the Language servers dialog on screen: the rows and columns
//! at 80×24, the colours §6 of the design gives them, and small screens.

use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{buffer::Buffer, style::Modifier, Terminal};

use super::super::palette;
use crate::config::document::FILE_NAME;
use crate::models::{settings::Settings, App};
use crate::test_support::lsp_machine::{editor_mut, press, press_with, select, LspMachine};
use crate::test_support::{buffer_string, framed_row, last_component};
use crate::WritableTestBackend;

pub(super) fn draw(app: &mut App, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(WritableTestBackend::new(width, height)).expect("terminal");
    terminal.draw(|f| crate::ui::ui(f, app)).expect("draw");
    terminal.backend().buffer().clone()
}

/// The cell where `needle` first starts on screen.
pub(super) fn find(buffer: &Buffer, needle: &str) -> (u16, u16) {
    let area = buffer.area;
    for y in 0..area.height {
        let row: Vec<&str> = (0..area.width).map(|x| buffer[(x, y)].symbol()).collect();
        let joined: String = row.concat();
        if let Some(byte) = joined.find(needle) {
            let column = joined.get(..byte).expect("a char boundary").chars().count();
            return (u16::try_from(column).expect("column"), y);
        }
    }
    panic!("{needle:?} is not on screen:\n{}", buffer_string(buffer));
}

/// The cell where `needle` first starts on a row below `y`.
pub(super) fn find_below(buffer: &Buffer, needle: &str, y: u16) -> Option<(u16, u16)> {
    let area = buffer.area;
    (y + 1..area.height).find_map(|row| {
        let joined: String = (0..area.width).map(|x| buffer[(x, row)].symbol()).collect();
        let byte = joined.find(needle)?;
        Some((
            u16::try_from(joined.get(..byte)?.chars().count()).ok()?,
            row,
        ))
    })
}

pub(super) fn fg(buffer: &Buffer, needle: &str) -> ratatui::style::Color {
    buffer[find(buffer, needle)].fg
}

/// A changed python command (not installed, so its recipe runs instead), a
/// disabled rust, and an added `aaa` that also claims `.ts`.
fn machine_with_changes() -> (LspMachine, App) {
    let machine = LspMachine::new();
    machine.write_settings(
        "lsp:\n  python: {command: custom}\n  rust: {enabled: false}\n  \
         aaa: {command: aaa-server, extensions: [ts], install: null}\n",
    );
    let app = machine.opened();
    (machine, app)
}

#[test]
fn the_list_screen_has_its_rows_labels_legend_and_key_bar() {
    let (machine, mut app) = machine_with_changes();
    select(&mut app, "python");
    let screen = buffer_string(&draw(&mut app, 80, 24));
    assert!(screen.contains(" Language servers "), "{screen}");
    assert!(
        quotes_path(&framed_row(&screen, "Settings: "), &machine.settings()),
        "{screen}"
    );
    for label in ["Command:", "Arguments:", "Status:", "Auto-install:"] {
        assert!(screen.contains(label), "{label}\n{screen}");
    }
    assert!(screen.contains("▸ python"), "{screen}");
    assert!(screen.contains("recipe"), "{screen}");
    assert!(screen.contains("Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close"));
    assert!(screen.contains("2Save"), "{screen}");
    assert!(screen.contains("8Reset"), "{screen}");
    assert!(screen.contains("No changes"), "{screen}");
}

#[test]
fn an_added_server_offers_remove_and_a_change_is_marked() {
    let (_machine, mut app) = machine_with_changes();
    select(&mut app, "aaa");
    press(&mut app, KeyCode::Char(' '));
    let buffer = draw(&mut app, 80, 24);
    let screen = buffer_string(&buffer);
    assert!(screen.contains("Del Remove"), "{screen}");
    assert!(screen.contains("8Remove"), "{screen}");
    assert!(
        screen.contains("Modified: aaa. F2 saves only what changed."),
        "{screen}"
    );
    let (x, y) = find(&buffer, "▸ aaa");
    assert_eq!(buffer[(x + 19, y)].symbol(), "*", "{screen}");
}

/// `aaa` and typescript both claim `.ts` in the file: `!` in amber. A
/// conflict the draft makes is red, and so is the claimed extension.
#[test]
fn a_conflict_is_marked_by_whether_it_blocks() {
    let (_machine, mut app) = machine_with_changes();
    select(&mut app, "json");
    let buffer = draw(&mut app, 80, 24);
    let (x, y) = find(&buffer, "aaa ");
    assert_eq!(buffer[(x + 18, y)].symbol(), "!");
    assert_eq!(buffer[(x + 18, y)].fg, palette::STATUS_PENDING);

    select(&mut app, "css");
    press(&mut app, KeyCode::F(4));
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::End);
    for ch in " ts".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    press(&mut app, KeyCode::Enter);
    select(&mut app, "python");
    let buffer = draw(&mut app, 80, 24);
    let (x, y) = find(&buffer, "css ");
    assert_eq!(
        buffer[(x + 18, y)].fg,
        palette::STATUS_FAILED,
        "the draft made it"
    );

    select(&mut app, "aaa");
    let buffer = draw(&mut app, 80, 24);
    let (x, y) = find(&buffer, "[enabled]  .ts");
    assert_eq!(buffer[(x + 11, y)].fg, palette::STATUS_FAILED);
}

#[test]
fn the_colours_follow_what_each_cell_says() {
    let (_machine, mut app) = machine_with_changes();
    select(&mut app, "python");
    let buffer = draw(&mut app, 80, 24);
    let heading = find(&buffer, "python  [enabled]");
    assert_eq!(buffer[heading].fg, palette::CODE_KEYWORD);
    assert!(buffer[heading].modifier.contains(Modifier::BOLD));
    assert_eq!(fg(&buffer, "[enabled]"), palette::STATUS_OK);
    assert_eq!(fg(&buffer, "command not found"), palette::STATUS_PENDING);
    assert_eq!(fg(&buffer, "python-lsp-server"), palette::CODE_STRING);
    assert_eq!(fg(&buffer, "(none)"), palette::DISABLED_FG);
    assert_eq!(fg(&buffer, "java "), palette::CODE_KEYWORD);
    assert_eq!(fg(&buffer, "auto"), palette::STATUS_PENDING);
    let (row_x, row_y) = find(&buffer, "▸ python");
    for column in row_x..row_x + 76 {
        assert_eq!(
            buffer[(column, row_y)].bg,
            palette::CURSOR_BG,
            "column {column}"
        );
        assert_eq!(
            buffer[(column, row_y)].fg,
            palette::CURSOR_FG,
            "column {column}"
        );
    }
    select(&mut app, "shellscript");
    let buffer = draw(&mut app, 80, 24);
    assert_eq!(fg(&buffer, "missing"), palette::STATUS_FAILED);
}

#[test]
fn a_disabled_server_is_dimmed_in_the_list_and_its_detail() {
    let (_machine, mut app) = machine_with_changes();
    select(&mut app, "shellscript");
    let buffer = draw(&mut app, 80, 24);
    assert_eq!(fg(&buffer, "[ ] rust"), palette::DISABLED_FG);
    let (x, y) = find(&buffer, "[ ] rust");
    assert_eq!(
        buffer[(x + 4, y)].fg,
        palette::DISABLED_FG,
        "its command too"
    );
    select(&mut app, "rust");
    let buffer = draw(&mut app, 80, 24);
    assert_eq!(fg(&buffer, "[disabled]"), palette::DISABLED_FG);
    assert_eq!(fg(&buffer, "off"), palette::DISABLED_FG);
}

#[test]
fn f3_shows_where_the_view_is() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::F(3));
    let screen = buffer_string(&draw(&mut app, 80, 24));
    assert!(screen.contains(" Language servers: all "), "{screen}");
    assert!(screen.contains("Availability is what FFM would start; it is not a process list."));
    assert!(screen.contains("Lines 1-18 of "), "{screen}");
    assert!(screen.contains("3List"), "{screen}");
    assert!(screen.contains("10Back"), "{screen}");
    press(&mut app, KeyCode::End);
    let screen = buffer_string(&draw(&mut app, 80, 24));
    assert!(screen.contains("yaml  [enabled]"), "{screen}");
}

/// A read-only dialog names the file and the reason, and its key bar drops
/// the keys that would change the table.
#[test]
fn a_read_only_dialog_says_why_on_its_first_row() {
    let machine = LspMachine::new();
    machine.write_settings("{broken");
    let mut app = machine.opened();
    let buffer = draw(&mut app, 130, 24);
    let screen = buffer_string(&buffer);
    assert!(
        screen.contains("— read only: settings.yaml does not parse: "),
        "{screen}"
    );
    assert_eq!(fg(&buffer, "— read only"), palette::STATUS_FAILED);
    assert!(!screen.contains("2Save"), "{screen}");
    assert!(screen.contains("F3 View  F5 Check  Esc Close"), "{screen}");
    assert!(
        screen.contains(
            "Read only: settings.yaml does not parse: did not find expected ',' or '}' at \
             line 2 column 1, while parsing a flow mapping."
        ),
        "the message row has room for the whole reason\n{screen}"
    );
}

/// Whether `row` is its `Settings: ` caption and then `path`: whole, or cut
/// from its head to a tail that still ends in the file's name.
fn quotes_path(row: &str, path: &Path) -> bool {
    let path = path.display().to_string();
    let file = last_component(FILE_NAME);
    row.strip_prefix("Settings: ").is_some_and(|shown| {
        shown == path
            || shown
                .strip_prefix('…')
                .is_some_and(|tail| path.ends_with(tail) && tail.ends_with(&file))
    })
}

/// A directory name long enough that no 80-column row holds a path through
/// it whole, whatever the temp directory's own length.
const DEEP: &str = "a-settings-directory-long-enough-that-no-row-holds-the-whole-path";

/// A settings file in a directory deeper than the row is wide.
fn deep_settings(machine: &LspMachine) -> PathBuf {
    let directory = machine.home.path().join(DEEP);
    std::fs::create_dir(&directory).expect("directory");
    directory.join(FILE_NAME)
}

/// The dialog open on the settings file `file`.
fn opened_on(machine: &LspMachine, file: &Path) -> App {
    let mut app = machine.app();
    app.settings = Settings::with_file(file.to_owned());
    crate::ops::lsp_editor::open(&mut app);
    app
}

/// The Settings row keeps the file's name however deep it sits: the path
/// gives up its head. On the Windows runner the temp directory alone was
/// longer than the row, and the name was what fell off its end.
#[test]
fn a_deep_settings_file_keeps_its_name() {
    let machine = LspMachine::new();
    let file = deep_settings(&machine);
    let mut app = opened_on(&machine, &file);
    let screen = buffer_string(&draw(&mut app, 80, 24));
    let row = framed_row(&screen, "Settings: ");
    assert!(row.starts_with("Settings: …"), "{row:?}");
    assert!(quotes_path(&row, &file), "{row:?}");
}

/// A read-only file deep in the tree keeps its name and the start of the
/// reason: the path is cut before the reason is.
#[test]
fn a_deep_read_only_file_keeps_its_name_and_the_reason() {
    let machine = LspMachine::new();
    let file = deep_settings(&machine);
    std::fs::write(&file, "{broken").expect("settings");
    let mut app = opened_on(&machine, &file);
    let screen = buffer_string(&draw(&mut app, 80, 24));
    let row = framed_row(&screen, "Settings: ");
    let expected = format!(
        "Settings: …{} — read only: {FILE_NAME} does not parse",
        last_component(FILE_NAME)
    );
    assert!(row.starts_with(&expected), "{row:?}");
}

/// A session without a settings file says so where the path would be.
#[test]
fn a_session_without_a_settings_file_says_so_on_its_first_row() {
    let machine = LspMachine::new();
    let mut app = machine.app();
    app.settings.persistence = crate::history::Persistence::SessionOnly;
    crate::ops::lsp_editor::open(&mut app);
    let buffer = draw(&mut app, 80, 24);
    let screen = buffer_string(&buffer);
    assert_eq!(
        framed_row(&screen, "Settings: "),
        "Settings: none for this session — read only"
    );
    assert_eq!(fg(&buffer, "none for this session"), palette::STATUS_FAILED);
}

#[test]
fn small_screens_draw_without_panicking_and_drop_the_detail() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    let screen = buffer_string(&draw(&mut app, 60, 16));
    assert!(screen.contains("Command:"), "{screen}");
    let screen = buffer_string(&draw(&mut app, 40, 10));
    assert!(!screen.contains("Command:"), "{screen}");
    assert!(screen.contains("F3 shows details"), "{screen}");
    for (width, height) in [(10, 3), (4, 3), (1, 1), (0, 0)] {
        drop(draw(&mut app, width, height));
    }
    press(&mut app, KeyCode::F(3));
    for (width, height) in [(40, 10), (10, 4), (1, 1)] {
        drop(draw(&mut app, width, height));
    }
}

#[path = "tests_form.rs"]
mod form;

#[path = "tests_capture.rs"]
mod capture;

/// A notice lasts one key: the next edit shows what is unsaved again, so a
/// "Saved to …" never stands over changes made after it.
#[test]
fn a_notice_gives_way_to_the_changes_on_the_next_key() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::F(5));
    assert!(buffer_string(&draw(&mut app, 100, 30)).contains("Checked every server again."));
    press(&mut app, KeyCode::Char(' '));
    let screen = buffer_string(&draw(&mut app, 100, 30));
    assert!(
        screen.contains("Modified: bat. F2 saves only what changed."),
        "{screen}"
    );
    press(&mut app, KeyCode::F(2));
    assert!(buffer_string(&draw(&mut app, 100, 30)).contains("Saved to "));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char(' '));
    let screen = buffer_string(&draw(&mut app, 100, 30));
    assert!(!screen.contains("Saved to "), "{screen}");
    assert!(
        screen.contains("Modified: css. F2 saves only what changed."),
        "{screen}"
    );
}

/// The key bar, the bottom row, has no label: no F-key acts.
fn assert_blank_key_bar(buffer: &Buffer) {
    let bar = buffer_string(buffer)
        .lines()
        .last()
        .unwrap_or_default()
        .to_owned();
    for label in ["Save", "View", "Edit", "Check", "Add", "Reset", "Quit"] {
        assert!(!bar.contains(label), "{label}: {bar}");
    }
}

/// Each mode names its own keys: no F-key acts in the form or the exit
/// question, so their key bar is blank, and the legend says what does act.
#[test]
fn the_form_and_the_exit_question_name_their_own_keys() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::Enter);
    let buffer = draw(&mut app, 100, 30);
    assert!(buffer_string(&buffer).contains("Enter OK  Esc Cancel  Tab Next  Ctrl+Space List"));
    assert_blank_key_bar(&buffer);
    // After a space in Extensions every other extension is offered.
    for code in [KeyCode::Tab, KeyCode::Tab, KeyCode::End, KeyCode::Char(' ')] {
        press(&mut app, code);
    }
    press_with(&mut app, KeyCode::Char(' '), KeyModifiers::CONTROL);
    let screen = buffer_string(&draw(&mut app, 100, 30));
    assert!(
        screen.contains("Up Down Choose  Enter Insert  Esc Close list"),
        "{screen}"
    );
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Esc);
    let buffer = draw(&mut app, 100, 30);
    assert!(
        buffer_string(&buffer).contains("Y Yes  N No  Esc Cancel"),
        "{}",
        buffer_string(&buffer)
    );
    assert_blank_key_bar(&buffer);
}

#[path = "tests_layout.rs"]
mod layout;
