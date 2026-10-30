//! Purpose: Pin the Language servers dialog's colours: the conflict marks by
//! whether they block, each cell in the colour of what it says, and a
//! disabled server dimmed in the list and its detail.

use super::*;

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
