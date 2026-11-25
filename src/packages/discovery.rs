//! Purpose: Finding the platform's package manager on `PATH` — never running it.

use std::path::{Path, PathBuf};

use super::{Elevation, Manager, UpdatePlan};
use crate::prompt::Privilege;

#[cfg(windows)]
const PLATFORM_MANAGERS: &[Manager] = &[Manager::Chocolatey];
#[cfg(target_os = "macos")]
const PLATFORM_MANAGERS: &[Manager] = &[Manager::Brew];
#[cfg(not(any(windows, target_os = "macos")))]
const PLATFORM_MANAGERS: &[Manager] = &[Manager::Apt, Manager::Brew];

pub fn detect() -> Option<UpdatePlan> {
    let path = std::env::var_os("PATH")?;
    let directories: Vec<_> = std::env::split_paths(&path)
        .filter(|path| path.is_absolute())
        .collect();
    discover(PLATFORM_MANAGERS, &directories)
}

pub(super) fn discover(managers: &[Manager], directories: &[PathBuf]) -> Option<UpdatePlan> {
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
                elevation: Elevation::for_manager(manager, Privilege::current()),
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
        let mode = metadata.permissions().mode();
        metadata.is_file() && (mode & 0o111 != 0) && (mode & 0o002 == 0)
    }
    #[cfg(not(unix))]
    {
        metadata.is_file()
    }
}
