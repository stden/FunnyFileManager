//! Run the confirmed maintenance command off the UI thread and retain its report in Ctrl+O.

use super::Redraw;
use crate::models::{App, MaintenanceCommand};
use crate::status::Status;
use std::{io, thread};

#[derive(Default)]
pub(super) struct MaintenanceJobs {
    pending: Option<(String, thread::JoinHandle<io::Result<String>>)>,
}

impl MaintenanceJobs {
    pub(super) fn pump(&mut self, app: &mut App) -> Redraw {
        self.pump_with(app, |command| match command {
            MaintenanceCommand::Volume(op, volume) => {
                crate::ops::volume_maintenance::run(*op, volume)
            }
            MaintenanceCommand::SyncBranch(target) => crate::ops::git_sync::run(target),
        })
    }

    fn pump_with(
        &mut self,
        app: &mut App,
        run: impl FnOnce(&MaintenanceCommand) -> io::Result<String> + Send + 'static,
    ) -> Redraw {
        let redraw = self.collect(app);
        let Some(command) = app.maintenance_request.take() else {
            return redraw;
        };
        if let Some((label, _)) = &self.pending {
            app.set_status(Status::Error(format!("{label} is already running")));
            return Redraw::Needed;
        }
        let label = command.label();
        match thread::Builder::new()
            .name("maintenance".into())
            .spawn(move || run(&command))
        {
            Ok(handle) => {
                let message = format!("{label} started");
                app.console.record_output(&message);
                app.set_status(Status::CommandOutput(message));
                self.pending = Some((label, handle));
            }
            Err(error) => app.set_status(Status::Error(format!("{label}: {error}"))),
        }
        Redraw::Needed
    }

    fn collect(&mut self, app: &mut App) -> Redraw {
        let Some((label, job)) = self.pending.take_if(|(_, job)| job.is_finished()) else {
            return Redraw::Unchanged;
        };
        let result = job
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
        app.set_status(status);
        Redraw::Needed
    }
}

#[cfg(test)]
mod tests;
