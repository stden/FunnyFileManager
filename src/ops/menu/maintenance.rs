//! Confirm volume or repository maintenance against the active local file panel.

use crate::models::{
    App, ChoiceTarget, MaintenanceCommand, MenuChoice, PanelKind, VolumeOperation, VolumeTarget,
};
use std::io;

fn panel_path(app: &App) -> io::Result<&std::path::Path> {
    let panel = app.get_active_panel_ref();
    if !matches!(panel.kind, PanelKind::Files) {
        return Err(io::Error::other("Select a local file panel"));
    }
    Ok(&panel.path)
}

pub(super) fn volume(app: &mut App) -> io::Result<()> {
    let target = crate::ops::volume_maintenance::resolve(panel_path(app)?)?;
    show(app, &target);
    Ok(())
}

fn show(app: &mut App, target: &VolumeTarget) {
    let mut rows = vec![cancel()];
    let mount = crate::path_display::label(&target.mount);
    rows.extend(
        [VolumeOperation::Trim, VolumeOperation::Defragment].map(|op| MenuChoice {
            label: format!("{} {mount}", op.label()),
            target: ChoiceTarget::RunMaintenance(MaintenanceCommand::Volume(op, target.clone())),
        }),
    );
    super::choices(app, "Optimize volume (administrator required)", rows);
}

pub(super) fn git_sync(app: &mut App) -> io::Result<()> {
    let target = crate::ops::git_sync::inspect(panel_path(app)?)?;
    let label = if target.branch == "main" {
        "Fast-forward main to origin/main".to_owned()
    } else {
        format!(
            "Pull {}, update main and rebase onto origin/main",
            target.branch
        )
    };
    let title = format!(
        "Update Git branch in {}",
        crate::path_display::label(&target.root)
    );
    let run = MenuChoice {
        label,
        target: ChoiceTarget::RunMaintenance(MaintenanceCommand::SyncBranch(target)),
    };
    super::choices(app, &title, vec![cancel(), run]);
    Ok(())
}

fn cancel() -> MenuChoice {
    MenuChoice {
        label: "Cancel".to_owned(),
        target: ChoiceTarget::Close,
    }
}

#[cfg(test)]
mod tests;
