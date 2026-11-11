//! Purpose: A server's arguments on the form's one line — the quoting grammar
//! that reads them and the quoting that writes them back.

use super::ValueError;

/// A server's arguments, edited on one line.
///
/// Whitespace outside quotes separates arguments. `"` opens or closes a quoted
/// run anywhere in an argument, as in a shell, so `--x="a b"` is `--x=a b`;
/// inside a run, `""` is one literal quote. There are no backslash escapes, so
/// a Windows path survives, and a control character is refused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Arguments(Vec<String>);

impl Arguments {
    pub fn parse_line(line: &str) -> Result<Self, ValueError> {
        if line.chars().any(char::is_control) {
            return Err(ValueError::ArgumentControl);
        }
        let reader = LineReader::read(line);
        if reader.run == Run::Quoted {
            return Err(ValueError::QuoteNotClosed);
        }
        Ok(Self(reader.finish()))
    }

    /// The argument being typed at the end of `line`: the byte where it
    /// starts, and what it reads as so far. Between arguments it starts at the
    /// end of the line and reads as nothing.
    #[must_use]
    pub fn last(line: &str) -> (usize, String) {
        let reader = LineReader::read(line);
        match reader.current {
            Some(argument) => (reader.start, argument),
            None => (line.len(), String::new()),
        }
    }

    /// The line [`Self::parse_line`] reads back as these arguments: an
    /// argument that is empty or holds whitespace or a quote is quoted, with
    /// its quotes doubled.
    #[must_use]
    pub fn to_line(arguments: &[String]) -> String {
        arguments
            .iter()
            .map(|argument| quote(argument))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl From<Arguments> for Vec<String> {
    fn from(arguments: Arguments) -> Self {
        arguments.0
    }
}

/// One argument as [`Arguments::to_line`] writes it.
#[must_use]
pub fn quote(argument: &str) -> String {
    let plain = !argument.is_empty() && !argument.chars().any(|ch| ch.is_whitespace() || ch == '"');
    if plain {
        argument.to_owned()
    } else {
        format!("\"{}\"", argument.replace('"', "\"\""))
    }
}

/// Whether the argument reader is inside a quoted run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Run {
    #[default]
    Plain,
    Quoted,
}

/// The state of the argument grammar between characters.
#[derive(Default)]
struct LineReader {
    arguments: Vec<String>,
    /// The argument being read, `None` between arguments.
    current: Option<String>,
    /// The byte where `current` started.
    start: usize,
    run: Run,
}

impl LineReader {
    /// Read `line` to its end, whatever it holds.
    fn read(line: &str) -> Self {
        let mut reader = Self::default();
        let mut chars = line.char_indices().peekable();
        while let Some((index, ch)) = chars.next() {
            let doubled = ch == '"'
                && reader.run == Run::Quoted
                && chars.next_if(|&(_, next)| next == '"').is_some();
            reader.take(index, ch, doubled);
        }
        reader
    }

    /// Read the character at byte `index`; `doubled` is a `""` inside a
    /// quoted run.
    fn take(&mut self, index: usize, ch: char, doubled: bool) {
        let separates = ch.is_whitespace() && self.run == Run::Plain;
        if self.current.is_none() && !separates {
            self.start = index;
        }
        match ch {
            '"' if doubled => self.current.get_or_insert_default().push('"'),
            '"' => {
                self.run = match self.run {
                    Run::Plain => Run::Quoted,
                    Run::Quoted => Run::Plain,
                };
                self.current.get_or_insert_default();
            }
            _ if separates => self.arguments.extend(self.current.take()),
            ch => self.current.get_or_insert_default().push(ch),
        }
    }

    fn finish(mut self) -> Vec<String> {
        self.arguments.extend(self.current.take());
        self.arguments
    }
}
