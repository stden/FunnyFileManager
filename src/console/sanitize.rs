//! Purpose: Command output reduced to what a non-emulating transcript can
//! draw — escape sequences swallowed, tabs expanded, control characters gone.

use unicode_width::UnicodeWidthChar;

/// Columns a tab advances to, as every terminal has counted them since forever.
const TAB_WIDTH: usize = 8;

/// One output line reduced to what can actually be drawn.
///
/// `pub` because the status bar summarises the same output and has to agree
/// with the transcript about what that output says.
///
/// Command output comes from a program that believed it was talking to a
/// terminal: SGR colour sequences from `ls --color`, tabs from `df`, the odd
/// BEL. Nothing here emulates a terminal, so left alone an escape sequence
/// renders as the literal garbage `[31mred[0m` — ratatui drops the zero-width
/// `ESC` and keeps the rest — and a tab collapses to nothing, silently
/// destroying the alignment it exists to create.
///
/// Both are resolved once, here, at record time rather than at draw time, so
/// the cap, the scroll arithmetic and the width budget all see exactly the text
/// that will reach the screen.
pub fn printable(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut column = 0;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\u{1b}' => skip_escape(&mut chars),
            '\t' => {
                // Advance to the next stop; a tab always moves at least one
                // column, which is what stops `\t\t` collapsing into one.
                let stop = column / TAB_WIDTH * TAB_WIDTH + TAB_WIDTH;
                for _ in column..stop {
                    out.push(' ');
                }
                column = stop;
            }
            // A line separator becomes a space. It cannot stay — one record is
            // one row — but dropping it would fuse the words on either side,
            // so a two-line command would echo as `printf 'alphabeta'`.
            '\n' | '\r' => {
                column += 1;
                out.push(' ');
            }
            // Backspace, BEL, and friends: nothing sane to draw, and passing
            // them through would let output move the cursor around the panel.
            _ if ch.is_control() || is_formatting(ch) => {}
            _ => {
                column += UnicodeWidthChar::width(ch).unwrap_or(0);
                out.push(ch);
            }
        }
    }
    out
}

/// The first of `lines` that still shows something once made [`printable`].
///
/// A line of nothing but escapes (`tput sgr0`) or blanks says nothing, so a
/// one-line summary of some output skips it. Pass `lines().rev()` for the
/// last such line.
pub fn first_drawable<'a>(lines: impl Iterator<Item = &'a str>) -> Option<String> {
    lines.map(printable).find(|line| !line.trim().is_empty())
}

/// Consume the rest of an escape sequence that `ESC` just opened.
///
/// Enough of ANSI to swallow what real commands emit: `CSI` (`ESC [ … final`)
/// covers colours and cursor moves, `OSC` (`ESC ] … BEL` or `ST`) covers window
/// titles, ECMA-35 sequences (`ESC`, intermediates, final) cover character-set
/// switches such as `ESC ( B`, and anything else is a two-character sequence.
/// An unterminated sequence — output cut off by the shell timeout — swallows
/// the rest of the line, which is better than printing half of it as noise.
fn skip_escape(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    match chars.next() {
        // `ESC ( B` from `tput sgr0` and every ncurses program, `ESC # 8`,
        // `ESC % G`: any number of intermediates, then exactly one final byte.
        Some(ch) if is_intermediate(ch) => {
            while chars.next_if(|&next| is_intermediate(next)).is_some() {}
            chars.next();
        }
        Some('[') => {
            // Parameters and intermediates, then one final byte in `@`..=`~`.
            for ch in chars.by_ref() {
                if ('\u{40}'..='\u{7e}').contains(&ch) {
                    return;
                }
            }
        }
        Some(']' | 'P' | 'X' | '^' | '_') => {
            // Terminated by BEL, or by ST (`ESC \`).
            while let Some(ch) = chars.next() {
                if ch == '\u{7}' {
                    return;
                }
                if ch == '\u{1b}' && chars.peek() == Some(&'\\') {
                    chars.next();
                    return;
                }
            }
        }
        // A two-character sequence (`ESC c`, `ESC =`, …): already consumed.
        _ => {}
    }
}

/// An ECMA-35 intermediate byte: space through `/`.
fn is_intermediate(ch: char) -> bool {
    ('\u{20}'..='\u{2f}').contains(&ch)
}

/// Invisible formatting and bidirectional override/isolate characters.
fn is_formatting(ch: char) -> bool {
    matches!(
        ch,
        '\u{00ad}'
            | '\u{061c}'
            | '\u{200b}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}'
            | '\u{feff}'
    )
}
