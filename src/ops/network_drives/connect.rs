//! Purpose: Connect all configured network drives from ffm.yml without blocking the TUI.

use crate::models::network_drive::{NetworkDrive, NetworkProtocol};
use std::{io, process::Command, time::Duration};

const MOUNT_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ConnectSummary {
    pub connected: usize,
    pub skipped: usize,
    pub failed: usize,
    pub lines: Vec<String>,
}

/// Mount all configured `drives` that are not currently mounted and return a per-drive report.
pub fn run(drives: &[NetworkDrive]) -> io::Result<String> {
    format_report(run_with(drives, is_mounted, mount_one))
}

pub(super) fn run_with(
    drives: &[NetworkDrive],
    mut already_mounted: impl FnMut(&NetworkDrive) -> bool,
    mut mount: impl FnMut(&NetworkDrive) -> io::Result<()>,
) -> ConnectSummary {
    let mut summary = ConnectSummary::default();
    for drive in drives {
        let target = drive.path.as_str();
        let remote = drive.remote.as_str();
        if already_mounted(drive) {
            summary.skipped += 1;
            summary
                .lines
                .push(format!("SKIP  {target} -> {remote} (already mounted)"));
            continue;
        }
        match mount(drive) {
            Ok(()) => {
                summary.connected += 1;
                summary.lines.push(format!("OK    {target} -> {remote}"));
            }
            Err(error) => {
                summary.failed += 1;
                summary
                    .lines
                    .push(format!("FAIL  {target} -> {remote}: {error}"));
            }
        }
    }
    summary
}

pub(super) fn format_report(summary: ConnectSummary) -> io::Result<String> {
    let header = format!(
        "Network drives: {} connected, {} already mounted, {} failed",
        summary.connected, summary.skipped, summary.failed
    );
    let report = std::iter::once(header)
        .chain(summary.lines)
        .collect::<Vec<_>>()
        .join("\n");
    if summary.failed == 0 {
        Ok(report)
    } else {
        Err(io::Error::other(report))
    }
}

pub(super) fn is_mounted(drive: &NetworkDrive) -> bool {
    #[cfg(windows)]
    {
        std::path::Path::new(drive.path.as_str()).exists()
    }
    #[cfg(target_os = "linux")]
    {
        super::discover::read().is_ok_and(|list| list.iter().any(|item| item.path == drive.path))
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = drive;
        false
    }
}

pub(super) fn mount_one(drive: &NetworkDrive) -> io::Result<()> {
    mount_one_with(drive, run_command)
}

pub(super) fn mount_one_with(
    drive: &NetworkDrive,
    run: impl FnOnce(&mut Command) -> io::Result<()>,
) -> io::Result<()> {
    let mut command = mount_command(drive)?;
    run(&mut command)
}

pub(super) fn run_command(command: &mut Command) -> io::Result<()> {
    let output = crate::ops::menu::process::capture_with_timeout(
        command,
        MOUNT_TIMEOUT,
        &std::env::temp_dir(),
    )?;
    if output.status.success() {
        return Ok(());
    }
    let detail = output_detail(&output.stderr)
        .or_else(|| output_detail(&output.stdout))
        .unwrap_or_else(|| "mount command failed".to_owned());
    Err(io::Error::other(detail))
}

fn output_detail(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(3)
        .collect();
    (!lines.is_empty()).then(|| lines.join("; "))
}

#[cfg(windows)]
pub(super) fn mount_command(drive: &NetworkDrive) -> io::Result<Command> {
    let drive_spec = windows_drive_spec(drive.path.as_str())?;
    let remote = drive.remote.as_str();
    match drive.protocol {
        NetworkProtocol::Smb => {
            let unc = remote.replace('/', "\\");
            let mut cmd = Command::new("net.exe");
            cmd.args(["use", &drive_spec, &unc, "/persistent:yes"]);
            Ok(cmd)
        }
        NetworkProtocol::WebDav => {
            let mut cmd = Command::new("net.exe");
            cmd.args(["use", &drive_spec, remote, "/persistent:yes"]);
            Ok(cmd)
        }
        NetworkProtocol::Nfs => {
            let mut cmd = Command::new("mount.exe");
            cmd.args([remote, &drive_spec]);
            Ok(cmd)
        }
        NetworkProtocol::Sshfs => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "SSHFS mounting is not supported on Windows",
        )),
    }
}

#[cfg(windows)]
fn windows_drive_spec(path: &str) -> io::Result<String> {
    let trimmed = path.trim_end_matches(['\\', '/']);
    let bytes = trimmed.as_bytes();
    if bytes.len() == 2
        && bytes.first().is_some_and(u8::is_ascii_alphabetic)
        && bytes.get(1) == Some(&b':')
    {
        Ok(trimmed.to_ascii_uppercase())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Windows network drive path must be a drive letter like Z:\\ (got {path})"),
        ))
    }
}

#[cfg(not(windows))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "matches the fallible Windows mount_command signature"
)]
pub(super) fn mount_command(drive: &NetworkDrive) -> io::Result<Command> {
    let path = drive.path.as_str();
    let remote = drive.remote.as_str();
    match drive.protocol {
        NetworkProtocol::Smb => Ok(unix_mount("cifs", remote, path)),
        NetworkProtocol::Nfs => Ok(unix_mount("nfs", remote, path)),
        NetworkProtocol::WebDav => Ok(unix_mount("davfs", remote, path)),
        NetworkProtocol::Sshfs => {
            let mut sshfs = Command::new("sshfs");
            sshfs.args([remote, path]);
            Ok(sshfs)
        }
    }
}

#[cfg(not(windows))]
fn unix_mount(fs_type: &str, remote: &str, path: &str) -> Command {
    let mut cmd = Command::new("mount");
    cmd.args(["-t", fs_type, remote, path]);
    cmd
}

#[cfg(test)]
mod tests;
