//! Purpose: Pin the Language servers dialog's layout cell for cell, at the
//! design's 80×24 and at the sizes where its rows give way: the rules, the
//! list's half of the body, the detail pane and where it is cut, the message
//! row, and the F3 view.

use super::*;

/// The dialog of the documentation's screenshot, drawn at `width`×`height`,
/// against `expected` rows (trailing blanks left out).
fn assert_screen(app: &mut App, (width, height): (u16, u16), expected: &[&str]) {
    let screen = buffer_string(&draw(app, width, height));
    let rows: Vec<&str> = screen.lines().map(str::trim_end).collect();
    assert_eq!(rows, expected, "{width}×{height}:\n{screen}");
}

#[test]
fn the_list_screen_at_80_by_24_is_the_designs() {
    let mut app = capture::review_app();
    assert_screen(&mut app, (80, 24), LIST_80X24);
    let buffer = draw(&mut app, 80, 24);
    assert_eq!(
        buffer[(40, 21)].bg,
        crate::ui::FAR_BLUE,
        "the panel's blue ground"
    );
    assert_eq!(fg(&buffer, "Settings: "), palette::CODE_NORMAL);
}

/// At 16 rows the detail pane keeps what fits and says where the rest is;
/// one row fewer and it gives way to a pointer to F3.
#[test]
fn the_detail_pane_is_cut_at_16_rows_and_gone_below() {
    let mut app = capture::review_app();
    assert_screen(&mut app, (80, 16), LIST_80X16);
    assert_screen(&mut app, (80, 15), LIST_80X15);
    assert_screen(&mut app, (60, 12), LIST_60X12);
}

/// The smallest window that has a frame and a key bar is 4×3; below it only
/// the ground is drawn.
#[test]
fn a_window_needs_three_rows_and_four_columns() {
    let mut app = capture::review_app();
    assert_screen(&mut app, (4, 3), LIST_4X3);
    for size in [(3, 24), (80, 2)] {
        let screen = buffer_string(&draw(&mut app, size.0, size.1));
        assert!(
            screen.chars().all(|ch| ch == ' ' || ch == '\n'),
            "{size:?}:\n{screen}"
        );
    }
}

/// F3 lists every server under the note and a rule; on a short window the
/// servers go first, then the rule, then the note, and the bottom border is
/// left alone.
#[test]
fn the_view_screen_keeps_its_rows_inside_the_frame() {
    let mut app = capture::review_app();
    press(&mut app, KeyCode::F(3));
    assert_screen(&mut app, (80, 24), VIEW_80X24);
    assert_screen(&mut app, (80, 7), VIEW_80X7);
    assert_screen(&mut app, (80, 6), VIEW_80X6);
    assert_screen(&mut app, (80, 4), VIEW_80X4);
}

/// Each line of `wrap`'s output as the text it shows.
fn wrapped(pieces: &[&str], indent: usize, width: usize) -> Vec<String> {
    let style = ratatui::style::Style::default().fg(palette::CODE_STRING);
    let pieces = pieces
        .iter()
        .map(|text| ((*text).to_owned(), style))
        .collect();
    super::super::detail::wrap(pieces, indent, crate::ui::Cells::new(width))
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        })
        .collect()
}

/// A value fills its line to the last cell and goes on under itself; a piece
/// cut in two keeps its colour on both lines.
#[test]
fn wrap_fills_each_line_and_indents_the_rest() {
    assert_eq!(wrapped(&["abcdef"], 2, 6), ["abcdef"]);
    assert_eq!(
        wrapped(&["abc", "defghijklm"], 2, 6),
        ["abcdef", "  ghij", "  klm"]
    );
    let style = ratatui::style::Style::default().fg(palette::CODE_STRING);
    let lines = super::super::detail::wrap(
        vec![("abcdefgh".to_owned(), style)],
        2,
        crate::ui::Cells::new(6),
    );
    for line in &lines {
        let text = line
            .spans
            .iter()
            .rev()
            .find(|span| !span.content.trim().is_empty());
        assert_eq!(text.map(|span| span.style), Some(style), "{line:?}");
    }
}

/// On a screen narrower than the indent every line still shows a character:
/// a wide one overhangs rather than leave an empty line, and a combining mark
/// stays with the letter it belongs to.
#[test]
fn wrap_narrower_than_its_indent_still_makes_progress() {
    assert_eq!(wrapped(&["中文字"], 2, 3), ["中文", "  字"]);
    assert_eq!(
        wrapped(&["abc\u{301}de"], 2, 1),
        ["abc\u{301}", "  d", "  e"]
    );
}

/// The bottom border's corners and last `═`, and the key bar's first cells, at
/// `height` rows, 80 wide.
fn assert_bottom_intact(buffer: &Buffer, height: u16) {
    let border = height - 2;
    let bar = height - 1;
    assert_eq!(buffer[(0, border)].symbol(), "╚", "{height} rows");
    assert_eq!(buffer[(79, border)].symbol(), "╝", "{height} rows");
    for x in [0, 77, 79] {
        assert_eq!(
            buffer[(x, border)].fg,
            palette::FRAME,
            "{height} rows, column {x}"
        );
    }
    assert_eq!(buffer[(0, bar)].symbol(), "1", "{height} rows");
    assert_eq!(
        buffer[(0, bar)].bg,
        ratatui::style::Color::Black,
        "{height} rows"
    );
    assert_eq!(buffer[(1, bar)].bg, palette::CURSOR_BG, "{height} rows");
}

/// Nothing drawn inside the window spills onto its bottom border or the key
/// bar, at any height: not a rule, not a row's colours.
#[test]
fn nothing_inside_the_window_reaches_its_bottom_border_or_the_key_bar() {
    let mut app = capture::review_app();
    for height in 3..=24 {
        assert_bottom_intact(&draw(&mut app, 80, height), height);
    }
    press(&mut app, KeyCode::F(3));
    for height in 3..=24 {
        assert_bottom_intact(&draw(&mut app, 80, height), height);
    }
}

/// A window too short for every row keeps the settings row, then the message
/// row, then the rules, and never draws one row over another.
#[test]
fn a_short_window_drops_rows_without_drawing_one_over_another() {
    let mut app = capture::review_app();
    assert_screen(&mut app, (80, 3), LIST_80X3);
    assert_screen(&mut app, (80, 4), LIST_80X4);
    assert_screen(&mut app, (80, 5), LIST_80X5);
    assert_screen(&mut app, (80, 6), LIST_80X6);
    assert_screen(&mut app, (80, 7), LIST_80X7);
    assert_screen(&mut app, (80, 8), LIST_80X8);
}

/// Extensions longer than their column are cut with `…` inside it, and the
/// status keeps its place at the end of the row, at full width and narrow.
#[test]
fn extensions_too_long_for_their_column_are_cut_inside_it() {
    let machine = LspMachine::new();
    machine.write_settings(
        "lsp:\n  aaa: {command: aaa-server, extensions: [aaaa, bbbb, cccc, dddd], install: null}\n",
    );
    let mut app = machine.opened();
    for (width, expected) in [
        (
            80,
            "║ ▸ aaa                 [x] aaa-server                 .aaaa .bbbb .c… missing ║",
        ),
        (
            60,
            "║ ▸ aaa                 [x] aaa-ser… .aaaa .bbbb … missing ║",
        ),
    ] {
        let screen = buffer_string(&draw(&mut app, width, 24));
        let row = screen.lines().find(|line| line.contains("▸ aaa"));
        assert_eq!(row, Some(expected), "{screen}");
    }
}

const LIST_80X24: &[&str] = &[
    "╔══════════════════════════════ Language servers ══════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║   bat                 [ ] batch-language-server      .bat .cmd       missing ║",
    "║   css                 [x] vscode-css-language-server .css              ready ║",
    "║   html                [x] vscode-html-language-serv… .html .htm        ready ║",
    "║   java                [x] jdtls                      .java              auto ║",
    "║   javascript          [x] typescript-language-server .js .mjs .cjs      auto ║",
    "║   javascriptreact     [x] typescript-language-server .jsx               auto ║",
    "║   json                [x] vscode-json-language-serv… .json .jsonc      ready ║",
    "║ ▸ python              [x] pylsp                      .py .pyi          ready ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ python  [enabled]  .py .pyi                                                  ║",
    "║   Command:          pylsp                                                    ║",
    "║   Arguments:        (none)                                                   ║",
    "║   Status:           installed in FFM cache                                   ║",
    "║   Executable:       /home/bob/.config/ffm/lsp/7c1e/venv/bin/python           ║",
    "║   Launch arguments: -m pylsp                                                 ║",
    "║   Auto-install:     {\"kind\":\"python\",\"package\":\"python-lsp-server[pyflakes,p ║",
    "║                     ycodestyle]\",\"module\":\"pylsp\"}                           ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ No changes                                                                   ║",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═════════════════════╝",
    "1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12",
];

const LIST_80X16: &[&str] = &[
    "╔══════════════════════════════ Language servers ══════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║   javascript          [x] typescript-language-server .js .mjs .cjs      auto ║",
    "║   javascriptreact     [x] typescript-language-server .jsx               auto ║",
    "║   json                [x] vscode-json-language-serv… .json .jsonc      ready ║",
    "║ ▸ python              [x] pylsp                      .py .pyi          ready ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ python  [enabled]  .py .pyi                                                  ║",
    "║   Command:          pylsp                                                    ║",
    "║   Arguments:        (none)                                                   ║",
    "║ … F3 shows everything                                                        ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ No changes                                                                   ║",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═════════════════════╝",
    "1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12",
];

const LIST_80X15: &[&str] = &[
    "╔══════════════════════════════ Language servers ══════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║   bat                 [ ] batch-language-server      .bat .cmd       missing ║",
    "║   css                 [x] vscode-css-language-server .css              ready ║",
    "║   html                [x] vscode-html-language-serv… .html .htm        ready ║",
    "║   java                [x] jdtls                      .java              auto ║",
    "║   javascript          [x] typescript-language-server .js .mjs .cjs      auto ║",
    "║   javascriptreact     [x] typescript-language-server .jsx               auto ║",
    "║   json                [x] vscode-json-language-serv… .json .jsonc      ready ║",
    "║ ▸ python              [x] pylsp                      .py .pyi          ready ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ F3 shows details                                                             ║",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═════════════════════╝",
    "1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12",
];

const LIST_60X12: &[&str] = &[
    "╔════════════════════ Language servers ════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml            ║",
    "╟──────────────────────────────────────────────────────────╢",
    "║   java                [x] jdtls    .java            auto ║",
    "║   javascript          [x] typescr… .js .mjs .cjs    auto ║",
    "║   javascriptreact     [x] typescr… .jsx             auto ║",
    "║   json                [x] vscode-… .json .jsonc    ready ║",
    "║ ▸ python              [x] pylsp    .py .pyi        ready ║",
    "╟──────────────────────────────────────────────────────────╢",
    "║ F3 shows details                                         ║",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═╝",
    "1    2Save3View4Edit5Che…6    7Add 8Res…9    10Qu…11   12",
];

const LIST_4X3: &[&str] = &["╔══╗", "╚ E╝", "1234"];

const VIEW_80X24: &[&str] = &[
    "╔═══════════════════════════ Language servers: all ════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "║ Availability is what FFM would start; it is not a process list.              ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ bat  [disabled]  .bat .cmd                                                   ║",
    "║   Command:          batch-language-server                                    ║",
    "║   Arguments:        --stdio                                                  ║",
    "║   Status:           not installed; auto-install disabled                     ║",
    "║   Auto-install:     {\"kind\":\"npm\",\"packages\":[\"@jcs-pr/batch-language-server ║",
    "║                     \"],\"package\":\"@jcs-pr/batch-language-server\",\"binary\":\"b ║",
    "║                     atch-language-server\"}                                   ║",
    "║                                                                              ║",
    "║ css  [enabled]  .css                                                         ║",
    "║   Command:          vscode-css-language-server                               ║",
    "║   Arguments:        --stdio                                                  ║",
    "║   Status:           found configured executable                              ║",
    "║   Executable:       /usr/local/bin/vscode-css-language-server                ║",
    "║   Launch arguments: --stdio                                                  ║",
    "║   Auto-install:     {\"kind\":\"npm\",\"packages\":[\"vscode-langservers-extracted\" ║",
    "║                     ],\"package\":\"vscode-langservers-extracted\",\"binary\":\"vsc ║",
    "║                     ode-css-language-server\"}                                ║",
    "║                                                                              ║",
    "╚ Lines 1-18 of 110  Up Down PgUp PgDn Home End  F3/Esc List ══════════════════╝",
    "1      2      3List  4      5Check 6      7      8      9     10Back11    12",
];

const VIEW_80X4: &[&str] = &[
    "╔═══════════════════════════ Language servers: all ════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "╚ Lines 0-0 of 110  Up Down PgUp PgDn Home End  F3/Esc List ═══════════════════╝",
    "1      2      3List  4      5Check 6      7      8      9     10Back11    12",
];

const LIST_80X3: &[&str] = &[
    "╔══════════════════════════════ Language servers ══════════════════════════════╗",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═════════════════════╝",
    "1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12",
];

const LIST_80X4: &[&str] = &[
    "╔══════════════════════════════ Language servers ══════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═════════════════════╝",
    "1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12",
];

const LIST_80X5: &[&str] = &[
    "╔══════════════════════════════ Language servers ══════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "║ F3 shows details                                                             ║",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═════════════════════╝",
    "1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12",
];

const LIST_80X6: &[&str] = &[
    "╔══════════════════════════════ Language servers ══════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ F3 shows details                                                             ║",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═════════════════════╝",
    "1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12",
];

const LIST_80X7: &[&str] = &[
    "╔══════════════════════════════ Language servers ══════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ F3 shows details                                                             ║",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═════════════════════╝",
    "1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12",
];

const LIST_80X8: &[&str] = &[
    "╔══════════════════════════════ Language servers ══════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ ▸ python              [x] pylsp                      .py .pyi          ready ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ F3 shows details                                                             ║",
    "╚ Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close ═════════════════════╝",
    "1      2Save  3View  4Edit  5Check 6      7Add   8Reset 9     10Quit11    12",
];

const VIEW_80X6: &[&str] = &[
    "╔═══════════════════════════ Language servers: all ════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "║ Availability is what FFM would start; it is not a process list.              ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "╚ Lines 0-0 of 110  Up Down PgUp PgDn Home End  F3/Esc List ═══════════════════╝",
    "1      2      3List  4      5Check 6      7      8      9     10Back11    12",
];

const VIEW_80X7: &[&str] = &[
    "╔═══════════════════════════ Language servers: all ════════════════════════════╗",
    "║ Settings: /home/bob/.config/ffm/settings.yaml                                ║",
    "║ Availability is what FFM would start; it is not a process list.              ║",
    "╟──────────────────────────────────────────────────────────────────────────────╢",
    "║ bat  [disabled]  .bat .cmd                                                   ║",
    "╚ Lines 1-1 of 110  Up Down PgUp PgDn Home End  F3/Esc List ═══════════════════╝",
    "1      2      3List  4      5Check 6      7      8      9     10Back11    12",
];
