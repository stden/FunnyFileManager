//! Purpose: One argument quoted as a single word of the platform shell, for
//! the transcript and for a command the user is told to run by hand.

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
pub(super) fn windows_word(argument: &str) -> String {
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
