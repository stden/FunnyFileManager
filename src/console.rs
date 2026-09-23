//! Purpose: The Far-style console transcript living behind the panels.
//!
//! Far Manager keeps command output on the console screen and draws the panels
//! over it; `Ctrl+O` lifts the panels off to show what is underneath. FFM runs
//! on ratatui's alternate screen, which has no such screen to write to, so the
//! transcript is kept here and painted as the bottom layer of every frame.
//!
//! Pure: no `App`, no filesystem, no I/O. Scroll position lives beside the
//! lines it scrolls because the two are meaningless apart — appending output
//! has to snap the view back to the bottom, the way a terminal does.

use std::collections::VecDeque;
use std::path::Path;

use unicode_width::UnicodeWidthChar;

use crate::scroll::ScrollBack;

/// Lines the transcript keeps; the oldest fall off first.
///
/// Bounded because a single `cat` of a large file would otherwise hold every
/// line for the rest of the session. Deep enough to scroll back through a build
/// log, shallow enough that the memory is not worth thinking about.
pub const CONSOLE_CAP: usize = 5_000;

/// Columns a tab advances to, as every terminal has counted them since forever.
const TAB_WIDTH: usize = 8;

/// One transcript line, tagged with what produced it.
///
/// An enum rather than a `String` plus a colour: the tag is what the renderer
/// needs, and deciding it once at the point that *knows* — the dispatcher —
/// beats re-guessing from the text at draw time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsoleLine {
    /// The echoed prompt and command, as Far prints it before running.
    Command(String),
    /// A line the command wrote to stdout.
    Output(String),
    /// A line the command wrote to stderr, or a failure to launch it.
    Error(String),
}

impl ConsoleLine {
    #[must_use]
    pub fn text(&self) -> &str {
        match self {
            Self::Command(text) | Self::Output(text) | Self::Error(text) => text,
        }
    }
}

/// The console transcript and where the reader is looking at it.
#[derive(Debug, Clone, Default)]
pub struct ConsoleLog {
    lines: VecDeque<ConsoleLine>,
    /// Where the reader is looking; new output always brings it back to the
    /// bottom.
    scroll: ScrollBack,
}

impl ConsoleLog {
    /// Echo `command` behind its prompt, exactly as the command line shows it.
    pub fn record_command(&mut self, cwd: &Path, command: &str) {
        // Through `printable` like any other line: a Unix path may hold a
        // newline, and one record has to stay exactly one row or the scroll
        // arithmetic below is counting something the screen does not show.
        let echo = printable(&format!("{}> {command}", cwd.display()));
        self.push(ConsoleLine::Command(echo));
    }

    /// Append everything a command wrote to stdout.
    pub fn record_output(&mut self, text: &str) {
        self.record_lines(text, ConsoleLine::Output);
    }

    /// Append everything a command wrote to stderr.
    pub fn record_error(&mut self, text: &str) {
        self.record_lines(text, ConsoleLine::Error);
    }

    /// Split `text` into lines and append them under one tag.
    ///
    /// Only the last [`CONSOLE_CAP`] lines are even constructed: a command that
    /// printed a million lines would otherwise have every one of them allocated
    /// and immediately dropped by the ring's own cap.
    fn record_lines(&mut self, text: &str, tag: impl Fn(String) -> ConsoleLine) {
        let all: Vec<&str> = text.lines().collect();
        let first_kept = all.len().saturating_sub(CONSOLE_CAP);
        for line in all.iter().skip(first_kept) {
            self.push(tag(printable(line)));
        }
    }

    /// Add one line, drop the oldest past the cap, and return to the bottom.
    ///
    /// Snapping the view down is what a terminal does, and it is what keeps
    /// `Ctrl+O` after a command showing that command rather than wherever the
    /// reader happened to leave the scrollbar.
    fn push(&mut self, line: ConsoleLine) {
        self.lines.push_back(line);
        while self.lines.len() > CONSOLE_CAP {
            self.lines.pop_front();
        }
        self.scroll = ScrollBack::BOTTOM;
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// How far up from the newest line the view currently sits.
    #[must_use]
    pub const fn scroll(&self) -> ScrollBack {
        self.scroll
    }

    /// The at most `height` lines visible in a viewport that tall, oldest first.
    ///
    /// Clamped here rather than at every scroll call site: the viewport height
    /// is a rendering fact, and the scroll keys have no business knowing it.
    #[must_use]
    pub fn visible(&self, height: usize) -> Vec<&ConsoleLine> {
        let end = self
            .lines
            .len()
            .saturating_sub(self.scroll.lines().min(self.max_scroll()));
        let start = end.saturating_sub(height);
        self.lines.iter().skip(start).take(end - start).collect()
    }

    /// Furthest the view may scroll: the newest line always stays reachable, so
    /// scrolling can never empty the screen.
    fn max_scroll(&self) -> usize {
        self.lines.len().saturating_sub(1)
    }

    /// Scroll back in time by `lines`, stopping at the oldest kept line.
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll = self.scroll.back(lines, self.max_scroll());
    }

    /// Scroll forward in time by `lines`, stopping at the newest.
    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll = self.scroll.forward(lines);
    }

    /// Jump to the oldest kept line.
    pub fn scroll_to_top(&mut self) {
        self.scroll = ScrollBack::oldest(self.max_scroll());
    }

    /// Jump back to the newest line.
    pub const fn scroll_to_bottom(&mut self) {
        self.scroll = ScrollBack::BOTTOM;
    }
}

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
            _ if ch.is_control() => {}
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

/// `argument` as one word of the platform shell, for the transcript and for
/// a command the user is told to run by hand.
///
/// A plain word passes as it is. Anything else is single-quoted on Unix,
/// where the quotes protect everything but themselves, and double-quoted on
/// Windows, where the C runtime reads `\"` as a quote inside the word and
/// only the backslashes before a quote need doubling.
#[must_use]
pub fn shell_word(argument: &str) -> String {
    let plain = !argument.is_empty()
        && argument
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "-_./=:+,@%".contains(ch));
    if plain {
        return argument.to_owned();
    }
    quoted(argument)
}

#[cfg(windows)]
fn quoted(argument: &str) -> String {
    windows_word(argument)
}

#[cfg(not(windows))]
fn quoted(argument: &str) -> String {
    format!("'{}'", argument.replace('\'', "'\\''"))
}

/// `argument` double-quoted as the Microsoft C runtime splits a command line:
/// a run of backslashes is literal unless a quote follows it, so before a
/// quote — an inner one, or the closing one — the run is doubled.
#[cfg_attr(
    all(not(windows), not(test)),
    expect(
        dead_code,
        reason = "only Windows quotes this way; the rule is pure and tested on every platform"
    )
)]
fn windows_word(argument: &str) -> String {
    let mut word = String::from('"');
    let mut backslashes = 0_usize;
    for ch in argument.chars() {
        match ch {
            '\\' => backslashes = backslashes.saturating_add(1),
            '"' => {
                word.push_str(&"\\".repeat(backslashes.saturating_add(1)));
                backslashes = 0;
            }
            _ => backslashes = 0,
        }
        word.push(ch);
    }
    word.push_str(&"\\".repeat(backslashes));
    word.push('"');
    word
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
        Some(']') => {
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

#[cfg(test)]
mod tests;
