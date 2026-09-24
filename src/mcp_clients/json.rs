//! Purpose: The JSON configs (`mcpServers`) that Claude Code, Junie and Kiro
//! read: finding the `ffm-mcp` entry, and setting it while every other value
//! keeps the exact text its owner wrote.
//!
//! A `serde_json::Value` round trip is not exact: `1e3` comes back `1000.0`
//! and a 24-digit integer loses its tail. So only the entry is rebuilt; every
//! other value is carried as the raw text it was read in.

use indexmap::IndexMap;
use serde::Serialize;
use serde_json::{ser::PrettyFormatter, value::RawValue, Map, Value};

use super::{
    drops_launch_key, Args, Blocker, FoundEntry, Launch, ServerEntry, Switch, LAUNCH_KEYS,
    ROOT_ENV, SERVER_NAME,
};

const SERVERS: &str = "mcpServers";
const TOP_LEVEL: &str = "the top level";
const ENTRY: &str = "mcpServers.ffm-mcp";
const ENV: &str = "mcpServers.ffm-mcp.env";
const BYTE_ORDER_MARK: char = '\u{feff}';

/// Whether the file started with a byte-order mark, which goes back on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bom {
    Present,
    Absent,
}

impl Bom {
    const fn prefix(self) -> &'static str {
        match self {
            Self::Present => "\u{feff}",
            Self::Absent => "",
        }
    }
}

/// The line ending the file uses, which the rewrite keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Newline {
    Lf,
    CrLf,
}

impl Newline {
    fn of(body: &str) -> Self {
        if body.contains("\r\n") {
            Self::CrLf
        } else {
            Self::Lf
        }
    }

    fn apply(self, text: &str) -> String {
        match self {
            Self::Lf => text.to_owned(),
            Self::CrLf => text.replace("\r\n", "\n").replace('\n', "\r\n"),
        }
    }
}

/// The indentation of the file's first indented key, which the rewrite
/// uses so that the kept values, whose lines keep theirs, still line up; two
/// spaces when there is none to copy.
fn indent_unit(body: &str) -> &str {
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
fn pretty(root: &IndexMap<String, Member<'_>>, indent: &str) -> Result<String, Blocker> {
    let mut out = Vec::new();
    let formatter = PrettyFormatter::with_indent(indent.as_bytes());
    let mut serializer = serde_json::Serializer::with_formatter(&mut out, formatter);
    // Strings, raw JSON and a `Value` always serialize, as UTF-8; were one
    // not to, the file would be left alone rather than written half-done.
    root.serialize(&mut serializer).map_err(unparsable)?;
    String::from_utf8(out).map_err(unparsable)
}

/// A parser's or a printer's complaint, as the reason a file is left alone.
fn unparsable(error: impl std::fmt::Display) -> Blocker {
    Blocker::Unparsable(error.to_string())
}

/// The file's byte-order mark, and the JSON after it.
fn split(text: &str) -> (Bom, &str) {
    text.strip_prefix(BYTE_ORDER_MARK)
        .map_or((Bom::Absent, text), |rest| (Bom::Present, rest))
}

/// The top-level object. An empty body is a config nobody has written to yet.
fn parse(body: &str) -> Result<Map<String, Value>, Blocker> {
    if body.trim().is_empty() {
        return Ok(Map::new());
    }
    match serde_json::from_str(body) {
        Ok(Value::Object(root)) => Ok(root),
        Ok(_) => Err(Blocker::UnexpectedLayout(TOP_LEVEL)),
        Err(error) => Err(unparsable(error)),
    }
}

/// The object under `key`, `None` when absent.
fn object<'a>(
    parent: &'a Map<String, Value>,
    key: &str,
    layout: &'static str,
) -> Result<Option<&'a Map<String, Value>>, Blocker> {
    match parent.get(key) {
        None => Ok(None),
        Some(Value::Object(child)) => Ok(Some(child)),
        Some(_) => Err(Blocker::UnexpectedLayout(layout)),
    }
}

/// The object under `key`, created empty when absent.
fn object_mut<'a>(
    parent: &'a mut Map<String, Value>,
    key: &str,
    layout: &'static str,
) -> Result<&'a mut Map<String, Value>, Blocker> {
    match parent
        .entry(key)
        .or_insert_with(|| Value::Object(Map::new()))
    {
        Value::Object(child) => Ok(child),
        _ => Err(Blocker::UnexpectedLayout(layout)),
    }
}

/// The `ffm-mcp` object under `mcpServers`, `None` when either is absent.
fn old_entry(root: &Map<String, Value>) -> Result<Option<&Map<String, Value>>, Blocker> {
    let Some(servers) = object(root, SERVERS, SERVERS)? else {
        return Ok(None);
    };
    object(servers, SERVER_NAME, ENTRY)
}

/// The `ffm-mcp` entry `text` holds, or why the file cannot be used.
pub(super) fn entry(text: &str) -> Result<Option<FoundEntry>, Blocker> {
    let root = parse(split(text).1)?;
    let Some(entry) = old_entry(&root)? else {
        return Ok(None);
    };
    let env = object(entry, "env", ENV)?;
    Ok(Some(FoundEntry {
        command: entry
            .get("command")
            .and_then(Value::as_str)
            .map(str::to_owned),
        args: args(entry.get("args")),
        root: env
            .and_then(|env| env.get(ROOT_ENV))
            .and_then(Value::as_str)
            .map(str::to_owned),
        launch: Launch::read(|key| entry.get(key).map(Value::as_str)),
        switch: Switch::read(|key| entry.get(key).and_then(Value::as_bool)),
    }))
}

fn args(value: Option<&Value>) -> Args {
    match value {
        None => Args::Empty,
        Some(Value::Array(items)) if items.is_empty() => Args::Empty,
        Some(_) => Args::Given,
    }
}

/// The entry a registration leaves: the old one, when there is one, with
/// `desired` set and every other key kept where it was.
fn new_entry(
    root: &Map<String, Value>,
    desired: &ServerEntry,
) -> Result<Map<String, Value>, Blocker> {
    let mut entry = old_entry(root)?.cloned().unwrap_or_default();
    // Only what differs is set, so a key that already says the right thing
    // keeps its place and its spelling.
    if entry.get("command").and_then(Value::as_str) != Some(desired.command.as_str()) {
        entry.insert(
            "command".to_owned(),
            Value::String(desired.command.as_str().to_owned()),
        );
    }
    if !matches!(entry.get("args"), Some(Value::Array(items)) if items.is_empty()) {
        entry.insert("args".to_owned(), Value::Array(Vec::new()));
    }
    for key in LAUNCH_KEYS {
        if entry
            .get(key)
            .is_some_and(|value| drops_launch_key(key, value.as_str()))
        {
            entry.shift_remove(key);
        }
    }
    set_root(object_mut(&mut entry, "env", ENV)?, desired);
    Ok(entry)
}

/// `FFM_MCP_ROOT` as the scope asks: the folder, or absent.
fn set_root(env: &mut Map<String, Value>, desired: &ServerEntry) {
    match &desired.root {
        Some(root) => {
            env.insert(ROOT_ENV.to_owned(), Value::String(root.as_str().to_owned()));
        }
        None => {
            env.shift_remove(ROOT_ENV);
        }
    }
}

/// One value of the rewritten file.
#[derive(Serialize)]
#[serde(untagged)]
enum Member<'a> {
    /// Somebody else's value, written back in the exact text it was read in.
    Kept(&'a RawValue),
    /// `mcpServers`, rebuilt around the entry.
    Servers(IndexMap<String, Self>),
    /// The `ffm-mcp` entry.
    Entry(Map<String, Value>),
}

/// An object's members in file order, each value kept as its text.
fn members(json: &str) -> Result<IndexMap<String, Member<'_>>, Blocker> {
    if json.trim().is_empty() {
        return Ok(IndexMap::new());
    }
    let raw: IndexMap<String, &RawValue> = serde_json::from_str(json).map_err(unparsable)?;
    Ok(raw
        .into_iter()
        .map(|(key, value)| (key, Member::Kept(value)))
        .collect())
}

/// `text` with the `ffm-mcp` entry set to `desired`.
pub(super) fn upsert(text: &str, desired: &ServerEntry) -> Result<String, Blocker> {
    let (bom, body) = split(text);
    // The checks and the entry come from the parsed tree; the text around the
    // entry comes from the raw members.
    let entry = new_entry(&parse(body)?, desired)?;
    let mut root = members(body)?;
    let mut servers = match root.get(SERVERS) {
        Some(&Member::Kept(raw)) => members(raw.get())?,
        _ => IndexMap::new(),
    };
    servers.insert(SERVER_NAME.to_owned(), Member::Entry(entry));
    root.insert(SERVERS.to_owned(), Member::Servers(servers));
    let text = pretty(&root, indent_unit(body))?;
    let written = Newline::of(body).apply(&format!("{text}\n"));
    Ok(format!("{}{written}", bom.prefix()))
}

/// The entry [`upsert`] would write into `text`, as one line of JSON.
pub(super) fn entry_text(text: &str, desired: &ServerEntry) -> Result<String, Blocker> {
    let entry = new_entry(&parse(split(text).1)?, desired)?;
    Ok(Value::Object(entry).to_string())
}
