//! Purpose: Parse standard MLSD or Unix LIST without splitting file names on spaces.

use crate::models::{Attributes, DirEntry, EntryKind, FileSize, Modified};
use crate::remote::RemoteError;
use suppaftp::list::{File, ListParser, PosixPexQuery};

pub(super) fn parse(lines: &[String], machine: bool) -> Result<Vec<DirEntry>, RemoteError> {
    lines
        .iter()
        .filter(|line| !line.starts_with("total "))
        .map(|line| {
            let file = if machine {
                ListParser::parse_mlsd(line)
            } else {
                line.parse::<File>()
            }
            .map_err(|_protocol_error| {
                RemoteError::Failed("FTP returned an unsupported directory listing")
            })?;
            let mut entry = entry(&file);
            if machine {
                apply_machine_mode(&mut entry, line);
            }
            Ok(entry)
        })
        .filter(|entry| match entry {
            Ok(entry) => !matches!(entry.name.as_str(), "." | ".."),
            Err(_) => true,
        })
        .collect()
}

fn entry(file: &File) -> DirEntry {
    let kind = if file.is_symlink() {
        EntryKind::SymlinkToFile
    } else if file.is_directory() {
        EntryKind::Directory
    } else {
        EntryKind::File
    };
    let mode = mode(file, kind);
    DirEntry {
        name: file.name().to_owned(),
        kind,
        size: FileSize::from_bytes(u64::try_from(file.size()).unwrap_or(u64::MAX)),
        modified: Modified::new(file.modified()),
        attributes: Attributes::from_bits(mode),
        executable: mode & 0o111 != 0,
        owner: file.uid().map(|uid| uid.to_string()),
        ..DirEntry::default()
    }
}

fn apply_machine_mode(entry: &mut DirEntry, line: &str) {
    let mode = line
        .split_once(' ')
        .and_then(|(facts, _)| {
            facts
                .split(';')
                .find_map(|fact| fact.strip_prefix("unix.mode="))
        })
        .and_then(|text| crate::remote::UnixMode::parse(text).ok());
    entry.attributes = mode.map_or_else(Attributes::default, |mode| {
        Attributes::from_bits((entry.attributes.bits() & 0o170_000) | mode.bits())
    });
}

fn mode(file: &File, kind: EntryKind) -> u32 {
    let mut mode = if kind.is_symlink() {
        0o120_000
    } else if kind.is_dir() {
        0o040_000
    } else {
        0o100_000
    };
    for (who, shift) in [
        (PosixPexQuery::Owner, 6),
        (PosixPexQuery::Group, 3),
        (PosixPexQuery::Others, 0),
    ] {
        let bits = u32::from(file.can_read(who)) * 4
            + u32::from(file.can_write(who)) * 2
            + u32::from(file.can_execute(who));
        mode |= bits << shift;
    }
    mode
}
