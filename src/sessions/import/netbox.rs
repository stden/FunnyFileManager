//! Purpose: Import sessions from a `.netbox` file — Far `NetBox`'s export,
//! which is `WinSCP`'s XML:
//! `<NetBox><Sessions><Session name="…"><HostName>…</HostName>…`.
//!
//! The file is untrusted input and handled as such. It is read only when it
//! is a regular file of at most [`MAX_FILE_BYTES`]; a DTD is refused, so no
//! entity is expanded and nothing outside the file is fetched; a file whose
//! elements nest more than [`MAX_DEPTH`] deep is refused before it is parsed;
//! the parser stops after [`MAX_NODES`] nodes and the import after
//! [`MAX_SESSIONS`] sessions. Every value is URL-decoded and then checked into the session's
//! own types, and an entry with any bad value is skipped with the reason.
//! A saved password is counted and left alone: it is never decoded, never
//! stored, and never shown.

use std::collections::BTreeSet;
use std::path::Path;

use roxmltree::{Document, Node, ParsingOptions};

use super::{ImportError, Imported, SkipReason, Skipped};
use crate::models::{
    Auth, HostName, KeyFile, Origin, Port, Protocol, RemotePath, Session, SessionName, UserName,
};

mod decode;
mod depth;

use decode::decode;

/// The largest `.netbox` file read, in bytes: thousands of sessions.
pub const MAX_FILE_BYTES: u64 = 1024 * 1024;
/// The most sessions one import takes.
pub const MAX_SESSIONS: usize = 1000;
/// The most XML nodes parsed — elements, text and attributes alike.
pub const MAX_NODES: u32 = 100_000;
/// The deepest element nesting parsed. An export nests four deep
/// (`NetBox` > `Sessions` > `Session` > `HostName`).
pub const MAX_DEPTH: usize = 32;

/// The sessions in the `.netbox` file at `file`.
pub fn import(file: &Path) -> Result<Imported, ImportError> {
    let text = crate::sessions::read_text(file, MAX_FILE_BYTES).map_err(ImportError::Read)?;
    parse(&text, file)
}

/// The sessions in `text`, a `.netbox` export read from `file`.
pub fn parse(text: &str, file: &Path) -> Result<Imported, ImportError> {
    if depth::nested_deeper_than(text, MAX_DEPTH) {
        return Err(ImportError::TooDeep { limit: MAX_DEPTH });
    }
    let options = ParsingOptions {
        allow_dtd: false,
        nodes_limit: MAX_NODES,
        entity_resolver: None,
    };
    let document = Document::parse_with_options(text, options).map_err(ImportError::Xml)?;
    let root = document.root_element();
    if root.tag_name().name() != "NetBox" {
        return Err(ImportError::NotNetBox);
    }
    let entries: Vec<Node<'_, '_>> = elements(root, "Sessions")
        .flat_map(|sessions| elements(sessions, "Session"))
        .collect();
    if entries.len() > MAX_SESSIONS {
        return Err(ImportError::TooManySessions {
            limit: MAX_SESSIONS,
        });
    }
    Ok(read_sessions(&entries, file))
}

/// Every `<Session>` as a session, or as a skipped entry with its reason.
fn read_sessions(entries: &[Node<'_, '_>], file: &Path) -> Imported {
    let mut imported = Imported::default();
    let mut names = BTreeSet::new();
    for entry in entries.iter().copied() {
        imported.passwords_left_out += usize::from(has_password(entry));
        match read_session(entry, file) {
            Ok(session) if names.insert(session.name.clone()) => imported.sessions.push(session),
            Ok(session) => imported
                .skipped
                .push(Skipped::new(session.name.as_str(), SkipReason::Duplicate)),
            Err(reason) => imported
                .skipped
                .push(Skipped::new(&shown_name(entry), reason)),
        }
    }
    imported
}

/// The child elements of `node` named `tag`.
fn elements<'a, 'input: 'a>(
    node: Node<'a, 'input>,
    tag: &'static str,
) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children()
        .filter(move |child| child.is_element() && child.tag_name().name() == tag)
}

/// One `<Session>` as a session, or why it cannot be one.
fn read_session(entry: Node<'_, '_>, file: &Path) -> Result<Session, SkipReason> {
    let protocol = protocol(setting(entry, "FSProtocol")?, setting(entry, "Ftps")?)?;
    let host = HostName::new(&setting(entry, "HostName")?.unwrap_or_default())?;
    let port = match setting(entry, "PortNumber")? {
        Some(text) => Port::parse(&text)?,
        None => protocol.default_port(),
    };
    let user = filled(setting(entry, "UserName")?)
        .map(|text| UserName::new(&text))
        .transpose()?;
    let remote_dir = filled(setting(entry, "RemoteDirectory")?)
        .map(|text| RemotePath::parse(&text))
        .transpose()?;
    let auth = match filled(setting(entry, "PublicKeyFile")?) {
        Some(text) => Auth::KeyFile(KeyFile::new(&text)?),
        None => Auth::Password,
    };
    let name = match entry.attribute("name") {
        Some(text) => SessionName::new(&decode(text, "name")?)?,
        None => SessionName::default_for(user.as_ref(), &host, port, protocol.default_port()),
    };
    Ok(Session {
        name,
        protocol,
        host,
        port,
        user,
        auth,
        remote_dir,
        origin: Origin::NetBoxFile { path: file.into() },
    })
}

/// The decoded text of the one child element `tag`; `None` when there is
/// none, and an error when there are two.
fn setting(entry: Node<'_, '_>, tag: &'static str) -> Result<Option<String>, SkipReason> {
    let mut found = elements(entry, tag);
    let Some(first) = found.next() else {
        return Ok(None);
    };
    if found.next().is_some() {
        return Err(SkipReason::Repeated(tag));
    }
    decode(first.text().unwrap_or_default(), tag).map(Some)
}

/// `text` unless it is blank — an empty optional setting means "not set".
fn filled(text: Option<String>) -> Option<String> {
    text.filter(|text| !text.trim().is_empty())
}

/// `WinSCP`'s protocol, written as its number or its name, with the `Ftps`
/// setting turning FTP into FTPS.
fn protocol(fs_protocol: Option<String>, ftps: Option<String>) -> Result<Protocol, SkipReason> {
    let named = fs_protocol.map(|text| text.trim().to_ascii_uppercase());
    let protocol = match named.as_deref() {
        None | Some("" | "1" | "2" | "SFTP" | "SFTP (SCP)") => Protocol::Sftp,
        Some("0" | "SCP") => Protocol::Scp,
        Some("5" | "FTP") => Protocol::Ftp,
        Some("FTPS") => Protocol::Ftps,
        Some(_) => return Err(SkipReason::UnsupportedProtocol),
    };
    let encrypted = ftps.is_some_and(|mode| !matches!(mode.trim(), "" | "0"));
    Ok(match protocol {
        Protocol::Ftp if encrypted => Protocol::Ftps,
        other => other,
    })
}

/// Whether the entry carries a saved password. Looked at, never read.
fn has_password(entry: Node<'_, '_>) -> bool {
    elements(entry, "Password")
        .any(|password| password.text().is_some_and(|text| !text.trim().is_empty()))
}

/// What a skipped entry is reported as: its name, else its host, as written.
fn shown_name(entry: Node<'_, '_>) -> String {
    let host = || elements(entry, "HostName").find_map(|node| node.text());
    entry
        .attribute("name")
        .or_else(host)
        .unwrap_or("(unnamed session)")
        .to_owned()
}

#[cfg(test)]
mod tests;
