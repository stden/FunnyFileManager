//! Unit tests for the task list's table: which columns fit at every width,
//! the titles, the rows' text and colours, the tree guides and the sideways
//! scroll of the last column.

use ratatui::text::Line;

use super::table::{Columns, RowBar};
use super::tests::{draw, fg_of, lines, sketch};
use crate::models::{Pid, ProcessSample, SortKey, TaskList};
use crate::prompt::Privilege;
use crate::test_support::processes::process;
use crate::test_support::task_list::{pid, TaskApp};
use crate::ui::palette;
use crate::ui::Cells;

/// Every title, left to right, as drawn.
fn titles(line: &Line<'_>) -> Vec<String> {
    line.to_string()
        .split('│')
        .map(|title| title.trim().trim_end_matches(['▲', '▼']).to_owned())
        .collect()
}

const ALL: [&str; 12] = [
    "PID", "User", "PRI", "NI", "VIRT", "RES", "SHR", "S", "CPU%", "MEM%", "TIME+", "Command",
];
/// The columns in the order they give way.
const DROP_ORDER: [&str; 9] = [
    "SHR", "PRI", "VIRT", "MEM%", "TIME+", "NI", "User", "S", "RES",
];

pub(super) fn list_of(processes: Vec<ProcessSample>) -> TaskApp {
    TaskApp::open(processes, Privilege::User)
}

/// The columns a table shows after dropping the first `dropped` in order.
fn after_dropping(dropped: usize) -> Vec<String> {
    let gone = &DROP_ORDER[..dropped];
    ALL.iter()
        .filter(|title| !gone.contains(title))
        .map(|title| (*title).to_owned())
        .collect()
}

/// Each column's width with a five-cell PID, in [`ALL`]'s order, Command
/// last with none of its own.
const WIDTHS: [usize; 12] = [5, 8, 3, 3, 6, 6, 6, 1, 5, 5, 9, 0];

/// What Command keeps of `width` once the first `dropped` columns are gone:
/// the rest, less a separator between every two columns.
fn command_room(width: usize, dropped: usize) -> usize {
    let gone = &DROP_ORDER[..dropped];
    let kept: Vec<usize> = ALL
        .iter()
        .zip(WIDTHS)
        .filter(|(title, _)| !gone.contains(title))
        .map(|(_, cells)| cells)
        .collect();
    let used = kept.iter().sum::<usize>() + kept.len() - 1;
    width.saturating_sub(used)
}

#[test]
fn columns_give_way_in_order_until_command_keeps_sixteen_cells() {
    let mut app = list_of(vec![process(pid(4242), "firefox")]);
    let list = app.list();
    let shown = list.view()[0];
    for width in 20..=200 {
        let dropped = (0..DROP_ORDER.len())
            .find(|&dropped| command_room(width, dropped) >= 16)
            .unwrap_or(DROP_ORDER.len());
        let columns = Columns::fit(Cells::new(width), 4);
        let title_line = columns.titles(list);
        assert_eq!(titles(&title_line), after_dropping(dropped), "{width}");
        assert_eq!(Cells::of(&title_line.to_string()), Cells::new(width));
        let row = columns.row(list, shown, RowBar::Plain).to_string();
        assert_eq!(Cells::of(&row), Cells::new(width), "{width}: {row:?}");
    }
}

#[test]
fn at_80_columns_only_shr_goes_and_at_120_every_column_shows() {
    let app_list = TaskList::default();
    let seventy_eight = Columns::fit(Cells::new(78), 4);
    assert_eq!(titles(&seventy_eight.titles(&app_list)), after_dropping(1));
    let seven_digits = Columns::fit(Cells::new(78), 7);
    assert_eq!(titles(&seven_digits.titles(&app_list)), after_dropping(2));
    let wide = Columns::fit(Cells::new(118), 7);
    assert_eq!(titles(&wide.titles(&app_list)), after_dropping(0));
}

#[test]
fn the_pid_column_widens_for_the_largest_pid_in_the_table() {
    let big = Pid::new(1_234_567).unwrap();
    let mut app = list_of(vec![process(pid(1), "init"), process(big, "late")]);
    let columns = Columns::for_list(app.list(), Cells::new(118));
    let title_line = columns.titles(app.list()).to_string();
    assert!(title_line.starts_with("  PID  │"), "{title_line:?}");
}

#[test]
fn unprintable_and_wide_names_keep_the_row_aligned() {
    let hostile = ProcessSample {
        command: "漢字\u{1b}[31m\u{202e}漢字漢字漢字漢字漢字漢字".to_owned(),
        ..process(pid(66), "evil")
    };
    let mut app = list_of(vec![hostile]);
    let list = app.list();
    let columns = Columns::fit(Cells::new(78), 4);
    let row = columns.row(list, list.view()[0], RowBar::Plain).to_string();
    assert_eq!(Cells::of(&row), Cells::new(78), "{row:?}");
    assert!(row.contains("漢字\u{FFFD}[31m\u{FFFD}"), "{row:?}");
    assert!(row.ends_with('}'), "{row:?}");
}

#[test]
fn the_sort_title_is_lit_with_its_arrow_where_the_arrow_fits() {
    let mut app = sketch();
    let terminal = draw(&mut app, 80, 24);
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(40, 6)].bg, palette::CURSOR_BG);
    assert_eq!(fg_of(buffer, 6, "CPU%▼"), palette::CURSOR_FG);
    app.list().invert_order();
    let terminal = draw(&mut app, 80, 24);
    assert!(lines(terminal.backend().buffer())[6].contains("CPU%▲"));
    app.list().choose_sort(SortKey::State);
    let terminal = draw(&mut app, 80, 24);
    let buffer = terminal.backend().buffer();
    let row = &lines(buffer)[6];
    assert!(
        row.contains("│S│"),
        "the arrow does not fit one cell: {row}"
    );
    assert_eq!(buffer[(38, 6)].bg, palette::CURSOR_BG);
}

#[test]
fn sorting_by_resident_memory_lights_res_and_mem_percent_alike() {
    let mut app = sketch();
    app.list().choose_sort(SortKey::Resident);
    let terminal = draw(&mut app, 80, 24);
    let row = lines(terminal.backend().buffer())[6].clone();
    assert!(row.contains("│ RES▼ │"), "{row}");
    assert!(row.contains("│MEM%▼│"), "{row}");
}
