//! Purpose: Save setup (`Shift+F9`) — write the settings to disk and say
//! where they went.
//!
//! Far saves on demand: the Options pages edit the settings in memory, and
//! this row is what makes them outlive the session (or `auto_save_setup`, at
//! exit). The status names the path so a user can find what was written.

use crate::config::Settings;
use crate::models::App;
use crate::status::Status;

pub fn run(app: &mut App) {
    let report = match (app.config.save(), Settings::path()) {
        (Ok(()), Some(path)) => Status::CommandOutput(format!("Setup saved to {}", path.display())),
        (Ok(()), None) => Status::CommandOutput("Setup saved".to_owned()),
        (Err(error), _) => Status::Error(format!("Save setup: {error}")),
    };
    app.set_status(report);
}

#[cfg(test)]
mod tests;
