//! Purpose: The dotenv form of a `KeePass` export: one quoted variable per
//! current entry that has a password, named after its group path and title.

use keepass::db::{fields, GroupRef};
use keepass::Database;
use std::collections::HashSet;

pub(super) fn export_passwords_as_env(database: &Database) -> String {
    let mut lines = Vec::new();
    let mut used_names = HashSet::new();
    collect_passwords(
        &database.root(),
        &mut Vec::new(),
        &mut used_names,
        &mut lines,
    );
    lines.join("\n")
}

fn collect_passwords(
    group: &GroupRef<'_>,
    group_path: &mut Vec<String>,
    used_names: &mut HashSet<String>,
    lines: &mut Vec<String>,
) {
    for entry in group.entries() {
        let Some(password) = entry.get(fields::PASSWORD) else {
            continue;
        };
        let title = entry
            .get(fields::TITLE)
            .filter(|title| !title.is_empty())
            .map_or_else(|| entry.id().uuid().to_string(), str::to_owned);

        let mut parts: Vec<&str> = group_path.iter().map(String::as_str).collect();
        parts.push(&title);
        let base = env_name(&parts.join("_"));
        let variable = unique_env_name(&base, used_names);
        lines.push(format!("{variable}={}", quote_env_value(password)));
    }

    for child in group.groups() {
        group_path.push(child.name.clone());
        collect_passwords(&child, group_path, used_names, lines);
        group_path.pop();
    }
}

pub(super) fn env_name(value: &str) -> String {
    let mut name = String::new();
    let mut previous_was_separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            name.push(character.to_ascii_uppercase());
            previous_was_separator = false;
        } else if !name.is_empty() && !previous_was_separator {
            name.push('_');
            previous_was_separator = true;
        }
    }
    while name.ends_with('_') {
        name.pop();
    }
    if name.is_empty() {
        "ENTRY".to_owned()
    } else {
        name
    }
}

pub(super) fn unique_env_name(base: &str, used_names: &mut HashSet<String>) -> String {
    let first = format!("KDBX_{base}_PASSWORD");
    if used_names.insert(first.clone()) {
        return first;
    }

    let mut suffix = 2_u64;
    loop {
        let candidate = format!("KDBX_{base}_{suffix}_PASSWORD");
        if used_names.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
}

pub(super) fn quote_env_value(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for character in value.chars() {
        match character {
            '\\' => quoted.push_str("\\\\"),
            '"' => quoted.push_str("\\\""),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            '$' => quoted.push_str("\\$"),
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}
