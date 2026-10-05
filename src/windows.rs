//! Purpose: Contains Windows-specific behavior and diagnostic helpers: the
//! system-message and performance reports, and re-exports of the console
//! path spelling and the job object that owns a process tree.

use crate::models::App;
use std::io;

#[cfg(windows)]
mod console;
#[cfg(windows)]
mod process_tree;
#[cfg(any(test, windows))]
mod report;

#[cfg(windows)]
pub(crate) use console::console_directory;
#[cfg(windows)]
pub(crate) use process_tree::process_tree_job;
#[cfg(any(test, windows))]
use report::{open_report_with, PERFORMANCE_TITLE, SYSTEM_MESSAGES_TITLE};
#[cfg(windows)]
use report::{run_powershell_report, PERFORMANCE_SCRIPT, SYSTEM_MESSAGES_SCRIPT};

pub fn open_windows_system_messages(app: &mut App) -> io::Result<()> {
    #[cfg(windows)]
    {
        open_report_with(app, SYSTEM_MESSAGES_TITLE, || {
            run_powershell_report(SYSTEM_MESSAGES_SCRIPT)
        })
    }

    #[cfg(not(windows))]
    {
        app.set_status(crate::status::Status::WindowsOnly { tool: "system" });
        Ok(())
    }
}

pub fn open_windows_performance_checker(app: &mut App) -> io::Result<()> {
    #[cfg(windows)]
    {
        open_report_with(app, PERFORMANCE_TITLE, || {
            run_powershell_report(PERFORMANCE_SCRIPT)
        })
    }

    #[cfg(not(windows))]
    {
        app.set_status(crate::status::Status::WindowsOnly {
            tool: "performance",
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_command_output;
#[cfg(test)]
#[cfg(windows)]
mod tests_process_tree;
