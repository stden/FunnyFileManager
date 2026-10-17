//! Purpose: Which program reads an archive (`tar` or `unzip`), the runs it is
//! asked for, and what a failed run said.

use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::{Command, Output};

use super::is_archive;
use crate::models::ARCHIVE_SUFFIXES;

/// Which program reads an archive: `tar` reads every compression of its own
/// family by looking at the stream, so the tar rows need no per-extension
/// flags; a zip is `unzip`'s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Tool {
    Tar,
    Unzip,
}

impl Tool {
    pub(super) fn for_archive(archive: &Path) -> io::Result<Self> {
        let name = archive.file_name().unwrap_or_default().to_string_lossy();
        if !is_archive(&name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("not an archive ({})", ARCHIVE_SUFFIXES.join(", ")),
            ));
        }
        Ok(if name.to_ascii_lowercase().ends_with(".zip") {
            Self::Unzip
        } else {
            Self::Tar
        })
    }

    const fn program(self) -> &'static str {
        match self {
            Self::Tar => "tar",
            Self::Unzip => "unzip",
        }
    }

    /// The member names, one per line.
    pub(super) fn names(self, archive: &Path) -> Run {
        self.run(
            match self {
                Self::Tar => "-tf",
                Self::Unzip => "-Z1",
            },
            archive,
        )
    }

    /// The members with their modes, one per line.
    pub(super) fn details(self, archive: &Path) -> Run {
        self.run(
            match self {
                Self::Tar => "-tvf",
                Self::Unzip => "-Z",
            },
            archive,
        )
    }

    /// The listing shown to the user.
    pub(super) fn listing(self, archive: &Path) -> Run {
        self.run(
            match self {
                Self::Tar => "-tvf",
                Self::Unzip => "-l",
            },
            archive,
        )
    }

    /// Read everything, write nothing.
    pub(super) fn check(self, archive: &Path) -> Run {
        self.run(
            match self {
                Self::Tar => "-tf",
                Self::Unzip => "-tq",
            },
            archive,
        )
    }

    /// Unpack into `destination`, which `extract_archive` makes an empty
    /// private folder. `unzip` still gets `-o`: without it, a member stored
    /// twice makes it ask on the terminal, which behind a TUI is a hang.
    pub(super) fn unpack(self, archive: &Path, destination: &Path) -> Run {
        let (flags, into) = match self {
            Self::Tar => ("-xf", "-C"),
            Self::Unzip => ("-oq", "-d"),
        };
        Run::new(
            self.program(),
            [
                OsString::from(flags),
                safe_archive_arg(archive).into_owned().into(),
                OsString::from(into),
                destination.into(),
            ],
        )
    }

    fn run(self, flags: &str, archive: &Path) -> Run {
        Run::new(
            self.program(),
            [
                OsString::from(flags),
                safe_archive_arg(archive).into_owned().into(),
            ],
        )
    }
}

pub(super) fn safe_archive_arg(archive: &Path) -> std::borrow::Cow<'_, Path> {
    let text = archive.as_os_str().to_string_lossy();
    if archive.is_relative() && (text.starts_with('-') || text.contains(':')) {
        std::borrow::Cow::Owned(Path::new(".").join(archive))
    } else {
        std::borrow::Cow::Borrowed(archive)
    }
}

/// One external run: the program and its arguments.
pub(super) struct Run {
    program: &'static str,
    args: Vec<OsString>,
}

impl Run {
    pub(super) fn new(program: &'static str, args: impl IntoIterator<Item = OsString>) -> Self {
        Self {
            program,
            args: args.into_iter().collect(),
        }
    }

    /// The shell line that stands for this run in the console transcript.
    pub(super) fn command_line(&self) -> String {
        std::iter::once(self.program.to_owned())
            .chain(
                self.args
                    .iter()
                    .map(|arg| crate::console::shell_word(&arg.to_string_lossy())),
            )
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Run in `cwd`, bounded by the same time and output limits as any other
    /// captured command; a program that is not there is an error, not a hang.
    /// The error already says which step failed: the temp files, or
    /// starting the program.
    pub(super) fn execute(&self, cwd: &Path) -> io::Result<Output> {
        let mut command = Command::new(self.program);
        command.args(&self.args).current_dir(cwd);
        crate::ops::menu::process::capture(&mut command)
    }

    /// Stdout on success; the tool's first complaint as the error otherwise.
    pub(super) fn text(&self) -> io::Result<String> {
        let output = self.execute(Path::new("."))?;
        if !output.status.success() {
            return Err(io::Error::other(complaint(&output)));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// The first thing a failed tool said. `unzip` complains on stdout, `tar` on
/// stderr; whichever spoke first names the fault, and a silent failure is
/// named by its exit status.
///
/// `unzip` and `zipinfo` head a complaint with the archive's own path in
/// brackets, which names no fault, so such a line is passed over for the
/// reason below it.
pub(super) fn complaint(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let is_heading = |line: &str| line.starts_with('[') && line.ends_with(']');
    stderr
        .lines()
        .chain(stdout.lines())
        .map(str::trim)
        .find(|line| !line.is_empty() && !is_heading(line))
        .map_or_else(|| output.status.to_string(), crate::console::printable)
}
