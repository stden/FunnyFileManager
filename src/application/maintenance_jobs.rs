//! Run the confirmed maintenance command off the UI thread and retain its report in Ctrl+O.

use super::Redraw;
use crate::models::{App, MaintenanceCommand, PanelKind};
use crate::status::Status;
use std::{io, thread};

struct PendingJob {
    label: String,
    refresh_panels: bool,
    handle: thread::JoinHandle<io::Result<String>>,
}

#[derive(Default)]
pub(super) struct MaintenanceJobs {
    pending: Option<PendingJob>,
}

impl MaintenanceJobs {
    pub(super) fn pump(&mut self, app: &mut App) -> Redraw {
        self.pump_with(app, |command| match command {
            MaintenanceCommand::Volume(op, volume) => {
                crate::ops::volume_maintenance::run(*op, volume)
            }
            MaintenanceCommand::SyncBranch(target) => crate::ops::git_sync::run(target),
            MaintenanceCommand::ConnectNetworkDrives(drives) => {
                crate::ops::network_drives::connect::run(drives)
            }
        })
    }

    fn pump_with(
        &mut self,
        app: &mut App,
        run: impl FnOnce(&MaintenanceCommand) -> io::Result<String> + Send + 'static,
    ) -> Redraw {
        self.pump_with_builder(app, thread::Builder::new().name("maintenance".into()), run)
    }

    fn pump_with_builder(
        &mut self,
        app: &mut App,
        builder: thread::Builder,
        run: impl FnOnce(&MaintenanceCommand) -> io::Result<String> + Send + 'static,
    ) -> Redraw {
        let redraw = self.collect(app);
        let Some(command) = app.maintenance_request.take() else {
            return redraw;
        };
        if let Some(pending) = &self.pending {
            app.set_status(Status::Error(format!(
                "{} is already running",
                pending.label
            )));
            return Redraw::Needed;
        }
        self.spawn_job(app, command, builder, run);
        Redraw::Needed
    }

    fn spawn_job(
        &mut self,
        app: &mut App,
        command: MaintenanceCommand,
        builder: thread::Builder,
        run: impl FnOnce(&MaintenanceCommand) -> io::Result<String> + Send + 'static,
    ) {
        let label = command.label();
        let refresh_panels = matches!(command, MaintenanceCommand::ConnectNetworkDrives(_));
        match builder.spawn(move || run(&command)) {
            Ok(handle) => {
                let message = format!("{label} started");
                app.console.record_output(&message);
                app.set_status(Status::CommandOutput(message));
                self.pending = Some(PendingJob {
                    label,
                    refresh_panels,
                    handle,
                });
            }
            Err(error) => app.set_status(Status::Error(format!("{label}: {error}"))),
        }
    }

    fn collect(&mut self, app: &mut App) -> Redraw {
        let Some(job) = self.pending.take_if(|job| job.handle.is_finished()) else {
            return Redraw::Unchanged;
        };
        let label = job.label;
        let result = job
            .handle
            .join()
            .unwrap_or_else(|_| Err(io::Error::other("maintenance worker stopped")));
        let status = match result {
            Ok(report) => {
                app.console.record_output(&report);
                Status::CommandOutput(format!("{label} completed"))
            }
            Err(error) => {
                let message = format!("{label}: {error}");
                app.console.record_error(&message);
                Status::Error(message)
            }
        };
        if job.refresh_panels {
            reload_local_panels(app);
        }
        app.set_status(status);
        Redraw::Needed
    }
}

fn reload_local_panels(app: &mut App) {
    for panel in [&mut app.left_panel, &mut app.right_panel] {
        if panel.kind == PanelKind::Files && panel.path.is_dir() {
            drop(panel.reload());
        }
    }
}

#[cfg(test)]
mod tests;
