//! Purpose: Validate session dimensions and an explicit starting directory at the CLI boundary.
use ratatui::layout::Size;
use std::{ffi::OsString, io, path::PathBuf};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Profile {
    #[default]
    System,
    Isolated,
}

#[derive(Clone)]
pub(super) struct Options {
    pub root: PathBuf,
    pub size: Size,
    pub profile: Profile,
}

impl Options {
    pub(super) fn parse(args: &[OsString]) -> io::Result<Self> {
        let mut root = None;
        let mut size = Size::new(120, 40);
        let mut profile = Profile::System;
        let mut args = args.iter();
        while let Some(flag) = args.next() {
            match flag.to_str() {
                Some("--root") => root = Some(PathBuf::from(value(&mut args)?)),
                Some("--width") => size.width = dimension(value(&mut args)?, 20..=400)?,
                Some("--height") => size.height = dimension(value(&mut args)?, 8..=200)?,
                Some("--isolated") => profile = Profile::Isolated,
                _ => return Err(invalid("unknown automation option")),
            }
        }
        let root = std::fs::canonicalize(root.ok_or_else(|| invalid("--root is required"))?)?;
        if !root.is_dir() {
            return Err(invalid("the automation root must be a directory"));
        }
        Ok(Self {
            root,
            size,
            profile,
        })
    }
}

fn value<'a>(args: &mut impl Iterator<Item = &'a OsString>) -> io::Result<&'a OsString> {
    args.next()
        .ok_or_else(|| invalid("missing automation option value"))
}

fn dimension(value: &OsString, range: std::ops::RangeInclusive<u16>) -> io::Result<u16> {
    value
        .to_str()
        .and_then(|value| value.parse().ok())
        .filter(|value| range.contains(value))
        .ok_or_else(|| invalid("automation dimensions are outside the supported range"))
}

pub(super) fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
