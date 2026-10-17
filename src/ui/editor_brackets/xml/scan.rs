//! Purpose: Stateful XML/HTML tag-pair scanner across lines for [`super::matching_xml_pair`].

use super::TagSpan;

type Step = (usize, usize, Option<[TagSpan; 2]>);

#[derive(Debug, Default)]
pub(super) struct XmlScanner<'a> {
    mode: Mode<'a>,
    stack: Vec<(&'a str, TagSpan)>,
}

#[derive(Debug, Default)]
enum Mode<'a> {
    #[default]
    Text,
    Until(&'static str),
    OpenTag {
        pending: (&'a str, TagSpan),
        quote: Option<char>,
    },
}

impl<'a> XmlScanner<'a> {
    pub(super) fn find_pair(
        mut self,
        lines: &'a [String],
        cursor_line: usize,
        cursor_col: usize,
    ) -> Option<[TagSpan; 2]> {
        lines
            .iter()
            .enumerate()
            .find_map(|(idx, line)| self.scan_line(lines, idx, line, cursor_line, cursor_col))
    }

    fn scan_line(
        &mut self,
        lines: &'a [String],
        line_idx: usize,
        line: &'a str,
        cursor_line: usize,
        cursor_col: usize,
    ) -> Option<[TagSpan; 2]> {
        let (mut byte_pos, mut char_pos) = (0, 0);
        while byte_pos < line.len() {
            let rest = line.get(byte_pos..).unwrap_or_default();
            let (adv_bytes, adv_chars, pair) = match self.mode {
                Mode::Text => self.step_text(line_idx, char_pos, rest),
                Mode::Until(closer) => Self::step_until(&mut self.mode, rest, closer),
                Mode::OpenTag { .. } => self.step_open_tag(line_idx, char_pos, rest),
            };
            if let Some(matched) = pair.filter(|[open, close]| {
                open.matches_cursor(lines, cursor_line, cursor_col)
                    || close.matches_cursor(lines, cursor_line, cursor_col)
            }) {
                return Some(matched);
            }
            byte_pos += adv_bytes;
            char_pos += adv_chars;
        }
        None
    }

    fn step_until(mode: &mut Mode<'a>, rest: &'a str, closer: &str) -> Step {
        if rest.starts_with(closer) {
            *mode = Mode::Text;
            return (closer.len(), closer.len(), None);
        }
        (rest.chars().next().map_or(1, char::len_utf8), 1, None)
    }

    fn step_text(&mut self, line_idx: usize, char_pos: usize, rest: &'a str) -> Step {
        for (prefix, closer) in [("<!--", "-->"), ("<?", "?>"), ("<!", ">")] {
            if rest.starts_with(prefix) {
                self.mode = Mode::Until(closer);
                return (prefix.len(), prefix.len(), None);
            }
        }
        if rest.starts_with("</") {
            return self.step_close_tag(line_idx, char_pos, rest);
        }
        if rest.starts_with('<') {
            return self.step_open_tag_start(line_idx, char_pos, rest);
        }
        (rest.chars().next().map_or(1, char::len_utf8), 1, None)
    }

    fn step_close_tag(&mut self, line_idx: usize, char_pos: usize, rest: &'a str) -> Step {
        let after_slash = rest.get(2..).unwrap_or_default();
        let name_len = ascii_run(after_slash, is_tag_name_char);
        let after_name = after_slash.get(name_len..).unwrap_or_default();
        let ws_len = ascii_run(after_name, |c| c.is_ascii_whitespace());
        if name_len > 0
            && after_name
                .get(ws_len..)
                .unwrap_or_default()
                .starts_with('>')
        {
            let name = after_slash.get(..name_len).unwrap_or_default();
            let total = 2 + name_len + ws_len + 1;
            let close_span = TagSpan::new(line_idx, char_pos, char_pos + total);
            return (total, total, self.pop_matching(name, close_span));
        }
        (2, 2, None)
    }

    fn step_open_tag_start(&mut self, line_idx: usize, char_pos: usize, rest: &'a str) -> Step {
        let after_lt = rest.get(1..).unwrap_or_default();
        let name_len = ascii_run(after_lt, is_tag_name_char);
        if name_len == 0 {
            return (1, 1, None);
        }
        let name = after_lt.get(..name_len).unwrap_or_default();
        if after_lt
            .get(name_len..)
            .unwrap_or_default()
            .starts_with('>')
        {
            let total = 1 + name_len + 1;
            self.stack
                .push((name, TagSpan::new(line_idx, char_pos, char_pos + total)));
            return (total, total, None);
        }
        let open_span = TagSpan::new(line_idx, char_pos, char_pos + 1 + name_len);
        self.mode = Mode::OpenTag {
            pending: (name, open_span),
            quote: None,
        };
        (1 + name_len, 1 + name_len, None)
    }

    fn step_open_tag(&mut self, line_idx: usize, char_pos: usize, rest: &'a str) -> Step {
        let Mode::OpenTag {
            pending,
            ref mut quote,
        } = self.mode
        else {
            return (1, 1, None);
        };
        let ch = rest.chars().next().unwrap_or(' ');
        if quote.is_some() {
            if *quote == Some(ch) {
                *quote = None;
            }
            return (ch.len_utf8(), 1, None);
        }
        if rest.starts_with("/>") {
            let open_span = pending.1;
            self.mode = Mode::Text;
            return (
                2,
                2,
                Some([open_span, TagSpan::new(line_idx, char_pos, char_pos + 2)]),
            );
        }
        if ch == '>' {
            self.stack.push(pending);
            self.mode = Mode::Text;
            return (1, 1, None);
        }
        *quote = (ch == '"' || ch == '\'').then_some(ch);
        (ch.len_utf8(), 1, None)
    }

    fn pop_matching(&mut self, name: &str, close_span: TagSpan) -> Option<[TagSpan; 2]> {
        let idx = self.stack.iter().rposition(|(open, _)| *open == name)?;
        let (_, open_span) = self.stack.get(idx).copied()?;
        self.stack.truncate(idx);
        Some([open_span, close_span])
    }
}

fn ascii_run(text: &str, keep: impl Fn(char) -> bool) -> usize {
    text.chars()
        .take_while(|&c| keep(c))
        .map(char::len_utf8)
        .sum()
}

fn is_tag_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':')
}
