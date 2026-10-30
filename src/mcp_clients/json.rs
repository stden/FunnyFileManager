//! Purpose: The JSON configs (`mcpServers`) that Claude Code, Junie and Kiro
//! read: finding the `ffm-mcp` entry, and setting it while every other value
//! keeps the exact text its owner wrote.
//!
//! A `serde_json::Value` round trip is not exact: `1e3` comes back `1000.0`
//! and a 24-digit integer loses its tail. So only the entry is rebuilt; every
//! other value is carried as the raw text it was read in.

mod layout;
mod tree;

use indexmap::IndexMap;
use serde_json::Value;

use super::{Args, Blocker, FoundEntry, Launch, ServerEntry, Switch, ROOT_ENV, SERVER_NAME};
use layout::{indent_unit, members, pretty, split, Member, Newline};
use tree::{new_entry, object, old_entry, parse};

const SERVERS: &str = "mcpServers";
const ENV: &str = "mcpServers.ffm-mcp.env";

/// A parser's or a printer's complaint, as the reason a file is left alone.
fn unparsable(error: impl std::fmt::Display) -> Blocker {
    Blocker::Unparsable(error.to_string())
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
