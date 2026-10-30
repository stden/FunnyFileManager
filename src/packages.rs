//! Typed update plans. Discovery never runs a package manager.
use crate::prompt::Privilege;
use std::{
    io,
    path::{Path, PathBuf},
    process::ExitStatus,
};

mod discovery;

pub(crate) use discovery::detect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Manager {
    Chocolatey,
    Apt,
    Brew,
}

/// Who the package manager runs as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elevation {
    /// As the user FFM runs as: root already, a manager that needs no root
    /// (Homebrew), or one that asks Windows for it itself (Chocolatey).
    CurrentUser,
    /// Through `sudo --`, which asks for the password on the terminal.
    Sudo,
}

#[derive(Debug, Clone)]
pub struct UpdatePlan {
    pub manager: Manager,
    pub executable: PathBuf,
    /// Decided when the plan is made, so the dialog can show the `sudo` it
    /// is about to run.
    pub elevation: Elevation,
}

/// How one step of a plan ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepOutcome {
    pub exit: StepExit,
    /// The last line the step wrote to stderr: sudo's or the manager's own
    /// words for what went wrong.
    pub reason: Option<String>,
}

/// How a step's process ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepExit {
    /// The process exited with this code.
    Code(i32),
    /// A signal ended the process, so it has no exit code. Not a code of its
    /// own: Chocolatey really exits -1 on an error.
    Signal,
}

impl From<ExitStatus> for StepExit {
    fn from(status: ExitStatus) -> Self {
        status.code().map_or(Self::Signal, Self::Code)
    }
}

/// `choco upgrade` exit codes (with enhanced exit codes) that are not errors.
const CHOCO_NOTHING_TO_DO: i32 = 2;
const CHOCO_REBOOT_INITIATED: i32 = 1641;
const CHOCO_REBOOT_REQUIRED: i32 = 3010;

/// sudo's own failures (no password, not in sudoers, no terminal) exit 1,
/// while apt-get reports its errors as 100 — so a 1 through sudo is sudo's.
const SUDO_REFUSED: i32 = 1;

impl Manager {
    /// The commands to run, in order.
    ///
    /// Everything that prompts is answered up front: the menu suspends the TUI
    /// and hands the terminal to the package manager, so a confirmation prompt
    /// would sit there with nobody to type into it. `apt-get` and `choco` both
    /// take `-y`; `brew` has no such flag and never asks.
    pub const fn commands(self) -> &'static [&'static [&'static str]] {
        match self {
            Self::Chocolatey => &[&["upgrade", "all", "-y"]],
            Self::Apt => &[&["update"], &["upgrade", "-y"]],
            Self::Brew => &[&["update"], &["upgrade"]],
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Chocolatey => "choco",
            Self::Apt => "apt-get",
            Self::Brew => "brew",
        }
    }
}

impl Elevation {
    /// apt-get needs root and does not ask for it; the other managers either
    /// need no root or ask for it themselves.
    #[cfg(unix)]
    const fn for_manager(manager: Manager, privilege: Privilege) -> Self {
        match (manager, privilege) {
            (Manager::Apt, Privilege::User) => Self::Sudo,
            _ => Self::CurrentUser,
        }
    }

    /// Windows has no `sudo`; Chocolatey asks for elevation itself.
    #[cfg(not(unix))]
    const fn for_manager(_: Manager, _: Privilege) -> Self {
        Self::CurrentUser
    }

    const fn prefix(self) -> &'static str {
        match self {
            Self::CurrentUser => "",
            Self::Sudo => "sudo ",
        }
    }
}

impl UpdatePlan {
    pub fn description(&self) -> String {
        self.manager
            .commands()
            .iter()
            .map(|args| self.command_line(args))
            .collect::<Vec<_>>()
            .join(" && ")
    }

    /// One step as it runs: `sudo apt-get update`.
    pub(crate) fn command_line(&self, args: &[&str]) -> String {
        format!("{}{}", self.elevation.prefix(), self.step(args))
    }

    /// One step without the wrapper: `apt-get update`.
    fn step(&self, args: &[&str]) -> String {
        format!("{} {}", self.manager.label(), args.join(" "))
    }

    pub(crate) fn execute_with(
        &self,
        mut run: impl FnMut(&Path, &[&str]) -> io::Result<StepOutcome>,
    ) -> io::Result<String> {
        for args in self.manager.commands() {
            let outcome = run(&self.executable, args)?;
            match (self.manager, outcome.exit) {
                (
                    Manager::Chocolatey,
                    StepExit::Code(CHOCO_REBOOT_INITIATED | CHOCO_REBOOT_REQUIRED),
                ) => return Ok("Packages updated; restart required".to_owned()),
                (_, StepExit::Code(0))
                | (Manager::Chocolatey, StepExit::Code(CHOCO_NOTHING_TO_DO)) => {}
                _ => return Err(io::Error::other(self.failure(args, &outcome))),
            }
        }
        Ok("Package update completed".to_owned())
    }

    /// What a failed step reads as: sudo's refusal named as sudo's, not
    /// blamed on the package manager it never started, and the reason the
    /// step printed after it.
    fn failure(&self, args: &[&str], outcome: &StepOutcome) -> String {
        let step = self.step(args);
        let headline = match outcome.exit {
            StepExit::Code(SUDO_REFUSED) if self.elevation == Elevation::Sudo => {
                format!("sudo did not run {step} (exit code {SUDO_REFUSED})")
            }
            StepExit::Code(code) => format!("{step} exited with code {code}"),
            StepExit::Signal => format!("{step} was stopped by a signal"),
        };
        match &outcome.reason {
            Some(reason) => format!("{headline}: {reason}"),
            None => headline,
        }
    }
}

#[cfg(test)]
mod tests;
