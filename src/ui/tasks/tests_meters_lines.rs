//! Unit tests for the task list's header rows past the summary bars: the
//! grouped per-core bars and the Tasks/Load/Uptime line losing fields from
//! the right.

use std::num::NonZeroUsize;
use std::time::Duration;

use super::meters::{cores, tasks};
use super::tests::{draw, lines, sketch};
use super::tests_meters::{meters_of, parts};
use crate::models::{Load, LoadAverage};
use crate::ui::Cells;

#[test]
fn grouped_cores_are_labelled_by_their_range_and_padded_to_one_width() {
    let meters = meters_of(8, parts(1, 0, 0, 1, 0), |_| {});
    let two = NonZeroUsize::new(2).unwrap();
    let rows: Vec<String> = cores(&meters, two, 1, Cells::new(78))
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(rows.len(), 1);
    let labels: Vec<&str> = rows[0]
        .split(']')
        .filter_map(|bar| bar.split('[').next())
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .collect();
    assert_eq!(labels, ["0-1", "2-3", "4-5", "6-7"]);
    let one = NonZeroUsize::new(1).unwrap();
    let singles = cores(&meters, one, 2, Cells::new(78));
    assert!(singles[1].to_string().starts_with(" 4["), "{}", singles[1]);
}

#[test]
fn the_tasks_line_drops_the_fields_that_do_not_fit_from_the_right() {
    let meters = meters_of(1, parts(1, 0, 0, 1, 0), |system| {
        system.running = Some(3);
        system.load = Some(LoadAverage::new(
            Load::from_hundredths(52),
            Load::from_hundredths(58),
            Load::from_hundredths(59),
        ));
        system.uptime = Some(Duration::from_secs(21_126));
    });
    let everything =
        "Tasks: 0, 0 thr, 0 kthr; 3 running  Load average: 0.52 0.58 0.59  Uptime: 05:52:06";
    let width = Cells::of(everything);
    assert_eq!(tasks(&meters, width).to_string(), everything);
    let no_uptime = "Tasks: 0, 0 thr, 0 kthr; 3 running  Load average: 0.52 0.58 0.59";
    let short = width.saturating_sub(Cells::new(1));
    assert_eq!(tasks(&meters, short).to_string(), no_uptime);
    assert_eq!(tasks(&meters, Cells::new(10)).to_string(), "");
}

#[test]
fn at_120_columns_the_uptime_shows_and_every_column_fits() {
    let mut app = sketch();
    let terminal = draw(&mut app, 120, 40);
    let drawn = lines(terminal.backend().buffer());
    assert!(drawn[4].contains("Uptime: 05:52:06"), "{}", drawn[4]);
    assert!(drawn[6].contains("│ SHR  │"), "{}", drawn[6]);
    assert!(
        drawn[6].contains("│                     Command                      ║"),
        "{}",
        drawn[6]
    );
}
