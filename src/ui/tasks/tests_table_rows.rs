//! Unit tests for the task list's table rows: the cursor and mark colours,
//! the state and kernel-thread colours, the tree guides and the sideways
//! scroll of the last column.

use ratatui::style::Color;
use ratatui::text::Line;

use super::table::{Columns, RowBar};
use super::tests::{draw, fg_of, kthreadd, lines, sketch};
use super::tests_table::list_of;
use crate::models::{ProcessSample, ProcessState};
use crate::test_support::processes::process;
use crate::test_support::task_list::{pid, TaskApp};
use crate::ui::palette;
use crate::ui::Cells;

/// The foreground of the cell at `x` of row `y`, and its background.
fn colours(app: &mut TaskApp, x: u16, y: u16) -> (Color, Color) {
    let terminal = draw(app, 80, 24);
    let cell = &terminal.backend().buffer()[(x, y)];
    (cell.fg, cell.bg)
}

#[test]
fn the_cursor_row_is_lit_and_a_marked_row_is_yellow_throughout() {
    let mut app = sketch();
    // Row 7 is 4242 under the cursor; 8 is 812, marked.
    assert_eq!(
        colours(&mut app, 3, 7),
        (palette::CURSOR_FG, palette::CURSOR_BG)
    );
    assert_eq!(colours(&mut app, 70, 7).1, palette::CURSOR_BG);
    assert_eq!(colours(&mut app, 3, 8).0, palette::MARKED);
    assert_eq!(colours(&mut app, 70, 8).0, palette::MARKED);
    // 1, unmarked: data white, the command in the file colour.
    assert_eq!(colours(&mut app, 3, 9).0, palette::DATA);
    assert_eq!(colours(&mut app, 70, 9).0, palette::FILE);
    app.list().select_index(1, 13);
    assert_eq!(
        colours(&mut app, 3, 8),
        (palette::MARKED, palette::CURSOR_BG)
    );
}

#[test]
fn stuck_and_dead_processes_have_their_state_letter_in_the_alert_colour() {
    let stuck = ProcessSample {
        state: ProcessState::DiskSleep,
        ..process(pid(20), "dd")
    };
    let dead = ProcessSample {
        state: ProcessState::Zombie,
        ..process(pid(30), "defunct")
    };
    let quiet = process(pid(40), "cat");
    let mut app = list_of(vec![process(pid(10), "sh"), stuck, dead, quiet]);
    let terminal = draw(&mut app, 80, 24);
    let buffer = terminal.backend().buffer();
    // One core draws no core bars: the table's rows start on row 5, and
    // the first is under the cursor.
    let state_x = 38;
    let found: Vec<(char, Color)> = (6..9)
        .map(|y| {
            let row = &lines(buffer)[usize::from(y)];
            let letter = row.chars().nth(usize::from(state_x)).unwrap();
            (letter, buffer[(state_x, y)].fg)
        })
        .collect();
    assert_eq!(
        found,
        [
            ('D', palette::STATE_ALERT),
            ('Z', palette::STATE_ALERT),
            ('S', palette::DATA)
        ]
    );
}

#[test]
fn kernel_threads_shown_by_k_are_dimmed() {
    let mut app = list_of(vec![process(pid(10), "sh"), kthreadd()]);
    app.list().toggle_kernel_threads();
    let terminal = draw(&mut app, 80, 24);
    let buffer = terminal.backend().buffer();
    let y = (0..24)
        .find(|&y| lines(buffer)[usize::from(y)].contains("[kthreadd]"))
        .unwrap();
    assert_eq!(fg_of(buffer, y, "[kthreadd]"), palette::HIDDEN);
}

/// The last `width` cells of `row`, the Command column, without the blanks
/// that pad it. Split by width, not at `│`, which a tree guide draws too.
fn command_cell(row: &Line<'_>, width: usize) -> String {
    let text: Vec<char> = row.to_string().chars().collect();
    let start = text.len() - width;
    text[start..]
        .iter()
        .collect::<String>()
        .trim_end()
        .to_owned()
}

fn child(raw: u32, parent: u32, name: &str) -> ProcessSample {
    ProcessSample {
        parent: Some(pid(parent)),
        ..process(pid(raw), name)
    }
}

#[test]
fn the_tree_draws_guides_before_each_command() {
    let mut app = list_of(vec![
        process(pid(1), "init"),
        child(10, 1, "bash"),
        child(20, 10, "vim"),
        child(11, 1, "sshd"),
    ]);
    app.list().toggle_tree();
    let list = app.list();
    let columns = Columns::fit(Cells::new(118), 4);
    let commands: Vec<String> = list
        .view()
        .iter()
        .map(|&shown| command_cell(&columns.row(list, shown, RowBar::Plain), 50))
        .collect();
    assert_eq!(
        commands,
        [
            "/usr/bin/init",
            "├─ /usr/bin/bash",
            "│  └─ /usr/bin/vim",
            "└─ /usr/bin/sshd"
        ]
    );
}

#[test]
fn a_chain_deeper_than_the_column_draws_only_the_guides_that_show() {
    let mut chain = vec![process(pid(1), "init")];
    chain.extend((2..=3_000).map(|raw| child(raw, raw - 1, "sh")));
    let mut app = list_of(chain);
    app.list().toggle_tree();
    let list = app.list();
    let columns = Columns::fit(Cells::new(78), 4);
    let deepest = *list.view().last().unwrap();
    let command = command_cell(&columns.row(list, deepest, RowBar::Plain), 17);
    assert_eq!(command, "                }");
}

#[test]
fn left_and_right_scroll_the_command_column_alone() {
    let mut app = sketch();
    app.list().pan_right(5);
    let terminal = draw(&mut app, 80, 24);
    let row = lines(terminal.backend().buffer())[9].clone();
    assert!(row.ends_with("│/init splash     ║"), "{row}");
    assert!(row.starts_with("║    1│root"), "{row}");
    app.list().pan_right(1_000);
    let terminal = draw(&mut app, 80, 24);
    let row = lines(terminal.backend().buffer())[9].clone();
    assert!(row.ends_with("│                 ║"), "{row}");
}
