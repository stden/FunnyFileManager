//! Purpose: `Shift+F1` — write the archive the Add to archive dialog asked
//! for, and the Archive commands row that adds to one that already exists.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Output;

use super::{complaint, Run};
use crate::models::{App, ArchiveFormat, ArchivePrompt, Overlay};
use crate::status::Status;

mod cleanup;
use cleanup::{delete_sources, discard_partial};

/// One archive to write: where, how, and from what.
///
/// Separated from the dialog so the run can be tested without one.
#[derive(Debug, Clone, Copy)]
pub struct ArchiveRequest<'a> {
    pub archive: &'a Path,
    pub format: ArchiveFormat,
    /// Names relative to the folder the run starts in.
    pub sources: &'a [String],
    /// The archive exists and the sources join it.
    pub append: bool,
}

/// The formats this machine can write: `tar` is assumed (every supported
/// platform ships one), `zip` only where it is on the PATH.
#[must_use]
pub fn offered_formats() -> Vec<ArchiveFormat> {
    ArchiveFormat::ALL
        .into_iter()
        .filter(|format| format.tool() != "zip" || on_path("zip"))
        .collect()
}

/// Whether `program` can be found on the PATH.
#[must_use]
pub fn on_path(program: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        let candidate = dir.join(program);
        candidate.is_file() || dir.join(format!("{program}.exe")).is_file()
    })
}

/// The accepted dialog: write the archive, tell the transcript and the status
/// bar, drop the sources if asked, and re-read both panels.
pub fn add(app: &mut App, prompt: &ArchivePrompt) {
    let cwd = app.get_active_panel_ref().path.clone();
    let archive = resolve_archive(prompt.name.text(), &cwd);
    // Never overwrite: the dialog comes back with the name to change, and
    // the status bar says why it came back. A link that points nowhere holds
    // the name too — the tool would write through it.
    let existed = crate::utils::name_is_taken(&archive);
    if existed && !prompt.append {
        app.set_status(Status::FileExists(archive.display().to_string()));
        app.set_overlay(Overlay::ArchivePrompt(Box::new(prompt.clone())));
        return;
    }
    let request = ArchiveRequest {
        archive: &archive,
        format: prompt.format,
        sources: &prompt.sources,
        append: prompt.append,
    };
    app.console.record_command(&cwd, &command_line(&request));
    let status = match create_archive(&request, &cwd) {
        Ok(output) => report(app, &request, &cwd, &output, prompt.delete_after),
        Err(error) => {
            app.console.record_error(&error.to_string());
            Status::Error(error.to_string())
        }
    };
    if !existed && !matches!(status, Status::AddedToArchive { .. }) {
        discard_partial(app, &archive);
    }
    if matches!(status, Status::AddedToArchive { kept: 0, .. }) {
        app.get_active_panel().clear_marks();
    }
    app.set_status(status);
    // A refresh failure is not the archiver's failure.
    drop(crate::utils::refresh_panels(app));
}

/// A typed name is absolute or lives in the active panel's folder, the way
/// the copy dialog resolves its destination.
fn resolve_archive(name: &str, cwd: &Path) -> PathBuf {
    let typed = Path::new(name.trim());
    if typed.is_absolute() {
        typed.to_path_buf()
    } else {
        cwd.join(typed)
    }
}

/// What a finished run says, once its output is in the transcript.
fn report(
    app: &mut App,
    request: &ArchiveRequest<'_>,
    cwd: &Path,
    output: &Output,
    delete_after: bool,
) -> Status {
    app.console
        .record_output(&String::from_utf8_lossy(&output.stdout));
    app.console
        .record_error(&String::from_utf8_lossy(&output.stderr));
    if !output.status.success() {
        return Status::Error(format!(
            "{} failed: {}",
            request.format.tool(),
            complaint(output)
        ));
    }
    let kept = if delete_after {
        delete_sources(cwd, request.sources, request.archive)
    } else {
        0
    };
    Status::AddedToArchive {
        count: request.sources.len(),
        archive: request.archive.to_owned(),
        kept,
    }
}

/// Write the archive `request` describes, running the tool in `cwd` so the
/// stored names are relative to it.
pub fn create_archive(request: &ArchiveRequest<'_>, cwd: &Path) -> io::Result<Output> {
    run(request).execute(cwd)
}

/// The shell line that stands for `request` in the transcript.
#[must_use]
pub fn command_line(request: &ArchiveRequest<'_>) -> String {
    run(request).command_line()
}

/// The tool run that writes `request`: `tar -c` or `-r` for the tar family,
/// `zip -r` for a zip, which updates an archive that is already there.
fn run(request: &ArchiveRequest<'_>) -> Run {
    let flags = match (request.format, request.append) {
        (ArchiveFormat::TarGz, _) => "-czf",
        (ArchiveFormat::Tar, false) => "-cf",
        (ArchiveFormat::Tar, true) => "-rf",
        (ArchiveFormat::Zip, _) => "-rq",
    };
    let archive = super::tool::safe_archive_arg(request.archive);
    let mut args = vec![OsString::from(flags), archive.into_owned().into()];
    // `--` so a name starting with `-` is a file, not an option.
    if request.format.tool() == "tar" || request.sources.iter().any(|s| s.starts_with('-')) {
        args.push("--".into());
    }
    args.extend(request.sources.iter().map(OsString::from));
    Run::new(request.format.tool(), args)
}
