//! Purpose: `Input::from_args` — the command-line flags that choose live input, a recording, or a replay.

use std::{
    cell::RefCell,
    fmt, io,
    path::{Path, PathBuf},
    rc::Rc,
};

use super::{default_path, default_root, Header, Input, Recorder, Replay};

/// Why a `--record` the command line asked for did not start. It goes to
/// stderr at once and to the status line once the TUI is up: the TUI covers
/// the stderr line the moment it takes the screen.
#[derive(Debug)]
pub struct RecordingDisabled(io::Error);

impl fmt::Display for RecordingDisabled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "recording disabled: {}", self.0)
    }
}

impl Input {
    /// The input the command line chose, and why a recording it asked for
    /// did not start, if it did not.
    pub(crate) fn from_args(
        args: &[std::ffi::OsString],
        size: (u16, u16),
    ) -> io::Result<(Self, Option<RecordingDisabled>)> {
        Self::from_args_in(args, size, &default_root())
    }

    /// [`Self::from_args`], with a bare `--record` naming its session in
    /// `recordings` rather than in the shared temporary directory.
    pub(super) fn from_args_in(
        args: &[std::ffi::OsString],
        size: (u16, u16),
        recordings: &Path,
    ) -> io::Result<(Self, Option<RecordingDisabled>)> {
        match args {
            // Recording is opt-in. A session file holds every character that
            // was typed or pasted, so a plain launch writes none — the same
            // rule the diagnostics log already follows.
            [] => Ok((Self::Live(None), None)),
            [flag] if flag == "--no-record" => Ok((Self::Live(None), None)),
            [flag] if flag == "--record" => Ok(Self::recording(default_path(recordings), size)),
            [flag, path] if flag == "--record" => {
                Ok(Self::recording(Ok(PathBuf::from(path)), size))
            }
            [flag, path] if flag == "--replay" => {
                Replay::load(Path::new(path), size).map(|replay| (Self::Replay(replay), None))
            }
            _ => Err(io::Error::other(
                "Usage: ffm [--record [session.py] | --no-record | --replay events.jsonl]",
            )),
        }
    }

    /// Start recording, or say why not and carry on live. A recording is a
    /// diagnostic, never the job: a directory owned by another user, or a full
    /// disk, must not be what stops the file manager from starting.
    #[expect(
        clippy::print_stderr,
        reason = "runs before the TUI takes the screen; the status line says it again once the TUI is up"
    )]
    fn recording(path: io::Result<PathBuf>, size: (u16, u16)) -> (Self, Option<RecordingDisabled>) {
        match path.and_then(|path| Self::open_recorder(&path, size)) {
            Ok(input) => (input, None),
            Err(error) => {
                let disabled = RecordingDisabled(error);
                eprintln!("ffm: {disabled}");
                (Self::Live(None), Some(disabled))
            }
        }
    }

    #[expect(
        clippy::print_stderr,
        reason = "runs before the TUI takes the screen, so stderr is where the user reads it"
    )]
    pub(super) fn open_recorder(path: &Path, size: (u16, u16)) -> io::Result<Self> {
        let header = Header {
            version: 1,
            executable: std::env::current_exe()?,
            directory: std::env::current_dir()?,
            size,
        };
        let recorder = Recorder::create(path, &header)?;
        eprintln!("Recording actions: {}", path.display());
        Ok(Self::Live(Some(Rc::new(RefCell::new(recorder)))))
    }
}
