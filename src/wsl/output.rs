//! Purpose: What a finished `wsl.exe` run means: its exit status, the
//! distribution list it printed, or the message an update left.

use super::encoding::decode_text;
use super::process;
use super::WslCommand;
use crate::models::WslOutput;

pub(super) fn decode_output(
    command: WslCommand<'_>,
    captured: &process::Captured,
) -> Result<WslOutput, String> {
    match command {
        WslCommand::Refresh => decode_distributions(captured).map(WslOutput::Distributions),
        WslCommand::Update => Ok(WslOutput::Updated(completed_message(
            captured,
            "WSL update completed.",
        )?)),
        WslCommand::UpdatePackages(name) => Ok(WslOutput::PackagesUpdated(completed_message(
            captured,
            &format!("Package update completed: {}.", name.as_str()),
        )?)),
    }
}

pub(super) fn check_exit(
    command: WslCommand<'_>,
    captured: &process::Captured,
) -> Result<(), String> {
    if let process::Exit::Failed(code) = captured.exit {
        let details = output_message(captured)?;
        let operation = match command {
            WslCommand::Refresh => "WSL distribution refresh".to_owned(),
            WslCommand::Update => "WSL update".to_owned(),
            WslCommand::UpdatePackages(name) => format!("Package update for {}", name.as_str()),
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

fn completed_message(captured: &process::Captured, summary: &str) -> Result<String, String> {
    let message = output_message(captured)?;
    Ok(if message.is_empty() {
        summary.to_owned()
    } else {
        format!("{summary}\n{message}")
    })
}

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
