//! Purpose: Host mount discovery reads local OS metadata, without contacting file servers.
use crate::models::network_drive::{MountPoint, NetworkAddress, NetworkDrive, NetworkProtocol};
use std::io;

#[cfg(windows)]
pub(super) fn read() -> io::Result<Vec<NetworkDrive>> {
    use std::{process::Command, time::Duration};
    let mut command = Command::new("pwsh.exe");
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        SCRIPT,
    ]);
    let output = crate::ops::menu::process::capture_with_timeout(
        &mut command,
        Duration::from_secs(10),
        &std::env::temp_dir(),
    )?;
    if !output.status.success() {
        return Err(io::Error::other("Windows network drive discovery failed"));
    }
    let pairs: Vec<WindowsMount> =
        serde_json::from_slice(&output.stdout).map_err(io::Error::other)?;
    pairs
        .into_iter()
        .map(|pair| mount(pair.path, pair.remote, NetworkProtocol::Smb))
        .collect()
}

#[cfg(windows)]
#[derive(serde::Deserialize)]
struct WindowsMount {
    path: String,
    remote: String,
}

#[cfg(windows)]
const SCRIPT: &str = r"
$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$result=@{}
Get-CimInstance Win32_LogicalDisk -Filter 'DriveType=4' | ForEach-Object {
    $result[$_.DeviceID]=@{path=$_.DeviceID+'\';remote=$_.ProviderName}
}
if(Test-Path 'HKCU:\Network') {
    Get-ChildItem 'HKCU:\Network' | ForEach-Object {
        $entry=Get-ItemProperty -LiteralPath $_.PSPath
        if($entry.RemotePath) {
            $result[$_.PSChildName+':']=@{path=$_.PSChildName+':\';remote=$entry.RemotePath}
        }
    }
}
ConvertTo-Json -InputObject @($result.Values) -Compress
";

#[cfg(target_os = "linux")]
pub(super) fn read() -> io::Result<Vec<NetworkDrive>> {
    parse_mounts(&std::fs::read_to_string("/proc/self/mounts")?)
}
#[cfg(not(any(windows, target_os = "linux")))]
pub(super) fn read() -> io::Result<Vec<NetworkDrive>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "network drive discovery is supported on Windows and Linux",
    ))
}

#[cfg(any(target_os = "linux", test))]
pub(super) fn parse_mounts(text: &str) -> io::Result<Vec<NetworkDrive>> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let source = fields.next()?;
            let path = fields.next()?;
            let protocol = match fields.next()? {
                "cifs" | "smb3" => NetworkProtocol::Smb,
                "nfs" | "nfs4" => NetworkProtocol::Nfs,
                "fuse.sshfs" => NetworkProtocol::Sshfs,
                "davfs" | "fuse.davfs" => NetworkProtocol::WebDav,
                _ => return None,
            };
            Some(mount(unescape(path), unescape(source), protocol))
        })
        .collect()
}
#[cfg(any(target_os = "linux", test))]
fn unescape(text: &str) -> String {
    text.replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\012", "\n")
        .replace("\\134", "\\")
}
fn mount(path: String, remote: String, protocol: NetworkProtocol) -> io::Result<NetworkDrive> {
    Ok(NetworkDrive {
        path: MountPoint::try_from(path).map_err(io::Error::other)?,
        remote: NetworkAddress::try_from(remote).map_err(io::Error::other)?,
        protocol,
    })
}
