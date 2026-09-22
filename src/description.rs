//! Purpose: Far-format file description files (`Descript.ion`): read once per
//! listing for the Descriptions columns, rewritten line by line by `Ctrl+Z`.
//!
//! The format is one entry per line — the file name, quoted when it holds a
//! space, then whitespace, then the description. Lines that name files which
//! no longer exist are kept on write, as Far does: the description outlives
//! the file so that a restored file gets its text back.

use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

/// Far's own default list, in the order it is searched.
pub const DEFAULT_NAMES: [&str; 2] = ["Descript.ion", "Files.bbs"];

/// `Settings.descriptions.names` as a fresh `Vec`, for the panel's own copy.
#[must_use]
pub fn default_names() -> Vec<String> {
    DEFAULT_NAMES
        .iter()
        .map(|name| (*name).to_owned())
        .collect()
}

/// The first description file in `names` that exists in `dir`.
fn existing_file(dir: &Path, names: &[String]) -> Option<PathBuf> {
    names
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}

/// Every description `dir` carries, keyed by file name; empty when none of
/// `names` exists there. A read failure is an empty map as well — a listing
/// must never fail over its descriptions.
#[must_use]
pub fn read(dir: &Path, names: &[String]) -> HashMap<String, String> {
    existing_file(dir, names)
        .and_then(|path| fs::read(path).ok())
        .map(|bytes| parse(&String::from_utf8_lossy(&bytes)))
        .unwrap_or_default()
}

/// The `(name, description)` pairs in `text`, in file order; malformed lines
/// (no name, or an unterminated quote) are skipped rather than guessed at.
#[must_use]
pub fn parse(text: &str) -> HashMap<String, String> {
    text.lines().filter_map(parse_line).collect()
}

fn parse_line(line: &str) -> Option<(String, String)> {
    let line = line.trim_end_matches(['\r', '\u{4}']);
    let (name, rest) = if let Some(quoted) = line.strip_prefix('"') {
        quoted.split_once('"')?
    } else {
        line.split_once(char::is_whitespace).unwrap_or((line, ""))
    };
    if name.is_empty() {
        return None;
    }
    Some((name.to_owned(), rest.trim().to_owned()))
}

/// One line as Far writes it: the name quoted when it contains a space.
fn format_line(name: &str, text: &str) -> String {
    if name.contains(char::is_whitespace) {
        format!("\"{name}\" {text}")
    } else {
        format!("{name} {text}")
    }
}

/// Give `name` the description `text` in `dir`.
///
/// The description file is rewritten with every other line untouched. An
/// empty `text` removes the line. The file written is the first of `names`
/// that exists, else the first name.
pub fn set_description(dir: &Path, names: &[String], name: &str, text: &str) -> io::Result<()> {
    let Some(first) = names.first() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no description file name configured",
        ));
    };
    let path = existing_file(dir, names).unwrap_or_else(|| dir.join(first));
    let existing = match fs::read(&path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };
    let mut lines: Vec<String> = existing
        .lines()
        .filter(|line| parse_line(line).is_none_or(|(existing_name, _)| existing_name != name))
        .map(str::to_owned)
        .collect();
    if !text.trim().is_empty() {
        lines.push(format_line(name, text.trim()));
    }
    let mut content = lines.join("\n");
    if !content.is_empty() {
        content.push('\n');
    }
    fs::write(path, content)
}

#[cfg(test)]
mod tests;
