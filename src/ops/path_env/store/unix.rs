use crate::path_env::{Scope, Snapshot};
use std::{
    fs, io,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

const START: &str = "# FFM PATH begin";
const END: &str = "# FFM PATH end";

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
    write_expanded_at(&path, format, previous, value)
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
    let user_home = std::env::var_os("HOME").ok_or_else(|| io::Error::other("HOME is missing"))?;
    let shell = std::env::var("SHELL").unwrap_or_default();
    Ok((user_profile(Path::new(&user_home), &shell), Format::Profile))
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

fn contents(path: &Path) -> io::Result<String> {
    match fs::read_to_string(path) {
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

fn assignment(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        line.trim()
            .strip_prefix("PATH=")
            .map(|value| unquote(value.trim()).replace("'\\''", "'"))
    })
}

fn unquote(value: &str) -> &str {
    for quote in ['\'', '"'] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner;
        }
    }
    value
}

fn profile_block(text: &str) -> io::Result<Option<&str>> {
    let block = match (text.find(START), text.find(END)) {
        (None, None) => return Ok(None),
        (Some(start), Some(end)) if start < end => text.get(start..end + END.len()),
        _ => None,
    };
    block
        .map(Some)
        .ok_or_else(|| io::Error::other("Incomplete FFM PATH block; fix the profile before saving"))
}

fn write_at(path: &Path, format: Format, previous: &Snapshot, value: &str) -> io::Result<()> {
    let current = contents(path)?;
    if current != previous.revision {
        return Err(io::Error::other(
            "PATH changed outside FFM; reopen before saving",
        ));
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

fn replace(text: &str, format: Format, value: &str) -> io::Result<String> {
    match format {
        Format::Paths => Ok(format!("{}\n", value.replace(':', "\n"))),
        Format::Profile => {
            let block = format!(
                "{START}\nPATH='{}'\nexport PATH\n{END}",
                value.replace('\'', "'\\''")
            );
            Ok(profile_block(text)?.map_or_else(
                || format!("{text}\n{block}\n"),
                |old| text.replace(old, &block),
            ))
        }
        Format::Environment => {
            if value.contains(['"', '\\', '$', '`']) {
                return Err(io::Error::other(
                    "This path cannot be represented in /etc/environment",
                ));
            }
            let mut lines: Vec<_> = text
                .lines()
                .filter(|line| !line.trim().starts_with("PATH="))
                .map(str::to_owned)
                .collect();
            lines.push(format!("PATH=\"{value}\""));
            Ok(format!("{}\n", lines.join("\n")))
        }
    }
}

#[cfg(test)]
mod tests;
