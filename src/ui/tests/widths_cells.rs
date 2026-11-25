//! Unit tests: the cell arithmetic under every column — clipping, padding,
//! fitting and tailing names, `Cells` measuring columns rather than chars,
//! printable names, and char columns converted to cells.

use super::*;
use crate::models::CharColumn;

/// Far marks a cut name with `}`, not an ellipsis — see [`clip_name`].
#[test]
fn test_clip_name_marks_cut_names_the_way_far_does() {
    assert_eq!(clip_name("abcdefgh", Cells::new(8)), "abcdefgh");
    assert_eq!(clip_name("abcdefghij", Cells::new(8)), "abcdefg}");
    // Cut by chars, not bytes: a byte cut would split the Cyrillic name.
    assert_eq!(clip_name("файл-очень-длинный", Cells::new(6)), "файл-}");
    assert_eq!(clip_name("файл", Cells::new(0)), "");
}

#[test]
fn test_pad_cells_fills_a_short_name_to_the_column_width() {
    assert_eq!(pad_cells("a.txt", Cells::new(8)), "a.txt   ");
    assert_eq!(pad_cells("abcdefgh", Cells::new(8)), "abcdefgh");
    // Already over budget: padding cannot shrink it, and must not panic.
    assert_eq!(pad_cells("abcdefghij", Cells::new(8)), "abcdefghij");
}

#[test]
fn fit_name_truncates_with_ellipsis_and_pads_to_exact_width() {
    assert_eq!(fit_name("a.txt", Cells::new(8)), "a.txt   ");
    assert_eq!(fit_name("abcdefgh", Cells::new(8)), "abcdefgh");
    assert_eq!(fit_name("abcdefghij", Cells::new(8)), "abcdefg…");
    // Zero width: nothing fits and nothing pads — no panic, no stray ellipsis.
    assert_eq!(fit_name("anything", Cells::new(0)), "");
}

#[test]
fn fit_name_handles_wide_chars() {
    // 2 chars / 4 cells into 6: two blanks of padding, not four.
    assert_eq!(fit_name("文档", Cells::new(6)), "文档  ");
    // The glyph before the ellipsis is dropped whole, so the cut lands one
    // cell short and the pad restores the exact width — a column built from
    // the cut alone drifts out of alignment here.
    assert_eq!(fit_name("文档档案", Cells::new(6)), "文档… ");
    assert_eq!(fit_name("文档档案", Cells::new(5)), "文档…");
}

#[test]
fn test_tail_cells_keeps_the_end_of_the_path() {
    assert_eq!(tail_cells("/home/user", Cells::new(20)), "/home/user");
    assert_eq!(tail_cells("/home/user/deep", Cells::new(6)), "…/deep");
    assert_eq!(tail_cells("/дом/юзер", Cells::new(5)), "…юзер");
    assert_eq!(tail_cells("/home", Cells::new(0)), "");
}

/// Budgets are in terminal CELLS, not chars: ratatui subtracts each grapheme's
/// `cell_width` from the row, so a CJK glyph spends two of them. Measuring in
/// chars over-fills the row and pushes the span that follows off the screen.
#[test]
fn test_width_helpers_budget_in_cells_not_chars() {
    // 5 chars, 10 cells — a char-budgeted `clip_name` would return it unchanged
    // and blow the 8-column allotment by two.
    assert_eq!(clip_name("文档档案资", Cells::new(8)), "文档档}");
    // 2 chars / 4 cells into a 6-cell column: two spaces of padding, not four.
    assert_eq!(pad_cells("文档", Cells::new(6)), "文档  ");
    // A double-width glyph is dropped whole rather than half-drawn, so the
    // leftover cell is padded on the right and the size column stays aligned.
    assert_eq!(
        pad_cells(&clip_name("文档档案", Cells::new(6)), Cells::new(6)),
        "文档} "
    );
    // Same rule cutting a path from the head.
    assert_eq!(tail_cells("/文档/资料", Cells::new(5)), "…资料");
}

// ── Cells ─────────────────────────────────────────────────────────────────

#[test]
fn cells_measures_columns_not_characters() {
    // The distinction the newtype exists to enforce: a CJK glyph is one char
    // and two columns, and every budget on a row is spent in columns.
    assert_eq!(Cells::of("abc").get(), 3);
    assert_eq!(Cells::of("漢字").get(), 4);
    assert_eq!("漢字".chars().count(), 2);
    assert_eq!(Cells::of("").get(), 0);
    assert!(Cells::of("").is_zero());
}

/// ratatui refuses to draw any grapheme holding a control character, so a
/// name carrying one must be measured without it. Counting it budgeted the row
/// for a column that never appeared and slid Size, Date and Time one cell left
/// of their headings for that row alone.
#[test]
fn cells_ignores_what_the_terminal_will_not_draw() {
    assert_eq!(Cells::of("tab\there").get(), Cells::of("tabhere").get());
    assert_eq!(
        Cells::of("bel\u{7}-cr\r-bs\u{8}").get(),
        Cells::of("bel-cr-bs").get()
    );
    assert_eq!(Cells::of("esc\u{1b}[31m").get(), Cells::of("esc[31m").get());
    assert_eq!(Cells::of("\u{1}\u{2}").get(), 0);
    // A control character between wide glyphs must not disturb their widths.
    assert_eq!(Cells::of("漢\u{7}字").get(), 4);
}

/// A name is data from the filesystem, not a terminal stream: what cannot be
/// drawn becomes one visible cell, so two entries differing only by an
/// invisible character cannot read as the same file.
#[test]
fn printable_name_replaces_the_undrawable_and_borrows_otherwise() {
    assert!(matches!(
        printable_name("ordinary.txt"),
        std::borrow::Cow::Borrowed("ordinary.txt")
    ));
    assert_eq!(printable_name("tab\there"), "tab\u{FFFD}here");
    assert_eq!(
        printable_name("bel\u{7}-cr\r"),
        format!("bel{UNPRINTABLE}-cr{UNPRINTABLE}")
    );
    // The replacement is one cell wide, so the row measures what it draws.
    assert_eq!(
        Cells::of(&printable_name("tab\there")).get(),
        "tab?here".len()
    );
}

/// A character that draws nothing is as good as a hidden one: two files whose
/// names differ only by a soft hyphen or a zero-width space would read as the
/// same row, and the bidi overrides can reverse an extension on screen.
#[test]
fn printable_name_shows_the_characters_that_draw_nothing() {
    let marker = UNPRINTABLE;
    for hidden in [
        '\u{00AD}', '\u{200B}', '\u{200E}', '\u{202E}', '\u{2060}', '\u{FEFF}',
    ] {
        let name = format!("cc{hidden}dd.txt");
        assert_eq!(
            printable_name(&name),
            format!("cc{marker}dd.txt"),
            "{hidden:?} passed through unseen"
        );
    }
    // A name that only looks similar is left exactly as it is.
    assert!(matches!(
        printable_name("ccdd.txt"),
        std::borrow::Cow::Borrowed("ccdd.txt")
    ));
}

#[test]
fn cells_arithmetic_saturates_instead_of_wrapping() {
    // Every budget in this module is "what is left after"; underflowing it
    // would wrap to a colossal width and draw past the terminal edge.
    assert_eq!(Cells::new(3).saturating_sub(Cells::new(10)), Cells::ZERO);
    assert_eq!(Cells::new(10).saturating_sub(Cells::new(3)).get(), 7);
    assert_eq!((Cells::new(2) + Cells::new(3)).get(), 5);
}

#[test]
fn cells_before_converts_a_char_index_to_a_column() {
    // The one place chars and columns legitimately meet: the editor's cursor
    // is char-indexed, the screen is column-indexed.
    let line = |text| EditorLine {
        text,
        tab: crate::models::TabSize::DEFAULT,
    };
    assert_eq!(
        line("漢字abc").cells_before(CharColumn::new(2)),
        Cells::new(4)
    );
    assert_eq!(line("abc").cells_before(CharColumn::new(2)), Cells::new(2));
    assert_eq!(line("abc").cells_before(CharColumn::new(99)), Cells::new(3));
    // A TAB reaches the next stop, wherever it starts.
    assert_eq!(line("a\tb").cells_before(CharColumn::new(2)), Cells::new(4));
    assert_eq!(
        line("abcd\tb").cells_before(CharColumn::new(5)),
        Cells::new(8)
    );
}
