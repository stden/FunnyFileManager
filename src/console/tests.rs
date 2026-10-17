//! Purpose: Pins the console transcript's tagging and cap. Scrolling is pinned
//! in `tests/scrolling.rs`, what the transcript can draw in
//! `tests/printable.rs`, and shell quoting in `tests/shell_word.rs`.

use super::*;

mod printable;
mod scrolling;
mod shell_word;

/// The visible window as plain text, oldest first.
fn shown(log: &ConsoleLog, height: usize) -> Vec<&str> {
    log.visible(height)
        .into_iter()
        .map(ConsoleLine::text)
        .collect()
}

#[test]
fn a_new_log_is_empty() {
    let log = ConsoleLog::default();
    assert!(log.is_empty());
    assert_eq!(log.len(), 0);
    assert!(shown(&log, 10).is_empty());
}

#[test]
fn a_command_is_echoed_behind_its_prompt() {
    let mut log = ConsoleLog::default();
    log.record_command(Path::new("/srv/work"), "ls -la");
    assert_eq!(shown(&log, 10), ["/srv/work> ls -la"]);
    assert!(matches!(log.visible(10)[0], ConsoleLine::Command(_)));
}

#[test]
fn stdout_and_stderr_keep_their_tags() {
    let mut log = ConsoleLog::default();
    log.record_output("out one\nout two");
    log.record_error("bad thing");

    let lines = log.visible(10);
    assert!(matches!(lines[0], ConsoleLine::Output(_)));
    assert!(matches!(lines[1], ConsoleLine::Output(_)));
    assert!(matches!(lines[2], ConsoleLine::Error(_)));
    assert_eq!(shown(&log, 10), ["out one", "out two", "bad thing"]);
}

#[test]
fn blank_lines_inside_output_are_kept() {
    // A blank line is part of what the command printed; swallowing it would
    // silently reformat the transcript.
    let mut log = ConsoleLog::default();
    log.record_output("a\n\nb");
    assert_eq!(shown(&log, 10), ["a", "", "b"]);
}

#[test]
fn empty_output_adds_nothing() {
    let mut log = ConsoleLog::default();
    log.record_output("");
    assert!(log.is_empty());
}

// ─── the cap ──────────────────────────────────────────────────────────────

#[test]
fn the_oldest_lines_fall_off_past_the_cap() {
    let mut log = ConsoleLog::default();
    let text: String = (0..CONSOLE_CAP + 5).fold(String::new(), |mut acc, i| {
        use std::fmt::Write;
        writeln!(acc, "line {i}").expect("writing to a String cannot fail");
        acc
    });
    log.record_output(&text);

    assert_eq!(log.len(), CONSOLE_CAP);
    log.scroll_to_top();
    assert_eq!(
        shown(&log, 1),
        [format!("line {}", CONSOLE_CAP + 5 - CONSOLE_CAP).as_str()]
    );
}

#[test]
fn a_single_oversized_write_is_capped_without_building_every_line() {
    // The ring would drop them anyway; constructing them first is the waste
    // this guards against, and the observable half of it is the length.
    let mut log = ConsoleLog::default();
    let text: String = (0..CONSOLE_CAP * 2).fold(String::new(), |mut acc, i| {
        use std::fmt::Write;
        writeln!(acc, "{i}").expect("writing to a String cannot fail");
        acc
    });
    log.record_output(&text);
    assert_eq!(log.len(), CONSOLE_CAP);
}

#[test]
fn pushing_past_a_full_cap_drops_the_oldest_line() {
    // A single oversized write pre-trims to `CONSOLE_CAP` lines before ever
    // calling `push`, so `push`'s own `pop_front` loop never runs in that
    // case. Filling the ring first and then adding one more line the way a
    // second command's output would is what actually exercises it.
    let mut log = numbered(CONSOLE_CAP);
    assert_eq!(log.len(), CONSOLE_CAP);

    log.record_output("one more");
    assert_eq!(
        log.len(),
        CONSOLE_CAP,
        "still capped after the overflow push"
    );

    log.scroll_to_top();
    assert_eq!(
        shown(&log, 1),
        ["line 1"],
        "line 0 must have fallen off the front"
    );
}

/// A log holding `count` numbered output lines.
fn numbered(count: usize) -> ConsoleLog {
    let mut log = ConsoleLog::default();
    for i in 0..count {
        log.record_output(&format!("line {i}"));
    }
    log
}
