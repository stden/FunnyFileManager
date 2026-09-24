//! Purpose: Colorer regions and `paint`: spans replacing the tokenizer, gap filling, overlap and multibyte clamping.

use super::*;

/// Colorer's regions, when present for a line, replace the tokenizer's answer
/// outright — this is what lets Colorer win over the built-in scanner. Built
/// by constructing a `Highlighter` directly (private fields are visible to
/// this child module) rather than through `for_file`, because this build has
/// no native Colorer-library to actually populate `regions` from: see
/// `colorer_regions_and_paint_are_unreachable_without_a_native_library` below.
#[cfg(feature = "colorer")]
#[test]
fn a_populated_colorer_region_is_painted_instead_of_tokenized() {
    let mut highlighter = Highlighter::new(&PathBuf::from("a.rs"));
    highlighter.regions = vec![vec![colorer::Span {
        start: 0,
        end: 3,
        token: Token::Meta,
    }]];
    // Without the region, `let` tokenizes as a keyword; Colorer's answer wins.
    let out = highlighter.line("let x = 1;");
    assert_eq!(out[0], ("let", Token::Meta));
}

/// `paint` fills every gap with `Token::Normal`, and where two spans overlap
/// the later one wins — Colorer's own rule for a bracket that is both
/// `def:Symbol` and `def:PairStart`. A span reaching past the end of the line
/// must be clamped rather than panic.
#[cfg(feature = "colorer")]
#[test]
fn paint_fills_gaps_and_lets_the_later_span_win_on_overlap() {
    let line = "let x = 1;";
    let spans = vec![
        colorer::Span {
            start: 0,
            end: 3,
            token: Token::Normal,
        },
        colorer::Span {
            start: 0,
            end: 3,
            token: Token::Keyword,
        },
        colorer::Span {
            start: 8,
            end: 999,
            token: Token::Number,
        },
    ];
    let out = paint(line, &spans);
    let rejoined: String = out.iter().map(|(text, _)| *text).collect();
    assert_eq!(rejoined, line, "paint must cover the whole line");

    assert_eq!(out[0], ("let", Token::Keyword));
    assert!(
        out.iter()
            .any(|(text, token)| *text == " x = " && *token == Token::Normal),
        "untouched middle must fall back to Normal: {out:?}"
    );
    assert_eq!(*out.last().unwrap(), ("1;", Token::Number));
}

/// `paint` converts Colorer's char offsets back to byte offsets, so a
/// multibyte line must not be cut mid-character.
#[cfg(feature = "colorer")]
#[test]
fn paint_handles_multibyte_lines_without_splitting_a_character() {
    let line = "имя = 1";
    let spans = vec![colorer::Span {
        start: 0,
        end: 3,
        token: Token::Type,
    }];
    let out = paint(line, &spans);
    let rejoined: String = out.iter().map(|(text, _)| *text).collect();
    assert_eq!(rejoined, line);
    assert_eq!(out[0], ("имя", Token::Type));
}

/// An empty line must not panic `paint` and must produce no runs.
#[cfg(feature = "colorer")]
#[test]
fn paint_handles_an_empty_line() {
    assert!(paint("", &[]).is_empty());
}

/// Documents, rather than forces, a genuinely unreachable path in this build.
///
/// `colorer-sys`'s build script (`colorer-sys/build.rs`) only defines
/// `cfg(colorer)` when it finds a real native Colorer-library; without one it
/// compiles `colorer_open` as a stub that unconditionally returns a null
/// handle (`colorer-sys/src/lib.rs`). `Colorer::open` — and therefore
/// `Colorer::discover`/`Colorer::shared`, which this crate's `for_file` calls
/// — turns that null into `None` regardless of whether a catalog file exists
/// on disk. So in any environment without the native library actually linked
/// in (this one included, per the `Colorer-library not found` build warning),
/// `Highlighter::for_file`'s `if let Some(engine) = colorer::Colorer::shared()`
/// body cannot obtain an engine, and `regions`
/// can never be populated by `for_file` itself. The tests above cover the same
/// lines' *behaviour* (a populated region wins over the tokenizer, and `paint`
/// itself) by constructing a `Highlighter`/calling `paint` directly instead.
#[cfg(feature = "colorer")]
#[test]
fn colorer_regions_and_paint_are_unreachable_without_a_native_library() {
    // This documents a no-native-library build. Once `./ffm.py colorer` has
    // installed one, `Colorer::shared()` legitimately returns `Some`, and the
    // scenario this test names no longer applies — so it steps aside, the
    // same way the sibling tests in `highlight/colorer/tests.rs` do.
    if colorer_sys::available() {
        return;
    }
    assert!(colorer::Colorer::shared().is_none());
}
