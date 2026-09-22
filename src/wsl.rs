//! Windows Subsystem for Linux adapter. Operations and state live in their own layers.

use crate::models::{WslOperation, WslOutput};

#[cfg(any(windows, test))]
mod process;

/// Executes WSL effects without coupling application operations to a host process.
pub trait WslBackend: Send + Sync {
    fn execute(&self, operation: &WslOperation) -> Result<WslOutput, String>;
}

/// Uses the Windows installation's `wsl.exe`, without shell or PATH lookup.
#[derive(Debug, Default)]
pub struct SystemWslBackend;

impl WslBackend for SystemWslBackend {
    fn execute(&self, operation: &WslOperation) -> Result<WslOutput, String> {
        #[cfg(windows)]
        {
            execute_with(operation, process::run)
        }
        #[cfg(not(windows))]
        {
            let _ = operation;
            Err("WSL is only available on Windows".to_owned())
        }
    }
}

// Distribution names are passed as a separate argument, never inserted into this script.
// These are ordinary installed-package upgrades, not distribution-release upgrades.
#[cfg(any(windows, test))]
const PACKAGE_UPDATE_SCRIPT: &str = r"set -eu
if command -v apt-get >/dev/null 2>&1; then
    export DEBIAN_FRONTEND=noninteractive
    apt-get update
    apt-get upgrade -y
elif command -v dnf >/dev/null 2>&1; then
    dnf upgrade -y
elif command -v yum >/dev/null 2>&1; then
    yum update -y
elif command -v zypper >/dev/null 2>&1; then
    zypper --non-interactive update
elif command -v pacman >/dev/null 2>&1; then
    pacman -Syu --noconfirm
elif command -v apk >/dev/null 2>&1; then
    apk update
    apk upgrade
else
    printf '%s\n' 'No supported package manager found (apt-get, dnf, yum, zypper, pacman, apk).' >&2
    exit 127
fi
";

#[cfg(any(windows, test))]
fn execute_with(
    operation: &WslOperation,
    runner: impl FnOnce(&[&str], std::time::Duration) -> Result<process::Captured, String>,
) -> Result<WslOutput, String> {
    use std::time::Duration;

    let captured = match operation {
        WslOperation::Open(distribution) => return Ok(WslOutput::Root(distribution.root_path())),
        WslOperation::Refresh => runner(&["--list", "--quiet"], Duration::from_secs(20))?,
        WslOperation::Update => runner(&["--update"], Duration::from_mins(5))?,
        WslOperation::UpdatePackages(distribution) => runner(
            &[
                "--distribution",
                distribution.as_str(),
                "--user",
                "root",
                "--exec",
                "sh",
                "-lc",
                PACKAGE_UPDATE_SCRIPT,
            ],
            Duration::from_hours(1),
        )?,
    };
    check_exit(operation, &captured)?;
    decode_output(operation, &captured)
}

#[cfg(any(windows, test))]
fn decode_output(
    operation: &WslOperation,
    captured: &process::Captured,
) -> Result<WslOutput, String> {
    match operation {
        WslOperation::Refresh => decode_distributions(captured).map(WslOutput::Distributions),
        WslOperation::Update => Ok(WslOutput::Updated(completed_message(
            captured,
            "WSL update completed.",
        )?)),
        WslOperation::UpdatePackages(name) => Ok(WslOutput::PackagesUpdated(completed_message(
            captured,
            &format!("Package update completed: {}.", name.as_str()),
        )?)),
        WslOperation::Open(_) => unreachable!("opening a root does not run a command"),
    }
}

#[cfg(any(windows, test))]
fn check_exit(operation: &WslOperation, captured: &process::Captured) -> Result<(), String> {
    if let process::Exit::Failed(code) = captured.exit {
        let details = output_message(captured)?;
        let operation = match operation {
            WslOperation::Refresh => "WSL distribution refresh".to_owned(),
            WslOperation::Update => "WSL update".to_owned(),
            WslOperation::UpdatePackages(name) => format!("Package update for {}", name.as_str()),
            WslOperation::Open(_) => unreachable!("opening a root does not run a command"),
        };
        let code = code.map_or_else(
            || "unknown exit status".to_owned(),
            |code| format!("exit code {code}"),
        );
        let summary = format!("{operation} failed ({code}).");
        return Err(if details.is_empty() {
            summary
        } else {
            format!("{summary}\n{details}")
        });
    }
    Ok(())
}

#[cfg(any(windows, test))]
fn decode_distributions(
    captured: &process::Captured,
) -> Result<Vec<crate::models::DistributionName>, String> {
    use crate::models::DistributionName;
    if captured.stdout.truncated {
        return Err("WSL distribution list exceeded the output limit".to_owned());
    }
    let text = decode_text(&captured.stdout.bytes, false)?;
    let mut distributions = Vec::new();
    for name in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let distribution = DistributionName::new(name)
            .ok_or_else(|| "WSL returned an invalid distribution name".to_owned())?;
        if !distributions.contains(&distribution) {
            distributions.push(distribution);
        }
    }
    Ok(distributions)
}

#[cfg(any(windows, test))]
fn completed_message(captured: &process::Captured, summary: &str) -> Result<String, String> {
    let message = output_message(captured)?;
    Ok(if message.is_empty() {
        summary.to_owned()
    } else {
        format!("{summary}\n{message}")
    })
}

#[cfg(any(windows, test))]
fn output_message(captured: &process::Captured) -> Result<String, String> {
    let mut messages = Vec::new();
    for stream in [&captured.stderr, &captured.stdout] {
        let text = decode_text(&stream.bytes, stream.truncated)?;
        let text = text.trim();
        if !text.is_empty() {
            messages.push(text.to_owned());
        }
    }
    if captured.stdout.truncated || captured.stderr.truncated {
        messages.push("[output truncated; command continued to completion]".to_owned());
    }
    Ok(messages.join("\n"))
}

/// WSL versions emit UTF-16LE (with or without a BOM) or UTF-8 on redirected pipes.
#[cfg(any(windows, test))]
fn decode_text(bytes: &[u8], truncated: bool) -> Result<String, String> {
    let invalid = || "WSL returned invalid text encoding".to_owned();
    let (bytes, encoding) = if let Some(bytes) = bytes.strip_prefix(&[0xff, 0xfe]) {
        (bytes, Some(u16::from_le_bytes as fn([u8; 2]) -> u16))
    } else if let Some(bytes) = bytes.strip_prefix(&[0xfe, 0xff]) {
        (bytes, Some(u16::from_be_bytes as fn([u8; 2]) -> u16))
    } else if bytes.as_chunks::<2>().0.iter().any(|pair| pair[1] == 0) {
        (bytes, Some(u16::from_le_bytes as fn([u8; 2]) -> u16))
    } else {
        (
            bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes),
            None,
        )
    };
    let text = if let Some(decode) = encoding {
        decode_utf16(bytes, truncated, decode)?
    } else if truncated {
        String::from_utf8_lossy(bytes).into_owned()
    } else {
        String::from_utf8(bytes.to_vec()).map_err(|_| invalid())?
    };
    Ok(text.trim_matches('\0').to_owned())
}

#[cfg(any(windows, test))]
fn decode_utf16(
    bytes: &[u8],
    truncated: bool,
    decode: fn([u8; 2]) -> u16,
) -> Result<String, String> {
    if !bytes.len().is_multiple_of(2) && !truncated {
        return Err("WSL returned invalid text encoding".to_owned());
    }
    let words: Vec<_> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .copied()
        .map(decode)
        .collect();
    if truncated {
        Ok(String::from_utf16_lossy(&words))
    } else {
        String::from_utf16(&words).map_err(|_| "WSL returned invalid text encoding".to_owned())
    }
}

#[cfg(test)]
mod tests;
