//! Purpose: Far-format file description files (`Descript.ion`): read once per
//! listing for the Descriptions columns, rewritten line by line by `Ctrl+Z`.
//!
//! One rule finds the file for every reader and the writer: the listing (and
//! so the Descriptions sort), the Descriptions views and `Ctrl+Z` all go
//! through [`existing_file`], which ignores letter case as Far does.
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

/// The description file `dir` holds: the first of `names`, in their order,
/// present there. Names match without regard to ASCII case, as in Far, so
/// `descript.ion`, `Descript.ion` and `DESCRIPT.ION` are the same file on a
/// case-sensitive filesystem too; the exact spelling wins a tie.
fn existing_file(dir: &Path, names: &[String]) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let rank = search_rank(names, entry.file_name().to_str()?)?;
            Some((rank, entry.path()))
        })
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, path)| path)
}

/// Where `file` falls in the search order: the index of the name it
/// matches, then whether its spelling differs from that name.
fn search_rank(names: &[String], file: &str) -> Option<(usize, bool)> {
    names
        .iter()
        .enumerate()
        .find(|(_, name)| name.eq_ignore_ascii_case(file))
        .map(|(index, name)| (index, name != file))
}

/// The text of the description file at `path`, empty when there is none.
///
/// Anything but a regular file is refused before it is opened: opening a
/// FIFO blocks until something writes to it, and a listing runs on the
/// event loop.
fn text_of(path: &Path) -> io::Result<String> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(String::new()),
        Err(error) => return Err(error),
    };
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the description file is not a regular file",
        ));
    }
    Ok(String::from_utf8_lossy(&fs::read(path)?).into_owned())
}

/// Every description `dir` carries, keyed by file name; empty when none of
/// `names` exists there. A description file that exists but cannot be read
/// is an error, for views that report it.
pub fn try_read(dir: &Path, names: &[String]) -> io::Result<HashMap<String, String>> {
    let Some(path) = existing_file(dir, names) else {
        return Ok(HashMap::new());
    };
    Ok(parse(&text_of(&path)?))
}

/// [`try_read`] for a listing, which must never fail over its descriptions:
/// a file that cannot be read gives no descriptions.
#[must_use]
pub fn read(dir: &Path, names: &[String]) -> HashMap<String, String> {
    try_read(dir, names).unwrap_or_default()
}

/// The `(name, description)` pairs in `text`, in file order; malformed lines
/// (no name, or an unterminated quote) are skipped rather than guessed at.
#[must_use]
pub fn parse(text: &str) -> HashMap<String, String> {
    text.lines().filter_map(parse_line).collect()
}

fn parse_line(line: &str) -> Option<(String, String)> {
    let (name, description) = parse_line_slices(line)?;
    Some((name.to_owned(), description.to_owned()))
}

fn parse_line_slices(line: &str) -> Option<(&str, &str)> {
    let line = line.trim_end_matches(['\r', '\u{4}']);
    let (name, rest) = if let Some(quoted) = line.strip_prefix('"') {
        quoted.split_once('"')?
    } else {
        line.split_once(char::is_whitespace).unwrap_or((line, ""))
    };
    if name.is_empty() {
        return None;
    }
    Some((name, rest.trim()))
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
/// empty `text` removes the line. The file written is the one the directory
/// already has, found as [`existing_file`] finds it, else the first name.
pub fn set_description(dir: &Path, names: &[String], name: &str, text: &str) -> io::Result<()> {
    let Some(first) = names.first() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no description file name configured",
        ));
    };
    let path = existing_file(dir, names).unwrap_or_else(|| dir.join(first));
    let existing = text_of(&path)?;
    let mut lines: Vec<String> = existing
        .lines()
        .filter(|line| {
            parse_line_slices(line).is_none_or(|(existing_name, _)| existing_name != name)
        })
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
