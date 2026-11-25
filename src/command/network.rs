//! Purpose: Typed arguments and usage strings for :netdrives and :ftp commands.

use super::Command;

/// What `:netdrives` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetDrivesCommand {
    Connect,
    Edit,
    List,
}

pub(super) const NETDRIVES_USAGE: &str = "netdrives expects connect|edit|list";
pub(super) const FTP_USAGE: &str = "ftp expects [ftp[s]://][user@]host[:port][/dir]";

pub(super) fn parse_netdrives(input: &str, parts: &mut std::str::SplitWhitespace<'_>) -> Command {
    match (parts.next(), parts.next()) {
        (None, _) | (Some("connect"), None) => Command::NetDrives(NetDrivesCommand::Connect),
        (Some("edit"), None) => Command::NetDrives(NetDrivesCommand::Edit),
        (Some("list"), None) => Command::NetDrives(NetDrivesCommand::List),
        _ => Command::invalid(input, NETDRIVES_USAGE),
    }
}

pub(super) fn parse_ftp(input: &str, parts: &mut std::str::SplitWhitespace<'_>) -> Command {
    match (parts.next(), parts.next()) {
        (None, _) => Command::Ftp(None),
        (Some(target), None) => Command::Ftp(Some(target.to_owned())),
        _ => Command::invalid(input, FTP_USAGE),
    }
}
