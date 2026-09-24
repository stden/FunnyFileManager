//! Purpose: `Input::from_args` — the command-line flags that choose live input, a recording, or a replay.

use std::{
    cell::RefCell,
    io,
    path::{Path, PathBuf},
    rc::Rc,
};

use super::{default_path, Header, Input, Recorder, Replay};

impl Input {
    pub(crate) fn from_args(args: &[std::ffi::OsString], size: (u16, u16)) -> io::Result<Self> {
        match args {
            // Recording is opt-in. A session file holds every character that
            // was typed or pasted, so a plain launch writes none — the same
            // rule the diagnostics log already follows.
            [] => Ok(Self::Live(None)),
            [flag] if flag == "--no-record" => Ok(Self::Live(None)),
            [flag] if flag == "--record" => Ok(Self::recording(default_path(), size)),
            [flag, path] if flag == "--record" => {
                Ok(Self::recording(Ok(PathBuf::from(path)), size))
            }
            [flag, path] if flag == "--replay" => {
                Replay::load(Path::new(path), size).map(Self::Replay)
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
        reason = "runs before the TUI takes the screen, so stderr is where the user reads it"
    )]
    fn recording(path: io::Result<PathBuf>, size: (u16, u16)) -> Self {
        path.and_then(|path| Self::open_recorder(&path, size))
            .unwrap_or_else(|error| {
                eprintln!("ffm: recording disabled: {error}");
                Self::Live(None)
            })
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
