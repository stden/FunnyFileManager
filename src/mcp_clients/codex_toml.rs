//! Purpose: Codex's TOML config (`[mcp_servers.ffm-mcp]`): finding the entry,
//! and setting it with `toml_edit` so the user's comments, layout, inline
//! tables and dotted keys all survive.

use toml_edit::{Array, DocumentMut, Item, Table, TableLike, TomlError, Value};

use super::{
    drops_launch_key, Args, Blocker, FoundEntry, Launch, ServerEntry, Switch, LAUNCH_KEYS,
    ROOT_ENV, SERVER_NAME,
};

const SERVERS: &str = "mcp_servers";
const ENTRY: &str = "mcp_servers.ffm-mcp";
const ENV: &str = "mcp_servers.ffm-mcp.env";

fn parse(text: &str) -> Result<DocumentMut, Blocker> {
    text.parse::<DocumentMut>()
        .map_err(|error| Blocker::Unparsable(one_line(&error)))
}

/// The parser's reason and where it happened, on one line: its own message
/// spans five, drawing a caret under the offending column.
fn one_line(error: &TomlError) -> String {
    let reason = error.message().lines().next().unwrap_or_default().trim();
    let shown = error.to_string();
    // "TOML parse error at line 2, column 13"; without a position, nothing.
    let position = shown
        .lines()
        .next()
        .and_then(|first| first.strip_prefix("TOML parse error "))
        .unwrap_or_default();
    format!("{reason} {position}").trim_end().to_owned()
}

/// The table under `key`, `None` when absent. Inline tables and dotted keys
/// are tables too.
fn table<'a>(
    parent: &'a dyn TableLike,
    key: &str,
    layout: &'static str,
) -> Result<Option<&'a dyn TableLike>, Blocker> {
    parent.get(key).map_or(Ok(None), |item| {
        item.as_table_like()
            .map(Some)
            .ok_or(Blocker::UnexpectedLayout(layout))
    })
}

/// The `ffm-mcp` entry `text` holds, or why the file cannot be used.
pub(super) fn entry(text: &str) -> Result<Option<FoundEntry>, Blocker> {
    let document = parse(text)?;
    let Some(servers) = table(document.as_table(), SERVERS, SERVERS)? else {
        return Ok(None);
    };
    let Some(entry) = table(servers, SERVER_NAME, ENTRY)? else {
        return Ok(None);
    };
    let env = table(entry, "env", ENV)?;
    Ok(Some(FoundEntry {
        command: entry
            .get("command")
            .and_then(Item::as_str)
            .map(str::to_owned),
        args: args(entry.get("args")),
        root: env
            .and_then(|env| env.get(ROOT_ENV))
            .and_then(Item::as_str)
            .map(str::to_owned),
        launch: Launch::read(|key| entry.get(key).map(Item::as_str)),
        switch: Switch::read(|key| entry.get(key).and_then(Item::as_bool)),
    }))
}

fn args(item: Option<&Item>) -> Args {
    match item.map(Item::as_array) {
        None => Args::Empty,
        Some(Some(items)) if items.is_empty() => Args::Empty,
        Some(_) => Args::Given,
    }
}

/// The table under `key` in `parent`, created empty when absent. A new one
/// is a `[section]` inside a section and an inline table inside an inline
/// one: `toml_edit` converts it on the way in.
fn table_mut<'a>(
    parent: &'a mut dyn TableLike,
    key: &str,
    layout: &'static str,
) -> Result<&'a mut dyn TableLike, Blocker> {
    if !parent.contains_key(key) {
        parent.insert(key, Item::Table(Table::new()));
    }
    parent
        .get_mut(key)
        .and_then(Item::as_table_like_mut)
        .ok_or(Blocker::UnexpectedLayout(layout))
}

/// `text` with the `ffm-mcp` entry set to `desired`.
pub(super) fn upsert(text: &str, desired: &ServerEntry) -> Result<String, Blocker> {
    let mut document = parse(text)?;
    if !document.contains_key(SERVERS) {
        // Implicit, so the file gains `[mcp_servers.ffm-mcp]` and not an
        // empty `[mcp_servers]` above it.
        let mut servers = Table::new();
        servers.set_implicit(true);
        document.insert(SERVERS, Item::Table(servers));
    }
    let servers = table_mut(document.as_table_mut(), SERVERS, SERVERS)?;
    let entry = table_mut(servers, SERVER_NAME, ENTRY)?;
    set_command(entry, desired);
    match &desired.root {
        Some(root) => {
            let env = table_mut(entry, "env", ENV)?;
            if env.get(ROOT_ENV).and_then(Item::as_str) != Some(root.as_str()) {
                env.insert(ROOT_ENV, Item::Value(Value::from(root.as_str())));
            }
        }
        None => {
            if let Some(env) = entry.get_mut("env") {
                env.as_table_like_mut()
                    .ok_or(Blocker::UnexpectedLayout(ENV))?
                    .remove(ROOT_ENV);
            }
        }
    }
    Ok(document.to_string())
}

/// `command = <server>` and `args = []`, each only where it says otherwise,
/// so a key that is already right keeps its formatting; and no key that
/// starts the server some other way (`url`, `cwd`, a `type` but stdio).
fn set_command(entry: &mut dyn TableLike, desired: &ServerEntry) {
    if entry.get("command").and_then(Item::as_str) != Some(desired.command.as_str()) {
        entry.insert(
            "command",
            Item::Value(Value::from(desired.command.as_str())),
        );
    }
    if !matches!(entry.get("args").and_then(Item::as_array), Some(items) if items.is_empty()) {
        entry.insert("args", Item::Value(Value::Array(Array::new())));
    }
    for key in LAUNCH_KEYS {
        if entry
            .get(key)
            .is_some_and(|item| drops_launch_key(key, item.as_str()))
        {
            entry.remove(key);
        }
    }
}
