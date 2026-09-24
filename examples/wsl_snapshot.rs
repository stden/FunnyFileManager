//! Export the real WSL manager with deterministic distribution and panel state.
//! Run: `cargo run --example wsl_snapshot -- docs/screenshots/wsl-manager.svg`
//! Optional second argument: `ready` (default), `busy`, or `failed`.

use std::{io, path::PathBuf};

use funny_file_manager::{
    models::{
        App, DirEntry, DistributionName, EntryKind, FileSize, Modified, Overlay, Panel,
        WslActivity, WslDialog, WslOperation, WslRequestId,
    },
    ui,
};
use ratatui::{backend::TestBackend, Terminal};

#[path = "support/snapshot.rs"]
mod snapshot;

fn entry(name: &str, kind: EntryKind) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    hide_process_identity();
    let mut arguments = std::env::args_os().skip(1);
    let output = arguments.next().map_or_else(
        || PathBuf::from("docs/screenshots/wsl-manager.svg"),
        PathBuf::from,
    );
    let state = arguments.next().unwrap_or_else(|| "ready".into());
    let dialog = fixture_dialog(&state)?;

    let mut app = fixture_app()?;
    app.set_overlay(Overlay::Wsl(Box::new(dialog)));
    let mut terminal = Terminal::new(TestBackend::new(100, 16)).expect("test backend");
    terminal
        .draw(|frame| ui::ui(frame, &mut app))
        .expect("render WSL manager");
    snapshot::save(
        terminal.backend().buffer(),
        "FFM WSL manager: distributions, platform and package updates",
        &output,
    )
}

fn hide_process_identity() {
    for name in ["USER", "LOGNAME", "USERNAME"] {
        std::env::set_var(name, "");
    }
}

fn fixture_dialog(state: &std::ffi::OsStr) -> Result<WslDialog, Box<dyn std::error::Error>> {
    let names = ["Ubuntu-24.04", "Debian", "Alpine"]
        .into_iter()
        .map(|name| DistributionName::new(name).ok_or("invalid fixture distribution"))
        .collect::<Result<_, _>>()?;
    let mut dialog = WslDialog::new(names);
    dialog.move_down();
    let selected = dialog
        .selected_distribution()
        .ok_or("no fixture distribution is selected")?
        .clone();
    dialog.activity = match state.to_str() {
        Some("ready") => WslActivity::Ready,
        Some("busy") => WslActivity::Busy {
            id: WslRequestId::new(1),
            operation: WslOperation::UpdatePackages(selected),
        },
        Some("failed") => {
            WslActivity::Failed("Debian package update failed: network unavailable".to_owned())
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "state must be ready, busy or failed",
            )
            .into())
        }
    };

    Ok(dialog)
}

fn fixture_app() -> io::Result<App> {
    let mut app = App::new()?;
    app.left_panel = Panel::from_entries(
        PathBuf::from("C:\\Projects"),
        vec![
            entry("..", EntryKind::Directory),
            entry("FunnyFileManager", EntryKind::Directory),
            entry("workspace", EntryKind::Directory),
        ],
    );
    app.right_panel = Panel::from_entries(
        PathBuf::from("C:\\Downloads"),
        vec![entry("..", EntryKind::Directory)],
    );
    Ok(app)
}
