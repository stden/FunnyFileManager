//! Resolve a confirmed Windows volume and run the selected native maintenance operation.

use crate::models::{VolumeOperation, VolumeTarget};
use std::{io, path::Path};

#[cfg(test)]
mod tests;
#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::{resolve, run};

#[cfg(not(windows))]
pub fn resolve(_: &Path) -> io::Result<VolumeTarget> {
    Err(unsupported())
}

#[cfg(not(windows))]
pub fn run(_: VolumeOperation, _: &VolumeTarget) -> io::Result<String> {
    Err(unsupported())
}

#[cfg(not(windows))]
fn unsupported() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "Volume maintenance requires Windows",
    )
}

#[cfg(any(windows, test))]
fn verify_target(confirmed: &VolumeTarget, current: &VolumeTarget) -> io::Result<()> {
    if confirmed != current {
        return Err(io::Error::other("The volume changed after confirmation"));
    }
    Ok(())
}

#[cfg(any(windows, test))]
fn command(program: &Path, operation: VolumeOperation, mount: &Path) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    let option = match operation {
        VolumeOperation::Trim => "/L",
        VolumeOperation::Defragment => "/D",
    };
    command
        .arg(mount)
        .args([option, "/U"])
        .stdin(std::process::Stdio::null());
    command
}

#[cfg(any(windows, test))]
fn report(success: bool, status: &str, stdout: &str, stderr: &str) -> io::Result<String> {
    let text = [stdout.trim(), stderr.trim()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if success {
        return Ok(text);
    }
    Err(io::Error::other(
        format!("defrag.exe failed ({status})\n{text}")
            .trim_end()
            .to_owned(),
    ))
}
