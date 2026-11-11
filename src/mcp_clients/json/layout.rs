//! Purpose: The text around the entry a JSON rewrite keeps: every other
//! member as the raw text it was read in, and the file's byte-order mark,
//! line ends and indentation.

use indexmap::IndexMap;
use serde::Serialize;
use serde_json::{ser::PrettyFormatter, value::RawValue, Map, Value};

use super::unparsable;
use crate::mcp_clients::Blocker;

const BYTE_ORDER_MARK: char = '\u{feff}';

/// Whether the file started with a byte-order mark, which goes back on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Bom {
    Present,
    Absent,
}

impl Bom {
    pub(super) const fn prefix(self) -> &'static str {
        match self {
            Self::Present => "\u{feff}",
            Self::Absent => "",
        }
    }
}

/// The line ending the file uses, which the rewrite keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Newline {
    Lf,
    CrLf,
}

impl Newline {
    pub(super) fn of(body: &str) -> Self {
        if body.contains("\r\n") {
            Self::CrLf
        } else {
            Self::Lf
        }
    }

    pub(super) fn apply(self, text: &str) -> String {
        match self {
            Self::Lf => text.to_owned(),
            Self::CrLf => text.replace("\r\n", "\n").replace('\n', "\r\n"),
        }
    }
}

/// The indentation of the file's first indented key, which the rewrite
/// uses so that the kept values, whose lines keep theirs, still line up; two
/// spaces when there is none to copy.
pub(super) fn indent_unit(body: &str) -> &str {
    body.lines()
        .find_map(|line| {
            let key = line.trim_start_matches([' ', '\t']);
            key.starts_with('"')
                .then(|| line.strip_suffix(key))
                .flatten()
        })
        .filter(|indent| !indent.is_empty())
        .unwrap_or("  ")
}

/// `root` pretty-printed with `indent`.
pub(super) fn pretty(root: &IndexMap<String, Member<'_>>, indent: &str) -> Result<String, Blocker> {
    let mut out = Vec::new();
    let formatter = PrettyFormatter::with_indent(indent.as_bytes());
    let mut serializer = serde_json::Serializer::with_formatter(&mut out, formatter);
    // Strings, raw JSON and a `Value` always serialize, as UTF-8; were one
    // not to, the file would be left alone rather than written half-done.
    root.serialize(&mut serializer).map_err(unparsable)?;
    String::from_utf8(out).map_err(unparsable)
}

/// The file's byte-order mark, and the JSON after it.
pub(super) fn split(text: &str) -> (Bom, &str) {
    text.strip_prefix(BYTE_ORDER_MARK)
        .map_or((Bom::Absent, text), |rest| (Bom::Present, rest))
}

/// One value of the rewritten file.
#[derive(Serialize)]
#[serde(untagged)]
pub(super) enum Member<'a> {
    /// Somebody else's value, written back in the exact text it was read in.
    Kept(&'a RawValue),
    /// `mcpServers`, rebuilt around the entry.
    Servers(IndexMap<String, Self>),
    /// The `ffm-mcp` entry.
    Entry(Map<String, Value>),
}

/// An object's members in file order, each value kept as its text.
pub(super) fn members(json: &str) -> Result<IndexMap<String, Member<'_>>, Blocker> {
    if json.trim().is_empty() {
        return Ok(IndexMap::new());
    }
    let raw: IndexMap<String, &RawValue> = serde_json::from_str(json).map_err(unparsable)?;
    Ok(raw
        .into_iter()
        .map(|(key, value)| (key, Member::Kept(value)))
        .collect())
}
