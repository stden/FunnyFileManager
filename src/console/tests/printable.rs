//! Purpose: Pins what a non-emulating transcript can draw: escapes stripped,
//! tabs expanded by cells, control characters dropped, one record one row.

use super::*;

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

/// `ESC ( B` (select US-ASCII) is what `tput sgr0` and every ncurses program
/// emit; its final byte used to survive as a stray `B`.
#[test]
fn charset_and_other_intermediate_escapes_are_stripped_whole() {
    assert_eq!(printable("\u{1b}(Bhello"), "hello");
    assert_eq!(printable("\u{1b}(B\u{1b}[m"), "");
    assert_eq!(
        printable("a\u{1b})0b\u{1b}(Bc\u{1b}#8d\u{1b}%Ge"),
        "abcde",
        "G0/G1 designation, DECALN and the UTF-8 switch"
    );
    assert_eq!(
        printable("\u{1b}(B\u{1b}(B[ New File ]\u{1b}(B"),
        "[ New File ]"
    );
}

/// A one-line summary of some output skips lines with nothing to show, from
/// either end.
#[test]
fn the_first_drawable_line_skips_blank_and_escape_only_lines() {
    let text = "first\n\u{1b}(B\u{1b}[m\n  \nlast\n\u{1b}[0m";
    assert_eq!(first_drawable(text.lines()), Some("first".to_owned()));
    assert_eq!(first_drawable(text.lines().rev()), Some("last".to_owned()));
    assert_eq!(first_drawable("\u{1b}(B\n \n".lines()), None);
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
