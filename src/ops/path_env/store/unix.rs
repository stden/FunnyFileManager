use crate::path_env::{Scope, Snapshot};
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

mod text;
use text::{assignment, profile_block, replace};
/// Why the user's PATH has no profile to live in.
const NO_HOME: &str = "No home folder: HOME is not set to an absolute path";

#[derive(Clone, Copy)]
enum Format {
    Profile,
    Environment,
    Paths,
}

pub fn read(scope: Scope) -> io::Result<Snapshot> {
    let (path, format) = location(scope)?;
    let inherited = if scope == Scope::User {
        std::env::var("PATH").unwrap_or_default()
    } else {
        String::new()
    };
    read_at(&path, format, &inherited)
}

pub fn write(scope: Scope, previous: &Snapshot, value: &str) -> io::Result<()> {
    let (path, format) = location(scope)?;
    write_expanded_at(&path, format, previous, value).map_err(|error| refused(scope, &path, error))
}

/// A refused save names the PATH file, not the temp file beside it that
/// the kernel refused (`Permission denied (os error 13) at path
/// "/etc/.tmp9N39xw"`), and says what the PATH of all users needs.
fn refused(scope: Scope, path: &Path, error: io::Error) -> io::Error {
    if error.kind() != io::ErrorKind::PermissionDenied {
        return error;
    }
    let rights = if scope == Scope::System {
        super::NEEDS_ADMINISTRATOR
    } else {
        ""
    };
    io::Error::new(
        error.kind(),
        format!("{}: permission denied{rights}", path.display()),
    )
}

fn write_expanded_at(
    path: &Path,
    format: Format,
    previous: &Snapshot,
    value: &str,
) -> io::Result<()> {
    let expanded = value
        .split(':')
        .filter(|part| !part.is_empty())
        .map(|part| {
            super::super::validation::expand(part, |name| std::env::var(name).ok())
                .ok_or_else(|| io::Error::other("Unknown PATH variable"))
        })
        .collect::<io::Result<Vec<_>>>()?
        .join(":");
    write_at(path, format, previous, &expanded)
}

fn location(scope: Scope) -> io::Result<(PathBuf, Format)> {
    if scope == Scope::System {
        return Ok(if cfg!(target_os = "macos") {
            ("/etc/paths".into(), Format::Paths)
        } else {
            ("/etc/environment".into(), Format::Environment)
        });
    }
    let user_home = crate::config::user_home().ok_or_else(|| io::Error::other(NO_HOME))?;
    let shell = std::env::var("SHELL").unwrap_or_default();
    Ok((user_profile(&user_home, &shell), Format::Profile))
}

fn user_profile(user_home: &Path, shell: &str) -> PathBuf {
    let name = if shell.ends_with("zsh") {
        ".zprofile"
    } else {
        [".bash_profile", ".bash_login"]
            .into_iter()
            .find(|name| user_home.join(name).is_file())
            .unwrap_or(".profile")
    };
    user_home.join(name)
}

/// The file's text, empty when there is none; never a FIFO's, which would wait.
fn contents(path: &Path) -> io::Result<String> {
    match crate::fileload::read_small_file(path, &path.display().to_string()) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error),
    }
}

fn read_at(path: &Path, format: Format, inherited: &str) -> io::Result<Snapshot> {
    let text = contents(path)?;
    let value = match format {
        Format::Paths => text
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect::<Vec<_>>()
            .join(":"),
        Format::Environment => assignment(&text).unwrap_or_default(),
        Format::Profile => profile_block(&text)?
            .and_then(assignment)
            .unwrap_or_else(|| inherited.to_owned()),
    };
    Ok(Snapshot {
        value,
        revision: text,
        location: path.display().to_string(),
    })
}

fn write_at(path: &Path, format: Format, previous: &Snapshot, value: &str) -> io::Result<()> {
    let current = contents(path)?;
    if current != previous.revision {
        return Err(io::Error::other(super::CHANGED_OUTSIDE));
    }
    if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(io::Error::other(
            "Edit the profile's real file, not a symbolic link",
        ));
    }
    let text = replace(&current, format, value)?;
    let mode = fs::metadata(path).map_or(0o644, |meta| meta.permissions().mode());
    if path.exists() {
        replace_file(&path.with_extension("ffm-path.bak"), &current, mode)?;
    }
    replace_file(path, &text, mode)
}

fn replace_file(path: &Path, text: &str, mode: u32) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("PATH file has no parent"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(text.as_bytes())?;
    temporary
        .as_file()
        .set_permissions(fs::Permissions::from_mode(mode))?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
use text::START;
#[cfg(test)]
mod tests;
