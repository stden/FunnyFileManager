//! Purpose: The parsed JSON tree: the checks on its shape, the `ffm-mcp`
//! entry it holds, and the entry a registration leaves in its place.

use serde_json::{Map, Value};

use super::{unparsable, ENV, SERVERS};
use crate::mcp_clients::{
    drops_launch_key, Blocker, ServerEntry, LAUNCH_KEYS, ROOT_ENV, SERVER_NAME,
};

const TOP_LEVEL: &str = "the top level";
const ENTRY: &str = "mcpServers.ffm-mcp";

/// The top-level object. An empty body is a config nobody has written to yet.
pub(super) fn parse(body: &str) -> Result<Map<String, Value>, Blocker> {
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
pub(super) fn object<'a>(
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
pub(super) fn old_entry(root: &Map<String, Value>) -> Result<Option<&Map<String, Value>>, Blocker> {
    let Some(servers) = object(root, SERVERS, SERVERS)? else {
        return Ok(None);
    };
    object(servers, SERVER_NAME, ENTRY)
}

/// The entry a registration leaves: the old one, when there is one, with
/// `desired` set and every other key kept where it was.
pub(super) fn new_entry(
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
