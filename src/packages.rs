//! Typed update plans. Discovery never runs a package manager.
use std::{
    io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Manager {
    Chocolatey,
    Apt,
    Brew,
}

#[derive(Debug, Clone)]
pub struct UpdatePlan {
    pub manager: Manager,
    pub executable: PathBuf,
}

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

impl UpdatePlan {
    pub fn description(&self) -> String {
        self.manager
            .commands()
            .iter()
            .map(|args| format!("{} {}", self.manager.label(), args.join(" ")))
            .collect::<Vec<_>>()
            .join(" && ")
    }

    pub(crate) fn execute_with(
        &self,
        mut run: impl FnMut(&Path, &[&str]) -> io::Result<i32>,
    ) -> io::Result<String> {
        for args in self.manager.commands() {
            let code = run(&self.executable, args)?;
            if self.manager == Manager::Chocolatey && matches!(code, 1641 | 3010) {
                return Ok("Packages updated; restart required".to_owned());
            }
            if code != 0 && !(self.manager == Manager::Chocolatey && code == 2) {
                return Err(io::Error::other(format!(
                    "{} {} exited with code {code}",
                    self.manager.label(),
                    args.join(" ")
                )));
            }
        }
        Ok("Package update completed".to_owned())
    }
}

pub(crate) fn detect() -> Option<UpdatePlan> {
    let path = std::env::var_os("PATH")?;
    let directories: Vec<_> = std::env::split_paths(&path)
        .filter(|path| path.is_absolute())
        .collect();
    let managers: &[Manager] = if cfg!(windows) {
        &[Manager::Chocolatey]
    } else if cfg!(target_os = "macos") {
        &[Manager::Brew]
    } else {
        &[Manager::Apt, Manager::Brew]
    };
    discover(managers, &directories)
}

fn discover(managers: &[Manager], directories: &[PathBuf]) -> Option<UpdatePlan> {
    managers.iter().find_map(|&manager| {
        let name = if manager == Manager::Chocolatey {
            "choco.exe"
        } else {
            manager.label()
        };
        directories
            .iter()
            .map(|dir| dir.join(name))
            .find(|path| executable(path))
            .map(|executable| UpdatePlan {
                manager,
                executable,
            })
    })
}

fn executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        metadata.is_file()
    }
}

#[cfg(test)]
mod tests;
