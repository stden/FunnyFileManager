//! Purpose: CLI options and environment resolution for `--things-patch`.

use std::{
    ffi::OsString,
    io::{self, ErrorKind},
    path::PathBuf,
};

use super::super::{Config, PatchScope, ROOT_ID_VARIABLE, TOKEN_VARIABLE, URL_VARIABLE};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOptions {
    pub scope: PatchScope,
    pub hosts: Vec<String>,
    pub exclude: Vec<String>,
    pub root_id: Option<String>,
    pub token_file: Option<PathBuf>,
    pub comment: String,
    pub reboot: bool,
    pub execute: bool,
    pub wait: bool,
    pub poll_secs: u64,
    pub timeout_secs: u64,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            scope: PatchScope::All,
            hosts: Vec::new(),
            exclude: Vec::new(),
            root_id: None,
            token_file: None,
            comment: "Patch servers via ffm --things-patch".to_owned(),
            reboot: true,
            execute: false,
            wait: false,
            poll_secs: 15,
            timeout_secs: 1800,
        }
    }
}

impl CliOptions {
    pub fn parse(args: &[OsString]) -> io::Result<Self> {
        let mut options = Self::default();
        let mut iter = args.iter();
        while let Some(flag) = iter.next() {
            apply_flag(&mut options, flag, &mut iter)?;
        }
        Ok(options)
    }
}

fn apply_flag<'a>(
    options: &mut CliOptions,
    flag: &OsString,
    iter: &mut impl Iterator<Item = &'a OsString>,
) -> io::Result<()> {
    match flag.to_str() {
        Some("--scope") => options.scope = parse_scope(&next_text(iter, "--scope")?)?,
        Some("--hosts") => push_csv(&mut options.hosts, &next_text(iter, "--hosts")?),
        Some("--exclude") => push_csv(&mut options.exclude, &next_text(iter, "--exclude")?),
        Some("--root") => options.root_id = Some(next_text(iter, "--root")?),
        Some("--token-file") => {
            options.token_file = Some(PathBuf::from(next_os(iter, "--token-file")?));
        }
        Some("--comment") => options.comment = next_text(iter, "--comment")?,
        Some("--no-reboot") => options.reboot = false,
        Some("--execute") => options.execute = true,
        Some("--wait") => options.wait = true,
        Some("--poll-secs") => {
            options.poll_secs = parse_u64(&next_text(iter, "--poll-secs")?, "--poll-secs")?;
        }
        Some("--timeout-secs") => {
            options.timeout_secs =
                parse_u64(&next_text(iter, "--timeout-secs")?, "--timeout-secs")?;
        }
        _ => return Err(invalid("unknown --things-patch option")),
    }
    Ok(())
}

fn next_os<'a>(
    iter: &mut impl Iterator<Item = &'a OsString>,
    flag: &str,
) -> io::Result<&'a OsString> {
    iter.next()
        .ok_or_else(|| invalid(&format!("missing value for {flag}")))
}

fn next_text<'a>(iter: &mut impl Iterator<Item = &'a OsString>, flag: &str) -> io::Result<String> {
    next_os(iter, flag)?
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| invalid(&format!("non-UTF-8 value for {flag}")))
}

fn parse_scope(raw: &str) -> io::Result<PatchScope> {
    PatchScope::parse_str(raw)
        .ok_or_else(|| invalid("unknown --scope value; valid: all|rhel|windows"))
}

fn parse_u64(raw: &str, flag: &str) -> io::Result<u64> {
    raw.trim()
        .parse::<u64>()
        .map_err(|_parse_err| invalid(&format!("invalid integer for {flag}")))
}

fn push_csv(target: &mut Vec<String>, raw: &str) {
    for part in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        target.push(part.to_owned());
    }
}

pub fn resolve_config(options: &CliOptions) -> io::Result<Config> {
    let url = std::env::var(URL_VARIABLE).ok();
    let token = match &options.token_file {
        Some(path) => Some(std::fs::read_to_string(path)?),
        None => std::env::var(TOKEN_VARIABLE).ok(),
    };
    let root_id = options
        .root_id
        .clone()
        .or_else(|| std::env::var(ROOT_ID_VARIABLE).ok());
    Config::from_values(url, token)
        .map(|cfg| cfg.with_root_id(root_id))
        .map_err(|error| invalid(&error.to_string()))
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(ErrorKind::InvalidInput, message)
}
