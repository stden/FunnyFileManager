//! Purpose: Archives (`Shift+F1` / `Shift+F2` / `Shift+F3`): the three dialogs share the `tar`, `zip` and `unzip` runs, so the runs live here once.
//!
//! The tools are the platform's own. They run without a shell — every path
//! reaches the tool as one argument, whatever is in it — and the line echoed
//! in the console transcript is the shell command that would do the same, so
//! what the user reads is what ran. Nothing here reads a dialog's keys, that
//! is `popup`'s, and nothing draws.

use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::{Command, Output};

use crate::models::{App, ArchiveMenu, ArchivePrompt, ExtractPrompt, Overlay, ARCHIVE_SUFFIXES};
use crate::status::Status;

mod add;
mod commands;
mod extract;

pub use add::{add, command_line, create_archive, offered_formats, on_path, ArchiveRequest};
pub use commands::run_command;
pub use extract::{extract_archive, run_extract};

/// Shift+F1: open the Add to archive dialog for the marked entries or the one
/// under the cursor.
pub fn open_add(app: &mut App) {
    let panel = app.get_active_panel_ref();
    let sources = panel.action_entry_names();
    if sources.is_empty() {
        app.set_status(Status::Error(
            "Add to archive: nothing to archive".to_owned(),
        ));
        return;
    }
    let single_is_dir = sources.len() == 1
        && panel
            .get_selected_entry()
            .is_some_and(|entry| entry.name == sources[0] && entry.kind.is_dir());
    let folder_name = panel.path.file_name().map_or_else(
        || "archive".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let stem = ArchivePrompt::suggest_stem(&sources, single_is_dir, &folder_name);
    let other = app.get_inactive_panel_ref().path.clone();
    let prompt = ArchivePrompt::new(sources, &stem, &other, offered_formats());
    app.set_overlay(Overlay::ArchivePrompt(Box::new(prompt)));
}

/// Shift+F2: open the Extract files dialog for the selected archives,
/// offering the other panel as the destination.
///
/// Opens even when nothing selected is an archive: the dialog then says so
/// with its Extract button dimmed, which tells the user more than a status
/// line they may not be looking at.
pub fn open_extract(app: &mut App) {
    let panel = app.get_active_panel_ref();
    let archives = panel
        .action_entry_names()
        .into_iter()
        .filter(|name| is_archive(name))
        .map(|name| panel.path.join(name))
        .collect();
    let destination = app.get_inactive_panel_ref().path.clone();
    app.set_overlay(Overlay::ExtractPrompt(Box::new(ExtractPrompt::new(
        archives,
        &destination,
    ))));
}

/// Shift+F3: open the Archive commands menu.
pub fn open_commands(app: &mut App) {
    app.set_overlay(Overlay::ArchiveCommands(ArchiveMenu::default()));
}

/// Whether `name` ends in one of the suffixes the dialogs read.
///
/// Case-insensitive, and the suffix must follow a stem: `.zip` on its own is
/// a dotfile, not an archive.
#[must_use]
pub fn is_archive(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    ARCHIVE_SUFFIXES
        .iter()
        .any(|suffix| lower.len() > suffix.len() && lower.ends_with(suffix))
}

/// What `archive` holds, one line per member as its reader prints it —
/// `tar -tvf` for the tar family, `unzip -l` for a zip.
pub fn list_archive(archive: &Path) -> io::Result<Vec<String>> {
    let text = Tool::for_archive(archive)?.listing(archive).text()?;
    Ok(text.lines().map(str::to_owned).collect())
}

/// Read `archive` end to end without unpacking anything.
///
/// A truncated stream, a member whose checksum no longer matches, or a file
/// that is not what its name says all come back as the reader's own words.
pub fn test_archive(archive: &Path) -> io::Result<()> {
    Tool::for_archive(archive)?.check(archive).text().map(drop)
}

/// Which program reads an archive: `tar` reads every compression of its own
/// family by looking at the stream, so the tar rows need no per-extension
/// flags; a zip is `unzip`'s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tool {
    Tar,
    Unzip,
}

impl Tool {
    fn for_archive(archive: &Path) -> io::Result<Self> {
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
    fn names(self, archive: &Path) -> Run {
        self.run(
            match self {
                Self::Tar => "-tf",
                Self::Unzip => "-Z1",
            },
            archive,
        )
    }

    /// The members with their modes, one per line.
    fn details(self, archive: &Path) -> Run {
        self.run(
            match self {
                Self::Tar => "-tvf",
                Self::Unzip => "-Z",
            },
            archive,
        )
    }

    /// The listing shown to the user.
    fn listing(self, archive: &Path) -> Run {
        self.run(
            match self {
                Self::Tar => "-tvf",
                Self::Unzip => "-l",
            },
            archive,
        )
    }

    /// Read everything, write nothing.
    fn check(self, archive: &Path) -> Run {
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
    fn unpack(self, archive: &Path, destination: &Path) -> Run {
        let (flags, into) = match self {
            Self::Tar => ("-xf", "-C"),
            Self::Unzip => ("-oq", "-d"),
        };
        Run::new(
            self.program(),
            [
                OsString::from(flags),
                archive.into(),
                OsString::from(into),
                destination.into(),
            ],
        )
    }

    fn run(self, flags: &str, archive: &Path) -> Run {
        Run::new(self.program(), [OsString::from(flags), archive.into()])
    }
}

/// One external run: the program and its arguments.
struct Run {
    program: &'static str,
    args: Vec<OsString>,
}

impl Run {
    fn new(program: &'static str, args: impl IntoIterator<Item = OsString>) -> Self {
        Self {
            program,
            args: args.into_iter().collect(),
        }
    }

    /// The shell line that stands for this run in the console transcript.
    fn command_line(&self) -> String {
        std::iter::once(self.program.to_owned())
            .chain(self.args.iter().map(|arg| quote(&arg.to_string_lossy())))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Run in `cwd`, bounded by the same time and output limits as any other
    /// captured command; a program that is not there is an error, not a hang.
    /// The error already says which step failed: the temp files, or
    /// starting the program.
    fn execute(&self, cwd: &Path) -> io::Result<Output> {
        let mut command = Command::new(self.program);
        command.args(&self.args).current_dir(cwd);
        super::menu::process::capture(&mut command)
    }

    /// Stdout on success; the tool's first complaint as the error otherwise.
    fn text(&self) -> io::Result<String> {
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
fn complaint(output: &Output) -> String {
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

/// `argument` as one word of the platform shell, for the transcript.
///
/// A plain word passes as it is. Anything else is single-quoted on Unix,
/// where the quotes protect everything but themselves, and double-quoted for
/// `cmd`, which knows no other kind — and a Windows name cannot hold one.
fn quote(argument: &str) -> String {
    let plain = !argument.is_empty()
        && argument
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "-_./=:+,@%".contains(ch));
    if plain {
        return argument.to_owned();
    }
    if cfg!(windows) {
        format!("\"{argument}\"")
    } else {
        format!("'{}'", argument.replace('\'', "'\\''"))
    }
}

/// Refuse an archive that would write outside its destination or leave a
/// link behind: a member named with `..`, an absolute path, a drive, or any
/// type other than a plain file or directory.
fn validate_listing(names: &str, details: &str) -> io::Result<()> {
    if names.lines().any(leaves_destination) {
        return Err(io::Error::other(
            "Archive contains a path outside its destination",
        ));
    }
    if details
        .lines()
        .filter_map(member_type)
        .any(|kind| !matches!(kind, '-' | 'd' | '?'))
    {
        return Err(io::Error::other(
            "Extraction supports regular files and directories only; links and special files are refused",
        ));
    }
    Ok(())
}

/// Whether a listed member name reaches outside the destination: it is
/// absolute, or one of its steps is `..`.
///
/// Both `/` and `\\` count as separators, because `unzip` turns a DOS-made
/// archive's `\\` into `/`. That is also why a backslash is not refused on
/// its own: on Unix it is an ordinary character, and `tar -tf` prints a tab,
/// a backslash or (under `LANG=C`) any non-ASCII byte as a backslash escape,
/// which never hides a `/` or a `.`.
fn leaves_destination(name: &str) -> bool {
    name.starts_with(['/', '\\'])
        || name.split(['/', '\\']).any(|step| step == "..")
        || names_a_drive_or_stream(name)
}

/// A colon on Windows starts a drive (`C:x`) or an alternate data stream
/// (`file:stream`); on Unix it is an ordinary character (`meeting 10:30`).
#[cfg(windows)]
fn names_a_drive_or_stream(name: &str) -> bool {
    name.contains(':')
}

#[cfg(not(windows))]
const fn names_a_drive_or_stream(_name: &str) -> bool {
    false
}

/// The type letter of a member line — the first of its ten-character mode.
///
/// `tar -tvf` prints nothing but member lines; `unzip -Z` wraps its rows in
/// a header and a footer, which have no mode and so answer `None`. A `?` is a
/// zip member stored without type bits, which `unzip` writes as a file.
fn member_type(line: &str) -> Option<char> {
    let mode = line.get(..10)?;
    let shaped = line.as_bytes().get(10).is_none_or(|byte| *byte == b' ')
        && mode.chars().all(|ch| "-dlhbcps?rwxsStT".contains(ch));
    shaped.then(|| mode.chars().next()).flatten()
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_existing;
#[cfg(test)]
mod tests_extract;
