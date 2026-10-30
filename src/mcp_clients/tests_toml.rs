//! Unit tests for Codex's TOML config: the same matrix as the JSON tests, plus
//! the TOML shapes a person writes by hand — comments, inline tables and
//! dotted keys.

use std::path::PathBuf;

use super::tests::{absolute, all_projects, current_folder, folder, server};
use super::*;

mod tests_shape;
mod tests_state;
mod tests_upsert;

fn toml_state(text: &str, desired: &ServerEntry) -> ConfigState {
    state(ConfigFormat::Toml, text, Some(desired))
}

fn upserted(text: &str, desired: &ServerEntry) -> String {
    upsert(ConfigFormat::Toml, text, desired).unwrap()
}

/// `text` as `toml_edit` writes a string: a literal string when it holds a
/// backslash, as a Windows path does, and a basic string otherwise. Rust's
/// `{:?}` agrees with it only on the second.
fn toml_string(text: &str) -> String {
    toml_edit::Value::from(text).to_string()
}

/// `command = <server>` as TOML writes it.
fn command_line() -> String {
    format!("command = {}", toml_string(server().as_str()))
}
