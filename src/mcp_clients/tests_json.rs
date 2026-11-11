//! Unit tests for the JSON configs: reading the state of `mcpServers` and
//! setting the `ffm-mcp` entry without disturbing anything else.

use serde_json::{json, Value};

use super::tests::{all_projects, current_folder, folder, server};
use super::*;

mod tests_layout;
mod tests_shape;
mod tests_state;
mod tests_upsert;

fn json_state(text: &str, desired: &ServerEntry) -> ConfigState {
    state(ConfigFormat::Json, text, Some(desired))
}

fn upserted(text: &str, desired: &ServerEntry) -> String {
    upsert(ConfigFormat::Json, text, desired).unwrap()
}

fn parsed(text: &str) -> Value {
    serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap()
}
