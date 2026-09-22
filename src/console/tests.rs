//! Purpose: Pins the console transcript's tagging, cap, and scroll behaviour.

use super::*;

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
        let _ = writeln!(acc, "line {i}");
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
        let _ = writeln!(acc, "{i}");
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

// ─── scrolling ────────────────────────────────────────────────────────────

/// A log holding `count` numbered output lines.
fn numbered(count: usize) -> ConsoleLog {
    let mut log = ConsoleLog::default();
    for i in 0..count {
        log.record_output(&format!("line {i}"));
    }
    log
}

#[test]
fn the_view_starts_at_the_newest_lines() {
    let log = numbered(10);
    assert_eq!(log.scroll().lines(), 0);
    assert_eq!(shown(&log, 3), ["line 7", "line 8", "line 9"]);
}

#[test]
fn scrolling_up_walks_back_in_time_and_stops_at_the_oldest() {
    let mut log = numbered(10);
    log.scroll_up(2);
    assert_eq!(shown(&log, 3), ["line 5", "line 6", "line 7"]);

    // Holding the key must not run off the end of the ring.
    log.scroll_up(1_000);
    assert_eq!(log.scroll().lines(), 9);
    assert_eq!(shown(&log, 3), ["line 0"]);
}

#[test]
fn scrolling_down_returns_to_the_newest_and_stops_there() {
    let mut log = numbered(10);
    log.scroll_up(5);
    log.scroll_down(2);
    assert_eq!(shown(&log, 2), ["line 5", "line 6"]);

    log.scroll_down(1_000);
    assert_eq!(log.scroll().lines(), 0);
    assert_eq!(shown(&log, 2), ["line 8", "line 9"]);
}

#[test]
fn home_and_end_jump_to_the_ends() {
    let mut log = numbered(10);
    log.scroll_to_top();
    assert_eq!(shown(&log, 2), ["line 0"]);
    log.scroll_to_bottom();
    assert_eq!(shown(&log, 2), ["line 8", "line 9"]);
}

#[test]
fn new_output_snaps_the_view_back_to_the_bottom() {
    // A terminal does this, and it is what makes Ctrl+O after a command show
    // that command instead of wherever the reader left the scrollback.
    let mut log = numbered(10);
    log.scroll_up(5);
    log.record_output("fresh");
    assert_eq!(log.scroll().lines(), 0);
    assert_eq!(shown(&log, 2), ["line 9", "fresh"]);
}

#[test]
fn a_viewport_taller_than_the_log_shows_everything() {
    let log = numbered(3);
    assert_eq!(shown(&log, 50), ["line 0", "line 1", "line 2"]);
}

#[test]
fn scrolling_an_empty_log_is_harmless() {
    let mut log = ConsoleLog::default();
    log.scroll_up(5);
    log.scroll_down(5);
    log.scroll_to_top();
    assert_eq!(log.scroll().lines(), 0);
    assert!(shown(&log, 10).is_empty());
}

// ─── printable: what a non-emulating transcript can draw ──────────────────

#[test]
fn colour_sequences_are_stripped_rather_than_shown_as_garbage() {
    // ratatui drops the zero-width ESC and keeps the rest, so left alone this
    // renders as the literal `[31mred[0m`.
    assert_eq!(printable("\u{1b}[31mred\u{1b}[0m"), "red");
    assert_eq!(printable("\u{1b}[1;32mok\u{1b}[m done"), "ok done");
}

#[test]
fn window_title_sequences_are_stripped() {
    // OSC, terminated by BEL or by ST (`ESC \`).
    assert_eq!(printable("\u{1b}]0;title\u{7}after"), "after");
    assert_eq!(printable("\u{1b}]0;title\u{1b}\\after"), "after");
}

#[test]
fn a_two_character_escape_is_stripped() {
    assert_eq!(printable("\u{1b}=text"), "text");
}

#[test]
fn an_unterminated_escape_swallows_the_rest_of_the_line() {
    // Output cut off by the shell timeout. Half a sequence as visible noise
    // would be worse than the truncation.
    assert_eq!(printable("before\u{1b}[31"), "before");
}

#[test]
fn tabs_advance_to_the_next_stop_instead_of_vanishing() {
    assert_eq!(printable("a\tb"), "a       b");
    assert_eq!(printable("12345678\tb"), "12345678        b");
    // Two tabs are two stops, not one.
    assert_eq!(printable("a\t\tb"), "a               b");
}

#[test]
fn tab_stops_are_measured_in_cells_not_characters() {
    // A CJK glyph is one char but two columns; counting chars would put the
    // stop in the wrong place and break the alignment tabs exist to create.
    assert_eq!(printable("漢字\tx"), "漢字    x");
}

#[test]
fn other_control_characters_are_dropped() {
    // A BEL or a backspace passed through would let command output move the
    // cursor around the panels.
    assert_eq!(printable("a\u{7}b\u{8}c"), "abc");
}

#[test]
fn line_separators_become_spaces_so_words_do_not_fuse() {
    // One record is one row, but dropping the separator outright would echo a
    // two-line command as `printf 'alphabeta'`.
    assert_eq!(printable("alpha\nbeta"), "alpha beta");
}

#[test]
fn a_path_with_a_newline_cannot_forge_a_second_row() {
    let mut log = ConsoleLog::default();
    log.record_command(Path::new("/tmp/we\nird"), "ls");
    assert_eq!(log.len(), 1, "one record must stay one row");
    assert!(!log.visible(10)[0].text().contains('\n'));
}

#[test]
fn crlf_output_leaves_no_stray_carriage_returns() {
    let mut log = ConsoleLog::default();
    log.record_output("progress\r\ndone\r\n");
    assert_eq!(shown(&log, 10), ["progress", "done"]);
}

#[test]
fn coloured_output_is_stored_already_clean() {
    let mut log = ConsoleLog::default();
    log.record_output("\u{1b}[31mred\u{1b}[0m\tcol");
    assert_eq!(shown(&log, 10), ["red     col"]);
}

#[test]
fn visible_survives_absurd_arguments() {
    let mut log = ConsoleLog::default();
    assert!(log.visible(0).is_empty());
    log.record_output("a\nb\nc");
    assert!(log.visible(0).is_empty());
    log.scroll_up(usize::MAX);
    assert_eq!(shown(&log, usize::MAX), ["a"]);
    log.scroll_down(usize::MAX);
    assert_eq!(shown(&log, 1), ["c"]);
}

#[test]
fn scroll_back_is_a_distinct_offset_from_a_history_ring_index() {
    // Both count backwards, in different rings; the newtypes are what stop one
    // being handed to the other.
    let mut log = numbered(5);
    assert_eq!(log.scroll(), ScrollBack::BOTTOM);
    log.scroll_up(2);
    assert_eq!(log.scroll().lines(), 2);
    log.scroll_to_bottom();
    assert_eq!(log.scroll(), ScrollBack::BOTTOM);
}
