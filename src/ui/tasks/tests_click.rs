//! Unit tests for what a click on the task list lands on: each answer
//! agrees with what the renderer drew there.

use crossterm::event::KeyCode;
use ratatui::layout::Rect;

use super::tests::{draw, lines, sketch};
use super::{task_click_at, TaskClick};
use crate::models::{Modal, SortKey};
use crate::mouse::fkey_at;
use crate::prompt::Privilege;
use crate::test_support::task_list::{named, TaskApp};

fn screen(width: u16, height: u16) -> Rect {
    Rect::new(0, 0, width, height)
}

#[test]
fn every_key_bar_cell_is_the_key_drawn_there() {
    let mut app = sketch();
    for (width, height) in [(80, 24), (120, 40), (40, 10), (85, 20)] {
        let list = app.list();
        let bottom = height - 1;
        for x in 0..width {
            let clicked = task_click_at(screen(width, height), list, x, bottom);
            let expected = fkey_at(width, 12, x).map(TaskClick::Key);
            assert_eq!(clicked, expected, "{width}x{height} at {x}");
        }
    }
}

/// The title drawn over column `x` of row `y`, between its rules.
fn title_under(row: &str, x: usize) -> String {
    let cells: Vec<char> = row.chars().collect();
    let start = cells[..x]
        .iter()
        .rposition(|&ch| ch == '│' || ch == '║')
        .unwrap()
        + 1;
    let end = x + cells[x..]
        .iter()
        .position(|&ch| ch == '│' || ch == '║')
        .unwrap();
    cells[start..end]
        .iter()
        .collect::<String>()
        .trim()
        .trim_end_matches(['▲', '▼'])
        .to_owned()
}

/// The column title a sort key's click is made on.
const fn title_of(key: SortKey) -> &'static [&'static str] {
    match key {
        SortKey::Pid => &["PID"],
        SortKey::User => &["User"],
        SortKey::Priority => &["PRI", "NI"],
        SortKey::Virtual => &["VIRT"],
        SortKey::Resident => &["RES", "MEM%"],
        SortKey::Shared => &["SHR"],
        SortKey::State => &["S"],
        SortKey::Cpu => &["CPU%"],
        SortKey::CpuTime => &["TIME+"],
        SortKey::Started => &[],
        SortKey::Name => &["Name"],
        SortKey::Command => &["Command"],
    }
}

/// Every title click on the drawn titles row sorts by the column drawn
/// there, and a click on a rule sorts by nothing.
fn assert_titles_agree(app: &mut TaskApp, width: u16) {
    let terminal = draw(app, width, 24);
    let drawn = lines(terminal.backend().buffer());
    let row = &drawn[6];
    let list = app.list();
    for x in 1..width - 1 {
        let under = row.chars().nth(usize::from(x)).unwrap();
        let clicked = task_click_at(screen(width, 24), list, x, 6);
        if under == '│' {
            assert_eq!(clicked, None, "{x}");
            continue;
        }
        let Some(TaskClick::Title(key)) = clicked else {
            panic!("{x}: {clicked:?}");
        };
        let title = title_under(row, usize::from(x));
        assert!(
            title_of(key).contains(&title.as_str()),
            "{x}: {title} sorts by {key:?}"
        );
    }
}

#[test]
fn title_clicks_sort_by_the_column_drawn_there() {
    let mut app = sketch();
    assert_titles_agree(&mut app, 80);
    assert_titles_agree(&mut app, 120);
    app.list().pan_right(7);
    app.list().toggle_command_view();
    app.list().pan_right(3);
    assert_titles_agree(&mut app, 80);
}

#[test]
fn a_row_click_names_the_shown_row_drawn_there() {
    let processes = (1..=40).map(|raw| named(raw, "worker")).collect();
    let mut app = TaskApp::open(processes, Privilege::User);
    app.press(KeyCode::End);
    let terminal = draw(&mut app, 80, 20);
    let drawn = lines(terminal.backend().buffer());
    let list = app.list();
    // One core draws no core bars: the titles are on row 4 and the body
    // runs from row 5 to 15.
    for y in 5..16_u16 {
        let Some(TaskClick::Row(index)) = task_click_at(screen(80, 20), list, 10, y) else {
            panic!("row {y}");
        };
        let pid = list.view()[index].key().pid().to_string();
        let drawn_pid: String = drawn[usize::from(y)].chars().skip(1).take(5).collect();
        let drawn_pid = drawn_pid.trim().to_owned();
        assert_eq!(pid, drawn_pid, "row {y}");
    }
    assert_eq!(task_click_at(screen(80, 20), list, 10, 3), None, "the rule");
    assert_eq!(
        task_click_at(screen(80, 20), list, 10, 16),
        None,
        "the rule"
    );
    assert_eq!(
        task_click_at(screen(80, 20), list, 10, 17),
        None,
        "the info row"
    );
}

#[test]
fn a_click_below_the_last_process_names_no_row() {
    let mut app = TaskApp::open(vec![named(10, "sh")], Privilege::User);
    let list = app.list();
    assert_eq!(
        task_click_at(screen(80, 24), list, 10, 5),
        Some(TaskClick::Row(0))
    );
    assert_eq!(task_click_at(screen(80, 24), list, 10, 6), None);
}

#[test]
fn a_menu_line_click_names_the_line_drawn_there() {
    let mut app = sketch();
    app.press(KeyCode::F(6));
    let terminal = draw(&mut app, 80, 24);
    let drawn = lines(terminal.backend().buffer());
    let list = app.list();
    let y = drawn
        .iter()
        .position(|row| row.contains("Start time"))
        .unwrap();
    let before = drawn[y].split_once("Start").unwrap().0;
    let x = u16::try_from(before.chars().count()).unwrap();
    let y = u16::try_from(y).unwrap();
    let clicked = task_click_at(screen(80, 24), list, x, y);
    assert_eq!(clicked, Some(TaskClick::MenuLine(9)));
    assert_eq!(
        task_click_at(screen(80, 24), list, 0, 0),
        None,
        "outside the menu"
    );
}

#[test]
fn a_dialog_a_bar_or_the_help_page_answers_no_list_click() {
    for key in [KeyCode::F(8), KeyCode::F(4), KeyCode::F(1), KeyCode::F(7)] {
        let mut app = sketch();
        app.press(key);
        assert!(!matches!(app.list().modal(), Modal::Browse), "{key:?}");
        for (x, y) in [(3, 23), (10, 6), (10, 8)] {
            assert_eq!(
                task_click_at(screen(80, 24), app.list(), x, y),
                None,
                "{key:?}"
            );
        }
    }
}

#[test]
fn below_ten_columns_only_the_key_bar_answers() {
    let mut app = sketch();
    let list = app.list();
    assert_eq!(task_click_at(screen(9, 24), list, 2, 8), None);
    assert_eq!(
        task_click_at(screen(9, 24), list, 0, 23),
        Some(TaskClick::Key(1))
    );
}
