//! Unit tests for the task list's whole screen: the design's sketch to the
//! character, a small terminal, and a list with no table yet.

use super::tests::{draw, lines, sketch};
use crate::models::TaskList;
use crate::prompt::Privilege;
use crate::test_support::task_list::TaskApp;

/// The design's sketch (section 2.1). Its counts are illustrative, so the
/// Tasks line, MEM% and the footer carry this fixture's own; the info row
/// uses the full interior width.
const SKETCH: [&str; 24] = [
    "╔═════════════════════════════════ Task list ══════════════════════════════════╗",
    "║CPU[|||||||         34.2%] Mem[||||||  38.2 G/125 G] Swp[             0/2.0 G]║",
    "║ 0[||        12.0%]  1[|          8.1%]  2[||||||    40.3%]  3[          3.0%]║",
    "║ 4[|          9.5%]  5[           2.2%]  6[|||       17.9%]  7[          1.4%]║",
    "║Tasks: 4, 98 thr, 1 kthr; 3 running  Load average: 0.52 0.58 0.59             ║",
    "╟─────┬────────┬───┬───┬──────┬──────┬─┬─────┬─────┬─────────┬─────────────────╢",
    "║ PID │  User  │PRI│NI │ VIRT │ RES  │S│CPU%▼│MEM% │  TIME+  │     Command     ║",
    "║ 4242│bob     │ 20│  0│ 3.1 G│ 612 M│R│ 38.4│  0.5│ 12:04.55│/usr/lib/firefox}║",
    "║  812│bob     │ 20│  0│ 1.2 G│ 301 M│S│  6.0│  0.2│  3:11.02│/usr/bin/gnome-s}║",
    "║    1│root    │ 20│  0│22.4 M│14.5 M│S│  0.0│  0.0│  1:32.07│/sbin/init splash║",
    "║ 2001│bob     │ 25│  5│ 7.4 M│ 4.0 M│S│  0.0│  0.0│  0:00.00│sleep 30         ║",
    "║     │        │   │   │      │      │ │     │     │         │                 ║",
    "║     │        │   │   │      │      │ │     │     │         │                 ║",
    "║     │        │   │   │      │      │ │     │     │         │                 ║",
    "║     │        │   │   │      │      │ │     │     │         │                 ║",
    "║     │        │   │   │      │      │ │     │     │         │                 ║",
    "║     │        │   │   │      │      │ │     │     │         │                 ║",
    "║     │        │   │   │      │      │ │     │     │         │                 ║",
    "║     │        │   │   │      │      │ │     │     │         │                 ║",
    "║     │        │   │   │      │      │ │     │     │         │                 ║",
    "╟─────┴────────┴───┴───┴──────┴──────┴─┴─────┴─────┴─────────┴─────────────────╢",
    "║4242 firefox: /usr/lib/firefox/firefox -contentproc -childID 12 -isForBrowser}║",
    "╚═════════════════════════ 4 of 5 processes, 2 marked ═════════════════════════╝",
    "1Help  2Name  3Info  4Filter5Tree  6Sort  7Nice  8Kill  9Sign…10Quit11Hold12Net ",
];

#[test]
fn the_sketch_is_drawn_to_the_character() {
    let mut opened = sketch();
    let terminal = draw(&mut opened, 80, 24);
    assert_eq!(lines(terminal.backend().buffer()), SKETCH);
}

#[test]
fn a_40_by_10_screen_drops_the_meters_and_the_narrow_columns() {
    let mut opened = sketch();
    let terminal = draw(&mut opened, 40, 10);
    assert_eq!(
        lines(terminal.backend().buffer()),
        [
            "╔═════════════ Task list ══════════════╗",
            "║ PID │ RES  │S│CPU%▼│     Command     ║",
            "║ 4242│ 612 M│R│ 38.4│/usr/lib/firefox}║",
            "║  812│ 301 M│S│  6.0│/usr/bin/gnome-s}║",
            "║    1│14.5 M│S│  0.0│/sbin/init splash║",
            "║ 2001│ 4.0 M│S│  0.0│sleep 30         ║",
            "╟─────┴──────┴─┴─────┴─────────────────╢",
            "║4242 firefox: /usr/lib/firefox/firefo}║",
            "╚═════ 4 of 5 processes, 2 marked ═════╝",
            "1He…2Na…3In…4Fi…5T…6S…7N…8K…9S…10…11…12…",
        ]
    );
}

#[test]
fn without_a_table_the_body_says_why() {
    let mut opened = TaskApp::open(Vec::new(), Privilege::User);
    *opened.list() = TaskList::default();
    let terminal = draw(&mut opened, 60, 8);
    let waiting = lines(terminal.backend().buffer());
    assert!(waiting[0].contains(" Task list "), "{}", waiting[0]);
    assert!(waiting[2].starts_with("║     │        │"), "{}", waiting[2]);
    assert_eq!(waiting[6], format!("╚{}╝", "═".repeat(58)), "no footer");
    opened.list().mark_stalled();
    let terminal = draw(&mut opened, 60, 8);
    let failed = lines(terminal.backend().buffer());
    assert!(
        failed[2].starts_with("║The process reader is not responding."),
        "{}",
        failed[2]
    );
    assert_eq!(failed[6], format!("╚{}╝", "═".repeat(58)), "no footer");
}
