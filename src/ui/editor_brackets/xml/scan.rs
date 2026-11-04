//! Purpose: Stateful XML/HTML tag-pair scanner across lines for [`super::matching_xml_pair`].

mod attr;

use attr::ascii_run;
pub(super) use attr::tag_name_len;

use super::TagSpan;

#[derive(Debug, Clone, Copy)]
pub(super) struct OpenTag<'a> {
    pub(super) name: &'a str,
    pub(super) span: TagSpan,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ScanStep {
    pub(super) bytes: usize,
    pub(super) chars: usize,
    pub(super) pair: Option<[TagSpan; 2]>,
}

#[derive(Debug, Default)]
pub(super) struct XmlScanner<'a> {
    mode: Mode<'a>,
    stack: Vec<OpenTag<'a>>,
}

#[derive(Debug, Default)]
enum Mode<'a> {
    #[default]
    Text,
    Until(&'static str),
    OpenTag {
        pending: OpenTag<'a>,
        quote: Option<u8>,
    },
}

impl<'a> XmlScanner<'a> {
    pub(super) fn find_pair(
        mut self,
        lines: &'a [String],
        line: usize,
        col: usize,
    ) -> Option<[TagSpan; 2]> {
        let limit = line.saturating_add(super::super::MAX_SCAN_LINES + 1);
        let first = line.saturating_sub(super::super::MAX_SCAN_LINES);
        for (idx, text) in lines.iter().enumerate().take(limit).skip(first) {
            if idx > line && !self.has_cursor_open(lines, line, col) {
                break;
            }
            if let Some(pair) = self.scan_line(lines, idx, text, (line, col)) {
                return Some(pair);
            }
        }
        None
    }

    fn has_cursor_open(&self, lines: &[String], line: usize, col: usize) -> bool {
        self.stack
            .iter()
            .any(|open| open.span.matches_cursor(lines, line, col))
            || matches!(
                self.mode,
                Mode::OpenTag { pending, .. } if pending.span.matches_cursor(lines, line, col)
            )
    }

    fn scan_line(
        &mut self,
        lines: &'a [String],
        line_idx: usize,
        line: &'a str,
        cur: (usize, usize),
    ) -> Option<[TagSpan; 2]> {
        let (mut byte_pos, mut char_pos) = (0, 0);
        while byte_pos < line.len() {
            let rest = line.get(byte_pos..).unwrap_or_default();
            let step = match self.mode {
                Mode::Text => self.step_text(line_idx, char_pos, rest),
                Mode::Until(closer) => Self::step_until(&mut self.mode, rest, closer),
                Mode::OpenTag { .. } => self.step_open_tag(line_idx, char_pos, rest),
            };
            if step
                .pair
                .is_some_and(|pair| pair.iter().any(|s| s.matches_cursor(lines, cur.0, cur.1)))
            {
                return step.pair;
            }
            byte_pos += step.bytes;
            char_pos += step.chars;
        }
        None
    }

    fn step_until(mode: &mut Mode<'a>, rest: &'a str, closer: &str) -> ScanStep {
        if let Some(pos) = rest.find(closer) {
            *mode = Mode::Text;
            return ScanStep::utf8(rest, pos + closer.len());
        }
        ScanStep::utf8(rest, rest.len())
    }

    fn step_text(&mut self, line_idx: usize, char_pos: usize, rest: &'a str) -> ScanStep {
        if !rest.starts_with('<') {
            return ScanStep::utf8(rest, rest.find('<').unwrap_or(rest.len()));
        }
        for (prefix, closer) in [
            ("<!--", "-->"),
            ("<![CDATA[", "]]>"),
            ("<?", "?>"),
            ("<!", ">"),
        ] {
            if rest.starts_with(prefix) {
                self.mode = Mode::Until(closer);
                return ScanStep::ascii(prefix.len());
            }
        }
        if rest.starts_with("</") {
            return self.step_close_tag(line_idx, char_pos, rest);
        }
        self.step_open_tag_start(line_idx, char_pos, rest)
    }

    fn step_close_tag(&mut self, line_idx: usize, char_pos: usize, rest: &'a str) -> ScanStep {
        let after_slash = rest.get(2..).unwrap_or_default();
        let name_len = tag_name_len(after_slash);
        let after_name = after_slash.get(name_len..).unwrap_or_default();
        let ws_len = ascii_run(after_name, |c| c.is_ascii_whitespace());
        if name_len > 0 && after_name.get(ws_len..).is_some_and(|s| s.starts_with('>')) {
            let name = after_slash.get(..name_len).unwrap_or_default();
            let total = 2 + name_len + ws_len + 1;
            let close_span = TagSpan::new(line_idx, char_pos, char_pos + total);
            return ScanStep {
                bytes: total,
                chars: total,
                pair: self.pop_matching(name, close_span),
            };
        }
        ScanStep::ascii(2)
    }

    fn step_open_tag_start(&mut self, line_idx: usize, char_pos: usize, rest: &'a str) -> ScanStep {
        let after_lt = rest.get(1..).unwrap_or_default();
        let name_len = tag_name_len(after_lt);
        if name_len == 0 {
            return ScanStep::ascii(1);
        }
        let name = after_lt.get(..name_len).unwrap_or_default();
        if after_lt.get(name_len..).is_some_and(|s| s.starts_with('>')) {
            let total = 1 + name_len + 1;
            let span = TagSpan::new(line_idx, char_pos, char_pos + total);
            self.stack.push(OpenTag { name, span });
            return ScanStep::ascii(total);
        }
        let span = TagSpan::new(line_idx, char_pos, char_pos + 1 + name_len);
        self.mode = Mode::OpenTag {
            pending: OpenTag { name, span },
            quote: None,
        };
        ScanStep::ascii(1 + name_len)
    }

    fn pop_matching(&mut self, name: &str, close_span: TagSpan) -> Option<[TagSpan; 2]> {
        let idx = self.stack.iter().rposition(|open| open.name == name)?;
        let open = *self.stack.get(idx)?;
        self.stack.truncate(idx);
        Some([open.span, close_span])
    }
}

#[cfg(test)]
mod tests_edges;
