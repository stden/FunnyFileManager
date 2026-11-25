//! Purpose: Parse user-entered FTP/FTPS addresses into validated Session values without passwords.

use super::{Field, HostName, InvalidField, Port, Problem, Protocol, Session, UserName};
use crate::values::remote_path::RemotePath;

/// Parse `[ftp[s]://][user@]host[:port][/dir]` into a [`Session`] (never persisting passwords).
pub fn parse_ftp_target(raw: &str) -> Result<Session, InvalidField> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(InvalidField::new(Field::Host, Problem::Empty));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(InvalidField::new(Field::Host, Problem::ControlCharacter));
    }
    let (protocol, rest) = parse_scheme(trimmed)?;
    let (authority, remote_dir) = split_authority_and_dir(rest)?;
    let (user, hostport) = split_user_and_host(authority)?;
    let (host_str, port) = split_host_and_port(hostport, protocol.default_port())?;
    let host = HostName::new(host_str)?;
    let mut session = Session::new(protocol, host, port, user);
    session.remote_dir = remote_dir;
    Ok(session)
}

fn parse_scheme(input: &str) -> Result<(Protocol, &str), InvalidField> {
    let Some((scheme, rest)) = input.split_once("://") else {
        return Ok((Protocol::Ftp, input));
    };
    if scheme.eq_ignore_ascii_case("ftp") {
        Ok((Protocol::Ftp, rest))
    } else if scheme.eq_ignore_ascii_case("ftps") {
        Ok((Protocol::Ftps, rest))
    } else {
        Err(InvalidField::new(Field::Host, Problem::UnsupportedScheme))
    }
}

fn split_authority_and_dir(rest: &str) -> Result<(&str, Option<RemotePath>), InvalidField> {
    let Some((authority, path)) = rest.split_once('/') else {
        return Ok((rest, None));
    };
    if path.is_empty() {
        return Ok((authority, None));
    }
    let parsed = RemotePath::parse(&format!("/{path}"))?;
    let remote_dir = (!parsed.is_root()).then_some(parsed);
    Ok((authority, remote_dir))
}

fn split_user_and_host(authority: &str) -> Result<(Option<UserName>, &str), InvalidField> {
    let Some((userinfo, hostport)) = authority.rsplit_once('@') else {
        return Ok((None, authority));
    };
    if userinfo.contains(':') {
        return Err(InvalidField::new(Field::User, Problem::EmbeddedPassword));
    }
    Ok((Some(UserName::new(userinfo)?), hostport))
}

fn split_host_and_port(hostport: &str, default_port: Port) -> Result<(&str, Port), InvalidField> {
    if let Some(after_open) = hostport.strip_prefix('[') {
        let Some((host, tail)) = after_open.split_once(']') else {
            return Err(InvalidField::new(Field::Host, Problem::Character('[')));
        };
        if let Some(port_str) = tail.strip_prefix(':') {
            return Ok((host, Port::parse(port_str)?));
        }
        return match tail.chars().next() {
            None => Ok((host, default_port)),
            Some(bad) => Err(InvalidField::new(Field::Host, Problem::Character(bad))),
        };
    }
    if let Some((host, port_str)) = hostport
        .split_once(':')
        .filter(|(_, right)| !right.contains(':'))
    {
        return Ok((host, Port::parse(port_str)?));
    }
    Ok((hostport, default_port))
}

#[cfg(test)]
mod tests;
