//! Purpose: The PATH files' text: the FFM block in a login profile, the
//! `PATH=` line of `/etc/environment`, and macOS's one-folder-per-line
//! `/etc/paths`, read and rewritten.

use std::io;

use super::Format;

/// The first line of the block FFM writes into a login profile.
pub(super) const START: &str = "# FFM PATH begin";
const END: &str = "# FFM PATH end";

pub(super) fn assignment(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        line.trim()
            .strip_prefix("PATH=")
            .map(|value| unquote(value.trim()).replace("'\\''", "'"))
    })
}

fn unquote(value: &str) -> &str {
    for quote in ['\'', '"'] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner;
        }
    }
    value
}

pub(super) fn profile_block(text: &str) -> io::Result<Option<&str>> {
    let block = match (text.find(START), text.find(END)) {
        (None, None) => return Ok(None),
        (Some(start), Some(end)) if start < end => text.get(start..end + END.len()),
        _ => None,
    };
    block
        .map(Some)
        .ok_or_else(|| io::Error::other("Incomplete FFM PATH block; fix the profile before saving"))
}

pub(super) fn replace(text: &str, format: Format, value: &str) -> io::Result<String> {
    match format {
        Format::Paths => Ok(format!("{}\n", value.replace(':', "\n"))),
        Format::Profile => {
            let block = format!(
                "{START}\nPATH='{}'\nexport PATH\n{END}",
                value.replace('\'', "'\\''")
            );
            Ok(profile_block(text)?.map_or_else(
                || format!("{text}\n{block}\n"),
                |old| text.replace(old, &block),
            ))
        }
        Format::Environment => {
            if value.contains(['"', '\\', '$', '`']) {
                return Err(io::Error::other(
                    "This path cannot be represented in /etc/environment",
                ));
            }
            let mut lines: Vec<_> = text
                .lines()
                .filter(|line| !line.trim().starts_with("PATH="))
                .map(str::to_owned)
                .collect();
            lines.push(format!("PATH=\"{value}\""));
            Ok(format!("{}\n", lines.join("\n")))
        }
    }
}
